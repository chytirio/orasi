//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Iceberg connector for OpenTelemetry Data Lake Bridge
//!
//! This module provides integration with Apache Iceberg for storing and querying
//! telemetry data in a lakehouse format.

pub mod catalog;
pub mod config;
pub mod connector;
pub mod error;
pub mod exporter;
pub mod metrics;
pub mod reader;
pub mod schema;
pub mod writer;

// Re-export main types
pub use bridge_core::traits::{LakehouseConnector, LakehouseReader, LakehouseWriter};
pub use bridge_core::types::{ExportResult, ProcessedBatch, TelemetryBatch};

// Re-export our types
pub use config::IcebergConfig;
pub use connector::IcebergConnector;
pub use error::{IcebergError, IcebergResult};
pub use reader::IcebergReader;
pub use writer::IcebergWriter;

/// Iceberg connector version
pub const ICEBERG_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Iceberg connector name
pub const ICEBERG_NAME: &str = "iceberg-connector";
/// Default Iceberg table format version
pub const DEFAULT_TABLE_FORMAT_VERSION: u32 = 2;
/// Default Iceberg compression codec
pub const DEFAULT_COMPRESSION_CODEC: &str = "snappy";
/// Default Iceberg partition columns
pub const DEFAULT_PARTITION_COLUMNS: &[&str] = &["year", "month", "day", "hour"];
/// Default Iceberg batch size
pub const DEFAULT_BATCH_SIZE: usize = 10000;
/// Default Iceberg flush interval in milliseconds
pub const DEFAULT_FLUSH_INTERVAL_MS: u64 = 5000;
/// Default Iceberg catalog type
pub const DEFAULT_CATALOG_TYPE: &str = "hive";
/// Default Iceberg warehouse location
pub const DEFAULT_WAREHOUSE_LOCATION: &str = "s3://iceberg-warehouse";
/// Default Iceberg snapshot retention days
pub const DEFAULT_SNAPSHOT_RETENTION_DAYS: u32 = 30;

/// Initialize Iceberg connector with default configuration
pub async fn init_iceberg_connector() -> anyhow::Result<()> {
    tracing::info!("Initializing Apache Iceberg connector v{}", ICEBERG_VERSION);

    // Initialize Iceberg specific components
    // This will be implemented based on the specific Iceberg requirements

    tracing::info!("Apache Iceberg connector initialization completed");
    Ok(())
}

/// Shutdown Iceberg connector gracefully
pub async fn shutdown_iceberg_connector() -> anyhow::Result<()> {
    tracing::info!("Shutting down Apache Iceberg connector");

    // Perform graceful shutdown operations
    // This will be implemented based on the specific Iceberg requirements

    tracing::info!("Apache Iceberg connector shutdown completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_iceberg_connector_initialization() {
        let result = init_iceberg_connector().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_iceberg_connector_shutdown() {
        let result = shutdown_iceberg_connector().await;
        assert!(result.is_ok());
    }
}
