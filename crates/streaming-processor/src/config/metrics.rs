//! Metrics configuration types

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Metrics endpoint
    pub endpoint: Option<String>,

    /// Metrics collection interval
    pub collection_interval: Duration,

    /// Enable health checks
    pub enable_health_checks: bool,

    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            endpoint: Some("0.0.0.0:9090".to_string()),
            collection_interval: Duration::from_secs(15),
            enable_health_checks: true,
            health_check_interval: Duration::from_secs(30),
        }
    }
}

impl MetricsConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.enable_metrics && self.endpoint.is_none() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Metrics endpoint must be specified when metrics are enabled".to_string(),
            ));
        }

        Ok(())
    }
}
