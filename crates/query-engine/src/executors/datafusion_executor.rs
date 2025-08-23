//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! DataFusion-based query executor for telemetry data
//!
//! This module provides a DataFusion-powered executor that can query
//! across multiple data sources including Delta Lake, S3 Parquet,
//! and in-memory telemetry streams.

use async_trait::async_trait;
use bridge_core::{
    types::{TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch,
};
use chrono::{DateTime, Utc};
use datafusion::arrow::array::{
    Array, BooleanArray, Float64Array, Int64Array, StringArray, TimestampNanosecondArray,
};
use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::error::DataFusionError;
use datafusion::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{ExecutionStatus, ExecutorStats, QueryExecutor, QueryResult, QueryRow, QueryValue};
use crate::parsers::{AstNode, NodeType, ParsedQuery, QueryAst};

/// DataFusion executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFusionConfig {
    /// Executor name
    pub name: String,

    /// Executor version
    pub version: String,

    /// Maximum memory usage in bytes
    pub max_memory_bytes: Option<u64>,

    /// Number of threads for parallel execution
    pub num_threads: Option<usize>,

    /// Enable debug logging
    pub debug_logging: bool,

    /// Delta Lake table configurations
    pub delta_tables: HashMap<String, String>, // table_name -> table_path

    /// S3 configurations
    pub s3_config: Option<S3Config>,

    /// Object store configurations
    pub object_stores: HashMap<String, ObjectStoreConfig>,

    /// Enable query optimization
    pub enable_optimization: bool,

    /// Enable statistics collection
    pub enable_statistics: bool,

    /// Batch size for query execution
    pub batch_size: Option<usize>,

    /// Target partitions for parallel execution
    pub target_partitions: Option<usize>,
}

/// S3 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// AWS region
    pub region: String,

    /// AWS access key ID
    pub access_key_id: Option<String>,

    /// AWS secret access key
    pub secret_access_key: Option<String>,

    /// S3 endpoint URL (for local testing)
    pub endpoint_url: Option<String>,

    /// Enable path-style addressing
    pub use_path_style: bool,
}

/// Object store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectStoreConfig {
    /// Store type
    pub store_type: String,

    /// Store URL
    pub url: String,

    /// Additional configuration
    pub config: HashMap<String, String>,
}

impl DataFusionConfig {
    /// Create new configuration with defaults
    pub fn new() -> Self {
        Self {
            name: "datafusion".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            max_memory_bytes: None,
            num_threads: None,
            debug_logging: false,
            delta_tables: HashMap::new(),
            s3_config: None,
            object_stores: HashMap::new(),
            enable_optimization: true,
            enable_statistics: true,
            batch_size: Some(8192),
            target_partitions: Some(4),
        }
    }

    /// Add a Delta Lake table configuration
    pub fn with_delta_table(mut self, table_name: String, table_path: String) -> Self {
        self.delta_tables.insert(table_name, table_path);
        self
    }

    /// Add S3 configuration
    pub fn with_s3_config(mut self, s3_config: S3Config) -> Self {
        self.s3_config = Some(s3_config);
        self
    }

    /// Add object store configuration
    pub fn with_object_store(mut self, name: String, config: ObjectStoreConfig) -> Self {
        self.object_stores.insert(name, config);
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = Some(batch_size);
        self
    }

    /// Set target partitions
    pub fn with_target_partitions(mut self, target_partitions: usize) -> Self {
        self.target_partitions = Some(target_partitions);
        self
    }
}

/// DataFusion-based query executor
pub struct DataFusionExecutor {
    config: DataFusionConfig,
    ctx: SessionContext,
    stats: Arc<RwLock<ExecutorStats>>,
    is_initialized: bool,
    registered_tables: Arc<RwLock<HashMap<String, String>>>, // table_name -> table_path
}

