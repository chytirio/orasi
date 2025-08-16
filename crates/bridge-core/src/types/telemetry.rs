//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry data structures for the OpenTelemetry Data Lake Bridge
//!
//! This module provides the core telemetry data structures including batches,
//! records, and data types used throughout the bridge.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{LogData, MetricData, TraceData};

/// Telemetry batch containing multiple telemetry records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryBatch {
    /// Unique batch identifier
    pub id: Uuid,

    /// Batch timestamp
    pub timestamp: DateTime<Utc>,

    /// Source identifier
    pub source: String,

    /// Batch size
    pub size: usize,

    /// Telemetry records
    pub records: Vec<TelemetryRecord>,

    /// Batch metadata
    pub metadata: HashMap<String, String>,
}

/// Individual telemetry record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryRecord {
    /// Record identifier
    pub id: Uuid,

    /// Record timestamp
    pub timestamp: DateTime<Utc>,

    /// Record type
    pub record_type: TelemetryType,

    /// Record data
    pub data: TelemetryData,

    /// Record attributes
    pub attributes: HashMap<String, String>,

    /// Record tags
    pub tags: HashMap<String, String>,

    /// Resource information
    pub resource: Option<ResourceInfo>,

    /// Service information
    pub service: Option<ServiceInfo>,
}

/// Telemetry data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TelemetryType {
    Metric,
    Trace,
    Log,
    Event,
}

/// Telemetry data content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryData {
    /// Metric data
    Metric(MetricData),

    /// Trace data
    Trace(TraceData),

    /// Log data
    Log(LogData),

    /// Event data
    Event(EventData),
}

/// Event data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    /// Event name
    pub name: String,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Event attributes
    pub attributes: HashMap<String, String>,

    /// Event data
    pub data: Option<serde_json::Value>,
}

/// Resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Resource type
    pub resource_type: String,

    /// Resource attributes
    pub attributes: HashMap<String, String>,

    /// Resource ID
    pub resource_id: Option<String>,
}

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,

    /// Service version
    pub version: Option<String>,

    /// Service namespace
    pub namespace: Option<String>,

    /// Service instance ID
    pub instance_id: Option<String>,
}

impl TelemetryBatch {
    /// Create a new telemetry batch
    pub fn new(source: String, records: Vec<TelemetryRecord>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source,
            size: records.len(),
            records,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the batch
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get batch size
    pub fn size(&self) -> usize {
        self.records.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl TelemetryRecord {
    /// Create a new telemetry record
    pub fn new(record_type: TelemetryType, data: TelemetryData) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type,
            data,
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }
    }

    /// Add attribute to the record
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// Add tag to the record
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Set resource information
    pub fn with_resource(mut self, resource: ResourceInfo) -> Self {
        self.resource = Some(resource);
        self
    }

    /// Set service information
    pub fn with_service(mut self, service: ServiceInfo) -> Self {
        self.service = Some(service);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MetricData, MetricType, MetricValue};

    #[test]
    fn test_telemetry_batch_creation() {
        let records = vec![TelemetryRecord::new(
            TelemetryType::Metric,
            TelemetryData::Metric(MetricData {
                name: "test_metric".to_string(),
                description: None,
                unit: None,
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1.0),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
        )];

        let batch = TelemetryBatch::new("test_source".to_string(), records);
        assert_eq!(batch.source, "test_source");
        assert_eq!(batch.size(), 1);
        assert!(!batch.is_empty());
    }
}
