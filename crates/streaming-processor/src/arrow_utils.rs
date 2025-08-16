//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.com>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Arrow serialization utilities for streaming processor
//! 
//! This module provides utilities for serializing and deserializing
//! telemetry data using Apache Arrow IPC format.

use bridge_core::{BridgeResult, TelemetryBatch, types::{TelemetryRecord, TelemetryData, TelemetryType}};
use arrow_array::{
    StringArray, Int64Array, Float64Array, TimestampNanosecondArray,
    RecordBatch, ArrayRef, Array,
};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use arrow_ipc::{reader::StreamReader, writer::StreamWriter};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Arrow schema for telemetry records
pub fn telemetry_schema() -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("timestamp", DataType::Timestamp(TimeUnit::Nanosecond, None), false),
        Field::new("record_type", DataType::Utf8, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("batch_id", DataType::Utf8, false),
        Field::new("metric_name", DataType::Utf8, true),
        Field::new("metric_value", DataType::Float64, true),
        Field::new("metric_unit", DataType::Utf8, true),
        Field::new("log_message", DataType::Utf8, true),
        Field::new("log_level", DataType::Utf8, true),
        Field::new("trace_duration_ns", DataType::Int64, true),
        Field::new("event_name", DataType::Utf8, true),
        Field::new("attributes", DataType::Utf8, true), // JSON string
        Field::new("tags", DataType::Utf8, true), // JSON string
    ])
}

/// Convert TelemetryBatch to Arrow RecordBatch
pub fn telemetry_batch_to_record_batch(batch: &TelemetryBatch) -> BridgeResult<RecordBatch> {
    let schema = telemetry_schema();
    let mut ids = Vec::new();
    let mut timestamps = Vec::new();
    let mut record_types = Vec::new();
    let mut sources = Vec::new();
    let mut batch_ids = Vec::new();
    let mut metric_names = Vec::new();
    let mut metric_values = Vec::new();
    let mut metric_units = Vec::new();
    let mut log_messages = Vec::new();
    let mut log_levels = Vec::new();
    let mut trace_durations = Vec::new();
    let mut event_names = Vec::new();
    let mut attributes_list = Vec::new();
    let mut tags_list = Vec::new();

    for record in &batch.records {
        ids.push(record.id.to_string());
        timestamps.push(record.timestamp.timestamp_nanos_opt().unwrap_or(0));
        record_types.push(format!("{:?}", record.record_type));
        sources.push(batch.source.clone());
        batch_ids.push(batch.id.to_string());

        // Initialize optional fields
        let mut metric_name = None;
        let mut metric_value = None;
        let mut metric_unit = None;
        let mut log_message = None;
        let mut log_level = None;
        let mut trace_duration = None;
        let mut event_name = None;

        // Extract data based on record type
        match &record.data {
            TelemetryData::Metric(metric) => {
                metric_name = Some(metric.name.clone());
                metric_unit = metric.unit.clone();
                
                match &metric.value {
                    bridge_core::types::MetricValue::Counter(v) => metric_value = Some(*v),
                    bridge_core::types::MetricValue::Gauge(v) => metric_value = Some(*v),
                    bridge_core::types::MetricValue::Histogram { sum, .. } => metric_value = Some(*sum),
                    bridge_core::types::MetricValue::Summary { sum, .. } => metric_value = Some(*sum),
                }
            }
            TelemetryData::Log(log) => {
                log_message = Some(log.message.clone());
                log_level = Some(format!("{:?}", log.level));
            }
            TelemetryData::Trace(trace) => {
                trace_duration = trace.duration_ns;
            }
            TelemetryData::Event(event) => {
                event_name = Some(event.name.clone());
            }
        }

        metric_names.push(metric_name);
        metric_values.push(metric_value);
        metric_units.push(metric_unit);
        log_messages.push(log_message);
        log_levels.push(log_level);
        trace_durations.push(trace_duration);
        event_names.push(event_name);

        // Serialize attributes and tags as JSON strings
        let attributes_json = serde_json::to_string(&record.attributes)
            .unwrap_or_else(|_| "{}".to_string());
        let tags_json = serde_json::to_string(&record.tags)
            .unwrap_or_else(|_| "{}".to_string());
        
        attributes_list.push(attributes_json);
        tags_list.push(tags_json);
    }

    let arrays: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(ids)),
        Arc::new(TimestampNanosecondArray::from(timestamps)),
        Arc::new(StringArray::from(record_types)),
        Arc::new(StringArray::from(sources)),
        Arc::new(StringArray::from(batch_ids)),
        Arc::new(StringArray::from(metric_names)),
        Arc::new(Float64Array::from(metric_values)),
        Arc::new(StringArray::from(metric_units)),
        Arc::new(StringArray::from(log_messages)),
        Arc::new(StringArray::from(log_levels)),
        Arc::new(Int64Array::from(trace_durations.into_iter().map(|x| x.map(|v| v as i64)).collect::<Vec<_>>())),
        Arc::new(StringArray::from(event_names)),
        Arc::new(StringArray::from(attributes_list)),
        Arc::new(StringArray::from(tags_list)),
    ];

    RecordBatch::try_new(Arc::new(schema), arrays)
        .map_err(|e| bridge_core::BridgeError::serialization(format!(
            "Failed to create Arrow RecordBatch: {}", e
        )))
}