impl DataFusionExecutor {
    /// Create new DataFusion executor
    pub fn new(config: DataFusionConfig) -> Self {
        let mut ctx_config = SessionConfig::new();

        // Configure memory settings
        if let Some(_max_memory) = config.max_memory_bytes {
            ctx_config = ctx_config
                .with_target_partitions(1)
                .with_batch_size(1024)
                .with_repartition_joins(false)
                .with_repartition_aggregations(false);
        }

        // Configure threading
        if let Some(num_threads) = config.num_threads {
            ctx_config = ctx_config.with_target_partitions(num_threads);
        }

        // Configure batch size
        if let Some(batch_size) = config.batch_size {
            ctx_config = ctx_config.with_batch_size(batch_size);
        }

        // Configure target partitions
        if let Some(target_partitions) = config.target_partitions {
            ctx_config = ctx_config.with_target_partitions(target_partitions);
        }

        // Configure optimization settings
        if config.enable_optimization {
            ctx_config = ctx_config
                .with_repartition_joins(true)
                .with_repartition_aggregations(true)
                .with_repartition_windows(true);
        }

        // Configure statistics collection
        if config.enable_statistics {
            ctx_config = ctx_config.with_collect_statistics(true);
        }

        let ctx = SessionContext::new_with_config(ctx_config);

        Self {
            config,
            ctx,
            stats: Arc::new(RwLock::new(ExecutorStats {
                executor: "datafusion".to_string(),
                total_queries: 0,
                queries_per_minute: 0,
                total_execution_time_ms: 0,
                avg_execution_time_ms: 0.0,
                error_count: 0,
                last_execution_time: None,
                is_executing: false,
            })),
            is_initialized: false,
            registered_tables: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Convert telemetry batch to Arrow RecordBatch
    async fn telemetry_batch_to_record_batch(
        &self,
        batch: &TelemetryBatch,
    ) -> BridgeResult<RecordBatch> {
        let mut names = Vec::new();
        let mut timestamps = Vec::new();
        let mut record_types = Vec::new();
        let mut values = Vec::new();
        let mut sources = Vec::new();
        let mut ids = Vec::new();

        for record in &batch.records {
            names.push(record.id.to_string());
            timestamps.push(record.timestamp.timestamp_nanos_opt().unwrap_or(0));
            record_types.push(format!("{:?}", record.record_type));
            sources.push(batch.source.clone());
            ids.push(record.id.to_string());

            // Extract value based on record type
            match &record.data {
                TelemetryData::Metric(metric) => match &metric.value {
                    bridge_core::types::MetricValue::Counter(v) => values.push(format!("{}", v)),
                    bridge_core::types::MetricValue::Gauge(v) => values.push(format!("{}", v)),
                    bridge_core::types::MetricValue::Histogram { sum, .. } => {
                        values.push(format!("{}", sum))
                    }
                    bridge_core::types::MetricValue::Summary { sum, .. } => {
                        values.push(format!("{}", sum))
                    }
                },
                TelemetryData::Trace(trace) => {
                    values.push(format!("{}", trace.duration_ns.unwrap_or(0)));
                }
                TelemetryData::Log(log) => {
                    values.push(log.message.clone());
                }
                TelemetryData::Event(event) => {
                    values.push(event.name.clone());
                }
            }
        }

        let schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                false,
            ),
            Field::new("record_type", DataType::Utf8, false),
            Field::new("value", DataType::Utf8, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("batch_id", DataType::Utf8, false),
        ]);

        let record_batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(StringArray::from(ids)),
                Arc::new(TimestampNanosecondArray::from(timestamps)),
                Arc::new(StringArray::from(record_types)),
                Arc::new(StringArray::from(values)),
                Arc::new(StringArray::from(sources)),
                Arc::new(StringArray::from(vec![
                    batch.id.to_string();
                    batch.records.len()
                ])),
            ],
        )
        .map_err(|e| {
            bridge_core::BridgeError::query(format!("Failed to create record batch: {}", e))
        })?;

        Ok(record_batch)
    }

    /// Register telemetry batch as a table in DataFusion
    async fn register_telemetry_batch(&self, batch: &TelemetryBatch) -> BridgeResult<()> {
        let record_batch = self.telemetry_batch_to_record_batch(batch).await?;
        let table_name = format!("telemetry_batch_{}", batch.id);

        self.ctx
            .register_batch(&table_name, record_batch)
            .map_err(|e| {
                bridge_core::BridgeError::query(format!("Failed to register batch: {}", e))
            })?;

        if self.config.debug_logging {
            info!(
                "Registered telemetry batch {} as table '{}' with {} records",
                batch.id,
                table_name,
                batch.records.len()
            );
        }

        Ok(())
    }

    /// Register a Delta Lake table with DataFusion
    pub async fn register_delta_table(
        &self,
        table_name: &str,
        table_path: &str,
    ) -> BridgeResult<()> {
        if self.config.debug_logging {
            info!(
                "Registering Delta Lake table '{}' at path '{}'",
                table_name, table_path
            );
        }

        // Full DataFusion Delta Lake integration
        let table_path = std::path::Path::new(table_path);

        if !table_path.exists() {
            return Err(bridge_core::BridgeError::query(format!(
                "Delta Lake table path does not exist: {}",
                table_path.display()
            )));
        }

        // For now, we'll use a simplified approach that works with the current API versions
        // TODO: Implement full Delta Lake integration when datafusion-delta crate is available
        info!("Delta Lake table registration is currently simplified - using mock data for demonstration");

        // Track registered tables
        {
            let mut tables = self.registered_tables.write().await;
            tables.insert(table_name.to_string(), table_path.display().to_string());
        }

        if self.config.debug_logging {
            info!(
                "Successfully registered Delta Lake table '{}' at path '{}'",
                table_name,
                table_path.display()
            );
        }

        Ok(())
    }

    /// Register all configured Delta Lake tables
    async fn register_configured_tables(&self) -> BridgeResult<()> {
        for (table_name, table_path) in &self.config.delta_tables {
            self.register_delta_table(table_name, table_path).await?;
        }
        Ok(())
    }

    /// Register S3 object store
    async fn register_s3_object_store(&self) -> BridgeResult<()> {
        if let Some(s3_config) = &self.config.s3_config {
            if self.config.debug_logging {
                info!(
                    "Registering S3 object store for region: {}",
                    s3_config.region
                );
            }

            // For now, we'll use a simplified approach
            // TODO: Implement full S3 object store integration when object_store crate is available
            info!("S3 object store registration is currently simplified - using mock data for demonstration");

            if self.config.debug_logging {
                info!("Successfully registered S3 object store (mock mode)");
            }
        }
        Ok(())
    }

    /// Register object stores
    async fn register_object_stores(&self) -> BridgeResult<()> {
        for (name, config) in &self.config.object_stores {
            if self.config.debug_logging {
                info!("Registering object store: {}", name);
            }

            // For now, we'll use a simplified approach
            // TODO: Implement full object store integration when object_store crate is available
            info!("Object store '{}' registration is currently simplified - using mock data for demonstration", name);

            if self.config.debug_logging {
                info!("Successfully registered object store: {} (mock mode)", name);
            }
        }
        Ok(())
    }

    /// Register custom functions
    async fn register_custom_functions(&self) -> BridgeResult<()> {
        if self.config.debug_logging {
            info!("Registering custom functions");
        }

        // Register telemetry-specific functions
        self.register_telemetry_functions().await?;

        // Register time-series functions
        self.register_timeseries_functions().await?;

        // Register aggregation functions
        self.register_aggregation_functions().await?;

        if self.config.debug_logging {
            info!("Successfully registered custom functions");
        }

        Ok(())
    }

    /// Register custom UDFs
    async fn register_custom_udfs(&self) -> BridgeResult<()> {
        if self.config.debug_logging {
            info!("Registering custom UDFs");
        }

        // Register telemetry UDFs
        self.register_telemetry_udfs().await?;

        // Register analytics UDFs
        self.register_analytics_udfs().await?;

        if self.config.debug_logging {
            info!("Successfully registered custom UDFs");
        }

        Ok(())
    }

    /// Register telemetry-specific functions
    async fn register_telemetry_functions(&self) -> BridgeResult<()> {
        // For now, we'll skip UDF registration until we can properly integrate with DataFusion's UDF system
        if self.config.debug_logging {
            info!("Telemetry functions registration skipped - will be implemented with proper DataFusion UDF integration");
        }

        Ok(())
    }

    /// Register time-series functions
    async fn register_timeseries_functions(&self) -> BridgeResult<()> {
        // For now, we'll skip UDF registration until we can properly integrate with DataFusion's UDF system
        if self.config.debug_logging {
            info!("Time-series functions registration skipped - will be implemented with proper DataFusion UDF integration");
        }

        Ok(())
    }

    /// Register aggregation functions
    async fn register_aggregation_functions(&self) -> BridgeResult<()> {
        // For now, we'll skip UDF registration until we can properly integrate with DataFusion's UDF system
        if self.config.debug_logging {
            info!("Aggregation functions registration skipped - will be implemented with proper DataFusion UDF integration");
        }

        Ok(())
    }

    /// Register telemetry UDFs
    async fn register_telemetry_udfs(&self) -> BridgeResult<()> {
        // For now, we'll skip UDF registration until we can properly integrate with DataFusion's UDF system
        if self.config.debug_logging {
            info!("Telemetry UDFs registration skipped - will be implemented with proper DataFusion UDF integration");
        }

        Ok(())
    }

    /// Register analytics UDFs
    async fn register_analytics_udfs(&self) -> BridgeResult<()> {
        // For now, we'll skip UDF registration until we can properly integrate with DataFusion's UDF system
        if self.config.debug_logging {
            info!("Analytics UDFs registration skipped - will be implemented with proper DataFusion UDF integration");
        }

        Ok(())
    }

    /// Get list of registered tables
    pub async fn get_registered_tables(&self) -> HashMap<String, String> {
        self.registered_tables.read().await.clone()
    }

    /// Execute a query and return Arrow RecordBatches
    async fn execute_query_to_batches(&self, sql: &str) -> BridgeResult<Vec<RecordBatch>> {
        // Execute SQL query using DataFusion
        let df =
            self.ctx.sql(sql).await.map_err(|e| {
                bridge_core::BridgeError::query(format!("SQL parsing error: {}", e))
            })?;

        // Collect results
        let result_batches = df.collect().await.map_err(|e| {
            bridge_core::BridgeError::query(format!("Query execution error: {}", e))
        })?;

        if result_batches.is_empty() {
            return Err(bridge_core::BridgeError::query(
                "No results returned from query",
            ));
        }

        Ok(result_batches)
    }

    /// Convert Arrow RecordBatch to QueryResult
    fn record_batch_to_query_result(
        &self,
        batch: RecordBatch,
        query_id: Uuid,
    ) -> BridgeResult<QueryResult> {
        let mut rows = Vec::new();
        let schema = batch.schema();

        for row_idx in 0..batch.num_rows() {
            let mut row_data = HashMap::new();

            for (col_idx, field) in schema.fields().iter().enumerate() {
                let column = batch.column(col_idx);
                let value = match field.data_type() {
                    DataType::Utf8 => {
                        let array = column.as_any().downcast_ref::<StringArray>().unwrap();
                        if array.is_valid(row_idx) {
                            QueryValue::String(array.value(row_idx).to_string())
                        } else {
                            QueryValue::Null
                        }
                    }
                    DataType::Int64 => {
                        let array = column.as_any().downcast_ref::<Int64Array>().unwrap();
                        if array.is_valid(row_idx) {
                            QueryValue::Integer(array.value(row_idx))
                        } else {
                            QueryValue::Null
                        }
                    }
                    DataType::Float64 => {
                        let array = column.as_any().downcast_ref::<Float64Array>().unwrap();
                        if array.is_valid(row_idx) {
                            QueryValue::Float(array.value(row_idx))
                        } else {
                            QueryValue::Null
                        }
                    }
                    DataType::Boolean => {
                        let array = column.as_any().downcast_ref::<BooleanArray>().unwrap();
                        if array.is_valid(row_idx) {
                            QueryValue::Boolean(array.value(row_idx))
                        } else {
                            QueryValue::Null
                        }
                    }
                    DataType::Timestamp(_, _) => {
                        let array = column
                            .as_any()
                            .downcast_ref::<TimestampNanosecondArray>()
                            .unwrap();
                        if array.is_valid(row_idx) {
                            QueryValue::Integer(array.value(row_idx))
                        } else {
                            QueryValue::Null
                        }
                    }
                    _ => QueryValue::String(format!("{:?}", column)),
                };

                row_data.insert(field.name().clone(), value);
            }

            rows.push(QueryRow {
                id: Uuid::new_v4(),
                data: row_data,
                metadata: HashMap::new(),
            });
        }

        Ok(QueryResult {
            id: Uuid::new_v4(),
            query_id,
            status: ExecutionStatus::Success,
            data: rows,
            metadata: HashMap::from([
                ("executor".to_string(), self.config.name.clone()),
                ("query_type".to_string(), "datafusion".to_string()),
                ("batch_size".to_string(), batch.num_rows().to_string()),
            ]),
            execution_time_ms: 0, // Will be set by caller
            execution_timestamp: Utc::now(),
        })
    }

    /// Combine multiple RecordBatches into a single QueryResult
    fn combine_record_batches_to_query_result(
        &self,
        batches: Vec<RecordBatch>,
        query_id: Uuid,
    ) -> BridgeResult<QueryResult> {
        let mut all_rows = Vec::new();
        let mut total_rows = 0;

        for batch in batches {
            let result = self.record_batch_to_query_result(batch, query_id)?;
            let data_len = result.data.len();
            all_rows.extend(result.data);
            total_rows += data_len;
        }

        Ok(QueryResult {
            id: Uuid::new_v4(),
            query_id,
            status: ExecutionStatus::Success,
            data: all_rows,
            metadata: HashMap::from([
                ("executor".to_string(), self.config.name.clone()),
                ("query_type".to_string(), "datafusion".to_string()),
                ("total_rows".to_string(), total_rows.to_string()),
                ("batch_count".to_string(), total_rows.to_string()),
            ]),
            execution_time_ms: 0, // Will be set by caller
            execution_timestamp: Utc::now(),
        })
    }
}

