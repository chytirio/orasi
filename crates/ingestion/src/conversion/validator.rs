//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Data validation and sanitization utilities for telemetry data

use bridge_core::types::{
    LogData, LogLevel, MetricData, MetricType, MetricValue, SpanKind, StatusCode, TelemetryData,
    TelemetryRecord, TelemetryType,
};
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashMap;
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
    fn validate_trace_data(&self, trace_data: &mut bridge_core::types::TraceData) -> BridgeResult<()> {
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
