//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Iceberg data source for the query engine
//!
//! This module provides Iceberg integration for the query engine,
//! allowing queries to be executed against Iceberg tables.

use async_trait::async_trait;
use bridge_core::{BridgeResult, LakehouseConnector, LakehouseReader, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

// Iceberg connector imports
use lakehouse_iceberg::{
    config::IcebergConfig, connector::IcebergConnector, error::IcebergResult, reader::IcebergReader,
};

use super::{
    ColumnDataType, ColumnDefinition, DataSource, DataSourceConfig, DataSourceResult,
    DataSourceRow, DataSourceSchema, DataSourceStats, DataSourceValue,
};

/// Iceberg data source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcebergDataSourceConfig {
    /// Data source name
    pub name: String,

    /// Data source version
    pub version: String,

    /// Iceberg table path
    pub table_path: String,

    /// Iceberg table name
    pub table_name: String,

    /// Storage configuration
    pub storage_config: HashMap<String, String>,

    /// Query timeout in seconds
    pub query_timeout_seconds: u64,

    /// Enable debug logging
    pub debug_logging: bool,
}

impl IcebergDataSourceConfig {
    /// Create new configuration with defaults
    pub fn new(table_path: String, table_name: String) -> Self {
        Self {
            name: "iceberg".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            table_path,
            table_name,
            storage_config: HashMap::new(),
            query_timeout_seconds: 300,
            debug_logging: false,
        }
    }
}

/// Iceberg data source implementation
pub struct IcebergDataSource {
    config: IcebergDataSourceConfig,
    schema: Option<DataSourceSchema>,
    stats: Arc<Mutex<DataSourceStats>>,
    is_initialized: bool,
    connector: Option<Arc<IcebergConnector>>,
    reader: Option<Arc<IcebergReader>>,
}

impl IcebergDataSource {
    /// Create new Iceberg data source
    pub async fn new(config: &dyn DataSourceConfig) -> BridgeResult<Self> {
        let iceberg_config = if let Some(iceberg_config) =
            config.as_any().downcast_ref::<IcebergDataSourceConfig>()
        {
            iceberg_config.clone()
        } else {
            return Err(bridge_core::BridgeError::configuration(
                "Invalid configuration type for Iceberg data source",
            ));
        };

        Ok(Self {
            config: iceberg_config,
            schema: None,
            stats: Arc::new(Mutex::new(DataSourceStats {
                source: "iceberg".to_string(),
                total_queries: 0,
                queries_per_minute: 0,
                total_execution_time_ms: 0,
                avg_execution_time_ms: 0.0,
                error_count: 0,
                last_execution_time: None,
                is_connected: false,
                total_rows_processed: 0,
            })),
            is_initialized: false,
            connector: None,
            reader: None,
        })
    }
}

