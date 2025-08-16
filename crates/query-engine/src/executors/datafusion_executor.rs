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
        }
    }
}

/// DataFusion-based query executor
pub struct DataFusionExecutor {
    config: DataFusionConfig,
    ctx: SessionContext,
    stats: Arc<RwLock<ExecutorStats>>,
    is_initialized: bool,
}

impl DataFusionExecutor {
    /// Create new DataFusion executor
    pub fn new(config: DataFusionConfig) -> Self {
        let mut ctx_config = SessionConfig::new();

        if let Some(_max_memory) = config.max_memory_bytes {
            ctx_config = ctx_config
                .with_target_partitions(1)
                .with_batch_size(1024)
                .with_repartition_joins(false)
                .with_repartition_aggregations(false);
        }

        if let Some(num_threads) = config.num_threads {
            ctx_config = ctx_config.with_target_partitions(num_threads);
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
            metadata: HashMap::new(),
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

        // Register custom functions if needed
        // self.ctx.register_udf(...);

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

        // Execute SQL query using DataFusion
        let df =
            self.ctx.sql(&query.query_text).await.map_err(|e| {
                bridge_core::BridgeError::query(format!("SQL parsing error: {}", e))
            })?;
        let result_batch = df.collect().await.map_err(|e| {
            bridge_core::BridgeError::query(format!("Query execution error: {}", e))
        })?;

        if result_batch.is_empty() {
            return Err(bridge_core::BridgeError::query(
                "No results returned from query",
            ));
        }

        // Use the first batch for now (TODO: implement proper batch concatenation)
        let combined_batch = result_batch[0].clone();

        let result = self.record_batch_to_query_result(combined_batch, query.id);

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

    /// Get DataFusion context for advanced operations
    pub fn context(&self) -> &SessionContext {
        &self.ctx
    }
}
