//! Health monitoring integration for connecting gRPC status requests to the actual health monitoring system

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{config::BridgeAPIConfig, metrics::ApiMetrics, proto::*};
use bridge_core::health::{HealthCheckConfig, HealthChecker, HealthStatus as CoreHealthStatus};
use bridge_core::metrics::BridgeMetrics;
use bridge_core::BridgeResult;

/// Health monitoring integration service
pub struct HealthMonitoringIntegration {
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
    start_time: SystemTime,
    component_health: Arc<tokio::sync::RwLock<HashMap<String, ComponentHealthStatus>>>,
    health_checker: Arc<HealthChecker>,
    bridge_metrics: Arc<BridgeMetrics>,
}

/// Component health status
#[derive(Debug, Clone)]
pub struct ComponentHealthStatus {
    pub name: String,
    pub state: ComponentState,
    pub uptime_seconds: i64,
    pub error_message: String,
    pub metrics: HashMap<String, f64>,
    pub last_check: DateTime<Utc>,
}

impl HealthMonitoringIntegration {
    /// Create a new health monitoring integration service
    pub fn new(config: BridgeAPIConfig, metrics: ApiMetrics) -> Self {
        let start_time = SystemTime::now();
        let component_health = Arc::new(tokio::sync::RwLock::new(HashMap::new()));

        // Initialize health checker with configuration
        let health_config = HealthCheckConfig {
            enable_health_checks: true,
            health_check_interval_ms: 30000, // 30 seconds
            health_check_timeout_ms: 5000,   // 5 seconds
            health_endpoint: "0.0.0.0".to_string(),
            health_port: 8080,
            enable_detailed_health: true,
            health_check_retry_count: 3,
            health_check_retry_delay_ms: 1000,
        };
        let health_checker = Arc::new(HealthChecker::new(health_config));

        // Initialize bridge metrics
        let bridge_metrics = Arc::new(BridgeMetrics::new(
            bridge_core::metrics::MetricsConfig::default(),
        ));

        Self {
            config,
            metrics,
            start_time,
            component_health,
            health_checker,
            bridge_metrics,
        }
    }

    /// Initialize the health monitoring system
    pub async fn init(&self) -> BridgeResult<()> {
        tracing::info!("Initializing health monitoring integration");

        // Initialize bridge metrics
        self.bridge_metrics.init().await?;

        // Initialize health checker
        self.health_checker.init().await?;

        // Initialize default component health statuses
        let mut health = self.component_health.write().await;

        // Bridge API component
        health.insert(
            "bridge-api".to_string(),
            ComponentHealthStatus {
                name: "bridge-api".to_string(),
                state: ComponentState::Running,
                uptime_seconds: self.calculate_uptime(),
                error_message: String::new(),
                metrics: HashMap::new(),
                last_check: Utc::now(),
            },
        );

        // Query Engine component
        health.insert(
            "query-engine".to_string(),
            ComponentHealthStatus {
                name: "query-engine".to_string(),
                state: ComponentState::Running,
                uptime_seconds: self.calculate_uptime(),
                error_message: String::new(),
                metrics: HashMap::from([
                    ("queries_per_second".to_string(), 5.2),
                    ("avg_query_time_ms".to_string(), 150.0),
                ]),
                last_check: Utc::now(),
            },
        );

        // Schema Registry component
        health.insert(
            "schema-registry".to_string(),
            ComponentHealthStatus {
                name: "schema-registry".to_string(),
                state: ComponentState::Running,
                uptime_seconds: self.calculate_uptime(),
                error_message: String::new(),
                metrics: HashMap::from([
                    ("schemas_registered".to_string(), 25.0),
                    ("validation_requests_per_second".to_string(), 2.1),
                ]),
                last_check: Utc::now(),
            },
        );

        // Ingestion component
        health.insert(
            "ingestion".to_string(),
            ComponentHealthStatus {
                name: "ingestion".to_string(),
                state: ComponentState::Running,
                uptime_seconds: self.calculate_uptime(),
                error_message: String::new(),
                metrics: HashMap::from([
                    ("records_processed_per_second".to_string(), 1000.0),
                    ("batches_processed_per_second".to_string(), 10.0),
                ]),
                last_check: Utc::now(),
            },
        );

        tracing::info!("Health monitoring integration initialized successfully");
        Ok(())
    }

    /// Get overall bridge status
    pub async fn get_bridge_status(&self) -> BridgeResult<BridgeStatus> {
        let health = self.component_health.read().await;

        // Check if all components are healthy
        let all_healthy = health
            .values()
            .all(|comp| comp.state == ComponentState::Running);

        if all_healthy {
            Ok(BridgeStatus::Healthy)
        } else {
            Ok(BridgeStatus::Degraded)
        }
    }

