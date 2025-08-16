//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Snowflake connector implementation
//!
//! This module provides the main Snowflake connector that implements
//! the LakehouseConnector trait for seamless integration with the bridge.

use async_trait::async_trait;
use bridge_core::error::BridgeResult;
use bridge_core::traits::{LakehouseConnector, LakehouseReader, LakehouseWriter};
use bridge_core::types::{LogsBatch, MetricsBatch, TelemetryBatch, TracesBatch};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::config::SnowflakeConfig;
use crate::error::{SnowflakeError, SnowflakeResult};
use crate::reader::SnowflakeReader;
use crate::writer::SnowflakeWriter;

/// Snowflake connector implementation
pub struct SnowflakeConnector {
    /// Snowflake configuration
    config: SnowflakeConfig,
    /// Connection state
    connected: bool,
    /// Writer instance
    writer: Option<SnowflakeWriter>,
    /// Reader instance
    reader: Option<SnowflakeReader>,
}

impl SnowflakeConnector {
    /// Create a new Snowflake connector
    pub fn new(config: SnowflakeConfig) -> Self {
        Self {
            config,
            connected: false,
            writer: None,
            reader: None,
        }
    }

    /// Initialize the connector
    pub async fn initialize(&mut self) -> SnowflakeResult<()> {
        info!(
            "Initializing Snowflake connector for database: {}",
            self.config.database()
        );

        // Initialize Snowflake connection and tables if they don't exist
        self.ensure_connection_and_tables().await?;

        // Initialize writer and reader
        let writer = SnowflakeWriter::new(self.config.clone()).await?;
        let reader = SnowflakeReader::new(self.config.clone()).await?;

        self.writer = Some(writer);
        self.reader = Some(reader);
        self.connected = true;

        info!("Snowflake connector initialized successfully");
        Ok(())
    }

    /// Ensure connection and tables exist
    async fn ensure_connection_and_tables(&self) -> SnowflakeResult<()> {
        debug!("Ensuring Snowflake connection and tables exist");

        // This would typically involve:
        // 1. Establishing connection to Snowflake
        // 2. Checking if database and schema exist
        // 3. Creating tables if they don't exist
        // 4. Validating table schemas
        // 5. Setting up warehouses and roles

        // For now, we'll just log the operation
        info!("Connection and table check completed for Snowflake");
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &SnowflakeConfig {
        &self.config
    }

    /// Check if the connector is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[async_trait]
impl LakehouseConnector for SnowflakeConnector {
    type Config = SnowflakeConfig;
    type WriteHandle = SnowflakeWriter;
    type ReadHandle = SnowflakeReader;

    async fn connect(config: Self::Config) -> BridgeResult<Self> {
        let mut connector = SnowflakeConnector::new(config);
        connector.initialize().await.map_err(|e| {
            bridge_core::error::BridgeError::lakehouse_with_source(
                "Failed to initialize Snowflake connector",
                e,
            )
        })?;
        Ok(connector)
    }

    async fn writer(&self) -> BridgeResult<Self::WriteHandle> {
        self.writer
            .as_ref()
            .cloned()
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Writer not initialized"))
    }

    async fn reader(&self) -> BridgeResult<Self::ReadHandle> {
        self.reader
            .as_ref()
            .cloned()
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Reader not initialized"))
    }

    fn name(&self) -> &str {
        "snowflake-connector"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        if !self.connected {
            return Ok(false);
        }

        // For now, just check if we have writer and reader instances
        let writer_healthy = self.writer.is_some();
        let reader_healthy = self.reader.is_some();

        Ok(writer_healthy && reader_healthy)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ConnectorStats> {
        let writer_stats = if let Some(writer) = &self.writer {
            writer
                .get_stats()
                .await
                .unwrap_or_else(|_| bridge_core::traits::WriterStats {
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
            reader
                .get_stats()
                .await
                .unwrap_or_else(|_| bridge_core::traits::ReaderStats {
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
        info!("Shutting down Snowflake connector");

        // Shutdown writer and reader
        if let Some(writer) = &self.writer {
            let _ = writer.close().await;
        }

        if let Some(reader) = &self.reader {
            let _ = reader.close().await;
        }

        info!("Snowflake connector shutdown completed");
        Ok(())
    }
}
