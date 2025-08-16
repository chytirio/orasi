//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Snowflake connector for OpenTelemetry Data Lake Bridge
//!
//! This module provides integration with Snowflake for storing and querying
//! telemetry data in a lakehouse format.

pub mod config;
pub mod connector;
pub mod error;
pub mod exporter;
pub mod reader;
pub mod writer;

// Re-export main types
pub use bridge_core::traits::{LakehouseConnector, LakehouseReader, LakehouseWriter};
pub use bridge_core::types::{ExportResult, ProcessedBatch, TelemetryBatch};

// Re-export Snowflake specific types
pub use config::SnowflakeConfig;
pub use connector::SnowflakeConnector;
pub use error::{SnowflakeError, SnowflakeResult};
pub use reader::SnowflakeReader;
pub use writer::SnowflakeWriter;

/// Snowflake connector version
pub const SNOWFLAKE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Snowflake connector name
pub const SNOWFLAKE_NAME: &str = "snowflake-connector";
/// Default Snowflake warehouse size
pub const DEFAULT_WAREHOUSE_SIZE: &str = "X-SMALL";
/// Default Snowflake auto-suspend time in seconds
pub const DEFAULT_AUTO_SUSPEND_SECS: u64 = 300;
/// Default Snowflake connection timeout in seconds
pub const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 60;
/// Default Snowflake query timeout in seconds
pub const DEFAULT_QUERY_TIMEOUT_SECS: u64 = 300;
/// Default Snowflake batch size
pub const DEFAULT_BATCH_SIZE: usize = 10000;
/// Default Snowflake flush interval in milliseconds
pub const DEFAULT_FLUSH_INTERVAL_MS: u64 = 5000;

/// Initialize Snowflake connector with default configuration
pub async fn init_snowflake_connector() -> anyhow::Result<()> {
    tracing::info!("Initializing Snowflake connector v{}", SNOWFLAKE_VERSION);

    // Initialize Snowflake specific components
    // This will be implemented based on the specific Snowflake requirements

    tracing::info!("Snowflake connector initialization completed");
    Ok(())
}

/// Shutdown Snowflake connector gracefully
pub async fn shutdown_snowflake_connector() -> anyhow::Result<()> {
    tracing::info!("Shutting down Snowflake connector");

    // Perform graceful shutdown operations
    // This will be implemented based on the specific Snowflake requirements

    tracing::info!("Snowflake connector shutdown completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_snowflake_connector_initialization() {
        let result = init_snowflake_connector().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_snowflake_connector_shutdown() {
        let result = shutdown_snowflake_connector().await;
        assert!(result.is_ok());
    }
}
