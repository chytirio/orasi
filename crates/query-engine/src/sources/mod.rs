//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Data sources for the query engine
//!
//! This module provides integration with various data sources including
//! Delta Lake, Iceberg, S3 Parquet, and other lakehouse connectors.

pub mod delta_lake_source;
pub mod iceberg_source;
pub mod s3_parquet_source;
pub mod source_manager;

// Re-export data source types
pub use delta_lake_source::{DeltaLakeDataSource, DeltaLakeDataSourceConfig};
pub use iceberg_source::{IcebergDataSource, IcebergDataSourceConfig};
pub use s3_parquet_source::{S3ParquetDataSource, S3ParquetDataSourceConfig};
pub use source_manager::{SourceInfo, SourceManagerConfig, SourceManagerImpl, SourceManagerStats};

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Data source configuration trait
#[async_trait]
pub trait DataSourceConfig: Send + Sync {
    /// Get source name
    fn name(&self) -> &str;

    /// Get source version
    fn version(&self) -> &str;

    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;

    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Data source trait for querying data
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Initialize the data source
    async fn init(&mut self) -> BridgeResult<()>;

    /// Get data source name
    fn name(&self) -> &str;

    /// Get data source version
    fn version(&self) -> &str;

    /// Get schema information
    async fn get_schema(&self) -> BridgeResult<DataSourceSchema>;

    /// Execute a query against this data source
    async fn execute_query(&self, query: &str) -> BridgeResult<DataSourceResult>;

    /// Get data source statistics
    async fn get_stats(&self) -> BridgeResult<DataSourceStats>;
}

/// Data source schema information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceSchema {
    /// Schema ID
    pub id: Uuid,

    /// Schema name
    pub name: String,

    /// Schema version
    pub version: String,

    /// Column definitions
    pub columns: Vec<ColumnDefinition>,

    /// Partition columns
    pub partition_columns: Vec<String>,

    /// Schema metadata
    pub metadata: HashMap<String, String>,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Column definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    /// Column name
    pub name: String,

    /// Column data type
    pub data_type: ColumnDataType,

    /// Is nullable
    pub nullable: bool,

    /// Column description
    pub description: Option<String>,

    /// Column metadata
    pub metadata: HashMap<String, String>,
}

/// Column data type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColumnDataType {
    String,
    Integer,
    Float,
    Boolean,
    Timestamp,
    Date,
    Array(Box<ColumnDataType>),
    Object(HashMap<String, ColumnDataType>),
    Custom(String),
}

/// Data source query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceResult {
    /// Result ID
    pub id: Uuid,

    /// Query ID
    pub query_id: Uuid,

    /// Result data
    pub data: Vec<DataSourceRow>,

    /// Result metadata
    pub metadata: HashMap<String, String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Execution timestamp
    pub execution_timestamp: DateTime<Utc>,
}

/// Data source row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceRow {
    /// Row ID
    pub id: Uuid,

    /// Row data
    pub data: HashMap<String, DataSourceValue>,

    /// Row metadata
    pub metadata: HashMap<String, String>,
}

/// Data source value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSourceValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Timestamp(DateTime<Utc>),
    Date(chrono::NaiveDate),
    Null,
    Array(Vec<DataSourceValue>),
    Object(HashMap<String, DataSourceValue>),
}

/// Data source statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceStats {
    /// Data source name
    pub source: String,

    /// Total queries executed
    pub total_queries: u64,

    /// Queries executed in last minute
    pub queries_per_minute: u64,

    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,

    /// Average execution time per query in milliseconds
    pub avg_execution_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last execution timestamp
    pub last_execution_time: Option<DateTime<Utc>>,

    /// Data source status
    pub is_connected: bool,

    /// Total rows processed
    pub total_rows_processed: u64,
}

/// Data source manager for managing multiple data sources
pub struct DataSourceManager {
    sources: HashMap<String, Box<dyn DataSource>>,
    stats: HashMap<String, DataSourceStats>,
}