/// Convert Arrow RecordBatch to TelemetryBatch
pub fn record_batch_to_telemetry_batch(batch: &RecordBatch) -> BridgeResult<TelemetryBatch> {
    let _schema = batch.schema();
    let mut records = Vec::new();
    
    // Extract batch-level information from the first row
    let source = if batch.num_rows() > 0 {
        get_string_value(batch, "source", 0)?
    } else {
        "unknown".to_string()
    };
    
    let batch_id = if batch.num_rows() > 0 {
        let batch_id_str = get_string_value(batch, "batch_id", 0)?;
        Uuid::parse_str(&batch_id_str)
            .map_err(|e| bridge_core::BridgeError::serialization(format!(
                "Invalid batch UUID: {}", e
            )))?
    } else {
        Uuid::new_v4()
    };

    for row_idx in 0..batch.num_rows() {
        // Extract required fields
        let id_str = get_string_value(batch, "id", row_idx)?;
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| bridge_core::BridgeError::serialization(format!(
                "Invalid UUID in row {}: {}", row_idx, e
            )))?;

        let timestamp_ns = get_timestamp_value(batch, "timestamp", row_idx)?;
        let timestamp = DateTime::from_timestamp_nanos(timestamp_ns);

        let record_type_str = get_string_value(batch, "record_type", row_idx)?;
        let record_type = parse_record_type(&record_type_str)?;

        // Extract optional fields
        let metric_name = get_optional_string_value(batch, "metric_name", row_idx);
        let metric_value = get_optional_float_value(batch, "metric_value", row_idx);
        let metric_unit = get_optional_string_value(batch, "metric_unit", row_idx);
        let log_message = get_optional_string_value(batch, "log_message", row_idx);
        let log_level = get_optional_string_value(batch, "log_level", row_idx);
        let trace_duration = get_optional_int_value(batch, "trace_duration_ns", row_idx);
        let event_name = get_optional_string_value(batch, "event_name", row_idx);

        // Parse attributes and tags
        let attributes_json = get_string_value(batch, "attributes", row_idx)?;
        let tags_json = get_string_value(batch, "tags", row_idx)?;
        
        let attributes: HashMap<String, String> = serde_json::from_str(&attributes_json)
            .unwrap_or_else(|_| HashMap::new());
        let tags: HashMap<String, String> = serde_json::from_str(&tags_json)
            .unwrap_or_else(|_| HashMap::new());

        // Reconstruct telemetry data based on record type
        let data = match record_type {
            TelemetryType::Metric => {
                if let (Some(name), Some(value)) = (metric_name, metric_value) {
                    TelemetryData::Metric(bridge_core::types::MetricData {
                        name,
                        description: None,
                        unit: metric_unit,
                        metric_type: bridge_core::types::MetricType::Gauge,
                        value: bridge_core::types::MetricValue::Gauge(value),
                        labels: HashMap::new(),
                        timestamp,
                    })
                } else {
                    return Err(bridge_core::BridgeError::serialization(format!(
                        "Missing metric name or value in row {}", row_idx
                    )));
                }
            }
            TelemetryType::Log => {
                if let Some(message) = log_message {
                    TelemetryData::Log(bridge_core::types::LogData {
                        timestamp,
                        level: parse_log_level(&log_level.unwrap_or_else(|| "Info".to_string())),
                        message,
                        attributes: HashMap::new(),
                        body: None,
                        severity_number: None,
                        severity_text: None,
                    })
                } else {
                    return Err(bridge_core::BridgeError::serialization(format!(
                        "Missing log message in row {}", row_idx
                    )));
                }
            }
            TelemetryType::Trace => {
                TelemetryData::Trace(bridge_core::types::TraceData {
                    trace_id: id.to_string(),
                    span_id: id.to_string(),
                    parent_span_id: None,
                    name: "trace".to_string(),
                    kind: bridge_core::types::SpanKind::Internal,
                    start_time: timestamp,
                    end_time: Some(timestamp),
                    duration_ns: trace_duration.map(|v| v as u64),
                    status: bridge_core::types::SpanStatus {
                        code: bridge_core::types::StatusCode::Ok,
                        message: None,
                    },
                    attributes: HashMap::new(),
                    events: Vec::new(),
                    links: Vec::new(),
                })
            }
            TelemetryType::Event => {
                if let Some(name) = event_name {
                    TelemetryData::Event(bridge_core::types::EventData {
                        name,
                        timestamp,
                        attributes: HashMap::new(),
                        data: None,
                    })
                } else {
                    return Err(bridge_core::BridgeError::serialization(format!(
                        "Missing event name in row {}", row_idx
                    )));
                }
            }
        };

        let record = TelemetryRecord {
            id,
            timestamp,
            record_type,
            data,
            attributes,
            tags,
            resource: None,
            service: None,
        };

        records.push(record);
    }

    Ok(TelemetryBatch {
        id: batch_id,
        timestamp: Utc::now(),
        source,
        size: records.len(),
        records,
        metadata: HashMap::new(),
    })
}