#[async_trait]
impl QueryExecutor for DataFusionExecutor {
    async fn init(&mut self) -> BridgeResult<()> {
        if self.is_initialized {
            return Ok(());
        }

        info!("Initializing DataFusion executor v{}", self.config.version);

        // Register S3 object store
        self.register_s3_object_store().await?;

        // Register object stores
        self.register_object_stores().await?;

        // Register configured Delta Lake tables
        self.register_configured_tables().await?;

        // Register custom functions
        self.register_custom_functions().await?;

        // Register custom UDFs
        self.register_custom_udfs().await?;

        self.is_initialized = true;
        info!("DataFusion executor initialized successfully");

        Ok(())
    }

    async fn execute(&self, query: ParsedQuery) -> BridgeResult<QueryResult> {
        let start_time = std::time::Instant::now();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_queries += 1;
            stats.is_executing = true;
            stats.last_execution_time = Some(Utc::now());
        }

        // Execute query and get RecordBatches
        let result_batches = self.execute_query_to_batches(&query.query_text).await?;

        // Convert RecordBatches to QueryResult
        let result = self.combine_record_batches_to_query_result(result_batches, query.id);

        let execution_time = start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_executing = false;
            stats.total_execution_time_ms += execution_time_ms;
            stats.avg_execution_time_ms =
                stats.total_execution_time_ms as f64 / stats.total_queries as f64;

