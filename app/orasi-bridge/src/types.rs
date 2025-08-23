//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! API-specific types for Bridge API

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,

    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ResponseMetadata {
    /// Request ID
    pub request_id: String,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// API version
    pub version: String,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

impl<T> ApiResponse<T> {
    /// Create a new API response
    pub fn new(data: T, request_id: String, processing_time_ms: u64) -> Self {
        Self {
            data,
            metadata: ResponseMetadata {
                request_id,
                timestamp: chrono::Utc::now(),
                version: crate::BRIDGE_API_VERSION.to_string(),
                processing_time_ms,
            },
        }
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    /// Service status
    pub status: HealthStatus,

    /// Service name
    pub name: String,

    /// Service version
    pub version: String,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// Component health checks
    pub components: HashMap<String, ComponentHealth>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component status
    pub status: HealthStatus,

    /// Component message
    pub message: Option<String>,

    /// Last check timestamp
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    /// Service status
    pub status: String,

    /// Service name
    pub name: String,

    /// Service version
    pub version: String,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// Active components
    pub components: Vec<String>,

    /// Configuration hash
    pub config_hash: String,
}

/// Telemetry ingestion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryIngestionRequest {
    /// Telemetry batch
    pub batch: bridge_core::types::TelemetryBatch,

    /// Processing options
    pub options: Option<ProcessingOptions>,
}

/// Processing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    /// Enable validation
    pub validate: bool,

    /// Enable transformation
    pub transform: bool,

    /// Enable aggregation
    pub aggregate: bool,

    /// Processing timeout in seconds
    pub timeout_seconds: Option<u64>,
}

/// Telemetry ingestion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryIngestionResponse {
    /// Processing result
    pub result: bridge_core::types::WriteResult,

    /// Processed batch ID
    pub batch_id: Uuid,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Validation errors (if any)
    pub validation_errors: Option<Vec<String>>,
}

/// Query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// Query type
    pub query_type: QueryType,

    /// Query parameters
    pub parameters: QueryParameters,

    /// Query options
    pub options: Option<QueryOptions>,
}

/// Query type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Metrics,
    Traces,
    Logs,
    Analytics,
}

/// Query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParameters {
    /// Time range
    pub time_range: bridge_core::types::TimeRange,

    /// Filters
    pub filters: Option<Vec<bridge_core::types::Filter>>,

    /// Aggregations
    pub aggregations: Option<Vec<bridge_core::types::Aggregation>>,

    /// Limit
    pub limit: Option<usize>,

    /// Offset
    pub offset: Option<usize>,
}

/// Query options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptions {
    /// Enable caching
    pub enable_cache: bool,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: Option<u64>,

    /// Query timeout in seconds
    pub timeout_seconds: Option<u64>,

    /// Enable streaming
    pub enable_streaming: bool,
}

/// Query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// Query results
    pub results: QueryResults,

    /// Query metadata
    pub metadata: QueryMetadata,
}

/// Query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResults {
    /// Metrics results
    Metrics(bridge_core::types::MetricsResult),

    /// Traces results
    Traces(bridge_core::types::TracesResult),

    /// Logs results
    Logs(bridge_core::types::LogsResult),

    /// Analytics results
    Analytics(bridge_core::types::AnalyticsResponse),
}

/// Query metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Query ID
    pub query_id: Uuid,

    /// Query execution time in milliseconds
    pub execution_time_ms: u64,

    /// Result count
    pub result_count: usize,

    /// Cache hit
    pub cache_hit: bool,

    /// Query plan (if available)
    pub query_plan: Option<String>,
}

/// Analytics request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRequest {
    /// Analytics type
    pub analytics_type: bridge_core::types::AnalyticsType,

    /// Analytics parameters
    pub parameters: bridge_core::types::AnalyticsRequest,

    /// Analytics options
    pub options: Option<AnalyticsOptions>,
}

/// Analytics options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsOptions {
    /// Enable real-time processing
    pub real_time: bool,

    /// Processing window in seconds
    pub window_seconds: Option<u64>,

    /// Enable alerts
    pub enable_alerts: bool,

    /// Alert thresholds
    pub alert_thresholds: Option<HashMap<String, f64>>,
}

/// Analytics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    /// Analytics results
    pub results: bridge_core::types::AnalyticsResponse,

    /// Analytics metadata
    pub metadata: AnalyticsMetadata,
}

/// Analytics metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsMetadata {
    /// Analytics ID
    pub analytics_id: Uuid,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Data points processed
    pub data_points_processed: usize,

    /// Insights generated
    pub insights_count: usize,

    /// Alerts triggered
    pub alerts_triggered: Vec<String>,
}

/// Configuration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRequest {
    /// Configuration section
    pub section: String,

    /// Configuration data
    pub data: serde_json::Value,

    /// Configuration options
    pub options: Option<ConfigOptions>,
}

/// Configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOptions {
    /// Validate configuration
    pub validate: bool,

    /// Hot reload
    pub hot_reload: bool,

    /// Backup existing configuration
    pub backup: bool,
}

/// Configuration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    /// Configuration status
    pub status: String,

    /// Configuration hash
    pub config_hash: String,

    /// Applied changes
    pub changes: Vec<String>,

    /// Validation errors (if any)
    pub validation_errors: Option<Vec<String>>,
}

/// Component status request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatusRequest {
    /// Component name
    pub component_name: String,
}

/// Component status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatusResponse {
    /// Component name
    pub component_name: String,

    /// Component status
    pub status: ComponentStatus,

    /// Component health
    pub health: ComponentHealth,

    /// Component metrics
    pub metrics: Option<HashMap<String, f64>>,

    /// Component logs (if requested)
    pub logs: Option<Vec<String>>,
}

/// Component status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Error,
    Unknown,
}

/// Component restart request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRestartRequest {
    /// Component name
    pub component_name: String,

    /// Restart options
    pub options: Option<RestartOptions>,
}

/// Restart options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartOptions {
    /// Graceful restart
    pub graceful: bool,

    /// Restart timeout in seconds
    pub timeout_seconds: Option<u64>,

    /// Clear state
    pub clear_state: bool,
}

/// Component restart response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRestartResponse {
    /// Component name
    pub component_name: String,

    /// Restart status
    pub status: String,

    /// Restart time in milliseconds
    pub restart_time_ms: u64,

    /// Previous status
    pub previous_status: ComponentStatus,

    /// New status
    pub new_status: ComponentStatus,
}

/// Plugin capabilities request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapabilitiesRequest {
    /// Plugin name (optional)
    pub plugin_name: Option<String>,
}

/// Restart components request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartComponentsRequest {
    /// Components to restart
    pub components: Vec<String>,
    /// Force restart
    pub force: Option<bool>,
}

/// Components list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentsListResponse {
    /// List of components
    pub components: Vec<ComponentInfo>,
    /// Total number of components
    pub total_components: u32,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Component information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    /// Component name
    pub name: String,
    /// Component status
    pub status: ComponentStatus,
    /// Component version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last health check
    pub last_health_check: chrono::DateTime<chrono::Utc>,
}

/// Plugin capabilities response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapabilitiesResponse {
    /// Available plugins
    pub plugins: Vec<PluginInfo>,

    /// Capabilities summary
    pub capabilities: HashMap<String, Vec<String>>,
}

/// Plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin description
    pub description: String,

    /// Plugin capabilities
    pub capabilities: Vec<String>,

    /// Plugin status
    pub status: PluginStatus,
}

/// Plugin status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginStatus {
    Active,
    Inactive,
    Error,
    Loading,
}

/// Plugin query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginQueryRequest {
    /// Plugin name
    pub plugin_name: String,

    /// Query data
    pub query_data: serde_json::Value,

    /// Query options
    pub options: Option<PluginQueryOptions>,
}

/// Plugin query options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginQueryOptions {
    /// Query timeout in seconds
    pub timeout_seconds: Option<u64>,

    /// Enable caching
    pub enable_cache: bool,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: Option<u64>,
}

/// Plugin query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginQueryResponse {
    /// Plugin name
    pub plugin_name: String,

    /// Query results
    pub results: serde_json::Value,

    /// Query metadata
    pub metadata: PluginQueryMetadata,
}

/// Plugin query metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginQueryMetadata {
    /// Query ID
    pub query_id: Uuid,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Cache hit
    pub cache_hit: bool,

    /// Plugin version
    pub plugin_version: String,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-based)
    pub page: Option<u32>,

    /// Page size
    pub page_size: Option<u32>,

    /// Sort field
    pub sort_by: Option<String>,

    /// Sort order
    pub sort_order: Option<SortOrder>,
}

/// Sort order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Pagination response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationResponse<T> {
    /// Page data
    pub data: Vec<T>,

    /// Pagination metadata
    pub pagination: PaginationMetadata,
}

/// Pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMetadata {
    /// Current page
    pub current_page: u32,

    /// Page size
    pub page_size: u32,

    /// Total pages
    pub total_pages: u32,

    /// Total items
    pub total_items: u64,

    /// Has next page
    pub has_next: bool,

    /// Has previous page
    pub has_previous: bool,
}

impl<T> PaginationResponse<T> {
    /// Create a new pagination response
    pub fn new(data: Vec<T>, current_page: u32, page_size: u32, total_items: u64) -> Self {
        let total_pages = ((total_items as f64) / (page_size as f64)).ceil() as u32;

        Self {
            data,
            pagination: PaginationMetadata {
                current_page,
                page_size,
                total_pages,
                total_items,
                has_next: current_page < total_pages,
                has_previous: current_page > 1,
            },
        }
    }
}

// OTLP Protocol Types