/// Serialize TelemetryBatch to Arrow IPC format
pub fn serialize_to_arrow_ipc(batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
    let record_batch = telemetry_batch_to_record_batch(batch)?;
    let mut buffer = Vec::new();
    
    {
        let mut writer = StreamWriter::try_new(&mut buffer, record_batch.schema().as_ref())
            .map_err(|e| bridge_core::BridgeError::serialization(format!(
                "Failed to create Arrow IPC writer: {}", e
            )))?;
        
        writer.write(&record_batch)
            .map_err(|e| bridge_core::BridgeError::serialization(format!(
                "Failed to write Arrow IPC data: {}", e
            )))?;
        
        writer.finish()
            .map_err(|e| bridge_core::BridgeError::serialization(format!(
                "Failed to finish Arrow IPC writer: {}", e
            )))?;
    }
    
    Ok(buffer)
}

/// Deserialize Arrow IPC format to TelemetryBatch
pub fn deserialize_from_arrow_ipc(data: &[u8]) -> BridgeResult<TelemetryBatch> {
    let mut reader = StreamReader::try_new(data, None)
        .map_err(|e| bridge_core::BridgeError::serialization(format!(
            "Failed to create Arrow IPC reader: {}", e
        )))?;
    
    let record_batch = reader.next()
        .ok_or_else(|| bridge_core::BridgeError::serialization(
            "No data found in Arrow IPC stream".to_string()
        ))?
        .map_err(|e| bridge_core::BridgeError::serialization(format!(
            "Failed to read Arrow IPC data: {}", e
        )))?;
    
    record_batch_to_telemetry_batch(&record_batch)
}

// Helper functions for extracting values from Arrow arrays
fn get_string_value(batch: &RecordBatch, column_name: &str, row_idx: usize) -> BridgeResult<String> {
    let column_idx = batch.schema().column_with_name(column_name)
        .ok_or_else(|| bridge_core::BridgeError::serialization(format!(
            "Column '{}' not found", column_name
        )))?.0;
    
    let column = batch.column(column_idx);
    let array = column.as_any().downcast_ref::<StringArray>()
        .ok_or_else(|| bridge_core::BridgeError::serialization(format!(
            "Column '{}' is not a string array", column_name
        )))?;
    
    if array.is_valid(row_idx) {
        Ok(array.value(row_idx).to_string())
    } else {
        Err(bridge_core::BridgeError::serialization(format!(
            "Null value in column '{}' at row {}", column_name, row_idx
        )))
    }
}