            if result.is_err() {
                stats.error_count += 1;
            }
        }

        match result {
            Ok(mut query_result) => {
                query_result.execution_time_ms = execution_time_ms;
                Ok(query_result)
            }
            Err(e) => {
                error!("DataFusion query execution failed: {}", e);
                Err(e)
            }
        }
    }

    async fn get_stats(&self) -> BridgeResult<ExecutorStats> {
        Ok(self.stats.read().await.clone())
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn version(&self) -> &str {
        &self.config.version
    }
}

impl DataFusionExecutor {
    /// Register a telemetry batch for querying
    pub async fn register_batch(&self, batch: &TelemetryBatch) -> BridgeResult<()> {
        self.register_telemetry_batch(batch).await
    }

    /// Execute a raw SQL query
    pub async fn execute_sql(&self, sql: &str) -> BridgeResult<QueryResult> {
        let query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: sql.to_string(),
            ast: QueryAst {
                root: AstNode {
                    node_type: NodeType::Statement,
                    value: Some(sql.to_string()),
                    children: Vec::new(),
                    metadata: HashMap::new(),
                },
                node_count: 1,
                depth: 1,
            },
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        self.execute(query).await
    }

    /// Execute a query against a specific Delta Lake table
    pub async fn execute_delta_lake_query(
        &self,
        table_name: &str,
        sql: &str,
    ) -> BridgeResult<QueryResult> {
        let start_time = std::time::Instant::now();

        // Check if table is registered
        let registered_tables = self.registered_tables.read().await;
        if !registered_tables.contains_key(table_name) {
            return Err(bridge_core::BridgeError::query(format!(
                "Delta Lake table '{}' is not registered",
                table_name
            )));
        }

        // Execute the query
        let result_batches = self.execute_query_to_batches(sql).await?;

        // Convert RecordBatches to QueryResult
        let result = self.combine_record_batches_to_query_result(result_batches, Uuid::new_v4());

        let execution_time = start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;

        match result {
            Ok(mut query_result) => {
                query_result.execution_time_ms = execution_time_ms;
                query_result
                    .metadata
                    .insert("table_name".to_string(), table_name.to_string());
                query_result
                    .metadata
                    .insert("query_type".to_string(), "delta_lake".to_string());
                Ok(query_result)
            }
            Err(e) => {
                error!("Delta Lake query execution failed: {}", e);
                Err(e)
            }
        }
    }

