//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OpenTelemetry Data Lake Bridge Core
//!
//! This crate provides the core functionality for the OpenTelemetry Data Lake Bridge,
//! enabling bidirectional data flow between OpenTelemetry observability data and
//! modern data lakehouse solutions.
//!
//! NOTE: Phase 2 - Adding back core components with proper APIs

pub mod config;
pub mod error;
pub mod exporters;
pub mod health;
pub mod metrics;
pub mod pipeline;
pub mod processors;
pub mod receivers;
pub mod traits;
pub mod types;
pub mod utils;

pub use config::{BridgeConfig, IngestionConfig, LakehouseConfig, ProcessingConfig};
pub use error::{BridgeError, BridgeResult};
pub use types::{
    Aggregation, AnalyticsRequest, AnalyticsResponse, AnalyticsType, ExportResult, Filter,
    LogsBatch, LogsQuery, LogsResult, MetricsBatch, MetricsQuery, MetricsResult, ProcessedBatch,
    TelemetryBatch, TelemetryQuery, TimeRange, TracesBatch, TracesQuery, TracesResult, WriteResult,
};

pub use traits::{
    BridgePlugin, LakehouseConnector, LakehouseExporter, LakehouseReader, LakehouseWriter,
    StreamProcessor, StreamSink, StreamSource, TelemetryProcessor, TelemetryReceiver,
};

pub use exporters::{MockExporter, ParquetExporter};
pub use health::{HealthChecker, HealthManager};
pub use metrics::BridgeMetrics;
pub use pipeline::TelemetryIngestionPipeline;
pub use processors::{AggregateProcessor, FilterProcessor, MockProcessor, TransformProcessor};
pub use receivers::{MockReceiver, OtlpReceiver};

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Bridge version information
pub const BRIDGE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Bridge name
pub const BRIDGE_NAME: &str = "orasi";

/// Default configuration file path
pub const DEFAULT_CONFIG_PATH: &str = "config/bridge.toml";

/// Default ingestion endpoint
pub const DEFAULT_INGESTION_ENDPOINT: &str = "0.0.0.0:8080";

/// Default health check endpoint
pub const DEFAULT_HEALTH_ENDPOINT: &str = "0.0.0.0:8081";

/// Default metrics endpoint
pub const DEFAULT_METRICS_ENDPOINT: &str = "0.0.0.0:9091";

/// Bridge system state
#[derive(Clone)]
pub struct BridgeSystem {
    /// Bridge configuration
    pub config: BridgeConfig,

    /// Health checker
    pub health_checker: Arc<HealthChecker>,

    /// Metrics collector
    pub metrics: Arc<BridgeMetrics>,

    /// Pipeline manager
    pub pipeline: Arc<RwLock<Option<Arc<RwLock<TelemetryIngestionPipeline>>>>>,

    /// System start time
    pub start_time: chrono::DateTime<chrono::Utc>,

    /// System state
    pub state: Arc<RwLock<BridgeState>>,
}

/// Bridge system state
#[derive(Debug, Clone)]
pub struct BridgeState {
    /// Whether the bridge is running
    pub running: bool,

    /// Whether the bridge is healthy
    pub healthy: bool,

    /// Total uptime in seconds
    pub uptime_seconds: u64,

    /// Error count
    pub error_count: u64,

    /// Last error message
    pub last_error: Option<String>,

    /// Active components
    pub active_components: Vec<String>,
}

impl Default for BridgeState {
    fn default() -> Self {
        Self {
            running: false,
            healthy: true,
            uptime_seconds: 0,
            error_count: 0,
            last_error: None,
            active_components: vec![
                "core".to_string(),
                "types".to_string(),
                "config".to_string(),
                "traits".to_string(),
                "pipeline".to_string(),
                "metrics".to_string(),
                "health".to_string(),
            ],
        }
    }
}

impl BridgeSystem {
    /// Create a new bridge system
    pub async fn new(config: BridgeConfig) -> BridgeResult<Self> {
        info!("Creating new bridge system with configuration");

        // Create health checker
        let health_config = health::config::HealthCheckConfig {
            enable_health_checks: true,
            health_check_interval_ms: 30000, // 30 seconds
            health_check_timeout_ms: 5000,   // 5 seconds
            health_endpoint: "0.0.0.0".to_string(),
            health_port: 8081,
            enable_detailed_health: true,
            health_check_retry_count: 3,
            health_check_retry_delay_ms: 1000,
        };
        let health_checker = Arc::new(HealthChecker::new(health_config));

        // Create metrics collector
        let metrics_config = metrics::config::MetricsConfig {
            enabled: true,
            endpoint: "0.0.0.0:9091".to_string(),
            collection_interval: std::time::Duration::from_secs(10), // 10 seconds
            export_prometheus: true,
            prometheus_endpoint: "0.0.0.0:9091".to_string(),
        };
        let metrics = Arc::new(BridgeMetrics::new(metrics_config));

        // Create pipeline placeholder (will be configured later)
        let pipeline = Arc::new(RwLock::new(None));

        // Create system state
        let state = Arc::new(RwLock::new(BridgeState::default()));

        Ok(Self {
            config,
            health_checker,
            metrics,
            pipeline,
            start_time: chrono::Utc::now(),
            state,
        })
    }