fn get_optional_string_value(batch: &RecordBatch, column_name: &str, row_idx: usize) -> Option<String> {
    let schema = batch.schema();
    let column_idx = schema.column_with_name(column_name)?;
    let column = batch.column(column_idx.0);
    let array = column.as_any().downcast_ref::<StringArray>()?;
    
    if array.is_valid(row_idx) {
        Some(array.value(row_idx).to_string())
    } else {
        None
    }
}

fn get_timestamp_value(batch: &RecordBatch, column_name: &str, row_idx: usize) -> BridgeResult<i64> {
    let column_idx = batch.schema().column_with_name(column_name)
        .ok_or_else(|| bridge_core::BridgeError::serialization(format!(
            "Column '{}' not found", column_name
        )))?.0;
    
    let column = batch.column(column_idx);
    let array = column.as_any().downcast_ref::<TimestampNanosecondArray>()
        .ok_or_else(|| bridge_core::BridgeError::serialization(format!(
            "Column '{}' is not a timestamp array", column_name
        )))?;
    
    if array.is_valid(row_idx) {
        Ok(array.value(row_idx))
    } else {
        Err(bridge_core::BridgeError::serialization(format!(
            "Null value in column '{}' at row {}", column_name, row_idx
        )))
    }
}

fn get_optional_float_value(batch: &RecordBatch, column_name: &str, row_idx: usize) -> Option<f64> {
    let schema = batch.schema();
    let column_idx = schema.column_with_name(column_name)?;
    let column = batch.column(column_idx.0);
    let array = column.as_any().downcast_ref::<Float64Array>()?;
    
    if array.is_valid(row_idx) {
        Some(array.value(row_idx))
    } else {
        None
    }
}

fn get_optional_int_value(batch: &RecordBatch, column_name: &str, row_idx: usize) -> Option<i64> {
    let schema = batch.schema();
    let column_idx = schema.column_with_name(column_name)?;
    let column = batch.column(column_idx.0);
    let array = column.as_any().downcast_ref::<Int64Array>()?;
    
    if array.is_valid(row_idx) {
        Some(array.value(row_idx))
    } else {
        None
    }
}

fn parse_record_type(record_type_str: &str) -> BridgeResult<TelemetryType> {
    match record_type_str {
        "Metric" => Ok(TelemetryType::Metric),
        "Log" => Ok(TelemetryType::Log),
        "Trace" => Ok(TelemetryType::Trace),
        "Event" => Ok(TelemetryType::Event),
        _ => Err(bridge_core::BridgeError::serialization(format!(
            "Unknown record type: {}", record_type_str
        ))),
    }
}

fn parse_log_level(level_str: &str) -> bridge_core::types::LogLevel {
    match level_str {
        "Error" => bridge_core::types::LogLevel::Error,
        "Warn" => bridge_core::types::LogLevel::Warn,
        "Info" => bridge_core::types::LogLevel::Info,
        "Debug" => bridge_core::types::LogLevel::Debug,
        "Trace" => bridge_core::types::LogLevel::Trace,
        _ => bridge_core::types::LogLevel::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_batch_roundtrip() {
        let original_batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![
                TelemetryRecord {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    record_type: TelemetryType::Metric,
                    data: TelemetryData::Metric(bridge_core::types::MetricData {
                        name: "test_metric".to_string(),
                        description: None,
                        unit: Some("count".to_string()),
                        metric_type: bridge_core::types::MetricType::Gauge,
                        value: bridge_core::types::MetricValue::Gauge(42.0),
                        labels: HashMap::new(),
                        timestamp: Utc::now(),
                    }),
                    attributes: HashMap::new(),
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                }
            ],
            metadata: HashMap::new(),
        };

        // Convert to Arrow IPC
        let arrow_data = serialize_to_arrow_ipc(&original_batch).unwrap();
        
        // Convert back from Arrow IPC
        let deserialized_batch = deserialize_from_arrow_ipc(&arrow_data).unwrap();
        
        // Verify roundtrip
        assert_eq!(original_batch.id, deserialized_batch.id);
        assert_eq!(original_batch.source, deserialized_batch.source);
        assert_eq!(original_batch.size, deserialized_batch.size);
        assert_eq!(original_batch.records.len(), deserialized_batch.records.len());
    }
}