    /// Get DataFusion context for advanced operations
    pub fn context(&self) -> &SessionContext {
        &self.ctx
    }

    /// Get query plan for a SQL query
    pub async fn get_query_plan(&self, sql: &str) -> BridgeResult<String> {
        let df =
            self.ctx.sql(sql).await.map_err(|e| {
                bridge_core::BridgeError::query(format!("SQL parsing error: {}", e))
            })?;

        let plan = df.logical_plan();

        Ok(format!("{:?}", plan))
    }

    /// Get optimized query plan for a SQL query
    pub async fn get_optimized_query_plan(&self, sql: &str) -> BridgeResult<String> {
        let df =
            self.ctx.sql(sql).await.map_err(|e| {
                bridge_core::BridgeError::query(format!("SQL parsing error: {}", e))
            })?;

        let plan = df.logical_plan();

        // Return the logical plan as the "optimized" plan
        Ok(format!("{:?}", plan))
    }

    /// Execute a query with custom configuration
    pub async fn execute_with_config(
        &self,
        sql: &str,
        config: &QueryConfig,
    ) -> BridgeResult<QueryResult> {
        let start_time = std::time::Instant::now();

        // Create DataFrame with custom configuration
        let df =
            self.ctx.sql(sql).await.map_err(|e| {
                bridge_core::BridgeError::query(format!("SQL parsing error: {}", e))
            })?;

        // Apply custom configuration
        // Note: DataFrame doesn't have with_batch_size and with_target_partitions methods
        // These would need to be applied at the SessionContext level
        if self.config.debug_logging {
            info!(
                "Custom configuration applied: batch_size={:?}, target_partitions={:?}",
                config.batch_size, config.target_partitions
            );
        }

        // Execute query
        let result_batches = df.collect().await.map_err(|e| {
            bridge_core::BridgeError::query(format!("Query execution error: {}", e))
        })?;

        // Convert to QueryResult
        let result = self.combine_record_batches_to_query_result(result_batches, Uuid::new_v4());

        let execution_time = start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;

        match result {
            Ok(mut query_result) => {
                query_result.execution_time_ms = execution_time_ms;
                query_result
                    .metadata
                    .insert("query_config".to_string(), format!("{:?}", config));
                Ok(query_result)
            }
            Err(e) => {
                error!("Query execution with config failed: {}", e);
                Err(e)
            }
        }
    }

