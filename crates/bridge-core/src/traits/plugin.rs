//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Bridge plugin traits for the OpenTelemetry Data Lake Bridge
//!
//! This module provides traits and types for IDE plugin integration.

use crate::error::BridgeResult;
use crate::types::{AnalyticsRequest, AnalyticsResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bridge plugin trait for IDE integration
#[async_trait]
pub trait BridgePlugin: Send + Sync {
    /// Initialize the plugin
    async fn initialize(&self, config: PluginConfig) -> BridgeResult<()>;

    /// Query telemetry data
    async fn query_telemetry(
        &self,
        query: crate::types::queries::TelemetryQuery,
    ) -> BridgeResult<TelemetryQueryResult>;

    /// Get analytics data
    async fn get_analytics(&self, request: AnalyticsRequest) -> BridgeResult<AnalyticsResponse>;

    /// Stream updates
    async fn stream_updates(&self) -> BridgeResult<UpdateStream>;

    /// Get plugin name
    fn name(&self) -> &str;

    /// Get plugin version
    fn version(&self) -> &str;

    /// Get plugin capabilities
    fn capabilities(&self) -> Vec<PluginCapability>;

    /// Check if plugin is healthy
    async fn health_check(&self) -> BridgeResult<bool>;

    /// Get plugin statistics
    async fn get_stats(&self) -> BridgeResult<PluginStats>;

    /// Shutdown the plugin
    async fn shutdown(&self) -> BridgeResult<()>;
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin configuration parameters
    pub parameters: HashMap<String, serde_json::Value>,

    /// Plugin metadata
    pub metadata: HashMap<String, String>,
}

/// Plugin capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapability {
    /// Capability name
    pub name: String,

    /// Capability description
    pub description: String,

    /// Capability version
    pub version: String,

    /// Capability parameters
    pub parameters: Vec<CapabilityParameter>,
}

/// Capability parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityParameter {
    /// Parameter name
    pub name: String,

    /// Parameter type
    pub parameter_type: ParameterType,

    /// Parameter description
    pub description: String,

    /// Parameter required
    pub required: bool,

    /// Parameter default value
    pub default_value: Option<serde_json::Value>,
}

/// Parameter types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Custom(String),
}

/// Plugin statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStats {
    /// Total queries processed
    pub total_queries: u64,

    /// Total analytics requests processed
    pub total_analytics: u64,

    /// Queries processed in last minute
    pub queries_per_minute: u64,

    /// Analytics requests processed in last minute
    pub analytics_per_minute: u64,

    /// Average query time in milliseconds
    pub avg_query_time_ms: f64,

    /// Average analytics time in milliseconds
    pub avg_analytics_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last operation timestamp
    pub last_operation_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Telemetry query result for plugin responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryQueryResult {
    /// Query ID
    pub query_id: uuid::Uuid,

    /// Query timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Query status
    pub status: QueryResultStatus,

    /// Query data
    pub data: serde_json::Value,

    /// Query metadata
    pub metadata: HashMap<String, String>,

    /// Query errors
    pub errors: Vec<QueryResultError>,
}

/// Query result status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResultStatus {
    Success,
    Partial,
    Failed,
    Timeout,
}

/// Query result error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error details
    pub details: Option<serde_json::Value>,
}

/// Update stream for plugin streaming
#[derive(Debug, Clone)]
pub struct UpdateStream {
    /// Stream identifier
    pub stream_id: String,

    /// Stream metadata
    pub metadata: HashMap<String, String>,
}
