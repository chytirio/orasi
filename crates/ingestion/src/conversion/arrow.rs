//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Arrow converter implementation

use arrow_array::{
    Array, Float64Array, RecordBatch, StringArray, TimestampNanosecondArray, UInt64Array,
};
use arrow_ipc::{reader::StreamReader, writer::StreamWriter};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use bridge_core::types::{
    LogData, LogLevel, MetricData, MetricType, MetricValue, SpanKind, StatusCode, TelemetryData,
    TelemetryRecord, TelemetryType,
};
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use super::converter::TelemetryConverter;

/// Arrow converter implementation
pub struct ArrowConverter;

#[async_trait::async_trait]
impl TelemetryConverter for ArrowConverter {
    async fn convert(
        &self,
        data: &[u8],
        source_format: &str,
        target_format: &str,
    ) -> BridgeResult<Vec<u8>> {
        match (source_format, target_format) {
            ("arrow", "internal") => {
                let batch = self.convert_from_arrow(data).await?;
                // Serialize to internal format using serde_json
                let serialized = serde_json::to_vec(&batch).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to serialize TelemetryBatch to JSON: {}",
                        e
                    ))
                })?;
                Ok(serialized)
            }
            ("internal", "arrow") => {
                // Deserialize from internal format using serde_json
                let batch: TelemetryBatch = serde_json::from_slice(data).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to deserialize TelemetryBatch from JSON: {}",
                        e
                    ))
                })?;
                self.convert_to_arrow(&batch).await
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported conversion: {} -> {}",
                source_format, target_format
            ))),
        }
    }

    fn supported_source_formats(&self) -> Vec<String> {
        vec!["arrow".to_string(), "internal".to_string()]
    }

    fn supported_target_formats(&self) -> Vec<String> {
        vec!["arrow".to_string(), "internal".to_string()]
    }
}

impl ArrowConverter {
    /// Create new Arrow converter
    pub fn new() -> Self {
        Self
    }