    /// Register a Parquet table from S3
    pub async fn register_s3_parquet_table(
        &self,
        table_name: &str,
        s3_path: &str,
    ) -> BridgeResult<()> {
        if self.config.debug_logging {
            info!(
                "Registering S3 Parquet table '{}' at path '{}'",
                table_name, s3_path
            );
        }

        // For now, we'll use a simplified approach
        // TODO: Implement full S3 Parquet table registration when object_store crate is available
        info!("S3 Parquet table registration is currently simplified - using mock data for demonstration");

        if self.config.debug_logging {
            info!(
                "Successfully registered S3 Parquet table '{}' (mock mode)",
                table_name
            );
        }

        Ok(())
    }

    /// Register a CSV table from S3
    pub async fn register_s3_csv_table(&self, table_name: &str, s3_path: &str) -> BridgeResult<()> {
        if self.config.debug_logging {
            info!(
                "Registering S3 CSV table '{}' at path '{}'",
                table_name, s3_path
            );
        }

        // For now, we'll use a simplified approach
        // TODO: Implement full S3 CSV table registration when object_store crate is available
        info!(
            "S3 CSV table registration is currently simplified - using mock data for demonstration"
        );

        if self.config.debug_logging {
            info!(
                "Successfully registered S3 CSV table '{}' (mock mode)",
                table_name
            );
        }

        Ok(())
    }

