//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Monitoring and observability system for the ingestion platform
//!
//! This module provides comprehensive monitoring capabilities including
//! metrics collection, health checks, structured logging, and distributed tracing.

pub mod distributed_tracing;
pub mod health_checks;
pub mod metrics;
pub mod structured_logging;

use bridge_core::traits::MetricsCollector as BridgeMetricsCollector;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// Re-export monitoring components
pub use distributed_tracing::{
    DistributedTracingConfig, DistributedTracingManager, SamplingStrategy, SpanStatus, TraceContext,
};
pub use health_checks::{
    HealthCheckConfig, HealthCheckManager, HealthChecker, HealthStatus, HttpEndpointHealthChecker,
    MemoryUsageHealthChecker,
};
pub use metrics::{MetricsCollector, MetricsConfig};
pub use structured_logging::{LogLevel, StructuredLoggingConfig, StructuredLoggingManager};

/// Comprehensive monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,

    /// Metrics configuration
    pub metrics: MetricsConfig,

    /// Health checks configuration
    pub health_checks: HealthCheckConfig,

    /// Structured logging configuration
    pub structured_logging: StructuredLoggingConfig,

    /// Distributed tracing configuration
    pub distributed_tracing: DistributedTracingConfig,

    /// Service information
    pub service_info: ServiceInfo,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,

    /// Service version
    pub version: String,

    /// Service environment
    pub environment: String,

    /// Service instance ID
    pub instance_id: String,

    /// Service tags
    pub tags: HashMap<String, String>,
}

/// Unified monitoring manager
pub struct MonitoringManager {
    config: MonitoringConfig,
    health_check_manager: Arc<HealthCheckManager>,
    structured_logging_manager: Arc<StructuredLoggingManager>,
    distributed_tracing_manager: Arc<DistributedTracingManager>,
    is_initialized: Arc<RwLock<bool>>,
}

impl MonitoringConfig {
    /// Create new monitoring configuration
    pub fn new() -> Self {
        Self {
            enabled: true,
            metrics: MetricsConfig::default(),
            health_checks: HealthCheckConfig::new(),
            structured_logging: StructuredLoggingConfig::new(),
            distributed_tracing: DistributedTracingConfig::new(),
            service_info: ServiceInfo {
                name: "ingestion-service".to_string(),
                version: "1.0.0".to_string(),
                environment: "development".to_string(),
                instance_id: uuid::Uuid::new_v4().to_string(),
                tags: HashMap::new(),
            },
            additional_config: HashMap::new(),
        }
    }

    /// Create configuration for production
    pub fn production() -> Self {
        let mut config = Self::new();
        config.service_info.environment = "production".to_string();
        config.metrics = MetricsConfig::default();
        config.health_checks = HealthCheckConfig::with_interval(30, 10);
        config.structured_logging = StructuredLoggingConfig::production();
        config.distributed_tracing = DistributedTracingConfig::production();
        config
    }

    /// Create configuration for development
    pub fn development() -> Self {
        let mut config = Self::new();
        config.service_info.environment = "development".to_string();
        config.metrics = MetricsConfig::default();
        config.health_checks = HealthCheckConfig::with_interval(10, 5);
        config.structured_logging = StructuredLoggingConfig::development();
        config.distributed_tracing = DistributedTracingConfig::development();
        config
    }

    /// Create configuration with custom service info
    pub fn with_service_info(service_info: ServiceInfo) -> Self {
        let mut config = Self::new();
        config.service_info = service_info;
        config
    }
}

