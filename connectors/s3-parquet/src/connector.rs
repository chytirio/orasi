//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! S3/Parquet connector implementation
//!
//! This module provides the main S3/Parquet connector that implements
//! the LakehouseConnector trait for seamless integration with the bridge.

use async_trait::async_trait;
use bridge_core::error::BridgeResult;
use bridge_core::traits::{LakehouseConnector, LakehouseReader, LakehouseWriter};
use std::sync::Arc;
use tracing::{debug, info};

use crate::config::S3ParquetConfig;
use crate::error::{S3ParquetError, S3ParquetResult};
use crate::reader::S3ParquetReader;
use crate::writer::S3ParquetWriter;

/// S3/Parquet connector implementation
pub struct S3ParquetConnector {
    /// S3/Parquet configuration
    config: S3ParquetConfig,
    /// Connection state
    connected: bool,
    /// Writer instance
    writer: Option<S3ParquetWriter>,
    /// Reader instance
    reader: Option<S3ParquetReader>,
}

impl S3ParquetConnector {
    /// Create a new S3/Parquet connector
    pub fn new(config: S3ParquetConfig) -> Self {
        Self {
            config,
            connected: false,
            writer: None,
            reader: None,
        }
    }

    /// Initialize the connector
    pub async fn initialize(&mut self) -> S3ParquetResult<()> {
        info!(
            "Initializing S3/Parquet connector for bucket: {}",
            self.config.bucket
        );

        // Validate configuration
        self.config
            .validate_config()
            .map_err(|e| S3ParquetError::configuration(format!("Invalid configuration: {}", e)))?;

        // Initialize writer and reader
        let writer = S3ParquetWriter::new(self.config.clone()).await?;
        let reader = S3ParquetReader::new(self.config.clone()).await?;

        self.writer = Some(writer);
        self.reader = Some(reader);
        self.connected = true;

        info!("S3/Parquet connector initialized successfully");
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &S3ParquetConfig {
        &self.config
    }

    /// Check if the connector is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[async_trait]
impl LakehouseConnector for S3ParquetConnector {
    type Config = S3ParquetConfig;
    type WriteHandle = S3ParquetWriter;
    type ReadHandle = S3ParquetReader;

    async fn connect(config: Self::Config) -> BridgeResult<Self> {
        let mut connector = S3ParquetConnector::new(config);
        connector.initialize().await.map_err(|e| {
            bridge_core::error::BridgeError::lakehouse_with_source(
                "Failed to initialize S3/Parquet connector",
                e,
            )
        })?;
        Ok(connector)
    }

    async fn writer(&self) -> BridgeResult<Self::WriteHandle> {
        self.writer
            .clone()
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Writer not initialized"))
    }

    async fn reader(&self) -> BridgeResult<Self::ReadHandle> {
        self.reader
            .clone()
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Reader not initialized"))
    }

    fn name(&self) -> &str {
        "s3-parquet-connector"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        if !self.connected {
            return Ok(false);
        }

        // Check writer health
        let writer_healthy = if let Some(writer) = &self.writer {
            writer.get_stats().await.is_ok()
        } else {
            false
        };

        // Check reader health
        let reader_healthy = if let Some(reader) = &self.reader {
            reader.get_stats().await.is_ok()
        } else {
            false
        };

        Ok(writer_healthy && reader_healthy)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ConnectorStats> {
        let mut total_writes = 0;
        let mut total_reads = 0;
        let mut avg_write_time_ms = 0.0;
        let mut avg_read_time_ms = 0.0;
        let mut error_count = 0;

        // Get writer stats
        if let Some(writer) = &self.writer {
            if let Ok(writer_stats) = writer.get_stats().await {
                total_writes = writer_stats.total_writes;
                avg_write_time_ms = writer_stats.avg_write_time_ms;
                error_count += writer_stats.error_count;
            }
        }

        // Get reader stats
        if let Some(reader) = &self.reader {
            if let Ok(reader_stats) = reader.get_stats().await {
                total_reads = reader_stats.total_reads;
                avg_read_time_ms = reader_stats.avg_read_time_ms;
                error_count += reader_stats.error_count;
            }
        }

        Ok(bridge_core::traits::ConnectorStats {
            total_connections: if self.connected { 1 } else { 0 },
            active_connections: if self.connected { 1 } else { 0 },
            total_writes,
            total_reads,
            avg_write_time_ms,
            avg_read_time_ms,
            error_count,
            last_operation_time: Some(chrono::Utc::now()),
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down S3/Parquet connector");

        // Close writer
        if let Some(writer) = &self.writer {
            let _ = writer.close().await;
        }

        // Close reader
        if let Some(reader) = &self.reader {
            let _ = reader.close().await;
        }

        info!("S3/Parquet connector shutdown complete");
        Ok(())
    }
}
