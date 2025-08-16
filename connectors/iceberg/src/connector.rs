//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Iceberg connector implementation
//! 
//! This module provides the main Apache Iceberg connector that implements
//! the LakehouseConnector trait for seamless integration with the bridge.

use std::sync::Arc;
use async_trait::async_trait;
use tracing::{debug, error, info, warn};
use chrono::Utc;
use bridge_core::traits::{LakehouseConnector, LakehouseWriter, LakehouseReader, ConnectorStats};
use bridge_core::error::BridgeResult;

use crate::config::IcebergConfig;
use crate::error::IcebergResult;
use crate::writer::IcebergWriter;
use crate::reader::IcebergReader;
use crate::catalog::IcebergCatalog;
use crate::schema::IcebergSchema;

/// Apache Iceberg connector implementation
pub struct IcebergConnector {
    /// Iceberg configuration
    config: IcebergConfig,
    /// Connection state
    connected: bool,
    /// Writer instance
    writer: Option<Arc<IcebergWriter>>,
    /// Reader instance
    reader: Option<Arc<IcebergReader>>,
    /// Catalog instance
    catalog: Option<Arc<IcebergCatalog>>,
    /// Schema instance
    schema: Option<Arc<IcebergSchema>>,
    /// Statistics
    stats: ConnectorStats,
}

impl IcebergConnector {
    /// Create a new Apache Iceberg connector
    pub fn new(config: IcebergConfig) -> Self {
        Self {
            config,
            connected: false,
            writer: None,
            reader: None,
            catalog: None,
            schema: None,
            stats: ConnectorStats {
                total_connections: 0,
                active_connections: 0,
                total_writes: 0,
                total_reads: 0,
                avg_write_time_ms: 0.0,
                avg_read_time_ms: 0.0,
                error_count: 0,
                last_operation_time: None,
            },
        }
    }

    /// Initialize the connector
    pub async fn initialize(&mut self) -> IcebergResult<()> {
        info!("Initializing Apache Iceberg connector for table: {}", self.config.table.table_name);
        
        // Initialize catalog
        let catalog = Arc::new(IcebergCatalog::new(self.config.catalog.clone()).await?);
        self.catalog = Some(catalog.clone());
        
        // Initialize schema
        let schema = Arc::new(IcebergSchema::new(self.config.schema.clone()).await?);
        self.schema = Some(schema.clone());
        
        // Initialize Iceberg table if it doesn't exist
        self.ensure_table_exists().await?;
        
        // Initialize writer and reader
        let writer = Arc::new(IcebergWriter::new(self.config.clone(), catalog.clone(), schema.clone()).await?);
        let reader = Arc::new(IcebergReader::new(self.config.clone(), catalog.clone(), schema.clone()).await?);
        
        self.writer = Some(writer);
        self.reader = Some(reader);
        self.connected = true;
        
        // Update stats
        self.stats.total_connections += 1;
        self.stats.active_connections = 1;
        self.stats.last_operation_time = Some(Utc::now());
        
        info!("Apache Iceberg connector initialized successfully");
        Ok(())
    }

    /// Ensure the Iceberg table exists
    async fn ensure_table_exists(&self) -> IcebergResult<()> {
        debug!("Ensuring Apache Iceberg table exists: {}", self.config.table.table_name);
        
        if let Some(catalog) = &self.catalog {
            // Check if table exists
            if !catalog.table_exists(&self.config.table.table_name).await? {
                info!("Creating Apache Iceberg table: {}", self.config.table.table_name);
                
                // Create table with schema
                if let Some(schema) = &self.schema {
                    catalog.create_table(
                        &self.config.table.table_name,
                        schema.get_schema().await?,
                        &self.config.table.partition_columns,
                        &self.config.table.properties,
                    ).await?;
                }
            } else {
                debug!("Table already exists: {}", self.config.table.table_name);
            }
        }
        
        info!("Table existence check completed for: {}", self.config.table.table_name);
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &IcebergConfig {
        &self.config
    }

    /// Check if the connector is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[async_trait]
impl LakehouseConnector for IcebergConnector {
    type Config = IcebergConfig;
    type WriteHandle = IcebergWriter;
    type ReadHandle = IcebergReader;

    async fn connect(config: Self::Config) -> BridgeResult<Self> {
        let mut connector = IcebergConnector::new(config);
        connector.initialize().await
            .map_err(|e| bridge_core::error::BridgeError::lakehouse_with_source("Failed to initialize Apache Iceberg connector", e))?;
        Ok(connector)
    }

    async fn writer(&self) -> BridgeResult<Self::WriteHandle> {
        if let Some(writer) = &self.writer {
            Ok(writer.as_ref().clone())
        } else {
            Err(bridge_core::error::BridgeError::lakehouse("Writer not initialized"))
        }
    }

    async fn reader(&self) -> BridgeResult<Self::ReadHandle> {
        if let Some(reader) = &self.reader {
            Ok(reader.as_ref().clone())
        } else {
            Err(bridge_core::error::BridgeError::lakehouse("Reader not initialized"))
        }
    }

    fn name(&self) -> &str {
        crate::ICEBERG_NAME
    }

    fn version(&self) -> &str {
        crate::ICEBERG_VERSION
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        if !self.connected {
            return Ok(false);
        }

        // Check if catalog is accessible
        if let Some(catalog) = &self.catalog {
            match catalog.health_check().await {
                Ok(healthy) => {
                    if !healthy {
                        warn!("Apache Iceberg catalog health check failed");
                        return Ok(false);
                    }
                }
                Err(e) => {
                    error!("Apache Iceberg catalog health check error: {}", e);
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    async fn get_stats(&self) -> BridgeResult<ConnectorStats> {
        Ok(self.stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down Apache Iceberg connector");

        // Close writer
        if let Some(writer) = &self.writer {
            if let Err(e) = writer.close().await {
                warn!("Error closing Apache Iceberg writer: {}", e);
            }
        }

        // Close reader
        if let Some(reader) = &self.reader {
            if let Err(e) = reader.close().await {
                warn!("Error closing Apache Iceberg reader: {}", e);
            }
        }

        // Update stats
        // Note: In a real implementation, we'd need to make stats mutable
        // For now, we'll just log the shutdown

        info!("Apache Iceberg connector shutdown completed");
        Ok(())
    }
}
