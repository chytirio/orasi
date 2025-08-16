//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry data conversion utilities
//!
//! This module provides utilities for converting between different telemetry
//! data formats and protocols.

use arrow_array::{
    Array, Float64Array, RecordBatch, StringArray, TimestampNanosecondArray, UInt64Array,
};
use arrow_ipc::{reader::StreamReader, writer::StreamWriter};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use bridge_core::types::{
    LogData, LogLevel, MetricData, MetricType, MetricValue, SpanKind, StatusCode, TelemetryData,
    TelemetryRecord, TelemetryType, TraceData,
};
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use opentelemetry_proto::tonic::{
    collector::{
        logs::v1::ExportLogsServiceRequest, metrics::v1::ExportMetricsServiceRequest,
        trace::v1::ExportTraceServiceRequest,
    },
    common::v1::{any_value::Value as AnyValueValue, AnyValue, KeyValue},
    logs::v1::{LogRecord, ResourceLogs, ScopeLogs},
    metrics::v1::{
        metric::Data as MetricDataProto, number_data_point::Value as NumberDataPointValue, Metric,
        NumberDataPoint, ResourceMetrics, ScopeMetrics,
    },
    resource::v1::Resource,
    trace::v1::{ResourceSpans, ScopeSpans, Span},
};
use prost::Message;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Data validation and sanitization utilities
pub struct DataValidator {
    metric_name_regex: Regex,
    label_key_regex: Regex,
    max_string_length: usize,
    max_attributes: usize,
    max_labels: usize,
}

impl DataValidator {
    /// Create new data validator with default settings
    pub fn new() -> Self {
        Self {
            metric_name_regex: Regex::new(r"^[a-zA-Z_:][a-zA-Z0-9_:]*$").unwrap(),
            label_key_regex: Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap(),
            max_string_length: 1024,
            max_attributes: 100,
            max_labels: 50,
        }
    }

    /// Create validator with custom settings
    pub fn with_limits(max_string_length: usize, max_attributes: usize, max_labels: usize) -> Self {
        Self {
            metric_name_regex: Regex::new(r"^[a-zA-Z_:][a-zA-Z0-9_:]*$").unwrap(),
            label_key_regex: Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap(),
            max_string_length,
            max_attributes,
            max_labels,
        }
    }

    /// Validate and sanitize telemetry batch
    pub fn validate_batch(&self, batch: &mut TelemetryBatch) -> BridgeResult<()> {
        info!(
            "Validating telemetry batch with {} records",
            batch.records.len()
        );

        // Validate batch metadata
        self.validate_metadata(&batch.metadata)?;

        // Validate and sanitize each record
        let mut valid_records = Vec::new();
        for record in &batch.records {
            if let Ok(sanitized_record) = self.validate_record(record) {
                valid_records.push(sanitized_record);
            } else {
                info!("Skipping invalid record: {}", record.id);
            }
        }

        batch.records = valid_records;
        batch.size = batch.records.len();

        info!("Validation complete: {} valid records", batch.records.len());
        Ok(())
    }

    /// Validate and sanitize individual record
    pub fn validate_record(&self, record: &TelemetryRecord) -> BridgeResult<TelemetryRecord> {
        let mut sanitized_record = record.clone();

        // Validate timestamp
        self.validate_timestamp(&mut sanitized_record.timestamp)?;

        // Validate ID
        self.validate_uuid(&sanitized_record.id)?;

        // Validate and sanitize attributes
        sanitized_record.attributes = self.sanitize_attributes(&record.attributes)?;

        // Validate and sanitize tags
        sanitized_record.tags = self.sanitize_attributes(&record.tags)?;

        // Validate record type specific data
        match &mut sanitized_record.data {
            TelemetryData::Metric(metric_data) => {
                self.validate_metric_data(metric_data)?;
            }
            TelemetryData::Log(log_data) => {
                self.validate_log_data(log_data)?;
            }
            TelemetryData::Trace(trace_data) => {
                self.validate_trace_data(trace_data)?;
            }
            TelemetryData::Event(event_data) => {
                self.validate_event_data(event_data)?;
            }
        }

        Ok(sanitized_record)
    }

    /// Validate timestamp
    fn validate_timestamp(&self, timestamp: &mut DateTime<Utc>) -> BridgeResult<()> {
        let now = Utc::now();
        let min_time = now - chrono::Duration::days(30); // Allow records up to 30 days old
        let max_time = now + chrono::Duration::hours(1); // Allow records up to 1 hour in future

        if *timestamp < min_time || *timestamp > max_time {
            info!(
                "Timestamp out of range, using current time: {:?}",
                timestamp
            );
            *timestamp = now;
        }

        Ok(())
    }