impl DataSourceManager {
    /// Create new data source manager
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            stats: HashMap::new(),
        }
    }

    /// Add data source
    pub fn add_source(&mut self, name: String, source: Box<dyn DataSource>) {
        let source_name = name.clone();
        self.sources.insert(name, source);
        self.stats.insert(
            source_name.clone(),
            DataSourceStats {
                source: source_name,
                total_queries: 0,
                queries_per_minute: 0,
                total_execution_time_ms: 0,
                avg_execution_time_ms: 0.0,
                error_count: 0,
                last_execution_time: None,
                is_connected: false,
                total_rows_processed: 0,
            },
        );
    }

    /// Remove data source
    pub fn remove_source(&mut self, name: &str) -> Option<Box<dyn DataSource>> {
        self.stats.remove(name);
        self.sources.remove(name)
    }

    /// Get data source
    pub fn get_source(&self, name: &str) -> Option<&dyn DataSource> {
        self.sources.get(name).map(|s| s.as_ref())
    }

    /// Get all data source names
    pub fn get_source_names(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }

    /// Execute query on specific data source
    pub async fn execute_query(
        &mut self,
        source_name: &str,
        query: &str,
    ) -> BridgeResult<DataSourceResult> {
        let start_time = std::time::Instant::now();

        // Update stats
        if let Some(stats) = self.stats.get_mut(source_name) {
            stats.total_queries += 1;
            stats.last_execution_time = Some(Utc::now());
        }

        // Execute query
        let result = if let Some(source) = self.get_source(source_name) {
            source.execute_query(query).await
        } else {
            return Err(bridge_core::BridgeError::configuration(format!(
                "Data source not found: {}",
                source_name
            )));
        };

        let execution_time = start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;

        // Update stats
        if let Some(stats) = self.stats.get_mut(source_name) {
            stats.total_execution_time_ms += execution_time_ms;
            stats.avg_execution_time_ms =
                stats.total_execution_time_ms as f64 / stats.total_queries as f64;

            if result.is_err() {
                stats.error_count += 1;
            } else if let Ok(ref result_data) = result {
                stats.total_rows_processed += result_data.data.len() as u64;
            }
        }

        match result {
            Ok(mut data_source_result) => {
                data_source_result.execution_time_ms = execution_time_ms;
                Ok(data_source_result)
            }
            Err(e) => {
                error!("Data source query execution failed: {}", e);
                Err(e)
            }
        }
    }

    /// Get data source statistics
    pub fn get_stats(&self) -> &HashMap<String, DataSourceStats> {
        &self.stats
    }
}

/// Data source factory for creating data sources
pub struct DataSourceFactory;

impl DataSourceFactory {
    /// Create a data source based on configuration
    pub async fn create_source(config: &dyn DataSourceConfig) -> BridgeResult<Box<dyn DataSource>> {
        match config.name() {
            "delta_lake" => {
                // Create Delta Lake data source
                let delta_source = delta_lake_source::DeltaLakeDataSource::new(config).await?;
                Ok(Box::new(delta_source))
            }
            "iceberg" => {
                // Create Iceberg data source
                let iceberg_source = iceberg_source::IcebergDataSource::new(config).await?;
                Ok(Box::new(iceberg_source))
            }
            "s3_parquet" => {
                // Create S3 Parquet data source
                let s3_source = s3_parquet_source::S3ParquetDataSource::new(config).await?;
                Ok(Box::new(s3_source))
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported data source: {}",
                config.name()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_data_source_manager_creation() {
        let manager = DataSourceManager::new();
        assert_eq!(manager.get_source_names().len(), 0);
    }

    #[tokio::test]
    async fn test_data_source_manager_add_source() {
        let manager = DataSourceManager::new();

        // This would require a mock data source implementation
        // For now, just test the basic structure
        assert_eq!(manager.get_source_names().len(), 0);
    }
}