impl MonitoringManager {
    /// Create new monitoring manager
    pub fn new(config: MonitoringConfig) -> Self {
        let health_check_manager = Arc::new(HealthCheckManager::new(config.health_checks.clone()));
        let structured_logging_manager = Arc::new(StructuredLoggingManager::new(
            config.structured_logging.clone(),
        ));
        let distributed_tracing_manager = Arc::new(DistributedTracingManager::new(
            config.distributed_tracing.clone(),
        ));

        Self {
            config,
            health_check_manager,
            structured_logging_manager,
            distributed_tracing_manager,
            is_initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize all monitoring components
    pub async fn initialize(&self) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut is_initialized = self.is_initialized.write().await;
        if *is_initialized {
            return Ok(());
        }

        // Initialize health checks
        self.health_check_manager.start().await?;

        // Initialize structured logging
        self.structured_logging_manager.initialize().await?;

        // Initialize distributed tracing
        self.distributed_tracing_manager.initialize().await?;

        // Add service info to all components
        self.add_service_info().await?;

        *is_initialized = true;
        Ok(())
    }

    /// Add service information to all monitoring components
    async fn add_service_info(&self) -> BridgeResult<()> {
        let service_info = &self.config.service_info;

        // Add service info to structured logging
        self.structured_logging_manager
            .add_custom_field(
                "service_name".to_string(),
                serde_json::json!(service_info.name),
            )
            .await?;
        self.structured_logging_manager
            .add_custom_field(
                "service_version".to_string(),
                serde_json::json!(service_info.version),
            )
            .await?;
        self.structured_logging_manager
            .add_custom_field(
                "environment".to_string(),
                serde_json::json!(service_info.environment),
            )
            .await?;
        self.structured_logging_manager
            .add_custom_field(
                "instance_id".to_string(),
                serde_json::json!(service_info.instance_id),
            )
            .await?;

        // Add service tags
        for (key, value) in &service_info.tags {
            self.structured_logging_manager
                .add_custom_field(key.clone(), serde_json::json!(value))
                .await?;
        }

        Ok(())
    }

    /// Get health check manager
    pub fn health_checks(&self) -> Arc<HealthCheckManager> {
        Arc::clone(&self.health_check_manager)
    }

    /// Get structured logging manager
    pub fn logging(&self) -> Arc<StructuredLoggingManager> {
        Arc::clone(&self.structured_logging_manager)
    }

    /// Get distributed tracing manager
    pub fn tracing(&self) -> Arc<DistributedTracingManager> {
        Arc::clone(&self.distributed_tracing_manager)
    }

    /// Record ingestion metrics
    pub async fn record_ingestion_metrics(
        &self,
        protocol: &str,
        record_count: usize,
        batch_size: usize,
        processing_time_ms: u64,
    ) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Create metrics collector if not already available
        let metrics_collector = MetricsCollector::new(self.config.metrics.clone())?;

        // Record batch processing metrics
        let processing_time = std::time::Duration::from_millis(processing_time_ms);
        metrics_collector
            .record_batch_processed(batch_size, processing_time)
            .await;

        // Record protocol-specific metrics using trait methods
        let collector_ref = &metrics_collector as &dyn BridgeMetricsCollector;
        collector_ref
            .increment_counter(
                "ingestion_records_total",
                record_count as u64,
                &[("protocol", protocol)],
            )
            .await?;
        collector_ref
            .increment_counter("ingestion_batches_total", 1, &[("protocol", protocol)])
            .await?;
        collector_ref
            .record_gauge(
                "ingestion_batch_size",
                batch_size as f64,
                &[("protocol", protocol)],
            )
            .await?;
        collector_ref
            .record_histogram(
                "ingestion_processing_time_ms",
                processing_time_ms as f64,
                &[("protocol", protocol)],
            )
            .await?;

        // Update throughput metrics
        let throughput = if processing_time_ms > 0 {
            (record_count as f64 * 1000.0) / processing_time_ms as f64
        } else {
            0.0
        };
        collector_ref
            .record_gauge(
                "ingestion_throughput_records_per_sec",
                throughput,
                &[("protocol", protocol)],
            )
            .await?;

        info!("Ingestion metrics recorded: protocol={}, records={}, batch_size={}, processing_time_ms={}, throughput={:.2} records/sec", 
              protocol, record_count, batch_size, processing_time_ms, throughput);
        Ok(())
    }

