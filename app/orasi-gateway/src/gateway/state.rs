//! Gateway state management

use crate::{
    config::GatewayConfig,
    types::{GatewayInfo, GatewayMetrics, GatewayStatus, HealthStatus},
};
use std::collections::HashMap;

/// Gateway state
#[derive(Debug)]
pub struct GatewayState {
    gateway_info: GatewayInfo,
    services: HashMap<String, crate::types::ServiceInfo>,
    metrics: GatewayMetrics,
}

impl GatewayState {
    /// Create a new gateway state
    pub fn new(config: &GatewayConfig) -> Self {
        let gateway_info = GatewayInfo {
            gateway_id: config.gateway_id.clone(),
            status: GatewayStatus::Starting,
            version: crate::GATEWAY_VERSION.to_string(),
            endpoint: config.gateway_endpoint.clone(),
            last_heartbeat: current_timestamp(),
            metadata: HashMap::new(),
        };

        Self {
            gateway_info,
            services: HashMap::new(),
            metrics: GatewayMetrics {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                avg_response_time_ms: 0.0,
                active_connections: 0,
                rate_limit_violations: 0,
                circuit_breaker_trips: 0,
            },
        }
    }

    /// Get gateway information
    pub fn get_gateway_info(&self) -> GatewayInfo {
        self.gateway_info.clone()
    }

    /// Set gateway status
    pub fn set_status(&mut self, status: GatewayStatus) {
        self.gateway_info.status = status;
    }

    /// Get gateway status
    pub fn get_status(&self) -> GatewayStatus {
        self.gateway_info.status.clone()
    }

    /// Update services
    pub fn update_services(&mut self, services: HashMap<String, crate::types::ServiceInfo>) {
        self.services = services;
    }

    /// Get services
    pub fn get_services(&self) -> HashMap<String, crate::types::ServiceInfo> {
        self.services.clone()
    }

    /// Update metrics
    pub fn update_metrics(&mut self, metrics: GatewayMetrics) {
        self.metrics = metrics;
    }

    /// Get metrics
    pub fn get_metrics(&self) -> GatewayMetrics {
        self.metrics.clone()
    }
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