    /// Validate UUID
    fn validate_uuid(&self, id: &Uuid) -> BridgeResult<()> {
        if id.is_nil() {
            return Err(bridge_core::BridgeError::validation(
                "Invalid UUID: nil UUID".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate and sanitize attributes
    fn sanitize_attributes(
        &self,
        attributes: &HashMap<String, String>,
    ) -> BridgeResult<HashMap<String, String>> {
        if attributes.len() > self.max_attributes {
            return Err(bridge_core::BridgeError::validation(format!(
                "Too many attributes: {} (max: {})",
                attributes.len(),
                self.max_attributes
            )));
        }

        let mut sanitized = HashMap::new();
        for (key, value) in attributes {
            let sanitized_key = self.sanitize_string(key)?;
            let sanitized_value = self.sanitize_string(value)?;

            if !sanitized_key.is_empty() && !sanitized_value.is_empty() {
                sanitized.insert(sanitized_key, sanitized_value);
            }
        }

        Ok(sanitized)
    }

    /// Validate metric data
    fn validate_metric_data(&self, metric_data: &mut MetricData) -> BridgeResult<()> {
        // Validate metric name
        if !self.metric_name_regex.is_match(&metric_data.name) {
            return Err(bridge_core::BridgeError::validation(format!(
                "Invalid metric name: {}",
                metric_data.name
            )));
        }

        // Sanitize metric name
        metric_data.name = self.sanitize_string(&metric_data.name)?;

        // Validate description
        if let Some(ref mut desc) = metric_data.description {
            *desc = self.sanitize_string(desc)?;
        }

        // Validate unit
        if let Some(ref mut unit) = metric_data.unit {
            *unit = self.sanitize_string(unit)?;
        }

        // Validate value
        self.validate_metric_value(&metric_data.value)?;

        // Validate and sanitize labels
        if metric_data.labels.len() > self.max_labels {
            return Err(bridge_core::BridgeError::validation(format!(
                "Too many labels: {} (max: {})",
                metric_data.labels.len(),
                self.max_labels
            )));
        }

        let mut sanitized_labels = HashMap::new();
        for (key, value) in &metric_data.labels {
            if !self.label_key_regex.is_match(key) {
                info!("Skipping invalid label key: {}", key);
                continue;
            }

            let sanitized_key = self.sanitize_string(key)?;
            let sanitized_value = self.sanitize_string(value)?;

            if !sanitized_key.is_empty() && !sanitized_value.is_empty() {
                sanitized_labels.insert(sanitized_key, sanitized_value);
            }
        }
        metric_data.labels = sanitized_labels;

        // Validate timestamp
        self.validate_timestamp(&mut metric_data.timestamp)?;

        Ok(())
    }

    /// Validate metric value
    fn validate_metric_value(&self, value: &MetricValue) -> BridgeResult<()> {
        match value {
            MetricValue::Gauge(v) | MetricValue::Counter(v) => {
                if !v.is_finite() {
                    return Err(bridge_core::BridgeError::validation(
                        "Invalid metric value: non-finite".to_string(),
                    ));
                }
                if *v < -1e9 || *v > 1e9 {
                    return Err(bridge_core::BridgeError::validation(
                        "Metric value out of reasonable range".to_string(),
                    ));
                }
            }
            MetricValue::Histogram {
                buckets: _,
                sum: _,
                count: _,
            } => {
                // Histogram validation is handled separately
            }
            MetricValue::Summary { .. } => {
                // Summary validation is handled separately
            }
        }
        Ok(())
    }

    /// Validate log data
    fn validate_log_data(&self, log_data: &mut LogData) -> BridgeResult<()> {
        // Validate timestamp
        self.validate_timestamp(&mut log_data.timestamp)?;

        // Sanitize message
        log_data.message = self.sanitize_string(&log_data.message)?;

        // Validate severity number
        if let Some(severity) = log_data.severity_number {
            if severity > 24 {
                log_data.severity_number = Some(9); // Default to INFO
            }
        }

        // Sanitize severity text
        if let Some(ref mut text) = log_data.severity_text {
            *text = self.sanitize_string(text)?;
        }

        // Sanitize body
        if let Some(ref mut body) = log_data.body {
            *body = self.sanitize_string(body)?;
        }

        // Validate and sanitize attributes
        log_data.attributes = self.sanitize_attributes(&log_data.attributes)?;

        Ok(())
    }

    /// Validate trace data
    fn validate_trace_data(&self, trace_data: &mut TraceData) -> BridgeResult<()> {
        // Validate trace ID
        if trace_data.trace_id.is_empty() {
            return Err(bridge_core::BridgeError::validation(
                "Empty trace ID".to_string(),
            ));
        }

        // Validate span ID
        if trace_data.span_id.is_empty() {
            return Err(bridge_core::BridgeError::validation(
                "Empty span ID".to_string(),
            ));
        }

        // Sanitize trace ID
        trace_data.trace_id = self.sanitize_string(&trace_data.trace_id)?;
        trace_data.span_id = self.sanitize_string(&trace_data.span_id)?;

        // Sanitize parent span ID
        if let Some(ref mut parent_id) = trace_data.parent_span_id {
            *parent_id = self.sanitize_string(parent_id)?;
        }

        // Sanitize name
        trace_data.name = self.sanitize_string(&trace_data.name)?;

        // Validate timestamps
        self.validate_timestamp(&mut trace_data.start_time)?;
        if let Some(ref mut end_time) = trace_data.end_time {
            self.validate_timestamp(end_time)?;

            if *end_time < trace_data.start_time {
                return Err(bridge_core::BridgeError::validation(
                    "End time before start time".to_string(),
                ));
            }
        }

        // Validate and sanitize attributes
        trace_data.attributes = self.sanitize_attributes(&trace_data.attributes)?;

        Ok(())
    }

    /// Validate event data
    fn validate_event_data(
        &self,
        event_data: &mut bridge_core::types::EventData,
    ) -> BridgeResult<()> {
        // Validate event name
        if event_data.name.trim().is_empty() {
            return Err(bridge_core::BridgeError::validation(
                "Empty event name".to_string(),
            ));
        }

        // Sanitize event name
        event_data.name = self.sanitize_string(&event_data.name)?;

        // Validate timestamp
        self.validate_timestamp(&mut event_data.timestamp)?;

        // Validate and sanitize attributes
        event_data.attributes = self.sanitize_attributes(&event_data.attributes)?;

        // Validate event data (optional JSON value)
        if let Some(ref mut data) = event_data.data {
            // For now, we'll just validate that it's valid JSON
            // In a more comprehensive implementation, we might validate the structure
            if let serde_json::Value::String(s) = data {
                let sanitized = self.sanitize_string(s)?;
                *data = serde_json::Value::String(sanitized);
            }
        }

        Ok(())
    }

    /// Validate metadata
    fn validate_metadata(&self, metadata: &HashMap<String, String>) -> BridgeResult<()> {
        if metadata.len() > self.max_attributes {
            return Err(bridge_core::BridgeError::validation(format!(
                "Too many metadata entries: {} (max: {})",
                metadata.len(),
                self.max_attributes
            )));
        }

        for (key, value) in metadata {
            if key.len() > self.max_string_length || value.len() > self.max_string_length {
                return Err(bridge_core::BridgeError::validation(
                    "Metadata key or value too long".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Sanitize string
    fn sanitize_string(&self, input: &str) -> BridgeResult<String> {
        if input.len() > self.max_string_length {
            return Err(bridge_core::BridgeError::validation(format!(
                "String too long: {} (max: {})",
                input.len(),
                self.max_string_length
            )));
        }

        // Remove null bytes and control characters
        let sanitized = input
            .chars()
            .filter(|&c| c != '\0' && !c.is_control())
            .collect::<String>();

        Ok(sanitized)
    }
}

impl Default for DataValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Conversion trait for telemetry data
#[async_trait::async_trait]
pub trait TelemetryConverter: Send + Sync {
    /// Convert from source format to target format
    async fn convert(
        &self,
        data: &[u8],
        source_format: &str,
        target_format: &str,
    ) -> BridgeResult<Vec<u8>>;

    /// Get supported source formats
    fn supported_source_formats(&self) -> Vec<String>;

    /// Get supported target formats
    fn supported_target_formats(&self) -> Vec<String>;
}

/// OTLP converter implementation
pub struct OtlpConverter;

#[async_trait::async_trait]
impl TelemetryConverter for OtlpConverter {
    async fn convert(
        &self,
        data: &[u8],
        source_format: &str,
        target_format: &str,
    ) -> BridgeResult<Vec<u8>> {
        match (source_format, target_format) {
            ("otlp", "internal") => {
                let batch = self.convert_from_otlp(data).await?;
                // Serialize to internal format using serde_json
                let serialized = serde_json::to_vec(&batch).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to serialize TelemetryBatch to JSON: {}",
                        e
                    ))
                })?;
                Ok(serialized)
            }
            ("internal", "otlp") => {
                // Deserialize from internal format using serde_json
                let batch: TelemetryBatch = serde_json::from_slice(data).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to deserialize TelemetryBatch from JSON: {}",
                        e
                    ))
                })?;
                self.convert_to_otlp(&batch).await
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported conversion: {} -> {}",
                source_format, target_format
            ))),
        }
    }

    fn supported_source_formats(&self) -> Vec<String> {
        vec!["otlp".to_string(), "internal".to_string()]
    }

    fn supported_target_formats(&self) -> Vec<String> {
        vec!["otlp".to_string(), "internal".to_string()]
    }
}