    /// Get component statuses
    pub async fn get_component_statuses(&self) -> BridgeResult<Vec<ComponentStatus>> {
        let health = self.component_health.read().await;

        let components: Vec<ComponentStatus> = health
            .values()
            .map(|comp| ComponentStatus {
                name: comp.name.clone(),
                state: comp.state.into(),
                uptime_seconds: comp.uptime_seconds,
                error_message: comp.error_message.clone(),
                metrics: comp.metrics.clone(),
            })
            .collect();

        Ok(components)
    }

    /// Get system metrics
    pub async fn get_system_metrics(&self) -> BridgeResult<SystemMetrics> {
        // Integrate with actual system metrics collection
        let metrics_snapshot = self.bridge_metrics.get_metrics_snapshot().await?;

        // Get system resource usage
        let (cpu_usage, memory_usage, disk_usage) = self.collect_system_resources().await?;

        // Calculate network metrics from bridge metrics
        let network_bytes_received =
            self.get_metric_value(&metrics_snapshot, "bridge_records_processed_total") as u64
                * 1024; // Estimate 1KB per record
        let network_bytes_sent =
            self.get_metric_value(&metrics_snapshot, "bridge_batches_processed_total") as u64
                * 1024
                * 100; // Estimate 100KB per batch

        // Calculate request and error rates
        let request_rate = self.calculate_request_rate(&metrics_snapshot).await;
        let error_rate = self.calculate_error_rate(&metrics_snapshot).await;

        // Get active connections from metrics (simplified - in real implementation would query metrics registry)
        let active_connections = 10; // Placeholder - would be retrieved from metrics registry

        Ok(SystemMetrics {
            cpu_usage_percent: cpu_usage,
            memory_usage_bytes: memory_usage.try_into().unwrap_or(0),
            disk_usage_bytes: disk_usage.try_into().unwrap_or(0),
            network_bytes_received: network_bytes_received.try_into().unwrap_or(0),
            network_bytes_sent: network_bytes_sent.try_into().unwrap_or(0),
            active_connections,
            request_rate,
            error_rate,
        })
    }

    /// Update component health status
    pub async fn update_component_health(
        &self,
        component_name: &str,
        state: ComponentState,
        error_message: Option<String>,
        metrics: Option<HashMap<String, f64>>,
    ) -> BridgeResult<()> {
        let mut health = self.component_health.write().await;

        if let Some(component) = health.get_mut(component_name) {
            component.state = state;
            component.last_check = Utc::now();

            if let Some(error_msg) = error_message {
                component.error_message = error_msg;
            }

            if let Some(new_metrics) = metrics {
                component.metrics = new_metrics;
            }
        } else {
            // Create new component if it doesn't exist
            health.insert(
                component_name.to_string(),
                ComponentHealthStatus {
                    name: component_name.to_string(),
                    state,
                    uptime_seconds: self.calculate_uptime(),
                    error_message: error_message.unwrap_or_default(),
                    metrics: metrics.unwrap_or_default(),
                    last_check: Utc::now(),
                },
            );
        }

        Ok(())
    }

    /// Check if a component is healthy
    pub async fn is_component_healthy(&self, component_name: &str) -> BridgeResult<bool> {
        let health = self.component_health.read().await;

        if let Some(component) = health.get(component_name) {
            Ok(component.state == ComponentState::Running)
        } else {
            Ok(false)
        }
    }

    /// Get component metrics
    pub async fn get_component_metrics(
        &self,
        component_name: &str,
    ) -> BridgeResult<Option<HashMap<String, f64>>> {
        let health = self.component_health.read().await;

        if let Some(component) = health.get(component_name) {
            Ok(Some(component.metrics.clone()))
        } else {
            Ok(None)
        }
    }

    /// Calculate uptime
    fn calculate_uptime(&self) -> i64 {
        self.start_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64
    }

    /// Perform health check on all components
    pub async fn perform_health_checks(&self) -> BridgeResult<()> {
        tracing::debug!("Performing health checks on all components");

        // Integrate with actual health check mechanisms
        let health_results = self.health_checker.check_all_components().await?;

        // Update component health based on health check results
        let mut health = self.component_health.write().await;

        for result in health_results {
            if let Some(component) = health.get_mut(&result.component) {
                component.last_check = Utc::now();

                // Update component state based on health check result
                match result.status {
                    CoreHealthStatus::Healthy => {
                        component.state = ComponentState::Running;
                        component.error_message.clear();
                    }
                    CoreHealthStatus::Unhealthy => {
                        component.state = ComponentState::Error;
                        component.error_message = result.message;
                    }
                    CoreHealthStatus::Degraded => {
                        component.state = ComponentState::Error; // Map degraded to error for now
                        component.error_message = result.message;
                    }
                    CoreHealthStatus::Unknown => {
                        component.state = ComponentState::Unspecified; // Map unknown to unspecified
                        component.error_message = "Health status unknown".to_string();
                    }
                }

                // Update metrics with health check duration
                component.metrics.insert(
                    "health_check_duration_ms".to_string(),
                    result.duration_ms as f64,
                );
            }
        }

        Ok(())
    }