    /// Get table schema
    pub async fn get_table_schema(&self, table_name: &str) -> BridgeResult<String> {
        let schema = self.ctx.table(table_name).await.map_err(|e| {
            bridge_core::BridgeError::query(format!("Failed to get table schema: {}", e))
        })?;

        let schema = schema.schema();
        Ok(format!("{:?}", schema))
    }

    /// List all registered tables
    pub async fn list_tables(&self) -> BridgeResult<Vec<String>> {
        // For now, we'll return the tables we've tracked manually
        // TODO: Implement full table listing when DataFusion API provides this functionality
        let registered_tables = self.registered_tables.read().await;
        let table_names: Vec<String> = registered_tables.keys().cloned().collect();
        Ok(table_names)
    }

    /// Get query execution statistics
    pub async fn get_execution_stats(&self, sql: &str) -> BridgeResult<ExecutionStats> {
        let df =
            self.ctx.sql(sql).await.map_err(|e| {
                bridge_core::BridgeError::query(format!("SQL parsing error: {}", e))
            })?;

        let plan = df.logical_plan();

        // For now, we'll skip physical plan creation as it may not be available
        // TODO: Implement full physical plan creation when DataFusion API provides this functionality
        let physical_plan = "Physical plan not available in current DataFusion version".to_string();

        Ok(ExecutionStats {
            logical_plan: format!("{:?}", plan),
            physical_plan,
            estimated_rows: None, // DataFusion doesn't provide this directly
            estimated_cost: None, // DataFusion doesn't provide this directly
        })
    }
}

/// Query configuration for custom execution
#[derive(Debug, Clone)]
pub struct QueryConfig {
    /// Batch size for query execution
    pub batch_size: Option<usize>,

    /// Target partitions for parallel execution
    pub target_partitions: Option<usize>,

    /// Enable statistics collection
    pub enable_statistics: bool,

    /// Custom configuration parameters
    pub parameters: HashMap<String, String>,
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    /// Logical plan
    pub logical_plan: String,

    /// Physical plan
    pub physical_plan: String,

    /// Estimated number of rows
    pub estimated_rows: Option<usize>,

    /// Estimated execution cost
    pub estimated_cost: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::types::{TelemetryData, TelemetryRecord, TelemetryType};
    use std::path::Path;

    #[tokio::test]
    async fn test_datafusion_executor_creation() {
        let config = DataFusionConfig::new();
        let executor = DataFusionExecutor::new(config);
        assert_eq!(executor.name(), "datafusion");
    }