    /// Initialize the bridge system
    pub async fn init(&self) -> BridgeResult<()> {
        info!("Initializing bridge system components");

        // Initialize health checker
        self.health_checker.init().await?;
        info!("Health checker initialized");

        // Initialize metrics
        self.metrics.init().await?;
        info!("Metrics collector initialized");

        // Update system state
        {
            let mut state = self.state.write().await;
            state.running = true;
            state.healthy = true;
        }

        info!("Bridge system initialization completed successfully");
        Ok(())
    }

    /// Start the bridge system
    pub async fn start(&self) -> BridgeResult<()> {
        info!("Starting bridge system");

        // Start health monitoring
        self.start_health_monitoring().await?;

        // Start metrics collection
        self.start_metrics_collection().await?;

        // Start uptime tracking
        self.start_uptime_tracking().await?;

        info!("Bridge system started successfully");
        Ok(())
    }

    /// Stop the bridge system
    pub async fn stop(&self) -> BridgeResult<()> {
        info!("Stopping bridge system");

        // Update system state
        {
            let mut state = self.state.write().await;
            state.running = false;
        }

        // Shutdown health checker
        self.health_checker.shutdown().await?;

        info!("Bridge system stopped successfully");
        Ok(())
    }

    /// Get bridge status
    pub async fn get_status(&self) -> BridgeStatus {
        let state = self.state.read().await;
        let uptime = (chrono::Utc::now() - self.start_time).num_seconds() as u64;

        BridgeStatus {
            version: BRIDGE_VERSION.to_string(),
            name: BRIDGE_NAME.to_string(),
            status: if state.running {
                "running".to_string()
            } else {
                "stopped".to_string()
            },
            uptime_seconds: uptime,
            components: state.active_components.clone(),
        }
    }

    /// Set the pipeline for the bridge system
    pub async fn set_pipeline(
        &self,
        pipeline: Arc<RwLock<TelemetryIngestionPipeline>>,
    ) -> BridgeResult<()> {
        let mut pipeline_guard = self.pipeline.write().await;
        *pipeline_guard = Some(pipeline);
        info!("Pipeline set for bridge system");
        Ok(())
    }

    /// Get the pipeline from the bridge system
    pub async fn get_pipeline(&self) -> Option<Arc<RwLock<TelemetryIngestionPipeline>>> {
        let pipeline_guard = self.pipeline.read().await;
        pipeline_guard.clone()
    }

    /// Start the pipeline
    pub async fn start_pipeline(&self) -> BridgeResult<()> {
        let pipeline_guard = self.pipeline.read().await;
        if let Some(pipeline) = pipeline_guard.as_ref() {
            let mut pipeline = pipeline.write().await;
            pipeline.start().await?;
            info!("Pipeline started successfully");
        } else {
            return Err(BridgeError::internal("No pipeline configured"));
        }
        Ok(())
    }

    /// Stop the pipeline
    pub async fn stop_pipeline(&self) -> BridgeResult<()> {
        let pipeline_guard = self.pipeline.read().await;
        if let Some(pipeline) = pipeline_guard.as_ref() {
            let mut pipeline = pipeline.write().await;
            pipeline.stop().await?;
            info!("Pipeline stopped successfully");
        } else {
            return Err(BridgeError::internal("No pipeline configured"));
        }
        Ok(())
    }