#[async_trait]
impl DataSourceConfig for IcebergDataSourceConfig {
    async fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Data source name cannot be empty",
            ));
        }

        if self.version.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Data source version cannot be empty",
            ));
        }

        if self.table_path.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Iceberg table path cannot be empty",
            ));
        }

        if self.table_name.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Iceberg table name cannot be empty",
            ));
        }

        if self.query_timeout_seconds == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Query timeout must be greater than 0",
            ));
        }

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl DataSource for IcebergDataSource {
    async fn init(&mut self) -> BridgeResult<()> {
        if self.is_initialized {
            return Ok(());
        }

        info!("Initializing Iceberg data source v{}", self.config.version);

        // Create Iceberg configuration from data source config
        let iceberg_config = IcebergConfig::new(
            self.config.table_path.clone(),
            self.config.table_name.clone(),
        );

        // Create and initialize Iceberg connector
        let mut connector = IcebergConnector::new(iceberg_config);
        connector.initialize().await.map_err(|e| {
            bridge_core::BridgeError::configuration(&format!(
                "Failed to initialize Iceberg connector: {}",
                e
            ))
        })?;

        // Get reader from connector
        let reader = connector.reader().await.map_err(|e| {
            bridge_core::BridgeError::configuration(&format!("Failed to get Iceberg reader: {}", e))
        })?;

        // Store connector components
        self.connector = Some(Arc::new(connector));
        self.reader = Some(Arc::new(reader));

        // Load schema information
        self.load_schema().await?;

        self.is_initialized = true;
        info!("Iceberg data source initialized successfully");

        Ok(())
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn version(&self) -> &str {
        &self.config.version
    }

    async fn get_schema(&self) -> BridgeResult<DataSourceSchema> {
        if let Some(schema) = &self.schema {
            Ok(schema.clone())
        } else {
            Err(bridge_core::BridgeError::configuration(
                "Schema not loaded. Please initialize the data source first.",
            ))
        }
    }

    async fn execute_query(&self, query: &str) -> BridgeResult<DataSourceResult> {
        let start_time = Instant::now();
        let query_id = Uuid::new_v4();

        if !self.is_initialized {
            return Err(bridge_core::BridgeError::configuration(
                "Data source not initialized. Please call init() first.",
            ));
        }

        let reader = self.reader.as_ref().ok_or_else(|| {
            bridge_core::BridgeError::configuration("Iceberg reader not available")
        })?;

        info!("Executing query against Iceberg table: {}", query);

        // Execute query using Iceberg reader
        let result = reader.execute_query(query.to_string()).await.map_err(|e| {
            bridge_core::BridgeError::configuration(&format!("Query execution failed: {}", e))
        })?;

        // Convert Iceberg result to DataSourceResult
        let data = self.convert_json_result_to_rows(result).await?;

        let execution_time = start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;

        // Update statistics
        self.update_stats(execution_time_ms, data.len() as u64);

        Ok(DataSourceResult {
            id: Uuid::new_v4(),
            query_id,
            data,
            metadata: HashMap::new(),
            execution_time_ms,
            execution_timestamp: Utc::now(),
        })
    }

    async fn get_stats(&self) -> BridgeResult<DataSourceStats> {
        Ok(self.stats.lock().unwrap().clone())
    }
}

impl IcebergDataSource {
    /// Load schema information from Iceberg
    async fn load_schema(&mut self) -> BridgeResult<()> {
        // For now, we'll create a default telemetry schema
        // In a real implementation, this would be loaded from the Iceberg table
        let columns = vec![
            ColumnDefinition {
                name: "timestamp".to_string(),
                data_type: ColumnDataType::Timestamp,
                nullable: false,
                description: Some("Event timestamp".to_string()),
                metadata: HashMap::new(),
            },
            ColumnDefinition {
                name: "service_name".to_string(),
                data_type: ColumnDataType::String,
                nullable: false,
                description: Some("Service name".to_string()),
                metadata: HashMap::new(),
            },
            ColumnDefinition {
                name: "instance_id".to_string(),
                data_type: ColumnDataType::String,
                nullable: false,
                description: Some("Instance ID".to_string()),
                metadata: HashMap::new(),
            },
            ColumnDefinition {
                name: "telemetry_type".to_string(),
                data_type: ColumnDataType::String,
                nullable: false,
                description: Some("Type of telemetry data".to_string()),
                metadata: HashMap::new(),
            },
            ColumnDefinition {
                name: "metric_name".to_string(),
                data_type: ColumnDataType::String,
                nullable: true,
                description: Some("Metric name".to_string()),
                metadata: HashMap::new(),
            },
            ColumnDefinition {
                name: "metric_value".to_string(),
                data_type: ColumnDataType::Float,
                nullable: true,
                description: Some("Metric value".to_string()),
                metadata: HashMap::new(),
            },
            ColumnDefinition {
                name: "trace_id".to_string(),
                data_type: ColumnDataType::String,
                nullable: true,
                description: Some("Trace ID".to_string()),
                metadata: HashMap::new(),
            },
            ColumnDefinition {
                name: "span_id".to_string(),
                data_type: ColumnDataType::String,
                nullable: true,
                description: Some("Span ID".to_string()),
                metadata: HashMap::new(),
            },
            ColumnDefinition {
                name: "log_level".to_string(),
                data_type: ColumnDataType::String,
                nullable: true,
                description: Some("Log level".to_string()),
                metadata: HashMap::new(),
            },
            ColumnDefinition {
                name: "log_message".to_string(),
                data_type: ColumnDataType::String,
                nullable: true,
                description: Some("Log message".to_string()),
                metadata: HashMap::new(),
            },
        ];

        // Create DataSourceSchema from the columns

        self.schema = Some(DataSourceSchema {
            id: Uuid::new_v4(),
            name: format!("iceberg_{}", self.config.table_name),
            version: self.config.version.clone(),
            columns,
            partition_columns: vec![
                "year".to_string(),
                "month".to_string(),
                "day".to_string(),
                "hour".to_string(),
            ],
            metadata: HashMap::new(),
            last_updated: Utc::now(),
        });

        Ok(())
    }

    /// Convert JSON query result to DataSourceRow format
    async fn convert_json_result_to_rows(
        &self,
        result: serde_json::Value,
    ) -> BridgeResult<Vec<DataSourceRow>> {
        let mut rows = Vec::new();

        // Handle different JSON result formats
        match result {
            serde_json::Value::Array(data_array) => {
                for item in data_array {
                    if let serde_json::Value::Object(obj) = item {
                        let mut row_data = HashMap::new();

                        for (key, value) in obj {
                            let data_value = match value {
                                serde_json::Value::String(s) => DataSourceValue::String(s),
                                serde_json::Value::Number(n) => {
                                    if let Some(i) = n.as_i64() {
                                        DataSourceValue::Integer(i)
                                    } else if let Some(f) = n.as_f64() {
                                        DataSourceValue::Float(f)
                                    } else {
                                        DataSourceValue::String(n.to_string())
                                    }
                                }
                                serde_json::Value::Bool(b) => DataSourceValue::Boolean(b),
                                serde_json::Value::Null => DataSourceValue::Null,
                                serde_json::Value::Array(arr) => {
                                    let values: Vec<DataSourceValue> = arr
                                        .iter()
                                        .map(|v| match v {
                                            serde_json::Value::String(s) => {
                                                DataSourceValue::String(s.clone())
                                            }
                                            serde_json::Value::Number(n) => {
                                                if let Some(i) = n.as_i64() {
                                                    DataSourceValue::Integer(i)
                                                } else if let Some(f) = n.as_f64() {
                                                    DataSourceValue::Float(f)
                                                } else {
                                                    DataSourceValue::String(n.to_string())
                                                }
                                            }
                                            serde_json::Value::Bool(b) => {
                                                DataSourceValue::Boolean(*b)
                                            }
                                            serde_json::Value::Null => DataSourceValue::Null,
                                            _ => DataSourceValue::String(v.to_string()),
                                        })
                                        .collect();
                                    DataSourceValue::Array(values)
                                }
                                serde_json::Value::Object(obj) => {
                                    let mut map = HashMap::new();
                                    for (k, v) in obj {
                                        map.insert(
                                            k.clone(),
                                            match v {
                                                serde_json::Value::String(s) => {
                                                    DataSourceValue::String(s.clone())
                                                }
                                                serde_json::Value::Number(n) => {
                                                    if let Some(i) = n.as_i64() {
                                                        DataSourceValue::Integer(i)
                                                    } else if let Some(f) = n.as_f64() {
                                                        DataSourceValue::Float(f)
                                                    } else {
                                                        DataSourceValue::String(n.to_string())
                                                    }
                                                }
                                                serde_json::Value::Bool(b) => {
                                                    DataSourceValue::Boolean(b)
                                                }
                                                serde_json::Value::Null => DataSourceValue::Null,
                                                _ => DataSourceValue::String(v.to_string()),
                                            },
                                        );
                                    }
                                    DataSourceValue::Object(map)
                                }
                            };

                            row_data.insert(key, data_value);
                        }

                        rows.push(DataSourceRow {
                            id: Uuid::new_v4(),
                            data: row_data,
                            metadata: HashMap::new(),
                        });
                    }
                }
            }
            serde_json::Value::Object(obj) => {
                // Single object result
                let mut row_data = HashMap::new();
                for (key, value) in obj {
                    let data_value = match value {
                        serde_json::Value::String(s) => DataSourceValue::String(s),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                DataSourceValue::Integer(i)
                            } else if let Some(f) = n.as_f64() {
                                DataSourceValue::Float(f)
                            } else {
                                DataSourceValue::String(n.to_string())
                            }
                        }
                        serde_json::Value::Bool(b) => DataSourceValue::Boolean(b),
                        serde_json::Value::Null => DataSourceValue::Null,
                        _ => DataSourceValue::String(value.to_string()),
                    };
                    row_data.insert(key, data_value);
                }

                rows.push(DataSourceRow {
                    id: Uuid::new_v4(),
                    data: row_data,
                    metadata: HashMap::new(),
                });
            }
            _ => {
                // Handle other cases (null, primitive values)
                rows.push(DataSourceRow {
                    id: Uuid::new_v4(),
                    data: HashMap::new(),
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(rows)
    }

    /// Update data source statistics
    fn update_stats(&self, execution_time_ms: u64, rows_processed: u64) {
        let mut stats = self.stats.lock().unwrap();
        stats.total_queries += 1;
        stats.total_execution_time_ms += execution_time_ms;
        stats.total_rows_processed += rows_processed;
        stats.last_execution_time = Some(Utc::now());
        stats.is_connected = true;

        // Calculate average execution time
        if stats.total_queries > 0 {
            stats.avg_execution_time_ms =
                stats.total_execution_time_ms as f64 / stats.total_queries as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_iceberg_data_source_creation() {
        let config = IcebergDataSourceConfig::new(
            "/path/to/table".to_string(),
            "telemetry_data".to_string(),
        );
        let data_source = IcebergDataSource::new(&config).await;
        assert!(data_source.is_ok());
    }

    #[tokio::test]
    async fn test_iceberg_data_source_config_validation() {
        let mut config = IcebergDataSourceConfig::new(
            "/path/to/table".to_string(),
            "telemetry_data".to_string(),
        );
        config.name = "".to_string();

        let result = config.validate().await;
        assert!(result.is_err());
    }
}