impl OtlpConverter {
    /// Create new OTLP converter
    pub fn new() -> Self {
        Self
    }

    /// Convert OTLP data to internal format
    pub async fn convert_from_otlp(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        info!(
            "Converting OTLP data of {} bytes to internal format",
            data.len()
        );

        // Try to decode as different OTLP message types
        if let Ok(metrics_request) = ExportMetricsServiceRequest::decode(data) {
            return self.convert_otlp_metrics(metrics_request).await;
        }

        if let Ok(logs_request) = ExportLogsServiceRequest::decode(data) {
            return self.convert_otlp_logs(logs_request).await;
        }

        if let Ok(traces_request) = ExportTraceServiceRequest::decode(data) {
            return self.convert_otlp_traces(traces_request).await;
        }

        Err(bridge_core::BridgeError::serialization(
            "Failed to decode OTLP data as any known message type".to_string(),
        ))
    }

    /// Convert OTLP metrics to internal format
    async fn convert_otlp_metrics(
        &self,
        request: ExportMetricsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        let mut all_records = Vec::new();

        for resource_metrics in request.resource_metrics {
            let resource_attrs = self.convert_attributes(&resource_metrics.resource);

            for scope_metrics in resource_metrics.scope_metrics {
                for metric in scope_metrics.metrics {
                    let records = self.convert_otlp_metric(metric, &resource_attrs).await?;
                    all_records.extend(records);
                }
            }
        }

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp_metrics".to_string(),
            size: all_records.len(),
            records: all_records,
            metadata: HashMap::new(),
        })
    }

    /// Convert a single OTLP metric to internal records
    async fn convert_otlp_metric(
        &self,
        metric: Metric,
        resource_attrs: &HashMap<String, String>,
    ) -> BridgeResult<Vec<TelemetryRecord>> {
        let mut records = Vec::new();

        match metric.data {
            Some(MetricDataProto::Gauge(gauge)) => {
                for point in gauge.data_points {
                    let value = match point.value {
                        Some(NumberDataPointValue::AsDouble(v)) => MetricValue::Gauge(v),
                        Some(NumberDataPointValue::AsInt(v)) => MetricValue::Gauge(v as f64),
                        None => continue,
                    };

                    let timestamp = if point.time_unix_nano > 0 {
                        DateTime::from_timestamp_nanos(point.time_unix_nano as i64)
                    } else {
                        Utc::now()
                    };

                    let record = TelemetryRecord {
                        id: Uuid::new_v4(),
                        timestamp,
                        record_type: TelemetryType::Metric,
                        data: TelemetryData::Metric(MetricData {
                            name: metric.name.clone(),
                            description: Some(metric.description.clone()),
                            unit: Some(metric.unit.clone()),
                            metric_type: MetricType::Gauge,
                            value,
                            labels: self.convert_key_values(&point.attributes),
                            timestamp,
                        }),
                        attributes: resource_attrs.clone(),
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    };

                    records.push(record);
                }
            }
            Some(MetricDataProto::Sum(sum)) => {
                for point in sum.data_points {
                    let value = match point.value {
                        Some(NumberDataPointValue::AsDouble(v)) => MetricValue::Gauge(v),
                        Some(NumberDataPointValue::AsInt(v)) => MetricValue::Gauge(v as f64),
                        None => continue,
                    };

                    let timestamp = if point.time_unix_nano > 0 {
                        DateTime::from_timestamp_nanos(point.time_unix_nano as i64)
                    } else {
                        Utc::now()
                    };

                    let metric_type = if sum.is_monotonic {
                        MetricType::Counter
                    } else {
                        MetricType::Gauge
                    };

                    let record = TelemetryRecord {
                        id: Uuid::new_v4(),
                        timestamp,
                        record_type: TelemetryType::Metric,
                        data: TelemetryData::Metric(MetricData {
                            name: metric.name.clone(),
                            description: Some(metric.description.clone()),
                            unit: Some(metric.unit.clone()),
                            metric_type,
                            value,
                            labels: self.convert_key_values(&point.attributes),
                            timestamp,
                        }),
                        attributes: resource_attrs.clone(),
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    };

                    records.push(record);
                }
            }
            Some(MetricDataProto::Histogram(histogram)) => {
                // For histogram, we'll create a summary metric
                for point in histogram.data_points {
                    let timestamp = if point.time_unix_nano > 0 {
                        DateTime::from_timestamp_nanos(point.time_unix_nano as i64)
                    } else {
                        Utc::now()
                    };

                    // Use count as the primary value for histogram
                    let value = MetricValue::Gauge(point.count as f64);

                    let record = TelemetryRecord {
                        id: Uuid::new_v4(),
                        timestamp,
                        record_type: TelemetryType::Metric,
                        data: TelemetryData::Metric(MetricData {
                            name: metric.name.clone(),
                            description: Some(metric.description.clone()),
                            unit: Some(metric.unit.clone()),
                            metric_type: MetricType::Histogram,
                            value,
                            labels: self.convert_key_values(&point.attributes),
                            timestamp,
                        }),
                        attributes: resource_attrs.clone(),
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    };

                    records.push(record);
                }
            }
            _ => {
                info!("Unsupported metric type for metric: {}", metric.name);
            }
        }

        Ok(records)
    }

    /// Convert OTLP logs to internal format
    async fn convert_otlp_logs(
        &self,
        request: ExportLogsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        let mut all_records = Vec::new();

        for resource_logs in request.resource_logs {
            let resource_attrs = self.convert_attributes(&resource_logs.resource);

            for scope_logs in resource_logs.scope_logs {
                for log_record in scope_logs.log_records {
                    let record = self.convert_otlp_log(log_record, &resource_attrs).await?;
                    all_records.push(record);
                }
            }
        }

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp_logs".to_string(),
            size: all_records.len(),
            records: all_records,
            metadata: HashMap::new(),
        })
    }

    /// Convert a single OTLP log to internal record
    async fn convert_otlp_log(
        &self,
        log: LogRecord,
        resource_attrs: &HashMap<String, String>,
    ) -> BridgeResult<TelemetryRecord> {
        let timestamp = if log.time_unix_nano > 0 {
            DateTime::from_timestamp_nanos(log.time_unix_nano as i64)
        } else {
            Utc::now()
        };

        let level = match log.severity_number {
            1..=4 => LogLevel::Trace,
            5..=8 => LogLevel::Debug,
            9..=12 => LogLevel::Info,
            13..=16 => LogLevel::Warn,
            17..=20 => LogLevel::Error,
            21..=24 => LogLevel::Fatal,
            _ => LogLevel::Info,
        };

        let message = log
            .body
            .and_then(|body| body.value)
            .map(|value| match value {
                AnyValueValue::StringValue(s) => s,
                AnyValueValue::IntValue(i) => i.to_string(),
                AnyValueValue::DoubleValue(d) => d.to_string(),
                AnyValueValue::BoolValue(b) => b.to_string(),
                _ => "".to_string(),
            })
            .unwrap_or_else(|| "".to_string());

        Ok(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp,
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp,
                level,
                message: message.clone(),
                attributes: self.convert_key_values(&log.attributes),
                body: Some(message),
                severity_number: Some(log.severity_number.try_into().unwrap_or(9)),
                severity_text: Some(log.severity_text),
            }),
            attributes: resource_attrs.clone(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        })
    }

    /// Convert OTLP traces to internal format
    async fn convert_otlp_traces(
        &self,
        request: ExportTraceServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        let mut all_records = Vec::new();

        for resource_spans in request.resource_spans {
            let resource_attrs = self.convert_attributes(&resource_spans.resource);

            for scope_spans in resource_spans.scope_spans {
                for span in scope_spans.spans {
                    let record = self.convert_otlp_span(span, &resource_attrs).await?;
                    all_records.push(record);
                }
            }
        }

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp_traces".to_string(),
            size: all_records.len(),
            records: all_records,
            metadata: HashMap::new(),
        })
    }

    /// Convert a single OTLP span to internal record
    async fn convert_otlp_span(
        &self,
        span: Span,
        resource_attrs: &HashMap<String, String>,
    ) -> BridgeResult<TelemetryRecord> {
        let timestamp = if span.start_time_unix_nano > 0 {
            DateTime::from_timestamp_nanos(span.start_time_unix_nano as i64)
        } else {
            Utc::now()
        };

        let kind = match span.kind() {
            opentelemetry_proto::tonic::trace::v1::span::SpanKind::Internal => SpanKind::Internal,
            opentelemetry_proto::tonic::trace::v1::span::SpanKind::Server => SpanKind::Server,
            opentelemetry_proto::tonic::trace::v1::span::SpanKind::Client => SpanKind::Client,
            opentelemetry_proto::tonic::trace::v1::span::SpanKind::Producer => SpanKind::Producer,
            opentelemetry_proto::tonic::trace::v1::span::SpanKind::Consumer => SpanKind::Consumer,
            _ => SpanKind::Internal,
        };

        let status = match span.status.as_ref() {
            Some(status) => match status.code() {
                opentelemetry_proto::tonic::trace::v1::status::StatusCode::Ok => StatusCode::Ok,
                opentelemetry_proto::tonic::trace::v1::status::StatusCode::Error => {
                    StatusCode::Error
                }
                _ => StatusCode::Unset,
            },
            None => StatusCode::Unset,
        };

        Ok(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp,
            record_type: TelemetryType::Trace,
            data: TelemetryData::Trace(TraceData {
                trace_id: hex::encode(span.trace_id),
                span_id: hex::encode(span.span_id),
                parent_span_id: Some(hex::encode(span.parent_span_id)),
                name: span.name,
                kind,
                status: bridge_core::types::SpanStatus {
                    code: status,
                    message: None,
                },
                start_time: timestamp,
                end_time: if span.end_time_unix_nano > 0 {
                    Some(DateTime::from_timestamp_nanos(
                        span.end_time_unix_nano as i64,
                    ))
                } else {
                    None
                },
                duration_ns: if span.start_time_unix_nano > 0 && span.end_time_unix_nano > 0 {
                    Some((span.end_time_unix_nano - span.start_time_unix_nano) as u64)
                } else {
                    None
                },
                attributes: self.convert_key_values(&span.attributes),
                events: self.convert_otlp_span_events(&span.events),
                links: self.convert_otlp_span_links(&span.links),
            }),
            attributes: resource_attrs.clone(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        })
    }

    /// Convert KeyValue list to HashMap
    fn convert_key_values(&self, key_values: &[KeyValue]) -> HashMap<String, String> {
        key_values.iter().fold(HashMap::new(), |mut acc, kv| {
            let value = match &kv.value {
                Some(any_value) => match &any_value.value {
                    Some(AnyValueValue::StringValue(s)) => s.clone(),
                    Some(AnyValueValue::IntValue(i)) => i.to_string(),
                    Some(AnyValueValue::DoubleValue(d)) => d.to_string(),
                    Some(AnyValueValue::BoolValue(b)) => b.to_string(),
                    _ => "".to_string(),
                },
                None => "".to_string(),
            };
            acc.insert(kv.key.clone(), value);
            acc
        })
    }

    /// Convert OTLP span events to internal format
    fn convert_span_events(
        &self,
        events: &[bridge_core::types::SpanEvent],
    ) -> Vec<bridge_core::types::SpanEvent> {
        events
            .iter()
            .map(|event| bridge_core::types::SpanEvent {
                name: event.name.clone(),
                timestamp: event.timestamp,
                attributes: event.attributes.clone(),
            })
            .collect()
    }

    /// Convert OTLP span links to internal format
    fn convert_span_links(
        &self,
        links: &[bridge_core::types::SpanLink],
    ) -> Vec<bridge_core::types::SpanLink> {
        links
            .iter()
            .map(|link| bridge_core::types::SpanLink {
                trace_id: link.trace_id.clone(),
                span_id: link.span_id.clone(),
                attributes: link.attributes.clone(),
            })
            .collect()
    }

    /// Convert OTLP span events to internal format
    fn convert_otlp_span_events(
        &self,
        events: &[opentelemetry_proto::tonic::trace::v1::span::Event],
    ) -> Vec<bridge_core::types::SpanEvent> {
        events
            .iter()
            .map(|event| bridge_core::types::SpanEvent {
                name: event.name.clone(),
                timestamp: if event.time_unix_nano > 0 {
                    DateTime::from_timestamp_nanos(event.time_unix_nano as i64)
                } else {
                    Utc::now()
                },
                attributes: self.convert_key_values(&event.attributes),
            })
            .collect()
    }

    /// Convert OTLP span links to internal format
    fn convert_otlp_span_links(
        &self,
        links: &[opentelemetry_proto::tonic::trace::v1::span::Link],
    ) -> Vec<bridge_core::types::SpanLink> {
        links
            .iter()
            .map(|link| bridge_core::types::SpanLink {
                trace_id: hex::encode(&link.trace_id),
                span_id: hex::encode(&link.span_id),
                attributes: self.convert_key_values(&link.attributes),
            })
            .collect()
    }

    /// Convert Resource attributes
    fn convert_attributes(&self, resource: &Option<Resource>) -> HashMap<String, String> {
        resource
            .as_ref()
            .map(|r| self.convert_key_values(&r.attributes))
            .unwrap_or_default()
    }

    /// Convert internal format to OTLP
    pub async fn convert_to_otlp(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        info!(
            "Converting internal format to OTLP for batch with {} records",
            batch.records.len()
        );

        // Group records by type
        let mut metrics = Vec::new();
        let mut logs = Vec::new();
        let mut traces = Vec::new();

        for record in &batch.records {
            match &record.record_type {
                TelemetryType::Metric => {
                    if let TelemetryData::Metric(metric_data) = &record.data {
                        let otlp_metric = self.convert_to_otlp_metric(metric_data, record).await?;
                        metrics.push(otlp_metric);
                    }
                }
                TelemetryType::Log => {
                    if let TelemetryData::Log(log_data) = &record.data {
                        let otlp_log = self.convert_to_otlp_log(log_data, record).await?;
                        logs.push(otlp_log);
                    }
                }
                TelemetryType::Trace => {
                    if let TelemetryData::Trace(trace_data) = &record.data {
                        let otlp_span = self.convert_to_otlp_span(trace_data, record).await?;
                        traces.push(otlp_span);
                    }
                }
                TelemetryType::Event => {
                    if let TelemetryData::Event(event_data) = &record.data {
                        // Convert event to log since OTLP doesn't have native events
                        let otlp_log = self.convert_event_to_otlp_log(event_data, record).await?;
                        logs.push(otlp_log);
                    }
                }
            }
        }

        // Create appropriate OTLP request based on content
        if !metrics.is_empty() {
            let mut request = ExportMetricsServiceRequest::default();
            request.resource_metrics = vec![ResourceMetrics {
                schema_url: "".to_string(),
                resource: Some(Resource {
                    attributes: self.convert_to_key_values(&batch.metadata),
                    dropped_attributes_count: 0,
                    entity_refs: vec![],
                }),
                scope_metrics: vec![ScopeMetrics {
                    scope: None,
                    metrics,
                    schema_url: "".to_string(),
                }],
            }];

            let mut buf = Vec::new();
            request.encode(&mut buf).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to encode OTLP metrics: {}",
                    e
                ))
            })?;
            return Ok(buf);
        }

        if !logs.is_empty() {
            let mut request = ExportLogsServiceRequest::default();
            request.resource_logs = vec![ResourceLogs {
                schema_url: "".to_string(),
                resource: Some(Resource {
                    attributes: self.convert_to_key_values(&batch.metadata),
                    dropped_attributes_count: 0,
                    entity_refs: vec![],
                }),
                scope_logs: vec![ScopeLogs {
                    schema_url: "".to_string(),
                    scope: None,
                    log_records: logs,
                }],
            }];

            let mut buf = Vec::new();
            request.encode(&mut buf).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to encode OTLP logs: {}",
                    e
                ))
            })?;
            return Ok(buf);
        }

        if !traces.is_empty() {
            let request = ExportTraceServiceRequest {
                resource_spans: vec![ResourceSpans {
                    schema_url: "".to_string(),
                    resource: Some(Resource {
                        dropped_attributes_count: 0,
                        attributes: self.convert_to_key_values(&batch.metadata),
                        entity_refs: vec![],
                    }),
                    scope_spans: vec![ScopeSpans {
                        schema_url: "".to_string(),
                        scope: None,
                        spans: traces,
                    }],
                }],
            };

            let mut buf = Vec::new();
            request.encode(&mut buf).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to encode OTLP traces: {}",
                    e
                ))
            })?;
            return Ok(buf);
        }

        // If no records, return empty metrics request
        let request = ExportMetricsServiceRequest {
            resource_metrics: vec![],
        };

        let mut buf = Vec::new();
        request.encode(&mut buf).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to encode empty OTLP request: {}",
                e
            ))
        })?;
        Ok(buf)
    }

    /// Convert internal metric to OTLP metric
    async fn convert_to_otlp_metric(
        &self,
        metric_data: &MetricData,
        record: &TelemetryRecord,
    ) -> BridgeResult<Metric> {
        let data_point = NumberDataPoint {
            attributes: self.convert_to_key_values(&metric_data.labels),
            time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
            value: Some(match metric_data.value {
                MetricValue::Gauge(v) => NumberDataPointValue::AsDouble(v),
                MetricValue::Counter(v) => NumberDataPointValue::AsDouble(v),
                MetricValue::Histogram {
                    buckets: _,
                    sum: _,
                    count: _,
                } => NumberDataPointValue::AsDouble(0.0),
                MetricValue::Summary { .. } => NumberDataPointValue::AsDouble(0.0),
            }),
            ..Default::default()
        };

        let metric_data_proto = match metric_data.metric_type {
            MetricType::Gauge => {
                MetricDataProto::Gauge(opentelemetry_proto::tonic::metrics::v1::Gauge {
                    data_points: vec![data_point],
                })
            }
            MetricType::Counter => {
                MetricDataProto::Sum(opentelemetry_proto::tonic::metrics::v1::Sum {
                    data_points: vec![data_point],
                    is_monotonic: true,
                    aggregation_temporality: 1, // Cumulative
                })
            }
            MetricType::Histogram => {
                MetricDataProto::Histogram(opentelemetry_proto::tonic::metrics::v1::Histogram {
                    data_points: vec![
                        opentelemetry_proto::tonic::metrics::v1::HistogramDataPoint {
                            attributes: self.convert_to_key_values(&metric_data.labels),
                            time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0)
                                as u64,
                            count: match &metric_data.value {
                                MetricValue::Gauge(v) => *v as u64,
                                MetricValue::Counter(v) => *v as u64,
                                MetricValue::Histogram {
                                    buckets: _,
                                    sum: _,
                                    count: _,
                                } => 0,
                                MetricValue::Summary { .. } => 0,
                            },
                            ..Default::default()
                        },
                    ],
                    aggregation_temporality: 1, // Cumulative
                })
            }
            MetricType::Summary => {
                MetricDataProto::Summary(opentelemetry_proto::tonic::metrics::v1::Summary {
                    data_points: vec![opentelemetry_proto::tonic::metrics::v1::SummaryDataPoint {
                        attributes: self.convert_to_key_values(&metric_data.labels),
                        time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
                        count: match &metric_data.value {
                            MetricValue::Gauge(v) => *v as u64,
                            MetricValue::Counter(v) => *v as u64,
                            MetricValue::Histogram {
                                buckets: _,
                                sum: _,
                                count: _,
                            } => 0,
                            MetricValue::Summary { .. } => 0,
                        },
                        ..Default::default()
                    }],
                })
            }
        };

        Ok(Metric {
            name: metric_data.name.clone(),
            description: metric_data.description.clone().unwrap_or_default(),
            unit: metric_data.unit.clone().unwrap_or_default(),
            data: Some(metric_data_proto),
            metadata: vec![],
        })
    }

    /// Convert internal log to OTLP log
    async fn convert_to_otlp_log(
        &self,
        log_data: &LogData,
        record: &TelemetryRecord,
    ) -> BridgeResult<LogRecord> {
        let severity_number = match log_data.level {
            LogLevel::Trace => 1,
            LogLevel::Debug => 5,
            LogLevel::Info => 9,
            LogLevel::Warn => 13,
            LogLevel::Error => 17,
            LogLevel::Fatal => 21,
        };

        Ok(LogRecord {
            time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
            severity_number,
            severity_text: log_data.severity_text.clone().unwrap_or_default(),
            body: Some(AnyValue {
                value: Some(AnyValueValue::StringValue(log_data.message.clone())),
            }),
            attributes: self.convert_to_key_values(&log_data.attributes),
            ..Default::default()
        })
    }

    /// Convert internal trace to OTLP span
    async fn convert_to_otlp_span(
        &self,
        trace_data: &TraceData,
        record: &TelemetryRecord,
    ) -> BridgeResult<Span> {
        let kind = match trace_data.kind {
            SpanKind::Internal => opentelemetry_proto::tonic::trace::v1::span::SpanKind::Internal,
            SpanKind::Server => opentelemetry_proto::tonic::trace::v1::span::SpanKind::Server,
            SpanKind::Client => opentelemetry_proto::tonic::trace::v1::span::SpanKind::Client,
            SpanKind::Producer => opentelemetry_proto::tonic::trace::v1::span::SpanKind::Producer,
            SpanKind::Consumer => opentelemetry_proto::tonic::trace::v1::span::SpanKind::Consumer,
        };

        let status = match trace_data.status.code {
            StatusCode::Ok => opentelemetry_proto::tonic::trace::v1::status::StatusCode::Ok,
            StatusCode::Error => opentelemetry_proto::tonic::trace::v1::status::StatusCode::Error,
            StatusCode::Unset => opentelemetry_proto::tonic::trace::v1::status::StatusCode::Unset,
        };

        Ok(Span {
            trace_id: trace_data.trace_id.clone().into(),
            span_id: trace_data.span_id.clone().into(),
            parent_span_id: trace_data
                .parent_span_id
                .clone()
                .map(|id| id.into())
                .unwrap_or_default(),
            name: trace_data.name.clone(),
            kind: kind as i32,
            start_time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
            end_time_unix_nano: trace_data
                .end_time
                .map(|t| t.timestamp_nanos_opt().unwrap_or(0) as u64)
                .unwrap_or(0),
            attributes: self.convert_to_key_values(&trace_data.attributes),
            status: Some(opentelemetry_proto::tonic::trace::v1::Status {
                code: status as i32,
                message: "".to_string(),
            }),
            ..Default::default()
        })
    }

    /// Convert internal event to OTLP log
    async fn convert_event_to_otlp_log(
        &self,
        event_data: &bridge_core::types::EventData,
        record: &TelemetryRecord,
    ) -> BridgeResult<LogRecord> {
        // Convert event to log since OTLP doesn't have native events
        let severity_number = 9; // INFO level for events

        let message = if let Some(data) = &event_data.data {
            format!(
                "Event: {} - {}",
                event_data.name,
                serde_json::to_string(data).unwrap_or_default()
            )
        } else {
            format!("Event: {}", event_data.name)
        };

        Ok(LogRecord {
            time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
            severity_number,
            severity_text: "INFO".to_string(),
            body: Some(AnyValue {
                value: Some(AnyValueValue::StringValue(message)),
            }),
            attributes: self.convert_to_key_values(&event_data.attributes),
            ..Default::default()
        })
    }

    /// Convert HashMap to KeyValue list
    fn convert_to_key_values(&self, map: &HashMap<String, String>) -> Vec<KeyValue> {
        map.iter()
            .map(|(k, v)| KeyValue {
                key: k.clone(),
                value: Some(AnyValue {
                    value: Some(AnyValueValue::StringValue(v.clone())),
                }),
            })
            .collect()
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
            data: TelemetryData::Trace(TraceData {
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

/// JSON converter implementation
pub struct JsonConverter;

#[async_trait::async_trait]
impl TelemetryConverter for JsonConverter {
    async fn convert(
        &self,
        data: &[u8],
        source_format: &str,
        target_format: &str,
    ) -> BridgeResult<Vec<u8>> {
        match (source_format, target_format) {
            ("json", "internal") => {
                let batch = self.convert_from_json(data).await?;
                // Serialize to internal format using serde_json
                let serialized = serde_json::to_vec(&batch).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to serialize TelemetryBatch to JSON: {}",
                        e
                    ))
                })?;
                Ok(serialized)
            }
            ("internal", "json") => {
                // Deserialize from internal format using serde_json
                let batch: TelemetryBatch = serde_json::from_slice(data).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to deserialize TelemetryBatch from JSON: {}",
                        e
                    ))
                })?;
                self.convert_to_json(&batch).await
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported conversion: {} -> {}",
                source_format, target_format
            ))),
        }
    }

    fn supported_source_formats(&self) -> Vec<String> {
        vec!["json".to_string(), "internal".to_string()]
    }

    fn supported_target_formats(&self) -> Vec<String> {
        vec!["json".to_string(), "internal".to_string()]
    }
}

