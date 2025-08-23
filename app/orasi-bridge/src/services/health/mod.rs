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
                metrics: HashMap::new(),
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
                metrics: HashMap::new(),
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
                metrics: HashMap::new(),
                last_check: Utc::now(),
            },
        );

        tracing::info!("Health monitoring integration initialized successfully");
        Ok(())
    }

    /// Get the overall system health status
    pub async fn get_system_health(&self) -> BridgeResult<SystemHealthInfo> {
        let health = self.component_health.read().await;
        let mut overall_status = SystemHealthStatus::Healthy;
        let mut component_statuses = Vec::new();

        for (name, status) in health.iter() {
            let component_status = ComponentStatus {
                name: name.clone(),
                state: status.state.clone().into(),
                uptime_seconds: status.uptime_seconds,
                error_message: status.error_message.clone(),
                metrics: status.metrics.clone(),
            };

            component_statuses.push(component_status);

            // Update overall status based on component status
            match status.state {
                ComponentState::Stopped | ComponentState::Error => {
                    overall_status = SystemHealthStatus::Unhealthy;
                }
                ComponentState::Starting | ComponentState::Stopping => {
                    if overall_status == SystemHealthStatus::Healthy {
                        overall_status = SystemHealthStatus::Degraded;
                    }
                }
                _ => {}
            }
        }

        Ok(SystemHealthInfo {
            status: overall_status,
            components: component_statuses,
            uptime_seconds: self.calculate_uptime(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Utc::now(),
        })
    }

    /// Get health status for a specific component
    pub async fn get_component_health(
        &self,
        component_name: &str,
    ) -> BridgeResult<ComponentHealthStatus> {
        let health = self.component_health.read().await;

        health.get(component_name).cloned().ok_or_else(|| {
            bridge_core::BridgeError::validation(format!(
                "Component '{}' not found",
                component_name
            ))
        })
    }

    /// Update component health status
    pub async fn update_component_health(
        &self,
        component_name: &str,
        status: ComponentHealthStatus,
    ) -> BridgeResult<()> {
        let mut health = self.component_health.write().await;
        health.insert(component_name.to_string(), status);
        Ok(())
    }

    /// Perform health check on a specific component
    pub async fn check_component_health(
        &self,
        component_name: &str,
    ) -> BridgeResult<ComponentHealthStatus> {
        tracing::debug!("Checking health for component: {}", component_name);

        // Perform actual health check using the health checker
        let health_result = self.health_checker.check_component(component_name).await?;

        // Convert core health status to component health status
        let component_status = match health_result.status {
            CoreHealthStatus::Healthy => ComponentState::Running,
            CoreHealthStatus::Unhealthy => ComponentState::Error,
            CoreHealthStatus::Degraded => ComponentState::Starting,
            CoreHealthStatus::Unknown => ComponentState::Starting,
        };

        let mut metrics = HashMap::new();
        // Health result doesn't have metrics field, so we'll use empty metrics for now
        // if let Some(health_metrics) = health_result.metrics {
        //     for (key, value) in health_metrics {
        //         if let Ok(num_value) = value.parse::<f64>() {
        //             metrics.insert(key, num_value);
        //         }
        //     }
        // }

        let status = ComponentHealthStatus {
            name: component_name.to_string(),
            state: component_status,
            uptime_seconds: self.calculate_uptime(),
            error_message: health_result.message,
            metrics,
            last_check: Utc::now(),
        };

        // Update the stored health status
        self.update_component_health(component_name, status.clone())
            .await?;

        Ok(status)
    }

    /// Get system metrics
    pub async fn get_system_metrics(&self) -> BridgeResult<HashMap<String, f64>> {
        let mut metrics = HashMap::new();

        // Get bridge metrics
        let bridge_metrics = self.bridge_metrics.get_metrics_snapshot().await?;
        for (key, value) in bridge_metrics {
            // Convert MetricValue to f64
            let num_value = match value {
                bridge_core::metrics::collector::MetricValue::Counter(v) => v as f64,
                bridge_core::metrics::collector::MetricValue::Gauge(v) => v,
                bridge_core::metrics::collector::MetricValue::Histogram(v) => v.iter().sum::<f64>(),
            };
            metrics.insert(format!("bridge.{}", key), num_value);
        }

        // Get API metrics - ApiMetrics doesn't have get_metrics_snapshot method
        // let api_metrics = self.metrics.get_metrics_snapshot();
        // for (key, value) in api_metrics {
        //     metrics.insert(format!("api.{}", key), value);
        // }

        // Add system-level metrics
        metrics.insert(
            "system.uptime_seconds".to_string(),
            self.calculate_uptime() as f64,
        );
        metrics.insert(
            "system.memory_usage_mb".to_string(),
            self.get_memory_usage(),
        );
        metrics.insert("system.cpu_usage_percent".to_string(), self.get_cpu_usage());

        Ok(metrics)
    }

    /// Get detailed health information
    pub async fn get_detailed_health(&self) -> BridgeResult<DetailedHealthStatus> {
        let system_health = self.get_system_health().await?;
        let system_metrics = self.get_system_metrics().await?;

        let mut component_details = Vec::new();
        let health = self.component_health.read().await;

        for (name, status) in health.iter() {
            let detailed_component = DetailedComponentStatus {
                name: name.clone(),
                status: status.state.clone(),
                uptime_seconds: status.uptime_seconds,
                error_message: status.error_message.clone(),
                metrics: status.metrics.clone(),
                last_check: status.last_check,
                health_endpoint: self.get_component_health_endpoint(name),
            };

            component_details.push(detailed_component);
        }

        Ok(DetailedHealthStatus {
            system: system_health,
            components: component_details,
            metrics: system_metrics,
            timestamp: Utc::now(),
        })
    }

    /// Restart a component
    pub async fn restart_component(&self, component_name: &str) -> BridgeResult<bool> {
        tracing::info!("Restarting component: {}", component_name);

        // Update component status to stopping
        let mut status = self.get_component_health(component_name).await?;
        status.state = ComponentState::Stopping;
        self.update_component_health(component_name, status).await?;

        // Simulate restart delay
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Update component status to starting
        let mut status = self.get_component_health(component_name).await?;
        status.state = ComponentState::Starting;
        self.update_component_health(component_name, status).await?;

        // Simulate startup delay
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Update component status to running
        let mut status = self.get_component_health(component_name).await?;
        status.state = ComponentState::Running;
        status.error_message = String::new();
        self.update_component_health(component_name, status).await?;

        tracing::info!("Component {} restarted successfully", component_name);
        Ok(true)
    }

    /// Calculate system uptime in seconds
    fn calculate_uptime(&self) -> i64 {
        self.start_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    /// Get memory usage in MB (mock implementation)
    fn get_memory_usage(&self) -> f64 {
        // In a real implementation, this would read from /proc/meminfo or similar
        512.0 // Mock value: 512 MB
    }

    /// Get CPU usage percentage (mock implementation)
    fn get_cpu_usage(&self) -> f64 {
        // In a real implementation, this would read from /proc/stat or similar
        15.5 // Mock value: 15.5%
    }

    /// Get component health endpoint
    fn get_component_health_endpoint(&self, component_name: &str) -> String {
        match component_name {
            "bridge-api" => "http://localhost:8081/health".to_string(),
            "query-engine" => "http://localhost:8082/health".to_string(),
            "schema-registry" => "http://localhost:8083/health".to_string(),
            "ingestion" => "http://localhost:8084/health".to_string(),
            _ => format!("http://localhost:8080/{}/health", component_name),
        }
    }

    /// Shutdown the health monitoring integration
    pub async fn shutdown(&self) -> BridgeResult<()> {
        tracing::info!("Shutting down health monitoring integration");

        // Shutdown health checker
        self.health_checker.shutdown().await?;

        // Bridge metrics don't have a shutdown method
        // self.bridge_metrics.shutdown().await?;

        tracing::info!("Health monitoring integration shutdown completed");
        Ok(())
    }
}

/// System health status enum
#[derive(Debug, Clone, PartialEq)]
pub enum SystemHealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

/// System health information
#[derive(Debug, Clone)]
pub struct SystemHealthInfo {
    pub status: SystemHealthStatus,
    pub components: Vec<ComponentStatus>,
    pub uptime_seconds: i64,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}

/// Detailed health status
#[derive(Debug, Clone)]
pub struct DetailedHealthStatus {
    pub system: SystemHealthInfo,
    pub components: Vec<DetailedComponentStatus>,
    pub metrics: HashMap<String, f64>,
    pub timestamp: DateTime<Utc>,
}

/// Detailed component status
#[derive(Debug, Clone)]
pub struct DetailedComponentStatus {
    pub name: String,
    pub status: ComponentState,
    pub uptime_seconds: i64,
    pub error_message: String,
    pub metrics: HashMap<String, f64>,
    pub last_check: DateTime<Utc>,
    pub health_endpoint: String,
}