    /// Record error metrics
    pub async fn record_error_metrics(
        &self,
        error_type: &str,
        component: &str,
    ) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Create metrics collector if not already available
        let metrics_collector = MetricsCollector::new(self.config.metrics.clone())?;

        // Record error metrics using trait methods
        let collector_ref = &metrics_collector as &dyn BridgeMetricsCollector;
        collector_ref
            .increment_counter(
                "errors_total",
                1,
                &[("type", error_type), ("component", component)],
            )
            .await?;
        metrics_collector.record_error(error_type).await;

        // Record error rate over time
        let error_rate = metrics_collector
            .get_counter(
                "errors_total",
                &[("type", error_type), ("component", component)],
            )
            .await;
        if let Some(total_errors) = error_rate {
            // Calculate error rate as percentage (simplified - in real implementation would track time windows)
            let error_rate_percentage = if total_errors > 0 {
                let total_errors_f64 = total_errors as f64;
                (total_errors_f64 / (total_errors_f64 + 100.0)) * 100.0 // Simplified calculation
            } else {
                0.0
            };
            collector_ref
                .record_gauge(
                    "error_rate_percentage",
                    error_rate_percentage,
                    &[("type", error_type), ("component", component)],
                )
                .await?;
        }

        // Log error with structured logging
        let mut error_fields = HashMap::new();
        error_fields.insert("error_type".to_string(), serde_json::json!(error_type));
        error_fields.insert("component".to_string(), serde_json::json!(component));
        error_fields.insert(
            "timestamp".to_string(),
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );

        self.structured_logging_manager
            .log(
                structured_logging::LogLevel::Error,
                &format!("Error occurred: {} in component {}", error_type, component),
                error_fields,
            )
            .await?;

