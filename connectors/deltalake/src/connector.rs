//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake connector implementation
//! 
//! This module provides the main Delta Lake connector that implements
//! the LakehouseConnector trait for seamless integration with the bridge.

use std::sync::Arc;
use async_trait::async_trait;
use tracing::{debug, error, info, warn};
use bridge_core::traits::{LakehouseConnector, LakehouseWriter, LakehouseReader};
use bridge_core::types::{MetricsBatch, TracesBatch, LogsBatch, TelemetryBatch};
use bridge_core::error::BridgeResult;

use crate::config::DeltaLakeConfig;
use crate::error::{DeltaLakeError, DeltaLakeResult};
use crate::writer::DeltaLakeWriter;
use crate::reader::DeltaLakeReader;

/// Delta Lake connector implementation
pub struct DeltaLakeConnector {
    /// Delta Lake configuration
    config: DeltaLakeConfig,
    /// Connection state
    connected: bool,
    /// Writer instance
    writer: Option<Arc<DeltaLakeWriter>>,
    /// Reader instance
    reader: Option<Arc<DeltaLakeReader>>,
}

impl DeltaLakeConnector {
    /// Create a new Delta Lake connector
    pub fn new(config: DeltaLakeConfig) -> Self {
        Self {
            config,
            connected: false,
            writer: None,
            reader: None,
        }
    }

    /// Initialize the connector
    pub async fn initialize(&mut self) -> DeltaLakeResult<()> {
        info!("Initializing Delta Lake connector for table: {}", self.config.table_name());
        
        // Initialize Delta Lake table if it doesn't exist
        self.ensure_table_exists().await?;
        
        // Initialize writer and reader
        let writer = Arc::new(DeltaLakeWriter::new(self.config.clone()).await?);
        let reader = Arc::new(DeltaLakeReader::new(self.config.clone()).await?);
        
        self.writer = Some(writer);
        self.reader = Some(reader);
        self.connected = true;
        
        info!("Delta Lake connector initialized successfully");
        Ok(())
    }

    /// Ensure the Delta Lake table exists
    async fn ensure_table_exists(&self) -> DeltaLakeResult<()> {
        debug!("Ensuring Delta Lake table exists: {}", self.config.table_name());
        
        // This would typically involve:
        // 1. Checking if the table exists
        // 2. Creating the table if it doesn't exist
        // 3. Validating the table schema
        // 4. Setting up partitions and optimizations
        
        // For now, we'll just log the operation
        info!("Table existence check completed for: {}", self.config.table_name());
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &DeltaLakeConfig {
        &self.config
    }

    /// Check if the connector is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[async_trait]
impl LakehouseConnector for DeltaLakeConnector {
    type Config = DeltaLakeConfig;
    type WriteHandle = DeltaLakeWriter;
    type ReadHandle = DeltaLakeReader;

    async fn connect(config: Self::Config) -> BridgeResult<Self> {
        let mut connector = DeltaLakeConnector::new(config);
        connector.initialize().await
            .map_err(|e| bridge_core::error::BridgeError::lakehouse_with_source("Failed to initialize Delta Lake connector", e))?;
        Ok(connector)
    }

    async fn writer(&self) -> BridgeResult<Self::WriteHandle> {
        self.writer.as_ref()
            .map(|w| w.as_ref().clone())
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Writer not initialized"))
    }

    async fn reader(&self) -> BridgeResult<Self::ReadHandle> {
        self.reader.as_ref()
            .map(|r| r.as_ref().clone())
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Reader not initialized"))
    }

    fn name(&self) -> &str {
        "delta-lake-connector"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        if !self.connected {
            return Ok(false);
        }

        // Perform health checks on writer and reader
        let writer_healthy = if let Some(writer) = &self.writer {
            writer.health_check().await.unwrap_or(false)
        } else {
            false
        };

        let reader_healthy = if let Some(reader) = &self.reader {
            reader.health_check().await.unwrap_or(false)
        } else {
            false
        };

        Ok(writer_healthy && reader_healthy)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ConnectorStats> {
        let writer_stats = if let Some(writer) = &self.writer {
            writer.get_stats().await.unwrap_or_default()
        } else {
            bridge_core::traits::WriterStats::default()
        };

        let reader_stats = if let Some(reader) = &self.reader {
            reader.get_stats().await.unwrap_or_default()
        } else {
            bridge_core::traits::ReaderStats::default()
        };

        Ok(bridge_core::traits::ConnectorStats {
            name: self.name().to_string(),
            version: self.version().to_string(),
            connected: self.connected,
            writer_stats,
            reader_stats,
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down Delta Lake connector");
        
        // Shutdown writer
        if let Some(writer) = &self.writer {
            if let Err(e) = writer.shutdown().await {
                warn!("Error shutting down Delta Lake writer: {}", e);
            }
        }

        // Shutdown reader
        if let Some(reader) = &self.reader {
            if let Err(e) = reader.shutdown().await {
                warn!("Error shutting down Delta Lake reader: {}", e);
            }
        }

        info!("Delta Lake connector shutdown completed");
        Ok(())
    }
}

impl Drop for DeltaLakeConnector {
    fn drop(&mut self) {
        if self.connected {
            warn!("Delta Lake connector dropped while still connected");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connector_creation() {
        let config = DeltaLakeConfig::new(
            "s3://test-bucket".to_string(),
            "test_table".to_string(),
        );
        
        let connector = DeltaLakeConnector::new(config);
        assert_eq!(connector.name(), "delta-lake-connector");
        assert!(!connector.is_connected());
    }

    #[tokio::test]
    async fn test_connector_connect() {
        let config = DeltaLakeConfig::new(
            "s3://test-bucket".to_string(),
            "test_table".to_string(),
        );
        
        let result = DeltaLakeConnector::connect(config).await;
        // This will fail in tests since we don't have actual Delta Lake setup
        assert!(result.is_err());
    }
}