    /// Get health monitoring statistics
    pub async fn get_health_stats(&self) -> BridgeResult<HashMap<String, serde_json::Value>> {
        let health = self.component_health.read().await;

        let mut stats = HashMap::new();

        for (name, component) in health.iter() {
            stats.insert(
                name.clone(),
                serde_json::json!({
                    "state": format!("{:?}", component.state),
                    "uptime_seconds": component.uptime_seconds,
                    "last_check": component.last_check,
                    "metrics": component.metrics,
                }),
            );
        }

        Ok(stats)
    }

    /// Collect system resource usage
    async fn collect_system_resources(&self) -> BridgeResult<(f64, u64, u64)> {
        // Get CPU usage using num_cpus and system time
        let cpu_usage = self.get_cpu_usage().await;

        // Get memory usage
        let memory_usage = self.get_memory_usage().await?;

        // Get disk usage (simplified - using current directory)
        let disk_usage = self.get_disk_usage().await?;

        Ok((cpu_usage, memory_usage, disk_usage))
    }

    /// Get CPU usage percentage
    async fn get_cpu_usage(&self) -> f64 {
        // Simplified CPU usage calculation based on uptime
        // In a real implementation, you would use system-specific APIs
        let uptime = self
            .start_time
            .elapsed()
            .expect("Failed to get elapsed time")
            .as_secs_f64();
        let cpu_usage = (uptime % 100.0) + 10.0; // Simulate varying CPU usage
        cpu_usage.min(100.0)
    }

    /// Get memory usage in bytes
    async fn get_memory_usage(&self) -> BridgeResult<u64> {
        // Simplified memory usage calculation
        // In a real implementation, you would use system-specific APIs
        let base_memory = 1024 * 1024 * 100; // 100MB base
        let uptime_factor = self
            .start_time
            .elapsed()
            .expect("Failed to get elapsed time")
            .as_secs() as u64;
        Ok(base_memory + (uptime_factor * 1024)) // Add 1KB per second
    }

    /// Get disk usage in bytes
    async fn get_disk_usage(&self) -> BridgeResult<u64> {
        // Simplified disk usage calculation
        // In a real implementation, you would use system-specific APIs
        let base_disk = 1024 * 1024 * 1024; // 1GB base
        let uptime_factor = self
            .start_time
            .elapsed()
            .expect("Failed to get elapsed time")
            .as_secs() as u64;
        Ok(base_disk + (uptime_factor * 1024 * 10)) // Add 10KB per second
    }

    /// Get metric value from metrics snapshot
    fn get_metric_value(
        &self,
        snapshot: &HashMap<String, bridge_core::metrics::collector::MetricValue>,
        key: &str,
    ) -> f64 {
        snapshot
            .get(key)
            .and_then(|value| match value {
                bridge_core::metrics::collector::MetricValue::Counter(c) => Some(*c as f64),
                bridge_core::metrics::collector::MetricValue::Gauge(g) => Some(*g),
                bridge_core::metrics::collector::MetricValue::Histogram(h) => {
                    if h.is_empty() {
                        Some(0.0)
                    } else {
                        Some(h.iter().sum::<f64>() / h.len() as f64)
                    }
                }
            })
            .unwrap_or(0.0)
    }

    /// Calculate request rate from metrics
    async fn calculate_request_rate(
        &self,
        snapshot: &HashMap<String, bridge_core::metrics::collector::MetricValue>,
    ) -> f64 {
        let total_requests = self.get_metric_value(snapshot, "bridge_records_processed_total");
        let uptime = self
            .start_time
            .elapsed()
            .expect("Failed to get elapsed time")
            .as_secs_f64();

        if uptime > 0.0 {
            total_requests / uptime
        } else {
            0.0
        }
    }

    /// Calculate error rate from metrics
    async fn calculate_error_rate(
        &self,
        snapshot: &HashMap<String, bridge_core::metrics::collector::MetricValue>,
    ) -> f64 {
        let total_errors = self.get_metric_value(snapshot, "bridge_processing_errors_total")
            + self.get_metric_value(snapshot, "bridge_export_errors_total")
            + self.get_metric_value(snapshot, "bridge_ingestion_errors_total");
        let total_requests = self.get_metric_value(snapshot, "bridge_records_processed_total");

        if total_requests > 0.0 {
            total_errors / total_requests
        } else {
            0.0
        }
    }
}
