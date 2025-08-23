//! Status service for integrating gRPC status requests with the bridge health monitoring system

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{
    config::BridgeAPIConfig, metrics::ApiMetrics, proto::*,
    services::health::HealthMonitoringIntegration,
};
use bridge_core::BridgeResult;

/// Status service for handling status requests
pub struct StatusService {
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
    start_time: SystemTime,
    health_monitoring: Option<HealthMonitoringIntegration>,
}

impl StatusService {
    /// Create a new status service
    pub fn new(config: BridgeAPIConfig, metrics: ApiMetrics) -> Self {
        Self {
            config,
            metrics,
            start_time: SystemTime::now(),
            health_monitoring: None,
        }
    }

    /// Initialize the health monitoring integration
    pub async fn init_health_monitoring(&mut self) -> BridgeResult<()> {
        tracing::info!("Initializing health monitoring integration");

        let health_monitoring =
            HealthMonitoringIntegration::new(self.config.clone(), self.metrics.clone());
        health_monitoring.init().await?;
        self.health_monitoring = Some(health_monitoring);

        tracing::info!("Health monitoring integration initialized successfully");
        Ok(())
    }

    /// Get bridge status
    pub async fn get_status(
        &self,
        status_request: &GetStatusRequest,
    ) -> BridgeResult<GetStatusResponse> {
        tracing::info!(
            "Processing status request: include_components={}, include_metrics={}",
            status_request.include_components,
            status_request.include_metrics
        );

        // Get bridge status
        let status = self.determine_bridge_status().await?;
        let uptime = self.calculate_uptime();

        // Get component statuses if requested
        let mut components = Vec::new();
        if status_request.include_components {
            components = self.get_component_statuses().await?;
        }

        // Get system metrics if requested
        let metrics = if status_request.include_metrics {
            Some(self.get_system_metrics().await?)
        } else {
            None
        };

        // Get version info
        let version = self.get_version_info();

        Ok(GetStatusResponse {
            status: status.into(),
            uptime_seconds: uptime,
            components,
            metrics,
            version: Some(version),
        })
    }

    /// Determine overall bridge status
    async fn determine_bridge_status(&self) -> BridgeResult<BridgeStatus> {
        // Use real health monitoring if available
        if let Some(health_monitoring) = &self.health_monitoring {
            let system_health = health_monitoring.get_system_health().await?;
            return Ok(match system_health.status {
                crate::services::health::SystemHealthStatus::Healthy => BridgeStatus::Healthy,
                crate::services::health::SystemHealthStatus::Unhealthy => BridgeStatus::Unhealthy,
                crate::services::health::SystemHealthStatus::Degraded => BridgeStatus::Degraded,
            });
        }

        // Fallback to mock status if health monitoring not initialized
        tracing::warn!("Health monitoring not initialized, using mock status");

        // Check if we can determine status from component health
        let component_statuses = self.get_component_statuses().await?;

        let all_healthy = component_statuses
            .iter()
            .all(|comp| comp.state == ComponentState::Running as i32);

        if all_healthy {
            Ok(BridgeStatus::Healthy)
        } else {
            Ok(BridgeStatus::Degraded)
        }
    }

    /// Calculate bridge uptime
    fn calculate_uptime(&self) -> i64 {
        self.start_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64
    }

    /// Get component statuses
    async fn get_component_statuses(&self) -> BridgeResult<Vec<ComponentStatus>> {
        // Use real health monitoring if available
        if let Some(health_monitoring) = &self.health_monitoring {
            let system_health = health_monitoring.get_system_health().await?;
            return Ok(system_health.components);
        }

        // Fallback to mock component statuses if health monitoring not initialized
        tracing::warn!("Health monitoring not initialized, using mock component statuses");

        let mut components = Vec::new();

        // Bridge API component
        components.push(ComponentStatus {
            name: "bridge-api".to_string(),
            state: ComponentState::Running.into(),
            uptime_seconds: self.calculate_uptime(),
            error_message: String::new(),
            metrics: HashMap::new(),
        });

        // Query Engine component
        components.push(ComponentStatus {
            name: "query-engine".to_string(),
            state: ComponentState::Running.into(),
            uptime_seconds: self.calculate_uptime(),
            error_message: String::new(),
            metrics: HashMap::from([
                ("queries_per_second".to_string(), 5.2),
                ("avg_query_time_ms".to_string(), 150.0),
            ]),
        });

        // Schema Registry component
        components.push(ComponentStatus {
            name: "schema-registry".to_string(),
            state: ComponentState::Running.into(),
            uptime_seconds: self.calculate_uptime(),
            error_message: String::new(),
            metrics: HashMap::from([
                ("schemas_registered".to_string(), 25.0),
                ("validation_requests_per_second".to_string(), 2.1),
            ]),
        });

        // Ingestion component
        components.push(ComponentStatus {
            name: "ingestion".to_string(),
            state: ComponentState::Running.into(),
            uptime_seconds: self.calculate_uptime(),
            error_message: String::new(),
            metrics: HashMap::from([
                ("records_processed_per_second".to_string(), 1000.0),
                ("batches_processed_per_second".to_string(), 10.0),
            ]),
        });

        Ok(components)
    }

    /// Get system metrics
    async fn get_system_metrics(&self) -> BridgeResult<SystemMetrics> {
        // Use real health monitoring if available
        if let Some(health_monitoring) = &self.health_monitoring {
            let metrics = health_monitoring.get_system_metrics().await?;
            return Ok(SystemMetrics {
                cpu_usage_percent: metrics
                    .get("system.cpu_usage_percent")
                    .copied()
                    .unwrap_or(0.0),
                memory_usage_bytes: (metrics
                    .get("system.memory_usage_mb")
                    .copied()
                    .unwrap_or(0.0)
                    * 1024.0
                    * 1024.0) as i64,
                disk_usage_bytes: 1024 * 1024 * 1024, // Default 1GB
                network_bytes_received: 1024 * 1024,  // Default 1MB
                network_bytes_sent: 512 * 1024,       // Default 512KB
                active_connections: 10,               // Default
                request_rate: metrics.get("api.request_rate").copied().unwrap_or(0.0),
                error_rate: metrics.get("api.error_rate").copied().unwrap_or(0.0),
            });
        }

        // Fallback to mock system metrics if health monitoring not initialized
        tracing::warn!("Health monitoring not initialized, using mock system metrics");

        Ok(SystemMetrics {
            cpu_usage_percent: 15.5,
            memory_usage_bytes: 1024 * 1024 * 100, // 100MB
            disk_usage_bytes: 1024 * 1024 * 1024,  // 1GB
            network_bytes_received: 1024 * 1024,   // 1MB
            network_bytes_sent: 512 * 1024,        // 512KB
            active_connections: 10,
            request_rate: 5.2,
            error_rate: 0.1,
        })
    }

    /// Get version information
    fn get_version_info(&self) -> VersionInfo {
        VersionInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            commit_hash: std::env::var("VERGEN_GIT_SHA").unwrap_or_else(|_| "unknown".to_string()),
            build_timestamp: std::env::var("VERGEN_BUILD_TIMESTAMP")
                .unwrap_or_else(|_| "unknown".to_string()),
            rust_version: std::env::var("VERGEN_RUSTC_SEMVER")
                .unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}