/// OTLP traces request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpTracesRequest {
    /// Resource spans
    pub resource_spans: Vec<OtlpResourceSpans>,
}

/// OTLP resource spans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpResourceSpans {
    /// Resource information
    pub resource: OtlpResource,
    /// Scope spans
    pub scope_spans: Vec<OtlpScopeSpans>,
}

/// OTLP resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpResource {
    /// Resource attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

/// OTLP scope spans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpScopeSpans {
    /// Scope information
    pub scope: OtlpScope,
    /// Spans
    pub spans: Vec<OtlpSpan>,
}

/// OTLP scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpScope {
    /// Scope name
    pub name: Option<String>,
    /// Scope version
    pub version: Option<String>,
    /// Scope attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

/// OTLP span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpSpan {
    /// Span ID
    pub span_id: String,
    /// Trace ID
    pub trace_id: String,
    /// Span name
    pub name: String,
    /// Span kind
    pub kind: i32,
    /// Start time in Unix nanoseconds
    pub start_time_unix_nano: u64,
    /// End time in Unix nanoseconds
    pub end_time_unix_nano: u64,
    /// Span attributes
    pub attributes: HashMap<String, serde_json::Value>,
    /// Span events
    pub events: Vec<OtlpSpanEvent>,
    /// Span links
    pub links: Vec<OtlpSpanLink>,
    /// Span status
    pub status: Option<OtlpSpanStatus>,
}

/// OTLP span event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpSpanEvent {
    /// Event time in Unix nanoseconds
    pub time_unix_nano: u64,
    /// Event name
    pub name: String,
    /// Event attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

/// OTLP span link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpSpanLink {
    /// Link trace ID
    pub trace_id: String,
    /// Link span ID
    pub span_id: String,
    /// Link attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

/// OTLP span status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpSpanStatus {
    /// Status code
    pub code: i32,
    /// Status message
    pub message: Option<String>,
}

// OTLP Metrics Types

/// OTLP metrics request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpMetricsRequest {
    /// Resource metrics
    pub resource_metrics: Vec<OtlpResourceMetrics>,
}

/// OTLP resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpResourceMetrics {
    /// Resource information
    pub resource: OtlpResource,
    /// Scope metrics
    pub scope_metrics: Vec<OtlpScopeMetrics>,
}

/// OTLP scope metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpScopeMetrics {
    /// Scope information
    pub scope: OtlpScope,
    /// Metrics
    pub metrics: Vec<OtlpMetric>,
}

/// OTLP metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpMetric {
    /// Metric name
    pub name: String,
    /// Metric description
    pub description: Option<String>,
    /// Metric unit
    pub unit: Option<String>,
    /// Metric data
    pub data: OtlpMetricData,
}

/// OTLP metric data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpMetricData {
    /// Metric type
    pub metric_type: OtlpMetricType,
    /// Data points
    pub data_points: Vec<OtlpDataPoint>,
}

/// OTLP metric types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OtlpMetricType {
    Gauge,
    Sum,
    Histogram,
    ExponentialHistogram,
    Summary,
}

/// OTLP data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpDataPoint {
    /// Start time in Unix nanoseconds
    pub start_time_unix_nano: Option<u64>,
    /// Time in Unix nanoseconds
    pub time_unix_nano: u64,
    /// Value
    pub value: OtlpValue,
    /// Attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

/// OTLP value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpValue {
    /// Value type
    pub value_type: OtlpValueType,
    /// Numeric value
    pub numeric_value: Option<f64>,
    /// String value
    pub string_value: Option<String>,
    /// Boolean value
    pub boolean_value: Option<bool>,
}

/// OTLP value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OtlpValueType {
    Double,
    Int,
    String,
    Bool,
}

// OTLP Logs Types

/// OTLP logs request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpLogsRequest {
    /// Resource logs
    pub resource_logs: Vec<OtlpResourceLogs>,
}

/// OTLP resource logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpResourceLogs {
    /// Resource information
    pub resource: OtlpResource,
    /// Scope logs
    pub scope_logs: Vec<OtlpScopeLogs>,
}

/// OTLP scope logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpScopeLogs {
    /// Scope information
    pub scope: OtlpScope,
    /// Log records
    pub log_records: Vec<OtlpLogRecord>,
}

/// OTLP log record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpLogRecord {
    /// Time in Unix nanoseconds
    pub time_unix_nano: u64,
    /// Observed time in Unix nanoseconds
    pub observed_time_unix_nano: Option<u64>,
    /// Severity number
    pub severity_number: Option<i32>,
    /// Severity text
    pub severity_text: Option<String>,
    /// Body
    pub body: Option<OtlpValue>,
    /// Attributes
    pub attributes: HashMap<String, serde_json::Value>,
    /// Trace ID
    pub trace_id: Option<String>,
    /// Span ID
    pub span_id: Option<String>,
}
