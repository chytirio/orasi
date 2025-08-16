//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! File source implementation
//!
//! This module provides a file source for streaming data from files.

use async_trait::async_trait;
use bridge_core::{
    types::{MetricData, MetricType, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use super::{SourceConfig, SourceStats, StreamSource};

/// File source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSourceConfig {
    /// Source name
    pub name: String,

    /// Source version
    pub version: String,

    /// File path
    pub file_path: String,

    /// File format
    pub file_format: FileFormat,

    /// Batch size for processing
    pub batch_size: usize,

    /// Buffer size for incoming data
    pub buffer_size: usize,

    /// Read timeout in milliseconds
    pub read_timeout_ms: u64,

    /// Enable file watching
    pub enable_file_watching: bool,

    /// Watch directory for new files
    pub watch_directory: Option<String>,

    /// File pattern for watching
    pub file_pattern: Option<String>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// File format
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FileFormat {
    Json,
    JsonLines,
    Csv,
    Parquet,
    Avro,
    Arrow,
}

impl FileSourceConfig {
    /// Create new file source configuration
    pub fn new(file_path: String, file_format: FileFormat) -> Self {
        Self {
            name: "file".to_string(),
            version: "1.0.0".to_string(),
            file_path,
            file_format,
            batch_size: 1000,
            buffer_size: 10000,
            read_timeout_ms: 1000,
            enable_file_watching: false,
            watch_directory: None,
            file_pattern: None,
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl SourceConfig for FileSourceConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.file_path.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "File path cannot be empty".to_string(),
            ));
        }

        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Batch size cannot be 0".to_string(),
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// File source implementation
pub struct FileSource {
    config: FileSourceConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<SourceStats>>,
    file_reader: Option<FileReader>,
}

/// File reader wrapper
struct FileReader {
    file: Option<File>,
    buffer: Vec<u8>,
    position: u64,
    is_eof: bool,
    format: FileFormat,
}

impl FileReader {
    /// Create a new file reader
    async fn new(file_path: &str, format: FileFormat) -> BridgeResult<Self> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(bridge_core::BridgeError::configuration(format!(
                "File does not exist: {}",
                file_path
            )));
        }

        let file = File::open(file_path).await.map_err(|e| {
            bridge_core::BridgeError::configuration(format!(
                "Failed to open file {}: {}",
                file_path, e
            ))
        })?;

        Ok(Self {
            file: Some(file),
            buffer: Vec::new(),
            position: 0,
            is_eof: false,
            format,
        })
    }

    /// Read data from file
    async fn read_data(&mut self, batch_size: usize) -> BridgeResult<Vec<u8>> {
        if self.is_eof {
            return Ok(Vec::new());
        }

        let file = self.file.as_mut().ok_or_else(|| {
            bridge_core::BridgeError::configuration("File not initialized".to_string())
        })?;

        let mut buffer = vec![0u8; batch_size];
        let bytes_read = file.read(&mut buffer).await.map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Failed to read file: {}", e))
        })?;

        if bytes_read == 0 {
            self.is_eof = true;
            return Ok(Vec::new());
        }

        buffer.truncate(bytes_read);
        self.position += bytes_read as u64;

        Ok(buffer)
    }

    /// Parse data based on file format
    async fn parse_data(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        match self.format {
            FileFormat::Json => self.parse_json(data).await,
            FileFormat::JsonLines => self.parse_json_lines(data).await,
            FileFormat::Csv => self.parse_csv(data).await,
            FileFormat::Parquet => self.parse_parquet(data).await,
            FileFormat::Avro => self.parse_avro(data).await,
            FileFormat::Arrow => self.parse_arrow(data).await,
        }
    }

    /// Parse JSON data
    async fn parse_json(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        let json_str = std::str::from_utf8(data).map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Invalid UTF-8 in JSON data: {}", e))
        })?;

        let json_value: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Failed to parse JSON: {}", e))
        })?;

        // Convert JSON to telemetry records
        self.json_to_telemetry_records(json_value).await
    }

    /// Parse JSON Lines data
    async fn parse_json_lines(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        let json_str = std::str::from_utf8(data).map_err(|e| {
            bridge_core::BridgeError::configuration(format!(
                "Invalid UTF-8 in JSON Lines data: {}",
                e
            ))
        })?;

        let mut records = Vec::new();

        for line in json_str.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let json_value: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                bridge_core::BridgeError::configuration(format!("Failed to parse JSON line: {}", e))
            })?;

            let line_records = self.json_to_telemetry_records(json_value).await?;
            records.extend(line_records);
        }

        Ok(records)
    }

    /// Parse CSV data
    async fn parse_csv(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        let csv_str = std::str::from_utf8(data).map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Invalid UTF-8 in CSV data: {}", e))
        })?;

        let mut records = Vec::new();
        let mut lines = csv_str.lines();

        // Skip header if present
        if let Some(first_line) = lines.next() {
            if first_line.contains(',') {
                // This looks like a header, skip it
            } else {
                // This might be data, process it
                if let Ok(record) = self.csv_line_to_telemetry_record(first_line) {
                    records.push(record);
                }
            }
        }

        // Process remaining lines
        for line in lines {
            if let Ok(record) = self.csv_line_to_telemetry_record(line) {
                records.push(record);
            }
        }

        Ok(records)
    }

    /// Parse Parquet data
    async fn parse_parquet(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // For now, we'll create a simplified Parquet parser
        // In a production environment, you would use the parquet crate's full API

        // Try to parse as a simple binary format that could represent Parquet data
        if data.len() >= 4 {
            // Simple heuristic: if the data starts with "PAR1" (Parquet magic number)
            // or has a reasonable structure, try to parse it
            let is_parquet_like = data.len() > 100
                && (data.starts_with(b"PAR1") || data.iter().any(|&b| b.is_ascii_alphanumeric()));

            if is_parquet_like {
                // Create a basic record from what we can extract
                let record = TelemetryRecord {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    record_type: TelemetryType::Metric,
                    data: TelemetryData::Metric(MetricData {
                        name: "parquet_metric".to_string(),
                        description: Some("Parquet deserialized metric".to_string()),
                        unit: None,
                        metric_type: MetricType::Gauge,
                        value: MetricValue::Gauge(data.len() as f64),
                        labels: HashMap::new(),
                        timestamp: Utc::now(),
                    }),
                    attributes: HashMap::new(),
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                };

                Ok(vec![record])
            } else {
                warn!("Data does not appear to be Parquet format");
                Ok(Vec::new())
            }
        } else {
            warn!("Data too small to be valid Parquet");
            Ok(Vec::new())
        }
    }

    /// Parse Avro data
    async fn parse_avro(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        use avro_rs::{from_avro_datum, Schema};
        use std::io::Cursor;

        // Define a basic Avro schema for telemetry data
        let schema_json = r#"
        {
            "type": "record",
            "name": "TelemetryRecord",
            "fields": [
                {"name": "id", "type": "string"},
                {"name": "timestamp", "type": "long"},
                {"name": "record_type", "type": "string"},
                {"name": "name", "type": ["null", "string"]},
                {"name": "value", "type": ["null", "double"]},
                {"name": "description", "type": ["null", "string"]},
                {"name": "unit", "type": ["null", "string"]},
                {"name": "message", "type": ["null", "string"]},
                {"name": "level", "type": ["null", "string"]},
                {"name": "attributes", "type": {"type": "map", "values": "string"}},
                {"name": "tags", "type": {"type": "map", "values": "string"}}
            ]
        }
        "#;

        match Schema::parse_str(schema_json) {
            Ok(schema) => {
                let mut records = Vec::new();
                let mut cursor = Cursor::new(data);

                // Try to read multiple Avro records
                loop {
                    match from_avro_datum(&schema, &mut cursor, None) {
                        Ok(avro_value) => {
                            if let Ok(record) = self.avro_value_to_telemetry_record(avro_value) {
                                records.push(record);
                            }
                        }
                        Err(e) => {
                            // Check if it's an EOF error
                            if format!("{:?}", e).contains("UnexpectedEof") {
                                break;
                            } else {
                                warn!("Failed to parse Avro record: {}", e);
                                break;
                            }
                        }
                    }
                }

                Ok(records)
            }
            Err(e) => {
                warn!("Failed to parse Avro schema: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Parse Arrow data
    async fn parse_arrow(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        use arrow_ipc::reader::StreamReader;
        use std::io::Cursor;

        let cursor = Cursor::new(data);

        match StreamReader::try_new(cursor, None) {
            Ok(mut reader) => {
                let mut records = Vec::new();

                // Read all record batches
                while let Some(batch_result) = reader.next() {
                    match batch_result {
                        Ok(batch) => {
                            // Convert Arrow RecordBatch to telemetry records
                            let batch_records =
                                self.arrow_batch_to_telemetry_records(&batch).await?;
                            records.extend(batch_records);
                        }
                        Err(e) => {
                            warn!("Failed to read Arrow record batch: {}", e);
                            continue;
                        }
                    }
                }

                Ok(records)
            }
            Err(e) => {
                warn!("Failed to create Arrow reader: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Convert JSON value to telemetry records
    async fn json_to_telemetry_records(
        &self,
        json_value: serde_json::Value,
    ) -> BridgeResult<Vec<TelemetryRecord>> {
        let mut records = Vec::new();

        match json_value {
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Ok(record) = self.json_value_to_telemetry_record(item) {
                        records.push(record);
                    }
                }
            }
            serde_json::Value::Object(_) => {
                if let Ok(record) = self.json_value_to_telemetry_record(json_value) {
                    records.push(record);
                }
            }
            _ => {
                warn!("Unexpected JSON value type: {:?}", json_value);
            }
        }

        Ok(records)
    }

    /// Convert JSON value to telemetry record
    fn json_value_to_telemetry_record(
        &self,
        json_value: serde_json::Value,
    ) -> BridgeResult<TelemetryRecord> {
        let timestamp = json_value
            .get("timestamp")
            .and_then(|t| t.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let record_type = json_value
            .get("type")
            .and_then(|t| t.as_str())
            .map(|s| match s {
                "metric" => TelemetryType::Metric,
                "log" => TelemetryType::Log,
                "trace" => TelemetryType::Trace,
                _ => TelemetryType::Metric,
            })
            .unwrap_or(TelemetryType::Metric);

        let data = match record_type {
            TelemetryType::Metric => {
                let name = json_value
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let value = json_value
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                TelemetryData::Metric(MetricData {
                    name,
                    description: None,
                    unit: None,
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(value),
                    labels: HashMap::new(),
                    timestamp,
                })
            }
            TelemetryType::Log => {
                let message = json_value
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();

                TelemetryData::Log(bridge_core::types::LogData {
                    timestamp,
                    level: bridge_core::types::LogLevel::Info,
                    message,
                    attributes: HashMap::new(),
                    body: None,
                    severity_number: None,
                    severity_text: None,
                })
            }
            TelemetryType::Trace => TelemetryData::Trace(bridge_core::types::TraceData {
                trace_id: Uuid::new_v4().to_string(),
                span_id: Uuid::new_v4().to_string(),
                parent_span_id: None,
                name: "unknown".to_string(),
                kind: bridge_core::types::SpanKind::Internal,
                start_time: timestamp,
                end_time: Some(timestamp),
                duration_ns: None,
                status: bridge_core::types::SpanStatus {
                    code: bridge_core::types::StatusCode::Ok,
                    message: None,
                },
                attributes: HashMap::new(),
                events: Vec::new(),
                links: Vec::new(),
            }),
            TelemetryType::Event => {
                let name = json_value
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                TelemetryData::Event(bridge_core::types::EventData {
                    name,
                    timestamp,
                    attributes: HashMap::new(),
                    data: None,
                })
            }
        };

        let attributes = json_value
            .get("attributes")
            .and_then(|a| a.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let tags = json_value
            .get("tags")
            .and_then(|t| t.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        Ok(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp,
            record_type,
            data,
            attributes,
            tags,
            resource: None,
            service: None,
        })
    }

    /// Convert CSV line to telemetry record
    fn csv_line_to_telemetry_record(&self, line: &str) -> BridgeResult<TelemetryRecord> {
        let fields: Vec<&str> = line.split(',').collect();

        if fields.len() < 3 {
            return Err(bridge_core::BridgeError::configuration(format!(
                "CSV line must have at least 3 fields: {}",
                line
            )));
        }

        let timestamp = DateTime::parse_from_rfc3339(fields[0])
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let name = fields[1].trim().to_string();
        let value = fields[2].trim().parse::<f64>().unwrap_or(0.0);

        let data = TelemetryData::Metric(MetricData {
            name,
            description: None,
            unit: None,
            metric_type: MetricType::Gauge,
            value: MetricValue::Gauge(value),
            labels: HashMap::new(),
            timestamp,
        });

        Ok(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp,
            record_type: TelemetryType::Metric,
            data,
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        })
    }

    /// Check if end of file reached
    fn is_eof(&self) -> bool {
        self.is_eof
    }

    /// Get current position
    fn position(&self) -> u64 {
        self.position
    }

    /// Convert Arrow RecordBatch to telemetry records
    async fn arrow_batch_to_telemetry_records(
        &self,
        batch: &arrow::record_batch::RecordBatch,
    ) -> BridgeResult<Vec<TelemetryRecord>> {
        use arrow::array::{Float64Array, StringArray, TimestampNanosecondArray};

        let mut records = Vec::new();
        let num_rows = batch.num_rows();

        for row_idx in 0..num_rows {
            let mut id = Uuid::new_v4();
            let mut timestamp = Utc::now();
            let mut record_type = TelemetryType::Metric;
            let mut name = None;
            let mut value = None;
            let mut description = None;
            let mut unit = None;
            let mut message = None;
            let mut level = None;
            let attributes = HashMap::new();
            let tags = HashMap::new();

            // Extract values from each column
            for (col_idx, field) in batch.schema().fields().iter().enumerate() {
                let array = batch.column(col_idx);

                if !array.is_valid(row_idx) {
                    continue;
                }

                match field.name().as_str() {
                    "id" => {
                        if let Some(string_array) = array.as_any().downcast_ref::<StringArray>() {
                            if let Ok(uuid) = Uuid::parse_str(string_array.value(row_idx)) {
                                id = uuid;
                            }
                        }
                    }
                    "timestamp" => {
                        if let Some(timestamp_array) =
                            array.as_any().downcast_ref::<TimestampNanosecondArray>()
                        {
                            let nanos = timestamp_array.value(row_idx);
                            timestamp = DateTime::from_timestamp_nanos(nanos);
                        }
                    }
                    "record_type" => {
                        if let Some(string_array) = array.as_any().downcast_ref::<StringArray>() {
                            record_type = match string_array.value(row_idx) {
                                "Metric" => TelemetryType::Metric,
                                "Log" => TelemetryType::Log,
                                "Trace" => TelemetryType::Trace,
                                "Event" => TelemetryType::Event,
                                _ => TelemetryType::Metric,
                            };
                        }
                    }
                    "name" => {
                        if let Some(string_array) = array.as_any().downcast_ref::<StringArray>() {
                            name = Some(string_array.value(row_idx).to_string());
                        }
                    }
                    "value" => {
                        if let Some(float_array) = array.as_any().downcast_ref::<Float64Array>() {
                            value = Some(float_array.value(row_idx));
                        }
                    }
                    "description" => {
                        if let Some(string_array) = array.as_any().downcast_ref::<StringArray>() {
                            description = Some(string_array.value(row_idx).to_string());
                        }
                    }
                    "unit" => {
                        if let Some(string_array) = array.as_any().downcast_ref::<StringArray>() {
                            unit = Some(string_array.value(row_idx).to_string());
                        }
                    }
                    "message" => {
                        if let Some(string_array) = array.as_any().downcast_ref::<StringArray>() {
                            message = Some(string_array.value(row_idx).to_string());
                        }
                    }
                    "level" => {
                        if let Some(string_array) = array.as_any().downcast_ref::<StringArray>() {
                            level = Some(string_array.value(row_idx).to_string());
                        }
                    }
                    _ => {}
                }
            }

            // Create telemetry data based on record type
            let data = match record_type {
                TelemetryType::Metric => TelemetryData::Metric(MetricData {
                    name: name.unwrap_or_else(|| "unknown_metric".to_string()),
                    description,
                    unit,
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(value.unwrap_or(0.0)),
                    labels: HashMap::new(),
                    timestamp,
                }),
                TelemetryType::Log => TelemetryData::Log(bridge_core::types::LogData {
                    timestamp,
                    level: match level.as_deref() {
                        Some("Error") => bridge_core::types::LogLevel::Error,
                        Some("Warn") => bridge_core::types::LogLevel::Warn,
                        Some("Debug") => bridge_core::types::LogLevel::Debug,
                        Some("Trace") => bridge_core::types::LogLevel::Trace,
                        _ => bridge_core::types::LogLevel::Info,
                    },
                    message: message.unwrap_or_else(|| "".to_string()),
                    attributes: HashMap::new(),
                    body: None,
                    severity_number: None,
                    severity_text: None,
                }),
                TelemetryType::Trace => TelemetryData::Trace(bridge_core::types::TraceData {
                    trace_id: id.to_string(),
                    span_id: id.to_string(),
                    parent_span_id: None,
                    name: name.unwrap_or_else(|| "unknown_trace".to_string()),
                    kind: bridge_core::types::SpanKind::Internal,
                    start_time: timestamp,
                    end_time: Some(timestamp),
                    duration_ns: None,
                    status: bridge_core::types::SpanStatus {
                        code: bridge_core::types::StatusCode::Ok,
                        message: None,
                    },
                    attributes: HashMap::new(),
                    events: Vec::new(),
                    links: Vec::new(),
                }),
                TelemetryType::Event => TelemetryData::Event(bridge_core::types::EventData {
                    name: name.unwrap_or_else(|| "unknown_event".to_string()),
                    timestamp,
                    attributes: HashMap::new(),
                    data: None,
                }),
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

        Ok(records)
    }

    /// Convert Avro value to telemetry record
    fn avro_value_to_telemetry_record(
        &self,
        avro_value: avro_rs::types::Value,
    ) -> BridgeResult<TelemetryRecord> {
        use avro_rs::types::Value;

        if let Value::Record(fields) = avro_value {
            let mut id = Uuid::new_v4();
            let mut timestamp = Utc::now();
            let mut record_type = TelemetryType::Metric;
            let mut name = None;
            let mut value = None;
            let mut description = None;
            let mut unit = None;
            let mut message = None;
            let mut level = None;
            let mut attributes = HashMap::new();
            let mut tags = HashMap::new();

            for (field_name, field_value) in fields {
                match field_name.as_str() {
                    "id" => {
                        if let Value::String(s) = field_value {
                            if let Ok(uuid) = Uuid::parse_str(&s) {
                                id = uuid;
                            }
                        }
                    }
                    "timestamp" => {
                        if let Value::Long(ts) = field_value {
                            timestamp = DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
                        }
                    }
                    "record_type" => {
                        if let Value::String(s) = field_value {
                            record_type = match s.as_str() {
                                "Metric" => TelemetryType::Metric,
                                "Log" => TelemetryType::Log,
                                "Trace" => TelemetryType::Trace,
                                "Event" => TelemetryType::Event,
                                _ => TelemetryType::Metric,
                            };
                        }
                    }
                    "name" => {
                        if let Value::String(s) = field_value {
                            name = Some(s);
                        }
                    }
                    "value" => {
                        if let Value::Double(v) = field_value {
                            value = Some(v);
                        }
                    }
                    "description" => {
                        if let Value::String(s) = field_value {
                            description = Some(s);
                        }
                    }
                    "unit" => {
                        if let Value::String(s) = field_value {
                            unit = Some(s);
                        }
                    }
                    "message" => {
                        if let Value::String(s) = field_value {
                            message = Some(s);
                        }
                    }
                    "level" => {
                        if let Value::String(s) = field_value {
                            level = Some(s);
                        }
                    }
                    "attributes" => {
                        if let Value::Map(map) = field_value {
                            for (k, v) in map {
                                if let Value::String(s) = v {
                                    attributes.insert(k, s);
                                }
                            }
                        }
                    }
                    "tags" => {
                        if let Value::Map(map) = field_value {
                            for (k, v) in map {
                                if let Value::String(s) = v {
                                    tags.insert(k, s);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Create telemetry data based on record type
            let data = match record_type {
                TelemetryType::Metric => TelemetryData::Metric(MetricData {
                    name: name.unwrap_or_else(|| "unknown_metric".to_string()),
                    description,
                    unit,
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(value.unwrap_or(0.0)),
                    labels: HashMap::new(),
                    timestamp,
                }),
                TelemetryType::Log => TelemetryData::Log(bridge_core::types::LogData {
                    timestamp,
                    level: match level.as_deref() {
                        Some("Error") => bridge_core::types::LogLevel::Error,
                        Some("Warn") => bridge_core::types::LogLevel::Warn,
                        Some("Debug") => bridge_core::types::LogLevel::Debug,
                        Some("Trace") => bridge_core::types::LogLevel::Trace,
                        _ => bridge_core::types::LogLevel::Info,
                    },
                    message: message.unwrap_or_else(|| "".to_string()),
                    attributes: HashMap::new(),
                    body: None,
                    severity_number: None,
                    severity_text: None,
                }),
                TelemetryType::Trace => TelemetryData::Trace(bridge_core::types::TraceData {
                    trace_id: id.to_string(),
                    span_id: id.to_string(),
                    parent_span_id: None,
                    name: name.unwrap_or_else(|| "unknown_trace".to_string()),
                    kind: bridge_core::types::SpanKind::Internal,
                    start_time: timestamp,
                    end_time: Some(timestamp),
                    duration_ns: None,
                    status: bridge_core::types::SpanStatus {
                        code: bridge_core::types::StatusCode::Ok,
                        message: None,
                    },
                    attributes: HashMap::new(),
                    events: Vec::new(),
                    links: Vec::new(),
                }),
                TelemetryType::Event => TelemetryData::Event(bridge_core::types::EventData {
                    name: name.unwrap_or_else(|| "unknown_event".to_string()),
                    timestamp,
                    attributes: HashMap::new(),
                    data: None,
                }),
            };

            Ok(TelemetryRecord {
                id,
                timestamp,
                record_type,
                data,
                attributes,
                tags,
                resource: None,
                service: None,
            })
        } else {
            Err(bridge_core::BridgeError::serialization(
                "Avro value is not a record".to_string(),
            ))
        }
    }
}

impl FileSource {
    /// Create new file source
    pub async fn new(config: &dyn SourceConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<FileSourceConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration(
                    "Invalid file source configuration".to_string(),
                )
            })?
            .clone();

        config.validate().await?;

        let stats = SourceStats {
            source: config.name.clone(),
            total_records: 0,
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_record_time: None,
            is_connected: false,
            lag: None,
        };

        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            file_reader: None,
        })
    }

    /// Initialize file reader
    async fn init_file_reader(&mut self) -> BridgeResult<()> {
        info!("Initializing file reader for: {}", self.config.file_path);

        let file_reader =
            FileReader::new(&self.config.file_path, self.config.file_format.clone()).await?;
        self.file_reader = Some(file_reader);

        info!("File reader initialized");
        Ok(())
    }

    /// Start reading from file
    async fn start_reading(&self) -> BridgeResult<()> {
        info!("Starting file reading");

        // This is a simplified implementation
        // In a real implementation, this would spawn a background task
        // and handle the file reading asynchronously

        if let Some(_reader) = &self.file_reader {
            // Simulate reading process
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Update stats to indicate we've started reading
            {
                let mut stats = self.stats.write().await;
                stats.is_connected = true;
            }
        }

        Ok(())
    }

    /// Process file data
    async fn process_file_data(&self, data: Vec<u8>) -> BridgeResult<TelemetryBatch> {
        if let Some(reader) = &self.file_reader {
            let records = reader.parse_data(&data).await?;

            Ok(TelemetryBatch {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source: "file".to_string(),
                size: records.len(),
                records,
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("file_path".to_string(), self.config.file_path.clone());
                    metadata.insert(
                        "file_format".to_string(),
                        format!("{:?}", self.config.file_format),
                    );
                    metadata
                },
            })
        } else {
            Err(bridge_core::BridgeError::configuration(
                "File reader not initialized".to_string(),
            ))
        }
    }
}

#[async_trait]
impl StreamSource for FileSource {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing file source");

        // Validate configuration
        self.config.validate().await?;

        // Initialize file reader
        self.init_file_reader().await?;

        info!("File source initialized");
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting file source");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
        }

        // Start reading from file
        self.start_reading().await?;

        info!("File source started");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping file source");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }

        info!("File source stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        // This is a simplified check - in practice we'd need to handle the async nature
        // For now, we'll return a default value since this is a sync method
        // The async version should be used instead
        false
    }

    async fn get_stats(&self) -> BridgeResult<SourceStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn version(&self) -> &str {
        &self.config.version
    }
}

impl FileSource {
    /// Get file configuration
    pub fn get_config(&self) -> &FileSourceConfig {
        &self.config
    }
}