    /// Start health monitoring task
    async fn start_health_monitoring(&self) -> BridgeResult<()> {
        let health_checker = Arc::clone(&self.health_checker);
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Perform health checks
                match health_checker.check_all_components().await {
                    Ok(results) => {
                        let healthy_count = results
                            .iter()
                            .filter(|r| r.status == health::types::HealthStatus::Healthy)
                            .count();
                        let total_count = results.len();

                        let mut state_guard = state.write().await;
                        state_guard.healthy = healthy_count == total_count;

                        if healthy_count < total_count {
                            warn!(
                                "Some components are unhealthy: {}/{} healthy",
                                healthy_count, total_count
                            );
                        }
                    }
                    Err(e) => {
                        error!("Health check failed: {}", e);
                        let mut state_guard = state.write().await;
                        state_guard.healthy = false;
                        state_guard.error_count += 1;
                        state_guard.last_error = Some(e.to_string());
                    }
                }
            }
        });

        Ok(())
    }

    /// Start metrics collection task
    async fn start_metrics_collection(&self) -> BridgeResult<()> {
        let metrics = Arc::clone(&self.metrics);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

            loop {
                interval.tick().await;

                // Update uptime metric
                metrics.update_uptime().await;
            }
        });

        Ok(())
    }

    /// Start uptime tracking task
    async fn start_uptime_tracking(&self) -> BridgeResult<()> {
        let state = Arc::clone(&self.state);
        let start_time = self.start_time;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

            loop {
                interval.tick().await;

                let uptime = (chrono::Utc::now() - start_time).num_seconds() as u64;
                let mut state_guard = state.write().await;
                state_guard.uptime_seconds = uptime;
            }
        });

        Ok(())
    }
}

/// Global bridge system instance
static BRIDGE_SYSTEM: tokio::sync::OnceCell<Arc<BridgeSystem>> = tokio::sync::OnceCell::const_new();

/// Initialize bridge system (Phase 2 - with traits and pipeline)
pub async fn init_bridge() -> BridgeResult<()> {
    tracing::info!(
        "Initializing OpenTelemetry Data Lake Bridge v{} (Phase 2)",
        BRIDGE_VERSION
    );

    // Load configuration
    let config = BridgeConfig::default();

    // Create bridge system
    let bridge_system = BridgeSystem::new(config).await?;

    // Initialize components
    bridge_system.init().await?;

    // Store global instance
    BRIDGE_SYSTEM
        .set(Arc::new(bridge_system))
        .map_err(|_| BridgeError::internal("Failed to set global bridge system instance"))?;

    tracing::info!("Bridge initialization completed (Phase 2 - with traits and pipeline)");
    Ok(())
}

/// Get bridge system instance
pub async fn get_bridge_system() -> BridgeResult<Arc<BridgeSystem>> {
    BRIDGE_SYSTEM
        .get()
        .cloned()
        .ok_or_else(|| BridgeError::internal("Bridge system not initialized"))
}

/// Shutdown bridge system (Phase 2 - with traits and pipeline)
pub async fn shutdown_bridge() -> BridgeResult<()> {
    tracing::info!("Shutting down OpenTelemetry Data Lake Bridge");

    if let Some(bridge_system) = BRIDGE_SYSTEM.get() {
        bridge_system.stop().await?;
    }

    tracing::info!("Bridge shutdown completed");
    Ok(())
}

/// Get bridge status (Phase 2 - with traits and pipeline)
pub async fn get_bridge_status() -> BridgeResult<BridgeStatus> {
    if let Some(bridge_system) = BRIDGE_SYSTEM.get() {
        Ok(bridge_system.get_status().await)
    } else {
        // Return default status if not initialized
        Ok(BridgeStatus {
            version: BRIDGE_VERSION.to_string(),
            name: BRIDGE_NAME.to_string(),
            status: "not_initialized".to_string(),
            uptime_seconds: 0,
            components: vec![
                "core".to_string(),
                "types".to_string(),
                "config".to_string(),
                "traits".to_string(),
                "pipeline".to_string(),
                "metrics".to_string(),
                "health".to_string(),
            ],
        })
    }
}

/// Bridge status information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeStatus {
    /// Bridge version
    pub version: String,

    /// Bridge name
    pub name: String,

    /// Current status
    pub status: String,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// Active components
    pub components: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_initialization() {
        let result = init_bridge().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bridge_shutdown() {
        let result = shutdown_bridge().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bridge_status() {
        let status = get_bridge_status().await.unwrap();
        assert_eq!(status.version, BRIDGE_VERSION);
        assert_eq!(status.name, BRIDGE_NAME);
        assert!(status.components.contains(&"traits".to_string()));
        assert!(status.components.contains(&"pipeline".to_string()));
        assert!(status.components.contains(&"metrics".to_string()));
        assert!(status.components.contains(&"health".to_string()));
    }

    #[tokio::test]
    async fn test_bridge_system_creation() {
        let config = BridgeConfig::default();
        let bridge_system = BridgeSystem::new(config).await.unwrap();

        let status = bridge_system.get_status().await;
        assert_eq!(status.name, BRIDGE_NAME);
        assert_eq!(status.version, BRIDGE_VERSION);
    }
}