        info!(
            "Error metrics recorded: type={}, component={}",
            error_type, component
        );
        Ok(())
    }

    /// Record performance metrics
    pub async fn record_performance_metrics(
        &self,
        operation: &str,
        duration_ms: u64,
        success: bool,
    ) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Create metrics collector if not already available
        let metrics_collector = MetricsCollector::new(self.config.metrics.clone())?;

        // Record operation metrics using trait methods
        let collector_ref = &metrics_collector as &dyn BridgeMetricsCollector;

        // Record operation duration histogram
        collector_ref
            .record_histogram(
                "operation_duration_ms",
                duration_ms as f64,
                &[("operation", operation)],
            )
            .await?;

        // Record operation success/failure counters
        let status = if success { "success" } else { "failure" };
        collector_ref
            .increment_counter(
                "operation_total",
                1,
                &[("operation", operation), ("status", status)],
            )
            .await?;

        // Record operation throughput (operations per second)
        let throughput = if duration_ms > 0 {
            1000.0 / duration_ms as f64
        } else {
            0.0
        };
        collector_ref
            .record_gauge(
                "operation_throughput_per_sec",
                throughput,
                &[("operation", operation)],
            )
            .await?;

        // Record cumulative operation time
        collector_ref
            .record_gauge(
                "operation_cumulative_time_ms",
                duration_ms as f64,
                &[("operation", operation)],
            )
            .await?;

        // Log performance metrics with structured logging
        let mut perf_fields = HashMap::new();
        perf_fields.insert("operation".to_string(), serde_json::json!(operation));
        perf_fields.insert("duration_ms".to_string(), serde_json::json!(duration_ms));
        perf_fields.insert("success".to_string(), serde_json::json!(success));
        perf_fields.insert(
            "throughput_per_sec".to_string(),
            serde_json::json!(throughput),
        );
        perf_fields.insert(
            "timestamp".to_string(),
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );

        let log_level = if success {
            structured_logging::LogLevel::Info
        } else {
            structured_logging::LogLevel::Warn
        };

        self.structured_logging_manager
            .log(
                log_level,
                &format!(
                    "Operation completed: {} in {}ms (success: {})",
                    operation, duration_ms, success
                ),
                perf_fields,
            )
            .await?;

        info!("Performance metrics recorded: operation={}, duration_ms={}, success={}, throughput={:.2} ops/sec", 
              operation, duration_ms, success, throughput);
        Ok(())
    }

    /// Start a trace span for ingestion
    pub async fn start_ingestion_span(
        &self,
        protocol: &str,
        batch_size: u64,
    ) -> BridgeResult<String> {
        let mut attributes = HashMap::new();
        attributes.insert("protocol".to_string(), serde_json::json!(protocol));
        attributes.insert("batch_size".to_string(), serde_json::json!(batch_size));
        attributes.insert("operation".to_string(), serde_json::json!("ingestion"));

        self.distributed_tracing_manager
            .start_span("ingestion", attributes)
            .await
    }

    /// End a trace span
    pub async fn end_ingestion_span(
        &self,
        span_id: &str,
        success: bool,
        error_message: Option<&str>,
    ) -> BridgeResult<()> {
        if let Some(error_message) = error_message {
            let mut attributes = HashMap::new();
            attributes.insert(
                "error_message".to_string(),
                serde_json::json!(error_message),
            );
            self.distributed_tracing_manager
                .add_span_event(span_id, "error", attributes)
                .await?;
        }

        let status = SpanStatus {
            code: if success {
                distributed_tracing::SpanStatusCode::Ok
            } else {
                distributed_tracing::SpanStatusCode::Error
            },
            message: error_message.map(|s| s.to_string()),
        };

        self.distributed_tracing_manager
            .set_span_status(span_id, status)
            .await?;

        self.distributed_tracing_manager.end_span(span_id).await
    }

    /// Get comprehensive monitoring statistics
    pub async fn get_statistics(&self) -> BridgeResult<MonitoringStatistics> {
        let health_stats = self.health_check_manager.get_statistics().await;
        let logging_stats = self.structured_logging_manager.get_statistics().await;
        let tracing_stats = self.distributed_tracing_manager.get_stats().await?;

        Ok(MonitoringStatistics {
            service_info: self.config.service_info.clone(),
            health_checks: health_stats,
            structured_logging: logging_stats,
            distributed_tracing: tracing_stats,
            is_initialized: *self.is_initialized.read().await,
        })
    }

    /// Check if monitoring is healthy
    pub async fn is_healthy(&self) -> bool {
        if !self.config.enabled {
            return true;
        }

        // Check if all components are initialized
        if !*self.is_initialized.read().await {
            return false;
        }

        // Check health status
        let health_status = self.health_check_manager.get_overall_health().await;
        matches!(health_status, HealthStatus::Healthy)
    }

    /// Shutdown monitoring
    pub async fn shutdown(&self) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Stop health checks
        self.health_check_manager.stop().await?;

        // Note: Other components don't have explicit shutdown methods
        // In a real implementation, you would add proper shutdown logic

        let mut is_initialized = self.is_initialized.write().await;
        *is_initialized = false;

        Ok(())
    }
}

impl Clone for MonitoringManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            health_check_manager: Arc::clone(&self.health_check_manager),
            structured_logging_manager: Arc::clone(&self.structured_logging_manager),
            distributed_tracing_manager: Arc::clone(&self.distributed_tracing_manager),
            is_initialized: Arc::clone(&self.is_initialized),
        }
    }
}

/// Comprehensive monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStatistics {
    /// Service information
    pub service_info: ServiceInfo,

    /// Health check statistics
    pub health_checks: health_checks::HealthCheckStatistics,

    /// Structured logging statistics
    pub structured_logging: structured_logging::LoggingStatistics,

    /// Distributed tracing statistics
    pub distributed_tracing: distributed_tracing::TracingStats,

    /// Whether monitoring is initialized
    pub is_initialized: bool,
}

