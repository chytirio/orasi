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
pub mod health;
pub mod metrics;
pub mod pipeline;
pub mod traits;
pub mod types;
pub mod utils; 

// Re-export commonly used types
pub use config::{BridgeConfig, IngestionConfig, LakehouseConfig, ProcessingConfig};
pub use error::{BridgeError, BridgeResult};
pub use types::{
    Aggregation, AnalyticsRequest, AnalyticsResponse, AnalyticsType, ExportResult, Filter,
    LogsBatch, LogsQuery, LogsResult, MetricsBatch, MetricsQuery, MetricsResult, ProcessedBatch,
    TelemetryBatch, TelemetryQuery, TimeRange, TracesBatch, TracesQuery, TracesResult, WriteResult,
};

// Re-export traits for Phase 2
pub use traits::{
    BridgePlugin, LakehouseConnector, LakehouseExporter, LakehouseReader, LakehouseWriter,
    StreamProcessor, StreamSink, StreamSource, TelemetryProcessor, TelemetryReceiver,
};

// Re-export pipeline for Phase 2
pub use pipeline::TelemetryIngestionPipeline;

// Re-export metrics for Phase 2
pub use metrics::BridgeMetrics;

// Commented out for Phase 2
// pub use health::HealthChecker;

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

/// Initialize bridge system (Phase 2 - with traits and pipeline)
pub async fn init_bridge() -> BridgeResult<()> {
    tracing::info!(
        "Initializing OpenTelemetry Data Lake Bridge v{} (Phase 2)",
        BRIDGE_VERSION
    );

    // Initialize basic components
    tracing::info!("Bridge initialization completed (Phase 2 - with traits and pipeline)");
    Ok(())
}

/// Shutdown bridge system (Phase 2 - with traits and pipeline)
pub async fn shutdown_bridge() -> BridgeResult<()> {
    tracing::info!("Shutting down OpenTelemetry Data Lake Bridge");

    // Shutdown basic components
    tracing::info!("Bridge shutdown completed");
    Ok(())
}

/// Get bridge status (Phase 2 - with traits and pipeline)
pub async fn get_bridge_status() -> BridgeResult<BridgeStatus> {
    Ok(BridgeStatus {
        version: BRIDGE_VERSION.to_string(),
        name: BRIDGE_NAME.to_string(),
        status: "running".to_string(),
        uptime_seconds: 0, // Placeholder
        components: vec![
            "core".to_string(),
            "types".to_string(),
            "config".to_string(),
            "traits".to_string(),   // Added for Phase 2
            "pipeline".to_string(), // Added for Phase 2
            "metrics".to_string(),  // Added for Phase 2
        ],
    })
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
        assert_eq!(status.status, "running");
        assert!(status.components.contains(&"traits".to_string())); // Check for Phase 2 component
        assert!(status.components.contains(&"pipeline".to_string())); // Check for Phase 2 component
        assert!(status.components.contains(&"metrics".to_string())); // Check for Phase 2 component
    }
}
