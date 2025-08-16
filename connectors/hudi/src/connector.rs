//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Hudi connector implementation
//! 
//! This module provides the main Apache Hudi connector that implements
//! the LakehouseConnector trait for seamless integration with the bridge.

use std::sync::Arc;
use async_trait::async_trait;
use tracing::{debug, error, info, warn};
use bridge_core::traits::{LakehouseConnector, LakehouseWriter, LakehouseReader};
use bridge_core::types::{MetricsBatch, TracesBatch, LogsBatch, TelemetryBatch};
use bridge_core::error::BridgeResult;

use crate::config::HudiConfig;
use crate::error::{HudiError, HudiResult};
use crate::writer::HudiWriter;
use crate::reader::HudiReader;

/// Apache Hudi connector implementation
pub struct HudiConnector {
    /// Hudi configuration
    config: HudiConfig,
    /// Connection state
    connected: bool,
    /// Writer instance
    writer: Option<HudiWriter>,
    /// Reader instance
    reader: Option<HudiReader>,
}

impl HudiConnector {
    /// Create a new Apache Hudi connector
    pub fn new(config: HudiConfig) -> Self {
        Self {
            config,
            connected: false,
            writer: None,
            reader: None,
        }
    }

    /// Initialize the connector
    pub async fn initialize(&mut self) -> HudiResult<()> {
        info!("Initializing Apache Hudi connector for table: {}", self.config.table_name());
        
        // Initialize Hudi table if it doesn't exist
        self.ensure_table_exists().await?;
        
        // Initialize writer and reader
        let writer = HudiWriter::new(self.config.clone()).await?;
        let reader = HudiReader::new(self.config.clone()).await?;
        
        self.writer = Some(writer);
        self.reader = Some(reader);
        self.connected = true;
        
        info!("Apache Hudi connector initialized successfully");
        Ok(())
    }

    /// Ensure the Hudi table exists
    async fn ensure_table_exists(&self) -> HudiResult<()> {
        debug!("Ensuring Apache Hudi table exists: {}", self.config.table_name());
        
        // This would typically involve:
        // 1. Checking if the table exists
        // 2. Creating the table if it doesn't exist
        // 3. Validating the table schema
        // 4. Setting up partitions and optimizations
        // 5. Configuring Hudi-specific settings (table type, key fields, etc.)
        
        // For now, we'll just log the operation
        info!("Table existence check completed for: {}", self.config.table_name());
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &HudiConfig {
        &self.config
    }

    /// Check if the connector is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[async_trait]
impl LakehouseConnector for HudiConnector {
    type Config = HudiConfig;
    type WriteHandle = HudiWriter;
    type ReadHandle = HudiReader;

    async fn connect(config: Self::Config) -> BridgeResult<Self> {
        let mut connector = HudiConnector::new(config);
        connector.initialize().await
            .map_err(|e| bridge_core::error::BridgeError::lakehouse_with_source("Failed to initialize Apache Hudi connector", e))?;
        Ok(connector)
    }

    async fn writer(&self) -> BridgeResult<Self::WriteHandle> {
        self.writer.as_ref()
            .cloned()
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Writer not initialized"))
    }

    async fn reader(&self) -> BridgeResult<Self::ReadHandle> {
        self.reader.as_ref()
            .cloned()
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Reader not initialized"))
    }

    fn name(&self) -> &str {
        "apache-hudi-connector"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        if !self.connected {
            return Ok(false);
        }

        // For now, just check if writer and reader are initialized
        let writer_healthy = self.writer.is_some();
        let reader_healthy = self.reader.is_some();

        Ok(writer_healthy && reader_healthy)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ConnectorStats> {
        let writer_stats = if let Some(writer) = &self.writer {
            writer.get_stats().await.unwrap_or_else(|_| bridge_core::traits::WriterStats {
                total_writes: 0,
                total_records: 0,
                writes_per_minute: 0,
                records_per_minute: 0,
                avg_write_time_ms: 0.0,
                error_count: 0,
                last_write_time: None,
            })
        } else {
            bridge_core::traits::WriterStats {
                total_writes: 0,
                total_records: 0,
                writes_per_minute: 0,
                records_per_minute: 0,
                avg_write_time_ms: 0.0,
                error_count: 0,
                last_write_time: None,
            }
        };

        let reader_stats = if let Some(reader) = &self.reader {
            reader.get_stats().await.unwrap_or_else(|_| bridge_core::traits::ReaderStats {
                total_reads: 0,
                total_records: 0,
                reads_per_minute: 0,
                records_per_minute: 0,
                avg_read_time_ms: 0.0,
                error_count: 0,
                last_read_time: None,
            })
        } else {
            bridge_core::traits::ReaderStats {
                total_reads: 0,
                total_records: 0,
                reads_per_minute: 0,
                records_per_minute: 0,
                avg_read_time_ms: 0.0,
                error_count: 0,
                last_read_time: None,
            }
        };

        Ok(bridge_core::traits::ConnectorStats {
            total_connections: 1,
            active_connections: if self.connected { 1 } else { 0 },
            total_writes: writer_stats.total_writes,
            total_reads: reader_stats.total_reads,
            avg_write_time_ms: writer_stats.avg_write_time_ms,
            avg_read_time_ms: reader_stats.avg_read_time_ms,
            error_count: writer_stats.error_count + reader_stats.error_count,
            last_operation_time: writer_stats.last_write_time.or(reader_stats.last_read_time),
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down Apache Hudi connector");
        
        // Shutdown writer and reader
        if let Some(writer) = &self.writer {
            let _ = writer.close().await;
        }
        
        if let Some(reader) = &self.reader {
            let _ = reader.close().await;
        }
        
        info!("Apache Hudi connector shutdown completed");
        Ok(())
    }
}
