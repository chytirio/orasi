//! Types for Orasi Gateway

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Gateway status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GatewayStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error,
}

/// Gateway information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayInfo {
    /// Gateway identifier
    pub gateway_id: String,
    
    /// Gateway status
    pub status: GatewayStatus,
    
    /// Gateway version
    pub version: String,
    
    /// Gateway endpoint
    pub endpoint: String,
    
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
    
    /// Gateway metadata
    pub metadata: HashMap<String, String>,
}

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    
    /// Service endpoints
    pub endpoints: Vec<ServiceEndpoint>,
    
    /// Service health status
    pub health_status: ServiceHealthStatus,
    
    /// Service metadata
    pub metadata: HashMap<String, String>,
}

/// Service endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Endpoint URL
    pub url: String,
    
    /// Endpoint weight
    pub weight: u32,
    
    /// Endpoint health status
    pub health_status: EndpointHealthStatus,
    
    /// Endpoint metadata
    pub metadata: HashMap<String, String>,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Endpoint health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EndpointHealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// Request context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    /// Request ID
    pub request_id: String,
    
    /// Client IP address
    pub client_ip: String,
    
    /// User agent
    pub user_agent: Option<String>,
    
    /// Request headers
    pub headers: HashMap<String, String>,
    
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Response context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContext {
    /// Response status code
    pub status_code: u16,
    
    /// Response headers
    pub headers: HashMap<String, String>,
    
    /// Response metadata
    pub metadata: HashMap<String, String>,
}

/// Route match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMatch {
    /// Route path
    pub path: String,
    
    /// HTTP method
    pub method: String,
    
    /// Route parameters
    pub parameters: HashMap<String, String>,
    
    /// Route metadata
    pub metadata: HashMap<String, String>,
}

/// Load balancer state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerState {
    /// Available endpoints
    pub available_endpoints: Vec<ServiceEndpoint>,
    
    /// Current endpoint index
    pub current_index: usize,
    
    /// Endpoint weights
    pub weights: HashMap<String, u32>,
    
    /// Endpoint connection counts
    pub connection_counts: HashMap<String, u32>,
}

/// Circuit breaker state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerInfo {
    /// Circuit breaker state
    pub state: CircuitBreakerState,
    
    /// Failure count
    pub failure_count: u32,
    
    /// Last failure time
    pub last_failure_time: Option<u64>,
    
    /// Next retry time
    pub next_retry_time: Option<u64>,
}

/// Rate limiter state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterState {
    /// Current request count
    pub current_requests: u32,
    
    /// Last reset time
    pub last_reset_time: u64,
    
    /// Rate limit exceeded
    pub rate_limit_exceeded: bool,
}

/// Metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMetrics {
    /// Total requests
    pub total_requests: u64,
    
    /// Successful requests
    pub successful_requests: u64,
    
    /// Failed requests
    pub failed_requests: u64,
    
    /// Average response time (ms)
    pub avg_response_time_ms: f64,
    
    /// Current active connections
    pub active_connections: u32,
    
    /// Rate limit violations
    pub rate_limit_violations: u64,
    
    /// Circuit breaker trips
    pub circuit_breaker_trips: u64,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Service name
    pub service: String,
    
    /// Health status
    pub status: HealthState,
    
    /// Health details
    pub details: HashMap<String, String>,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Health state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Utility functions
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn generate_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
