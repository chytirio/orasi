//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Metrics for the Schema Registry
//!
//! This module provides metrics collection and export functionality
//! for monitoring the schema registry.

use crate::registry::RegistryStats;
use std::sync::Arc;

#[cfg(feature = "metrics")]
use metrics::{counter, gauge, histogram};

#[cfg(not(feature = "metrics"))]
macro_rules! counter {
    ($name:expr, $value:expr $(, $label:expr => $label_value:expr)*) => {
        // No-op when metrics feature is disabled
    };
}

#[cfg(not(feature = "metrics"))]
macro_rules! gauge {
    ($name:expr, $value:expr $(, $label:expr => $label_value:expr)*) => {
        // No-op when metrics feature is disabled
    };
}

#[cfg(not(feature = "metrics"))]
macro_rules! histogram {
    ($name:expr, $value:expr $(, $label:expr => $label_value:expr)*) => {
        // No-op when metrics feature is disabled
    };
}

/// Metrics collector for the schema registry
pub struct MetricsCollector {
    /// Registry name for metrics
    registry_name: String,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(registry_name: String) -> Self {
        Self { registry_name }
    }

    /// Record schema registration
    pub fn record_schema_registration(&self, schema_type: &str) {
        counter!("schema_registry.schemas.registered", 1, "type" => schema_type.to_string());
    }

    /// Record schema retrieval
    pub fn record_schema_retrieval(&self, success: bool) {
        if success {
            counter!("schema_registry.schemas.retrieved", 1);
        } else {
            counter!("schema_registry.schemas.retrieval_failed", 1);
        }
    }

    /// Record schema deletion
    pub fn record_schema_deletion(&self, success: bool) {
        if success {
            counter!("schema_registry.schemas.deleted", 1);
        } else {
            counter!("schema_registry.schemas.deletion_failed", 1);
        }
    }

    /// Record validation
    pub fn record_validation(&self, success: bool, error_count: usize, warning_count: usize) {
        if success {
            counter!("schema_registry.validations.success", 1);
        } else {
            counter!("schema_registry.validations.failed", 1);
        }

        if error_count > 0 {
            counter!("schema_registry.validations.errors", error_count as u64);
        }

        if warning_count > 0 {
            counter!("schema_registry.validations.warnings", warning_count as u64);
        }
    }

    /// Record API request
    pub fn record_api_request(&self, endpoint: &str, method: &str, status_code: u16) {
        counter!("schema_registry.api.requests", 1,
            "endpoint" => endpoint.to_string(),
            "method" => method.to_string(),
            "status_code" => status_code.to_string()
        );
    }

    /// Record API response time
    pub fn record_api_response_time(&self, endpoint: &str, duration_ms: f64) {
        histogram!("schema_registry.api.response_time_ms", duration_ms,
            "endpoint" => endpoint.to_string()
        );
    }

    /// Update registry statistics
    pub fn update_registry_stats(&self, stats: &RegistryStats) {
        gauge!("schema_registry.schemas.total", stats.total_schemas as f64);
        gauge!(
            "schema_registry.versions.total",
            stats.total_versions as f64
        );
        gauge!(
            "schema_registry.storage.size_bytes",
            stats.total_size_bytes as f64
        );
        gauge!(
            "schema_registry.validations.errors.total",
            stats.validation_errors as f64
        );
        gauge!(
            "schema_registry.validations.warnings.total",
            stats.validation_warnings as f64
        );
    }

    /// Record storage operation
    pub fn record_storage_operation(&self, operation: &str, success: bool, duration_ms: f64) {
        if success {
            counter!("schema_registry.storage.operations", 1, "operation" => operation.to_string());
        } else {
            counter!("schema_registry.storage.operation_failed", 1, "operation" => operation.to_string());
        }

        histogram!("schema_registry.storage.operation_duration_ms", duration_ms,
            "operation" => operation.to_string()
        );
    }

    /// Record health check
    pub fn record_health_check(&self, healthy: bool) {
        if healthy {
            counter!("schema_registry.health.checks", 1);
        } else {
            counter!("schema_registry.health.checks_failed", 1);
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new("schema-registry".to_string())
    }
}

/// Metrics configuration
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,

    /// Metrics endpoint
    pub endpoint: String,

    /// Metrics port
    pub port: u16,

    /// Metrics collection interval in seconds
    pub collection_interval: u64,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "/metrics".to_string(),
            port: 9091,
            collection_interval: 60,
        }
    }
}

/// Metrics server
pub struct MetricsServer {
    /// Configuration
    config: MetricsConfig,

    /// Metrics collector
    collector: Arc<MetricsCollector>,

    /// Background task handle
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MetricsServer {
    /// Create a new metrics server
    pub fn new(config: MetricsConfig, collector: Arc<MetricsCollector>) -> Self {
        Self {
            config,
            collector,
            task_handle: None,
        }
    }

    /// Start the metrics server
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.config.enabled {
            return Ok(());
        }

        // Initialize metrics
        self.initialize_metrics();

        // Start metrics collection
        self.start_collection().await;

        Ok(())
    }

    /// Shutdown the metrics server
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            let _ = handle.await; // Wait for the task to finish
        }

        tracing::debug!("Metrics server shutdown completed");
        Ok(())
    }

    /// Initialize metrics
    fn initialize_metrics(&self) {
        // Initialize default metrics
        counter!("schema_registry.startup", 1);
        gauge!("schema_registry.uptime_seconds", 0.0);
    }

    /// Start metrics collection
    async fn start_collection(&mut self) {
        let collector = self.collector.clone();
        let interval = self.config.collection_interval;

        let handle = tokio::spawn(async move {
            let mut interval_timer =
                tokio::time::interval(std::time::Duration::from_secs(interval));

            loop {
                interval_timer.tick().await;

                // Update uptime metric
                gauge!(
                    "schema_registry.uptime_seconds",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as f64
                );
            }
        });

        self.task_handle = Some(handle);
    }

    /// Get metrics endpoint
    pub fn endpoint(&self) -> &str {
        &self.config.endpoint
    }

    /// Get metrics port
    pub fn port(&self) -> u16 {
        self.config.port
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new("test-registry".to_string());
        assert_eq!(collector.registry_name, "test-registry");
    }

    #[test]
    fn test_metrics_collector_default() {
        let collector = MetricsCollector::default();
        assert_eq!(collector.registry_name, "schema-registry");
    }

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.endpoint, "/metrics");
        assert_eq!(config.port, 9091);
        assert_eq!(config.collection_interval, 60);
    }

    #[test]
    fn test_metrics_server_creation() {
        let config = MetricsConfig::default();
        let collector = Arc::new(MetricsCollector::default());
        let server = MetricsServer::new(config, collector);

        assert_eq!(server.endpoint(), "/metrics");
        assert_eq!(server.port(), 9091);
    }
}