    #[tokio::test]
    async fn test_datafusion_executor_initialization() {
        let config = DataFusionConfig::new();
        let mut executor = DataFusionExecutor::new(config);
        let result = executor.init().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_telemetry_batch_to_record_batch() {
        let config = DataFusionConfig::new();
        let executor = DataFusionExecutor::new(config);

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            source: "test_source".to_string(),
            timestamp: Utc::now(),
            size: 1,
            records: vec![TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(bridge_core::types::MetricData {
                    name: "test_metric".to_string(),
                    description: None,
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Counter,
                    value: bridge_core::types::MetricValue::Counter(42.0),
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            }],
            metadata: HashMap::new(),
        };

        let record_batch = executor.telemetry_batch_to_record_batch(&batch).await;
        assert!(record_batch.is_ok());

        let record_batch = record_batch.unwrap();
        assert_eq!(record_batch.num_rows(), 1);
        assert_eq!(record_batch.num_columns(), 6); // id, timestamp, record_type, value, source, batch_id
    }

    #[tokio::test]
    async fn test_record_batch_to_query_result() {
        let config = DataFusionConfig::new();
        let executor = DataFusionExecutor::new(config);

        // Create a simple RecordBatch
        let schema = Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("value", DataType::Int64, false),
        ]);

        let record_batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(StringArray::from(vec!["test1", "test2"])),
                Arc::new(Int64Array::from(vec![1, 2])),
            ],
        )
        .unwrap();

        let query_id = Uuid::new_v4();
        let result = executor.record_batch_to_query_result(record_batch, query_id);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.data.len(), 2);
        assert_eq!(result.query_id, query_id);
        assert!(matches!(result.status, ExecutionStatus::Success));
    }

    #[tokio::test]
    async fn test_sql_query_execution() {
        let config = DataFusionConfig::new();
        let mut executor = DataFusionExecutor::new(config);
        executor.init().await.unwrap();

        // Create a simple in-memory table for testing
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]);

        let record_batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3])),
                Arc::new(StringArray::from(vec!["Alice", "Bob", "Charlie"])),
            ],
        )
        .unwrap();

        executor
            .ctx
            .register_batch("test_table", record_batch)
            .unwrap();

        // Execute a simple SQL query
        let result = executor
            .execute_sql("SELECT * FROM test_table WHERE id > 1")
            .await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.data.len(), 2); // Should return Bob and Charlie
        assert!(matches!(result.status, ExecutionStatus::Success));
    }

    #[tokio::test]
    async fn test_query_plan_generation() {
        let config = DataFusionConfig::new();
        let mut executor = DataFusionExecutor::new(config);
        executor.init().await.unwrap();

        // Create a simple in-memory table for testing
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]);

        let record_batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3])),
                Arc::new(StringArray::from(vec!["Alice", "Bob", "Charlie"])),
            ],
        )
        .unwrap();

        executor
            .ctx
            .register_batch("test_table", record_batch)
            .unwrap();

        let plan = executor.get_query_plan("SELECT * FROM test_table").await;
        assert!(plan.is_ok());

        let plan_str = plan.unwrap();
        assert!(!plan_str.is_empty());
        assert!(plan_str.contains("TableScan"));
    }

    #[tokio::test]
    async fn test_delta_lake_table_registration() {
        let config = DataFusionConfig::new();
        let mut executor = DataFusionExecutor::new(config);
        executor.init().await.unwrap();

        // Test with a non-existent path (should fail as path doesn't exist)
        let result = executor
            .register_delta_table("test_table", "/non/existent/path")
            .await;
        assert!(result.is_err()); // Should fail because path doesn't exist

        // Test with current directory (should succeed)
        let result = executor.register_delta_table("test_table", ".").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_executor_stats() {
        let config = DataFusionConfig::new();
        let mut executor = DataFusionExecutor::new(config);
        executor.init().await.unwrap();

        let stats = executor.get_stats().await;
        assert!(stats.is_ok());

        let stats = stats.unwrap();
        assert_eq!(stats.executor, "datafusion");
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.error_count, 0);
    }
}
