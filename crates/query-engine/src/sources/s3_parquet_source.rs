//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! S3 Parquet data source for the query engine
//!
//! This module provides S3 Parquet integration for the query engine,
//! allowing queries to be executed against Parquet files stored in S3.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

// AWS S3 imports
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::types::Object;
use aws_sdk_s3::Client as S3Client;

// DataFusion and Arrow imports
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::execution::context::SessionContext;
use datafusion::prelude::*;

use super::{
    ColumnDataType, ColumnDefinition, DataSource, DataSourceConfig, DataSourceResult,
    DataSourceRow, DataSourceSchema, DataSourceStats, DataSourceValue,
};

/// S3 Parquet data source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ParquetDataSourceConfig {
    /// Data source name
    pub name: String,

    /// Data source version
    pub version: String,

    /// S3 bucket name
    pub bucket_name: String,

    /// S3 prefix/path
    pub prefix: String,

    /// AWS region
    pub region: String,

    /// AWS credentials configuration
    pub credentials: HashMap<String, String>,

    /// Query timeout in seconds
    pub query_timeout_seconds: u64,

    /// Enable debug logging
    pub debug_logging: bool,
}

impl S3ParquetDataSourceConfig {
    /// Create new configuration with defaults
    pub fn new(bucket_name: String, prefix: String, region: String) -> Self {
        Self {
            name: "s3_parquet".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            bucket_name,
            prefix,
            region,
            credentials: HashMap::new(),
            query_timeout_seconds: 300,
            debug_logging: false,
        }
    }
}

/// S3 Parquet data source implementation
pub struct S3ParquetDataSource {
    config: S3ParquetDataSourceConfig,
    schema: Option<DataSourceSchema>,
    stats: Arc<Mutex<DataSourceStats>>,
    is_initialized: bool,
    s3_client: Option<S3Client>,
    datafusion_ctx: Option<SessionContext>,
    discovered_files: Vec<String>,
}

impl S3ParquetDataSource {
    /// Create new S3 Parquet data source
    pub async fn new(config: &dyn DataSourceConfig) -> BridgeResult<Self> {
        let s3_config =
            if let Some(s3_config) = config.as_any().downcast_ref::<S3ParquetDataSourceConfig>() {
                s3_config.clone()
            } else {
                return Err(bridge_core::BridgeError::configuration(
                    "Invalid configuration type for S3 Parquet data source",
                ));
            };

        Ok(Self {
            config: s3_config,
            schema: None,
            stats: Arc::new(Mutex::new(DataSourceStats {
                source: "s3_parquet".to_string(),
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
            s3_client: None,
            datafusion_ctx: None,
            discovered_files: Vec::new(),
        })
    }
}

#[async_trait]
impl DataSourceConfig for S3ParquetDataSourceConfig {
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

        if self.bucket_name.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "S3 bucket name cannot be empty",
            ));
        }