    /// Convert Arrow data to internal format
    pub async fn convert_from_arrow(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        info!(
            "Converting Arrow data of {} bytes to internal format",
            data.len()
        );

        // Read Arrow IPC stream
        let cursor = std::io::Cursor::new(data);
        let mut reader = StreamReader::try_new(cursor, None).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to create Arrow stream reader: {}",
                e
            ))
        })?;

        let mut all_records = Vec::new();
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "arrow".to_string());
        metadata.insert("converted_at".to_string(), Utc::now().to_rfc3339());

        loop {
            match reader.next() {
                Some(Ok(record_batch)) => {
                    let records = self.convert_arrow_record_batch(record_batch).await?;
                    all_records.extend(records);
                }
                Some(Err(e)) => {
                    return Err(bridge_core::BridgeError::serialization(format!(
                        "Failed to read Arrow record batch: {}",
                        e
                    )))
                }
                None => break,
            }
        }

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "arrow".to_string(),
            size: all_records.len(),
            records: all_records,
            metadata,
        })
    }

    /// Convert Arrow record batch to telemetry records
    async fn convert_arrow_record_batch(
        &self,
        record_batch: RecordBatch,
    ) -> BridgeResult<Vec<TelemetryRecord>> {
        let mut records = Vec::new();
        let num_rows = record_batch.num_rows();

        // Extract common columns
        let timestamp_col = record_batch.column_by_name("timestamp").ok_or_else(|| {
            bridge_core::BridgeError::serialization("Missing timestamp column".to_string())
        })?;
        let record_type_col = record_batch.column_by_name("record_type").ok_or_else(|| {
            bridge_core::BridgeError::serialization("Missing record_type column".to_string())
        })?;
        let id_col = record_batch.column_by_name("id").ok_or_else(|| {
            bridge_core::BridgeError::serialization("Missing id column".to_string())
        })?;

        for row in 0..num_rows {
            let timestamp = self.extract_timestamp(timestamp_col, row)?;
            let record_type = self.extract_string(record_type_col, row)?;
            let id = self.extract_string(id_col, row)?;

            let record = match record_type.as_str() {
                "metric" => {
                    self.convert_arrow_metric_row(&record_batch, row, &id, timestamp)
                        .await?
                }
                "log" => {
                    self.convert_arrow_log_row(&record_batch, row, &id, timestamp)
                        .await?
                }
                "trace" => {
                    self.convert_arrow_trace_row(&record_batch, row, &id, timestamp)
                        .await?
                }
                _ => {
                    info!("Unknown record type: {}", record_type);
                    continue;
                }
            };

            records.push(record);
        }

        Ok(records)
    }

    /// Convert Arrow row to metric record
    async fn convert_arrow_metric_row(
        &self,
        record_batch: &RecordBatch,
        row: usize,
        id: &str,
        timestamp: DateTime<Utc>,
    ) -> BridgeResult<TelemetryRecord> {
        let name = self
            .extract_string_opt(record_batch.column_by_name("name"), row)
            .unwrap_or_else(|| "unknown_metric".to_string());
        let value = self.extract_float64(
            record_batch.column_by_name("value").ok_or_else(|| {
                bridge_core::BridgeError::serialization("Missing value column".to_string())
            })?,
            row,
        )?;
        let metric_type = self
            .extract_string_opt(record_batch.column_by_name("metric_type"), row)
            .unwrap_or_else(|| "gauge".to_string());

        let metric_type_enum = match metric_type.as_str() {
            "gauge" => MetricType::Gauge,
            "counter" => MetricType::Counter,
            "histogram" => MetricType::Histogram,
            _ => MetricType::Gauge,
        };

        let labels = self.extract_attributes(record_batch, row, "label_");
        let attributes = self.extract_attributes(record_batch, row, "attr_");

        Ok(TelemetryRecord {
            id: Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4()),
            timestamp,
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name,
                description: None,
                unit: None,
                metric_type: metric_type_enum,
                value: MetricValue::Gauge(value),
                labels,
                timestamp,
            }),
            attributes,
            tags: HashMap::new(),
            resource: None,
            service: None,
        })
    }

    /// Convert Arrow row to log record
    async fn convert_arrow_log_row(
        &self,
        record_batch: &RecordBatch,
        row: usize,
        id: &str,
        timestamp: DateTime<Utc>,
    ) -> BridgeResult<TelemetryRecord> {
        let message = self
            .extract_string_opt(record_batch.column_by_name("message"), row)
            .unwrap_or_else(|| "".to_string());
        let level = self
            .extract_string_opt(record_batch.column_by_name("level"), row)
            .unwrap_or_else(|| "info".to_string());
        let severity_number = self
            .extract_u64_opt(record_batch.column_by_name("severity_number"), row)
            .unwrap_or(9);

        let log_level = match level.as_str() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            "fatal" => LogLevel::Fatal,
            _ => LogLevel::Info,
        };

        let attributes = self.extract_attributes(record_batch, row, "attr_");

        Ok(TelemetryRecord {
            id: Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4()),
            timestamp,
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp,
                level: log_level,
                message: message.clone(),
                attributes: attributes.clone(),
                body: Some(message),
                severity_number: Some(severity_number as u32),
                severity_text: Some(level),
            }),
            attributes,
            tags: HashMap::new(),
            resource: None,
            service: None,
        })
    }

    /// Convert Arrow row to trace record
    async fn convert_arrow_trace_row(
        &self,
        record_batch: &RecordBatch,
        row: usize,
        id: &str,
        timestamp: DateTime<Utc>,
    ) -> BridgeResult<TelemetryRecord> {
        let trace_id = self
            .extract_string_opt(record_batch.column_by_name("trace_id"), row)
            .unwrap_or_else(|| "".to_string());
        let span_id = self
            .extract_string_opt(record_batch.column_by_name("span_id"), row)
            .unwrap_or_else(|| "".to_string());
        let parent_span_id =
            self.extract_string_opt(record_batch.column_by_name("parent_span_id"), row);
        let name = self
            .extract_string_opt(record_batch.column_by_name("name"), row)
            .unwrap_or_else(|| "unknown_span".to_string());
        let kind = self
            .extract_string_opt(record_batch.column_by_name("kind"), row)
            .unwrap_or_else(|| "internal".to_string());

        let span_kind = match kind.as_str() {
            "server" => SpanKind::Server,
            "client" => SpanKind::Client,
            "producer" => SpanKind::Producer,
            "consumer" => SpanKind::Consumer,
            _ => SpanKind::Internal,
        };

        let attributes = self.extract_attributes(record_batch, row, "attr_");

        Ok(TelemetryRecord {
            id: Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4()),
            timestamp,
            record_type: TelemetryType::Trace,
            data: TelemetryData::Trace(bridge_core::types::TraceData {
                trace_id,
                span_id,
                parent_span_id,
                name,
                kind: span_kind,
                status: bridge_core::types::SpanStatus {
                    code: StatusCode::Unset,
                    message: None,
                },
                start_time: timestamp,
                end_time: None,
                duration_ns: None,
                attributes: attributes.clone(),
                events: vec![],
                links: vec![],
            }),
            attributes,
            tags: HashMap::new(),
            resource: None,
            service: None,
        })
    }

    /// Extract timestamp from Arrow column
    fn extract_timestamp(
        &self,
        column: &Arc<dyn Array>,
        row: usize,
    ) -> BridgeResult<DateTime<Utc>> {
        if let Some(timestamp_array) = column.as_any().downcast_ref::<TimestampNanosecondArray>() {
            let timestamp_nanos = timestamp_array.value(row);
            return Ok(DateTime::from_timestamp_nanos(timestamp_nanos));
        }
        Ok(Utc::now())
    }

    /// Extract string from Arrow column
    fn extract_string(&self, column: &Arc<dyn Array>, row: usize) -> BridgeResult<String> {
        if let Some(string_array) = column.as_any().downcast_ref::<StringArray>() {
            let value = string_array.value(row);
            return Ok(value.to_string());
        }
        Err(bridge_core::BridgeError::serialization(
            "Failed to extract string value".to_string(),
        ))
    }

    /// Extract optional string from Arrow column
    fn extract_string_opt(&self, column: Option<&Arc<dyn Array>>, row: usize) -> Option<String> {
        column.and_then(|col| {
            if let Some(string_array) = col.as_any().downcast_ref::<StringArray>() {
                Some(string_array.value(row).to_string())
            } else {
                None
            }
        })
    }

    /// Extract float64 from Arrow column
    fn extract_float64(&self, column: &Arc<dyn Array>, row: usize) -> BridgeResult<f64> {
        if let Some(float_array) = column.as_any().downcast_ref::<Float64Array>() {
            let value = float_array.value(row);
            return Ok(value);
        }
        Err(bridge_core::BridgeError::serialization(
            "Failed to extract float64 value".to_string(),
        ))
    }

    /// Extract optional u64 from Arrow column
    fn extract_u64_opt(&self, column: Option<&Arc<dyn Array>>, row: usize) -> Option<u64> {
        column.and_then(|col| {
            if let Some(uint_array) = col.as_any().downcast_ref::<UInt64Array>() {
                Some(uint_array.value(row))
            } else {
                None
            }
        })
    }

    /// Extract attributes from Arrow columns with prefix
    fn extract_attributes(
        &self,
        record_batch: &RecordBatch,
        row: usize,
        prefix: &str,
    ) -> HashMap<String, String> {
        let mut attributes = HashMap::new();

        let schema = record_batch.schema();
        for (field_name, _) in schema.fields().iter().enumerate() {
            let field = &schema.fields()[field_name];
            if field.name().starts_with(prefix) {
                if let Some(column) = record_batch.column_by_name(&field.name()) {
                    if let Some(value) = self.extract_string_opt(Some(column), row) {
                        let key = field.name().trim_start_matches(prefix).to_string();
                        attributes.insert(key, value);
                    }
                }
            }
        }

        attributes
    }

    /// Convert internal format to Arrow
    pub async fn convert_to_arrow(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        info!(
            "Converting internal format to Arrow for batch with {} records",
            batch.records.len()
        );

        // Group records by type for efficient schema generation
        let mut metrics = Vec::new();
        let mut logs = Vec::new();
        let mut traces = Vec::new();

        for record in &batch.records {
            match &record.record_type {
                TelemetryType::Metric => metrics.push(record),
                TelemetryType::Log => logs.push(record),
                TelemetryType::Trace => traces.push(record),
                TelemetryType::Event => {
                    // Convert events to logs for Arrow format
                    info!("Converting event records to logs for Arrow format");
                    logs.push(record);
                }
            }
        }

        // Implement proper Arrow streaming
        let mut arrow_batches = Vec::new();

        // Convert metrics to Arrow record batch
        if !metrics.is_empty() {
            let metrics_batch = self.convert_metrics_to_arrow(&metrics).await?;
            arrow_batches.push(metrics_batch);
        }

        // Convert logs to Arrow record batch
        if !logs.is_empty() {
            let logs_batch = self.convert_logs_to_arrow(&logs).await?;
            arrow_batches.push(logs_batch);
        }

        // Convert traces to Arrow record batch
        if !traces.is_empty() {
            let traces_batch = self.convert_traces_to_arrow(&traces).await?;
            arrow_batches.push(traces_batch);
        }

        // Write Arrow IPC stream
        let mut buffer = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buffer, &arrow_batches[0].schema())
                .map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to create Arrow stream writer: {}",
                        e
                    ))
                })?;

            for batch in arrow_batches {
                writer.write(&batch).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to write Arrow record batch: {}",
                        e
                    ))
                })?;
            }

            writer.finish().map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to finish Arrow stream: {}",
                    e
                ))
            })?;
        }

        Ok(buffer)
    }

    /// Convert metrics to Arrow record batch
    async fn convert_metrics_to_arrow(
        &self,
        metrics: &[&TelemetryRecord],
    ) -> BridgeResult<RecordBatch> {
        let mut timestamps = Vec::new();
        let mut ids = Vec::new();
        let mut names = Vec::new();
        let mut values = Vec::new();
        let mut metric_types = Vec::new();
        let mut labels = HashMap::new();
        let mut attributes = HashMap::new();

        for record in metrics {
            timestamps.push(record.timestamp.timestamp_nanos_opt().unwrap_or(0));
            ids.push(record.id.to_string());

            if let TelemetryData::Metric(metric_data) = &record.data {
                names.push(metric_data.name.clone());
                values.push(match &metric_data.value {
                    MetricValue::Gauge(v) => *v,
                    MetricValue::Counter(v) => *v,
                    MetricValue::Histogram {
                        buckets: _,
                        sum: _,
                        count: _,
                    } => 0.0,
                    MetricValue::Summary { .. } => 0.0,
                });
                metric_types.push(format!("{:?}", metric_data.metric_type).to_lowercase());

                // Collect all unique label and attribute keys
                for (k, v) in &metric_data.labels {
                    labels
                        .entry(k.clone())
                        .or_insert_with(Vec::new)
                        .push(v.clone());
                }
                for (k, v) in &record.attributes {
                    attributes
                        .entry(k.clone())
                        .or_insert_with(Vec::new)
                        .push(v.clone());
                }
            }
        }

        // Create schema
        let mut fields = vec![
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                false,
            ),
            Field::new("id", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("value", DataType::Float64, false),
            Field::new("metric_type", DataType::Utf8, false),
        ];

        // Add label fields
        for key in labels.keys() {
            fields.push(Field::new(format!("label_{}", key), DataType::Utf8, true));
        }

        // Add attribute fields
        for key in attributes.keys() {
            fields.push(Field::new(format!("attr_{}", key), DataType::Utf8, true));
        }

        let schema = Arc::new(Schema::new(fields));

        // Create arrays
        let mut arrays: Vec<Arc<dyn Array>> = vec![
            Arc::new(TimestampNanosecondArray::from(timestamps)),
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(names)),
            Arc::new(Float64Array::from(values)),
            Arc::new(StringArray::from(metric_types)),
        ];

        // Add label arrays
        for key in labels.keys() {
            let values: Vec<Option<String>> = (0..metrics.len())
                .map(|i| {
                    if let TelemetryData::Metric(metric_data) = &metrics[i].data {
                        metric_data.labels.get(key).cloned()
                    } else {
                        None
                    }
                })
                .collect();
            arrays.push(Arc::new(StringArray::from(values)));
        }

        // Add attribute arrays
        for key in attributes.keys() {
            let values: Vec<Option<String>> = (0..metrics.len())
                .map(|i| metrics[i].attributes.get(key).cloned())
                .collect();
            arrays.push(Arc::new(StringArray::from(values)));
        }

        RecordBatch::try_new(schema, arrays).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to create Arrow metrics record batch: {}",
                e
            ))
        })
    }

    /// Convert logs to Arrow record batch
    async fn convert_logs_to_arrow(&self, logs: &[&TelemetryRecord]) -> BridgeResult<RecordBatch> {
        let mut timestamps = Vec::new();
        let mut ids = Vec::new();
        let mut messages = Vec::new();
        let mut levels = Vec::new();
        let mut severity_numbers = Vec::new();
        let mut attributes = HashMap::new();

        for record in logs {
            timestamps.push(record.timestamp.timestamp_nanos_opt().unwrap_or(0));
            ids.push(record.id.to_string());

            if let TelemetryData::Log(log_data) = &record.data {
                messages.push(log_data.message.clone());
                levels.push(format!("{:?}", log_data.level).to_lowercase());
                severity_numbers.push(log_data.severity_number.unwrap_or(9) as u64);

                // Collect all unique attribute keys
                for (k, v) in &log_data.attributes {
                    attributes
                        .entry(k.clone())
                        .or_insert_with(Vec::new)
                        .push(v.clone());
                }
            }
        }

        // Create schema
        let mut fields = vec![
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                false,
            ),
            Field::new("id", DataType::Utf8, false),
            Field::new("message", DataType::Utf8, false),
            Field::new("level", DataType::Utf8, false),
            Field::new("severity_number", DataType::UInt64, false),
        ];

        // Add attribute fields
        for key in attributes.keys() {
            fields.push(Field::new(format!("attr_{}", key), DataType::Utf8, true));
        }

        let schema = Arc::new(Schema::new(fields));

        // Create arrays
        let mut arrays: Vec<Arc<dyn Array>> = vec![
            Arc::new(TimestampNanosecondArray::from(timestamps)),
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(messages)),
            Arc::new(StringArray::from(levels)),
            Arc::new(UInt64Array::from(severity_numbers)),
        ];

        // Add attribute arrays
        for key in attributes.keys() {
            let values: Vec<Option<String>> = (0..logs.len())
                .map(|i| {
                    if let TelemetryData::Log(log_data) = &logs[i].data {
                        log_data.attributes.get(key).cloned()
                    } else {
                        None
                    }
                })
                .collect();
            arrays.push(Arc::new(StringArray::from(values)));
        }

        RecordBatch::try_new(schema, arrays).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to create Arrow logs record batch: {}",
                e
            ))
        })
    }

    /// Convert traces to Arrow record batch
    async fn convert_traces_to_arrow(
        &self,
        traces: &[&TelemetryRecord],
    ) -> BridgeResult<RecordBatch> {
        let mut timestamps = Vec::new();
        let mut ids = Vec::new();
        let mut trace_ids = Vec::new();
        let mut span_ids = Vec::new();
        let mut parent_span_ids = Vec::new();
        let mut names = Vec::new();
        let mut kinds = Vec::new();
        let mut attributes = HashMap::new();

        for record in traces {
            timestamps.push(record.timestamp.timestamp_nanos_opt().unwrap_or(0));
            ids.push(record.id.to_string());

            if let TelemetryData::Trace(trace_data) = &record.data {
                trace_ids.push(trace_data.trace_id.clone());
                span_ids.push(trace_data.span_id.clone());
                parent_span_ids.push(trace_data.parent_span_id.clone().unwrap_or_default());
                names.push(trace_data.name.clone());
                kinds.push(format!("{:?}", trace_data.kind).to_lowercase());

                // Collect all unique attribute keys
                for (k, v) in &trace_data.attributes {
                    attributes
                        .entry(k.clone())
                        .or_insert_with(Vec::new)
                        .push(v.clone());
                }
            }
        }

        // Create schema
        let mut fields = vec![
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                false,
            ),
            Field::new("id", DataType::Utf8, false),
            Field::new("trace_id", DataType::Utf8, false),
            Field::new("span_id", DataType::Utf8, false),
            Field::new("parent_span_id", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("kind", DataType::Utf8, false),
        ];

        // Add attribute fields
        for key in attributes.keys() {
            fields.push(Field::new(format!("attr_{}", key), DataType::Utf8, true));
        }

        let schema = Arc::new(Schema::new(fields));

        // Create arrays
        let mut arrays: Vec<Arc<dyn Array>> = vec![
            Arc::new(TimestampNanosecondArray::from(timestamps)),
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(trace_ids)),
            Arc::new(StringArray::from(span_ids)),
            Arc::new(StringArray::from(parent_span_ids)),
            Arc::new(StringArray::from(names)),
            Arc::new(StringArray::from(kinds)),
        ];

        // Add attribute arrays
        for key in attributes.keys() {
            let values: Vec<Option<String>> = (0..traces.len())
                .map(|i| {
                    if let TelemetryData::Trace(trace_data) = &traces[i].data {
                        trace_data.attributes.get(key).cloned()
                    } else {
                        None
                    }
                })
                .collect();
            arrays.push(Arc::new(StringArray::from(values)));
        }

        RecordBatch::try_new(schema, arrays).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to create Arrow traces record batch: {}",
                e
            ))
        })
    }

    /// Convert to internal format (JSON)
    pub async fn convert_to_internal(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        serde_json::to_vec(batch).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to serialize TelemetryBatch to JSON: {}",
                e
            ))
        })
    }

    /// Convert from internal format (JSON)
    pub async fn convert_from_internal(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        serde_json::from_slice(data).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to deserialize TelemetryBatch from JSON: {}",
                e
            ))
        })
    }
}