/// Monitoring trait for easy integration
#[async_trait::async_trait]
pub trait Monitorable {
    /// Get monitoring manager
    fn monitoring(&self) -> Arc<MonitoringManager>;

    /// Record operation metrics
    async fn record_operation_metrics(
        &self,
        operation: &str,
        duration_ms: u64,
        success: bool,
    ) -> BridgeResult<()> {
        self.monitoring()
            .record_performance_metrics(operation, duration_ms, success)
            .await
    }

    /// Record error
    async fn record_error(
        &self,
        error_type: &str,
        error_message: &str,
        component: &str,
    ) -> BridgeResult<()> {
        self.monitoring()
            .record_error_metrics(error_type, component)
            .await
    }

    /// Start trace span
    async fn start_span(
        &self,
        name: &str,
        attributes: HashMap<String, serde_json::Value>,
    ) -> BridgeResult<String> {
        self.monitoring()
            .tracing()
            .start_span(name, attributes)
            .await
    }

    /// End trace span
    async fn end_span(
        &self,
        span_id: &str,
        success: bool,
        error_message: Option<&str>,
    ) -> BridgeResult<()> {
        if let Some(error_message) = error_message {
            let mut attributes = HashMap::new();
            attributes.insert(
                "error_message".to_string(),
                serde_json::json!(error_message),
            );
            self.monitoring()
                .tracing()
                .add_span_event(span_id, "error", attributes)
                .await?;
        }

        let status = SpanStatus {
            code: if success {
                distributed_tracing::SpanStatusCode::Ok
            } else {
                distributed_tracing::SpanStatusCode::Error
            },
            message: error_message.map(|s| s.to_string()),
        };

        self.monitoring()
            .tracing()
            .set_span_status(span_id, status)
            .await?;
        self.monitoring().tracing().end_span(span_id).await
    }

    /// Log structured message
    async fn log(
        &self,
        level: LogLevel,
        message: &str,
        fields: HashMap<String, serde_json::Value>,
    ) -> BridgeResult<()> {
        self.monitoring()
            .logging()
            .log(level, message, fields)
            .await
    }

    /// Check health
    async fn is_healthy(&self) -> bool {
        self.monitoring().is_healthy().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_config_creation() {
        let config = MonitoringConfig::new();

        assert!(config.enabled);
        assert_eq!(config.service_info.name, "ingestion-service");
        assert_eq!(config.service_info.environment, "development");
    }

    #[tokio::test]
    async fn test_monitoring_manager_creation() {
        let config = MonitoringConfig::new();
        let manager = MonitoringManager::new(config);

        let stats = manager.get_statistics().await.unwrap();
        assert_eq!(stats.service_info.name, "ingestion-service");
        assert!(!stats.is_initialized);
    }

    #[tokio::test]
    async fn test_monitoring_integration() {
        let config = MonitoringConfig::development();
        let manager = MonitoringManager::new(config);

        // Test metrics recording
        manager
            .record_ingestion_metrics("otlp", 100, 50, 150)
            .await
            .unwrap();

        // Test error recording
        manager
            .record_error_metrics("network", "Connection failed")
            .await
            .unwrap();

        // Test performance recording
        manager
            .record_performance_metrics("batch_processing", 200, true)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let config = MonitoringConfig::development();
        let manager = MonitoringManager::new(config);

        // Test ingestion metrics recording
        let result = manager.record_ingestion_metrics("otlp", 100, 50, 150).await;
        assert!(result.is_ok());

        // Test error metrics recording
        let result = manager.record_error_metrics("network", "connection").await;
        assert!(result.is_ok());

        // Test performance metrics recording
        let result = manager
            .record_performance_metrics("batch_processing", 200, true)
            .await;
        assert!(result.is_ok());
    }
}