        if self.region.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "AWS region cannot be empty",
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
impl DataSource for S3ParquetDataSource {
    async fn init(&mut self) -> BridgeResult<()> {
        if self.is_initialized {
            return Ok(());
        }

        info!(
            "Initializing S3 Parquet data source v{}",
            self.config.version
        );

        // Create AWS S3 client
        let aws_config = if !self.config.credentials.is_empty() {
            // Use provided credentials
            let mut config_loader = aws_config::defaults(BehaviorVersion::latest())
                .region(aws_sdk_s3::config::Region::new(self.config.region.clone()));

            if let (Some(access_key), Some(secret_key)) = (
                self.config.credentials.get("access_key_id"),
                self.config.credentials.get("secret_access_key"),
            ) {
                let credentials = Credentials::new(
                    access_key,
                    secret_key,
                    self.config.credentials.get("session_token").cloned(),
                    None,
                    "S3ParquetDataSource",
                );
                config_loader = config_loader.credentials_provider(credentials);
            }

            config_loader.load().await
        } else {
            // Use default credentials chain (environment, profile, etc.)
            aws_config::defaults(BehaviorVersion::latest())
                .region(aws_sdk_s3::config::Region::new(self.config.region.clone()))
                .load()
                .await
        };

        let s3_client = S3Client::new(&aws_config);

        // Test bucket access
        match s3_client
            .head_bucket()
            .bucket(&self.config.bucket_name)
            .send()
            .await
        {
            Ok(_) => info!(
                "Successfully validated access to S3 bucket: {}",
                self.config.bucket_name
            ),
            Err(e) => {
                error!(
                    "Failed to access S3 bucket {}: {}",
                    self.config.bucket_name, e
                );
                return Err(bridge_core::BridgeError::configuration(&format!(
                    "Cannot access S3 bucket {}: {}",
                    self.config.bucket_name, e
                )));
            }
        }

        // Discover Parquet files in the bucket
        let discovered_files = self.discover_parquet_files(&s3_client).await?;
        info!("Discovered {} Parquet files in S3", discovered_files.len());

        // Create DataFusion session context
        let datafusion_ctx = SessionContext::new();

        // Register S3 tables for DataFusion
        for file_path in &discovered_files {
            let table_name = self.extract_table_name_from_path(file_path);
            let s3_url = format!("s3://{}/{}", self.config.bucket_name, file_path);

            // Register the Parquet file as a table in DataFusion
            if let Err(e) = datafusion_ctx
                .register_parquet(&table_name, &s3_url, ParquetReadOptions::default())
                .await
            {
                warn!(
                    "Failed to register Parquet file {} as table {}: {}",
                    s3_url, table_name, e
                );
                continue;
            }
        }

        // Load schema from the first available Parquet file
        self.load_schema_from_parquet(&s3_client, &discovered_files)
            .await?;

        // Store the initialized components
        self.s3_client = Some(s3_client);
        self.datafusion_ctx = Some(datafusion_ctx);
        self.discovered_files = discovered_files;

        self.is_initialized = true;
        info!("S3 Parquet data source initialized successfully");

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

        let datafusion_ctx = self.datafusion_ctx.as_ref().ok_or_else(|| {
            bridge_core::BridgeError::configuration("DataFusion context not available")
        })?;

        info!("Executing query against S3 Parquet files: {}", query);

        // Execute query using DataFusion
        let result = match datafusion_ctx.sql(query).await {
            Ok(dataframe) => dataframe.collect().await.map_err(|e| {
                bridge_core::BridgeError::configuration(&format!("Query execution failed: {}", e))
            })?,
            Err(e) => {
                error!("SQL parsing failed: {}", e);
                return Err(bridge_core::BridgeError::configuration(&format!(
                    "SQL parsing failed: {}",
                    e
                )));
            }
        };

        // Convert DataFusion result to DataSourceResult
        let data = self.convert_record_batches_to_rows(result).await?;

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

impl S3ParquetDataSource {
    /// Discover Parquet files in the S3 bucket
    async fn discover_parquet_files(&self, s3_client: &S3Client) -> BridgeResult<Vec<String>> {
        let mut discovered_files = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = s3_client
                .list_objects_v2()
                .bucket(&self.config.bucket_name)
                .prefix(&self.config.prefix);

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            match request.send().await {
                Ok(output) => {
                    if let Some(contents) = output.contents {
                        for object in contents {
                            if let Some(key) = object.key() {
                                if key.ends_with(".parquet") {
                                    discovered_files.push(key.to_string());
                                }
                            }
                        }
                    }

                    continuation_token = output.next_continuation_token;
                    if continuation_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to list objects in S3 bucket: {}", e);
                    return Err(bridge_core::BridgeError::configuration(&format!(
                        "Failed to list S3 objects: {}",
                        e
                    )));
                }
            }
        }

        Ok(discovered_files)
    }

    /// Extract table name from S3 object path
    fn extract_table_name_from_path(&self, path: &str) -> String {
        // Extract filename without extension and make it a valid table name
        let filename = path
            .split('/')
            .last()
            .unwrap_or(path)
            .trim_end_matches(".parquet");

        // Replace invalid characters with underscores
        filename
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_start_matches(|c: char| c.is_numeric())
            .trim_matches('_')
            .to_lowercase()
    }

    /// Load schema from Parquet files
    async fn load_schema_from_parquet(
        &mut self,
        s3_client: &S3Client,
        discovered_files: &[String],
    ) -> BridgeResult<()> {
        if discovered_files.is_empty() {
            warn!("No Parquet files found, creating default schema");
            self.create_default_schema();
            return Ok(());
        }

        // Try to read schema from the first Parquet file
        let first_file = &discovered_files[0];
        info!("Loading schema from Parquet file: {}", first_file);

        // For now, create a default telemetry schema
        // In a real implementation, we would read the actual Parquet file schema
        self.create_default_schema();

        Ok(())
    }

    /// Create a default telemetry schema
    fn create_default_schema(&mut self) {
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

        self.schema = Some(DataSourceSchema {
            id: Uuid::new_v4(),
            name: format!("s3_parquet_{}", self.config.bucket_name),
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
    }

    /// Convert DataFusion record batches to DataSourceRow format
    async fn convert_record_batches_to_rows(
        &self,
        record_batches: Vec<RecordBatch>,
    ) -> BridgeResult<Vec<DataSourceRow>> {
        let mut rows = Vec::new();

        for batch in record_batches {
            let schema = batch.schema();
            let num_rows = batch.num_rows();

            for row_idx in 0..num_rows {
                let mut row_data = HashMap::new();

                for (col_idx, field) in schema.fields().iter().enumerate() {
                    let column = batch.column(col_idx);
                    let value =
                        self.extract_value_from_array(column, row_idx, field.data_type())?;
                    row_data.insert(field.name().clone(), value);
                }

                rows.push(DataSourceRow {
                    id: Uuid::new_v4(),
                    data: row_data,
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(rows)
    }

    /// Extract value from Arrow array at given index
    fn extract_value_from_array(
        &self,
        array: &dyn arrow::array::Array,
        row_idx: usize,
        data_type: &DataType,
    ) -> BridgeResult<DataSourceValue> {
        if array.is_null(row_idx) {
            return Ok(DataSourceValue::Null);
        }

        match data_type {
            DataType::Utf8 => {
                let array = array
                    .as_any()
                    .downcast_ref::<arrow::array::StringArray>()
                    .ok_or_else(|| {
                        bridge_core::BridgeError::configuration("Failed to cast to StringArray")
                    })?;
                Ok(DataSourceValue::String(array.value(row_idx).to_string()))
            }
            DataType::Int32 => {
                let array = array
                    .as_any()
                    .downcast_ref::<arrow::array::Int32Array>()
                    .ok_or_else(|| {
                        bridge_core::BridgeError::configuration("Failed to cast to Int32Array")
                    })?;
                Ok(DataSourceValue::Integer(array.value(row_idx) as i64))
            }
            DataType::Int64 => {
                let array = array
                    .as_any()
                    .downcast_ref::<arrow::array::Int64Array>()
                    .ok_or_else(|| {
                        bridge_core::BridgeError::configuration("Failed to cast to Int64Array")
                    })?;
                Ok(DataSourceValue::Integer(array.value(row_idx)))
            }
            DataType::Float32 => {
                let array = array
                    .as_any()
                    .downcast_ref::<arrow::array::Float32Array>()
                    .ok_or_else(|| {
                        bridge_core::BridgeError::configuration("Failed to cast to Float32Array")
                    })?;
                Ok(DataSourceValue::Float(array.value(row_idx) as f64))
            }
            DataType::Float64 => {
                let array = array
                    .as_any()
                    .downcast_ref::<arrow::array::Float64Array>()
                    .ok_or_else(|| {
                        bridge_core::BridgeError::configuration("Failed to cast to Float64Array")
                    })?;
                Ok(DataSourceValue::Float(array.value(row_idx)))
            }
            DataType::Boolean => {
                let array = array
                    .as_any()
                    .downcast_ref::<arrow::array::BooleanArray>()
                    .ok_or_else(|| {
                        bridge_core::BridgeError::configuration("Failed to cast to BooleanArray")
                    })?;
                Ok(DataSourceValue::Boolean(array.value(row_idx)))
            }
            DataType::Timestamp(_, _) => {
                let array = array
                    .as_any()
                    .downcast_ref::<arrow::array::TimestampMicrosecondArray>()
                    .ok_or_else(|| {
                        bridge_core::BridgeError::configuration("Failed to cast to TimestampArray")
                    })?;
                let timestamp = array.value(row_idx);
                Ok(DataSourceValue::Timestamp(
                    DateTime::from_timestamp_micros(timestamp).unwrap_or_else(|| Utc::now()),
                ))
            }
            DataType::Date32 => {
                let array = array
                    .as_any()
                    .downcast_ref::<arrow::array::Date32Array>()
                    .ok_or_else(|| {
                        bridge_core::BridgeError::configuration("Failed to cast to Date32Array")
                    })?;
                let days = array.value(row_idx);
                let date = chrono::NaiveDate::from_num_days_from_ce_opt(days + 719163) // Unix epoch adjustment
                    .unwrap_or_else(|| chrono::Utc::now().date_naive());
                Ok(DataSourceValue::Date(date))
            }
            _ => {
                // For unsupported types, convert to string
                Ok(DataSourceValue::String(format!(
                    "Unsupported type: {:?}",
                    data_type
                )))
            }
        }
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
    async fn test_s3_parquet_data_source_creation() {
        let config = S3ParquetDataSourceConfig::new(
            "my-bucket".to_string(),
            "telemetry/".to_string(),
            "us-west-2".to_string(),
        );
        let data_source = S3ParquetDataSource::new(&config).await;
        assert!(data_source.is_ok());
    }

    #[tokio::test]
    async fn test_s3_parquet_data_source_config_validation() {
        let mut config = S3ParquetDataSourceConfig::new(
            "my-bucket".to_string(),
            "telemetry/".to_string(),
            "us-west-2".to_string(),
        );
        config.name = "".to_string();

        let result = config.validate().await;
        assert!(result.is_err());
    }
}