impl JsonConverter {
    /// Create new JSON converter
    pub fn new() -> Self {
        Self
    }

    /// Convert JSON data to internal format
    pub async fn convert_from_json(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        // Parse JSON data and convert to TelemetryBatch
        let batch: TelemetryBatch = serde_json::from_slice(data).map_err(|e| {
            bridge_core::BridgeError::serialization(format!("Failed to parse JSON data: {}", e))
        })?;

        Ok(batch)
    }

    /// Convert internal format to JSON
    pub async fn convert_to_json(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        // Convert TelemetryBatch to JSON format
        let json_data = serde_json::to_vec_pretty(batch).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to serialize TelemetryBatch to JSON: {}",
                e
            ))
        })?;

        Ok(json_data)
    }
}

/// Conversion factory
pub struct ConversionFactory;

impl ConversionFactory {
    /// Create converter for specific format
    pub fn create_converter(format: &str) -> Option<Box<dyn TelemetryConverter>> {
        match format {
            "otlp" => Some(Box::new(OtlpConverter::new())),
            "arrow" => Some(Box::new(ArrowConverter::new())),
            "json" => Some(Box::new(JsonConverter::new())),
            _ => None,
        }
    }

    /// Get supported formats
    pub fn get_supported_formats() -> Vec<String> {
        vec!["otlp".to_string(), "arrow".to_string(), "json".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::types::{LogLevel, SpanKind, StatusCode};

    fn create_test_telemetry_batch() -> TelemetryBatch {
        let records = vec![
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "test_metric".to_string(),
                    description: Some("Test metric".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: MetricValue::Gauge(42.0),
                    labels: HashMap::from([
                        ("label1".to_string(), "value1".to_string()),
                        ("label2".to_string(), "value2".to_string()),
                    ]),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([("attr1".to_string(), "value1".to_string())]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Log,
                data: TelemetryData::Log(LogData {
                    timestamp: Utc::now(),
                    level: LogLevel::Info,
                    message: "Test log message".to_string(),
                    attributes: HashMap::from([("log_attr".to_string(), "log_value".to_string())]),
                    body: Some("Test log body".to_string()),
                    severity_number: Some(9),
                    severity_text: Some("INFO".to_string()),
                }),
                attributes: HashMap::from([("log_attr".to_string(), "log_value".to_string())]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
        ];

        TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::from([("test_key".to_string(), "test_value".to_string())]),
        }
    }

    #[tokio::test]
    async fn test_otlp_converter_creation() {
        let converter = OtlpConverter::new();
        assert_eq!(
            converter.supported_source_formats(),
            vec!["otlp", "internal"]
        );
        assert_eq!(
            converter.supported_target_formats(),
            vec!["otlp", "internal"]
        );
    }

    #[tokio::test]
    async fn test_arrow_converter_creation() {
        let converter = ArrowConverter::new();
        assert_eq!(
            converter.supported_source_formats(),
            vec!["arrow", "internal"]
        );
        assert_eq!(
            converter.supported_target_formats(),
            vec!["arrow", "internal"]
        );
    }

    #[tokio::test]
    async fn test_json_converter_creation() {
        let converter = JsonConverter::new();
        assert_eq!(
            converter.supported_source_formats(),
            vec!["json", "internal"]
        );
        assert_eq!(
            converter.supported_target_formats(),
            vec!["json", "internal"]
        );
    }

    #[tokio::test]
    async fn test_json_converter_roundtrip() {
        let converter = JsonConverter::new();
        let batch = create_test_telemetry_batch();

        // Convert to JSON
        let json_data = converter.convert_to_json(&batch).await.unwrap();
        assert!(!json_data.is_empty());

        // Convert back from JSON
        let converted_batch = converter.convert_from_json(&json_data).await.unwrap();
        assert_eq!(converted_batch.records.len(), batch.records.len());
        assert_eq!(converted_batch.source, batch.source);
    }

    #[tokio::test]
    async fn test_conversion_factory() {
        // Test supported formats
        let formats = ConversionFactory::get_supported_formats();
        assert!(formats.contains(&"otlp".to_string()));
        assert!(formats.contains(&"arrow".to_string()));
        assert!(formats.contains(&"json".to_string()));

        // Test converter creation
        assert!(ConversionFactory::create_converter("otlp").is_some());
        assert!(ConversionFactory::create_converter("arrow").is_some());
        assert!(ConversionFactory::create_converter("json").is_some());
        assert!(ConversionFactory::create_converter("unsupported").is_none());
    }

    #[tokio::test]
    async fn test_otlp_converter_unsupported_conversion() {
        let converter = OtlpConverter::new();
        let result = converter.convert(b"test", "otlp", "unsupported").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_arrow_converter_unsupported_conversion() {
        let converter = ArrowConverter::new();
        let result = converter.convert(b"test", "arrow", "unsupported").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_json_converter_unsupported_conversion() {
        let converter = JsonConverter::new();
        let result = converter.convert(b"test", "json", "unsupported").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_otlp_converter_internal_roundtrip() {
        let converter = OtlpConverter::new();
        let batch = create_test_telemetry_batch();

        // Convert to internal format (JSON)
        let internal_data = converter.convert_to_internal(&batch).await.unwrap();
        assert!(!internal_data.is_empty());

        // Convert back from internal format
        let converted_batch = converter
            .convert_from_internal(&internal_data)
            .await
            .unwrap();
        assert_eq!(converted_batch.records.len(), batch.records.len());
        assert_eq!(converted_batch.source, batch.source);
    }

    #[tokio::test]
    async fn test_arrow_converter_internal_roundtrip() {
        let converter = ArrowConverter::new();
        let batch = create_test_telemetry_batch();

        // Convert to internal format (JSON)
        let internal_data = converter.convert_to_internal(&batch).await.unwrap();
        assert!(!internal_data.is_empty());

        // Convert back from internal format
        let converted_batch = converter
            .convert_from_internal(&internal_data)
            .await
            .unwrap();
        assert_eq!(converted_batch.records.len(), batch.records.len());
        assert_eq!(converted_batch.source, batch.source);
    }
}
