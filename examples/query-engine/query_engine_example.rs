//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating query engine functionality
//!
//! This example shows how to create a simple query engine that can
//! query telemetry data from various sources and formats.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use bridge_core::{
    pipeline::PipelineConfig,
    traits::ExporterStats,
    types::{
        ExportResult, ExportStatus, MetricData, MetricValue, ProcessedBatch, ProcessingStatus,
        TelemetryData, TelemetryRecord, TelemetryType,
    },
    BridgeResult, LakehouseExporter, TelemetryIngestionPipeline, TelemetryReceiver,
};

/// Query types supported by the query engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    /// SQL query
    Sql(String),
    /// JSON query
    Json(String),
    /// GraphQL query
    GraphQL(String),
    /// Custom query
    Custom(String),
}

/// Query result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Query ID
    pub query_id: String,

    /// Query execution timestamp
    pub timestamp: DateTime<Utc>,

    /// Query status
    pub status: QueryStatus,

    /// Query results
    pub data: Vec<QueryRow>,

    /// Query metadata
    pub metadata: HashMap<String, String>,

    /// Query execution time in milliseconds
    pub execution_time_ms: u64,

    /// Number of rows returned
    pub row_count: usize,

    /// Query error message (if any)
    pub error_message: Option<String>,
}

/// Query status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryStatus {
    /// Query completed successfully
    Success,
    /// Query failed
    Failed,
    /// Query is still running
    Running,
    /// Query was cancelled
    Cancelled,
}

/// Query row structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRow {
    /// Row ID
    pub id: String,

    /// Row data as key-value pairs
    pub data: HashMap<String, serde_json::Value>,

    /// Row timestamp
    pub timestamp: DateTime<Utc>,
}

/// Query statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStats {
    /// Total queries executed
    pub total_queries: u64,

    /// Successful queries
    pub successful_queries: u64,

    /// Failed queries
    pub failed_queries: u64,

    /// Average query execution time in milliseconds
    pub avg_execution_time_ms: f64,

    /// Total rows returned
    pub total_rows_returned: u64,

    /// Last query time
    pub last_query_time: Option<DateTime<Utc>>,
}

/// Simple query engine implementation
pub struct SimpleQueryEngine {
    /// Engine name
    name: String,

    /// Query statistics
    stats: Arc<RwLock<QueryStats>>,

    /// Engine state
    is_running: Arc<RwLock<bool>>,

    /// Query cache (simplified)
    query_cache: Arc<RwLock<HashMap<String, QueryResult>>>,
}

impl SimpleQueryEngine {
    /// Create a new query engine
    pub fn new(name: String) -> Self {
        let stats = QueryStats {
            total_queries: 0,
            successful_queries: 0,
            failed_queries: 0,
            avg_execution_time_ms: 0.0,
            total_rows_returned: 0,
            last_query_time: None,
        };

        Self {
            name,
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            query_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the query engine
    pub async fn start(&mut self) {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        info!("Simple query engine '{}' started", self.name);
    }

    /// Stop the query engine
    pub async fn stop(&mut self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("Simple query engine '{}' stopped", self.name);
    }

    /// Execute a query
    pub async fn execute_query(&self, query: QueryType) -> BridgeResult<QueryResult> {
        let start_time = Instant::now();

        // Check if engine is running
        if !*self.is_running.read().await {
            return Err(bridge_core::error::BridgeError::internal(
                "Query engine is not running",
            ));
        }

        info!("Simple query engine '{}' executing query", self.name);

        // Generate query ID
        let query_id = Uuid::new_v4().to_string();

        // Execute the query based on type
        let (status, data, error_message) = match query {
            QueryType::Sql(sql) => self.execute_sql_query(&sql).await,
            QueryType::Json(json) => self.execute_json_query(&json).await,
            QueryType::GraphQL(graphql) => self.execute_graphql_query(&graphql).await,
            QueryType::Custom(custom) => self.execute_custom_query(&custom).await,
        };

        let execution_time = start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;

        // Create query result
        let result = QueryResult {
            query_id: query_id.clone(),
            timestamp: Utc::now(),
            status,
            data: data.clone(),
            metadata: HashMap::new(),
            execution_time_ms,
            row_count: data.len(),
            error_message,
        };

        // Cache the result
        {
            let mut cache = self.query_cache.write().await;
            cache.insert(query_id.clone(), result.clone());
        }

        // Update statistics
        self.update_stats(&result).await;

        info!(
            "Simple query engine '{}' completed query in {}ms, returned {} rows",
            self.name, execution_time_ms, result.row_count
        );

        Ok(result)
    }

    /// Execute SQL query
    async fn execute_sql_query(&self, sql: &str) -> (QueryStatus, Vec<QueryRow>, Option<String>) {
        info!("Executing SQL query: {}", sql);

        // Simple SQL parsing and execution
        // In a real implementation, this would use a proper SQL parser
        let mut rows = Vec::new();

        // Simulate SQL query execution
        if sql.to_lowercase().contains("select") {
            // Generate some mock data based on the query
            for i in 0..5 {
                let mut row_data = HashMap::new();
                row_data.insert("id".to_string(), serde_json::Value::String(i.to_string()));
                row_data.insert(
                    "metric_name".to_string(),
                    serde_json::Value::String(format!("metric_{}", i)),
                );
                row_data.insert(
                    "metric_value".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(i as f64).unwrap()),
                );
                row_data.insert(
                    "timestamp".to_string(),
                    serde_json::Value::String(Utc::now().to_rfc3339()),
                );

                let row = QueryRow {
                    id: Uuid::new_v4().to_string(),
                    data: row_data,
                    timestamp: Utc::now(),
                };
                rows.push(row);
            }

            (QueryStatus::Success, rows, None)
        } else {
            (
                QueryStatus::Failed,
                Vec::new(),
                Some("Invalid SQL query".to_string()),
            )
        }
    }

    /// Execute JSON query
    async fn execute_json_query(&self, json: &str) -> (QueryStatus, Vec<QueryRow>, Option<String>) {
        info!("Executing JSON query: {}", json);

        // Simple JSON query execution
        // In a real implementation, this would use a proper JSON query engine
        let mut rows = Vec::new();

        // Simulate JSON query execution
        if json.contains("metrics") {
            // Generate some mock data
            for i in 0..3 {
                let mut row_data = HashMap::new();
                row_data.insert(
                    "metric_name".to_string(),
                    serde_json::Value::String(format!("json_metric_{}", i)),
                );
                row_data.insert(
                    "value".to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64((i * 10) as f64).unwrap(),
                    ),
                );
                row_data.insert(
                    "source".to_string(),
                    serde_json::Value::String("json_query".to_string()),
                );

                let row = QueryRow {
                    id: Uuid::new_v4().to_string(),
                    data: row_data,
                    timestamp: Utc::now(),
                };
                rows.push(row);
            }

            (QueryStatus::Success, rows, None)
        } else {
            (
                QueryStatus::Failed,
                Vec::new(),
                Some("Invalid JSON query".to_string()),
            )
        }
    }

    /// Execute GraphQL query
    async fn execute_graphql_query(
        &self,
        graphql: &str,
    ) -> (QueryStatus, Vec<QueryRow>, Option<String>) {
        info!("Executing GraphQL query: {}", graphql);

        // Simple GraphQL query execution
        // In a real implementation, this would use a proper GraphQL engine
        let mut rows = Vec::new();

        // Simulate GraphQL query execution
        if graphql.contains("query") {
            // Generate some mock data
            for i in 0..4 {
                let mut row_data = HashMap::new();
                row_data.insert(
                    "field".to_string(),
                    serde_json::Value::String(format!("graphql_field_{}", i)),
                );
                row_data.insert(
                    "value".to_string(),
                    serde_json::Value::String(format!("value_{}", i)),
                );
                row_data.insert(
                    "type".to_string(),
                    serde_json::Value::String("graphql".to_string()),
                );

                let row = QueryRow {
                    id: Uuid::new_v4().to_string(),
                    data: row_data,
                    timestamp: Utc::now(),
                };
                rows.push(row);
            }

            (QueryStatus::Success, rows, None)
        } else {
            (
                QueryStatus::Failed,
                Vec::new(),
                Some("Invalid GraphQL query".to_string()),
            )
        }
    }

    /// Execute custom query
    async fn execute_custom_query(
        &self,
        custom: &str,
    ) -> (QueryStatus, Vec<QueryRow>, Option<String>) {
        info!("Executing custom query: {}", custom);

        // Simple custom query execution
        let mut rows = Vec::new();

        // Simulate custom query execution
        if custom.contains("custom") {
            // Generate some mock data
            for i in 0..2 {
                let mut row_data = HashMap::new();
                row_data.insert(
                    "custom_field".to_string(),
                    serde_json::Value::String(format!("custom_value_{}", i)),
                );
                row_data.insert(
                    "query_type".to_string(),
                    serde_json::Value::String("custom".to_string()),
                );

                let row = QueryRow {
                    id: Uuid::new_v4().to_string(),
                    data: row_data,
                    timestamp: Utc::now(),
                };
                rows.push(row);
            }

            (QueryStatus::Success, rows, None)
        } else {
            (
                QueryStatus::Failed,
                Vec::new(),
                Some("Invalid custom query".to_string()),
            )
        }
    }

    /// Update query statistics
    async fn update_stats(&self, result: &QueryResult) {
        let mut stats = self.stats.write().await;

        stats.total_queries += 1;
        stats.total_rows_returned += result.row_count as u64;
        stats.last_query_time = Some(Utc::now());

        match result.status {
            QueryStatus::Success => {
                stats.successful_queries += 1;

                // Update average execution time
                let execution_time_ms = result.execution_time_ms as f64;
                if stats.total_queries > 1 {
                    stats.avg_execution_time_ms = (stats.avg_execution_time_ms
                        * (stats.total_queries - 1) as f64
                        + execution_time_ms)
                        / stats.total_queries as f64;
                } else {
                    stats.avg_execution_time_ms = execution_time_ms;
                }
            }
            QueryStatus::Failed => {
                stats.failed_queries += 1;
            }
            _ => {}
        }
    }

    /// Get query statistics
    pub async fn get_stats(&self) -> QueryStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get cached query result
    pub async fn get_cached_result(&self, query_id: &str) -> Option<QueryResult> {
        let cache = self.query_cache.read().await;
        cache.get(query_id).cloned()
    }

    /// Clear query cache
    pub async fn clear_cache(&self) {
        let mut cache = self.query_cache.write().await;
        cache.clear();
        info!("Simple query engine '{}' cache cleared", self.name);
    }
}

/// Query-aware receiver that can handle query requests
pub struct QueryAwareReceiver {
    name: String,
    is_running: bool,
    query_engine: Arc<SimpleQueryEngine>,
    received_queries: Arc<RwLock<Vec<QueryType>>>,
}

impl QueryAwareReceiver {
    pub fn new(name: String, query_engine: Arc<SimpleQueryEngine>) -> Self {
        Self {
            name,
            is_running: false,
            query_engine,
            received_queries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_query(&self, query: QueryType) {
        let mut received_queries = self.received_queries.write().await;
        received_queries.push(query);

        info!("Query-aware receiver '{}' received query", self.name);
    }

    pub async fn get_received_queries_count(&self) -> usize {
        let received_queries = self.received_queries.read().await;
        received_queries.len()
    }
}

#[async_trait]
impl TelemetryReceiver for QueryAwareReceiver {
    async fn receive(&self) -> BridgeResult<bridge_core::types::TelemetryBatch> {
        // Get the most recent queries
        let received_queries = self.received_queries.read().await;

        if received_queries.is_empty() {
            // Return empty batch if no queries received
            let empty_batch = bridge_core::types::TelemetryBatch {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source: "query-aware".to_string(),
                size: 0,
                records: Vec::new(),
                metadata: HashMap::new(),
            };
            return Ok(empty_batch);
        }

        // Convert queries to telemetry records
        let mut records = Vec::new();

        for (i, query) in received_queries.iter().enumerate() {
            // Execute the query and create a telemetry record from the result
            match self.query_engine.execute_query(query.clone()).await {
                Ok(result) => {
                    let record = TelemetryRecord {
                        id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        record_type: TelemetryType::Metric,
                        data: TelemetryData::Metric(MetricData {
                            name: format!("query_result_{}", i),
                            description: Some(format!("Query result for query {}", i)),
                            unit: Some("count".to_string()),
                            metric_type: bridge_core::types::MetricType::Gauge,
                            value: MetricValue::Gauge(result.row_count as f64),
                            labels: HashMap::new(),
                            timestamp: Utc::now(),
                        }),
                        attributes: HashMap::new(),
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    };
                    records.push(record);
                }
                Err(e) => {
                    warn!("Failed to execute query {}: {}", i, e);
                }
            }
        }

        let batch = bridge_core::types::TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "query-aware".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        };

        info!(
            "Query-aware receiver '{}' generated batch with {} records from {} queries",
            self.name,
            batch.size,
            received_queries.len()
        );
        Ok(batch)
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(self.is_running)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ReceiverStats> {
        let queries_count = self.get_received_queries_count().await;

        Ok(bridge_core::traits::ReceiverStats {
            total_records: queries_count as u64,
            records_per_minute: queries_count as u64 / 60, // Simplified calculation
            total_bytes: (queries_count * 100) as u64,     // Estimate
            bytes_per_minute: (queries_count * 100) as u64 / 60,
            error_count: 0,
            last_receive_time: Some(Utc::now()),
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Query-aware receiver '{}' shutting down", self.name);
        Ok(())
    }
}

/// Query-aware exporter that can handle query results
pub struct QueryAwareExporter {
    name: String,
    query_engine: Arc<SimpleQueryEngine>,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ExporterStats>>,
}

impl QueryAwareExporter {
    pub fn new(name: String, query_engine: Arc<SimpleQueryEngine>) -> Self {
        let stats = ExporterStats {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
        };

        Self {
            name,
            query_engine,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    async fn update_stats(&self, records: usize, duration: Duration, success: bool) {
        let mut stats = self.stats.write().await;

        stats.total_batches += 1;
        stats.total_records += records as u64;
        stats.last_export_time = Some(Utc::now());

        if success {
            let duration_ms = duration.as_millis() as f64;
            if stats.total_batches > 1 {
                stats.avg_export_time_ms =
                    (stats.avg_export_time_ms * (stats.total_batches - 1) as f64 + duration_ms)
                        / stats.total_batches as f64;
            } else {
                stats.avg_export_time_ms = duration_ms;
            }
        } else {
            stats.error_count += 1;
        }
    }
}

#[async_trait]
impl LakehouseExporter for QueryAwareExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = Instant::now();

        // Check if exporter is running
        if !*self.is_running.read().await {
            return Err(bridge_core::error::BridgeError::export(
                "Query-aware exporter is not running",
            ));
        }

        info!(
            "Query-aware exporter exporting batch with {} records",
            batch.records.len()
        );

        // Process the batch and potentially execute additional queries
        let mut success_count = 0;
        let mut errors = Vec::new();

        for record in &batch.records {
            // In a real implementation, this might trigger additional queries
            // based on the processed data
            if let Some(telemetry_data) = &record.transformed_data {
                match telemetry_data {
                    TelemetryData::Metric(metric_data) => {
                        // Simulate query execution based on metric data
                        let query = QueryType::Sql(format!(
                            "SELECT * FROM metrics WHERE name = '{}'",
                            metric_data.name
                        ));

                        match self.query_engine.execute_query(query).await {
                            Ok(_) => success_count += 1,
                            Err(e) => {
                                errors.push(e.to_string());
                            }
                        }
                    }
                    _ => {
                        success_count += 1;
                    }
                }
            } else {
                success_count += 1;
            }
        }

        let duration = start_time.elapsed();

        // Update statistics
        self.update_stats(batch.records.len(), duration, errors.is_empty())
            .await;

        // Convert errors to ExportError format
        let export_errors: Vec<bridge_core::types::ExportError> = errors
            .iter()
            .map(|e| bridge_core::types::ExportError {
                code: "QUERY_ENGINE_ERROR".to_string(),
                message: e.clone(),
                details: None,
            })
            .collect();

        // Create export result
        let export_result = ExportResult {
            timestamp: Utc::now(),
            status: if errors.is_empty() {
                ExportStatus::Success
            } else {
                ExportStatus::Failed
            },
            records_exported: success_count,
            records_failed: batch.records.len() - success_count,
            duration_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            errors: export_errors,
        };

        if errors.is_empty() {
            info!(
                "Query-aware exporter successfully exported {} records in {:?}",
                success_count, duration
            );
        } else {
            warn!(
                "Query-aware exporter failed to export some records: {} errors",
                errors.len()
            );
        }

        Ok(export_result)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(*self.is_running.read().await)
    }

    async fn get_stats(&self) -> BridgeResult<ExporterStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Query-aware exporter shutting down");

        let mut is_running = self.is_running.write().await;
        *is_running = false;

        info!("Query-aware exporter shutdown completed");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting query engine example");

    // Create query engine
    let mut query_engine = SimpleQueryEngine::new("main-query-engine".to_string());
    query_engine.start().await;
    let query_engine = Arc::new(query_engine);

    // Create pipeline configuration
    let config = PipelineConfig {
        name: "query-engine-pipeline".to_string(),
        max_batch_size: 1000,
        flush_interval_ms: 5000,
        buffer_size: 10000,
        enable_backpressure: true,
        backpressure_threshold: 80,
        enable_metrics: true,
        enable_health_checks: true,
        health_check_interval_ms: 30000,
    };

    // Create pipeline
    let mut pipeline = TelemetryIngestionPipeline::new(config);

    // Create and add query-aware receiver
    let mut receiver = QueryAwareReceiver::new(
        "query-aware-receiver".to_string(),
        Arc::clone(&query_engine),
    );
    receiver.is_running = true; // Make receiver healthy
    let receiver_concrete = Arc::new(receiver);
    let receiver_trait = receiver_concrete.clone() as Arc<dyn TelemetryReceiver>;
    pipeline.add_receiver(receiver_trait);

    // Create and add query-aware exporter
    let query_exporter = QueryAwareExporter::new(
        "query-aware-exporter".to_string(),
        Arc::clone(&query_engine),
    );
    {
        let mut is_running = query_exporter.is_running.write().await;
        *is_running = true;
    }
    let query_exporter = Arc::new(query_exporter);
    pipeline.add_exporter(query_exporter);

    // Start pipeline
    pipeline.start().await?;

    info!("Query engine pipeline started successfully");

    // Execute some test queries
    let test_queries = vec![
        QueryType::Sql("SELECT * FROM metrics WHERE value > 10".to_string()),
        QueryType::Json(r#"{"query": "metrics", "filter": {"value": {"gt": 5}}}"#.to_string()),
        QueryType::GraphQL("query { metrics { name value } }".to_string()),
        QueryType::Custom("custom query for testing".to_string()),
    ];

    for (i, query) in test_queries.iter().enumerate() {
        info!("Executing test query {}: {:?}", i + 1, query);

        match query_engine.execute_query(query.clone()).await {
            Ok(result) => {
                info!(
                    "Query {} completed successfully: {} rows in {}ms",
                    i + 1,
                    result.row_count,
                    result.execution_time_ms
                );
            }
            Err(e) => {
                warn!("Query {} failed: {}", i + 1, e);
            }
        }

        // Add query to receiver for pipeline processing
        let receiver_ref = Arc::clone(&receiver_concrete);
        receiver_ref.add_query(query.clone()).await;
    }

    // Let it run for a few seconds
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Get query engine statistics
    let stats = query_engine.get_stats().await;
    info!("Query engine statistics: {:?}", stats);

    // Stop pipeline
    pipeline.stop().await?;

    info!("Query engine example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_engine_creation() {
        let query_engine = SimpleQueryEngine::new("test-engine".to_string());
        assert_eq!(query_engine.name, "test-engine");
    }

    #[tokio::test]
    async fn test_query_engine_lifecycle() {
        let mut query_engine = SimpleQueryEngine::new("test-engine".to_string());

        // Initially not running
        assert!(!*query_engine.is_running.read().await);

        // Start engine
        query_engine.start().await;
        assert!(*query_engine.is_running.read().await);

        // Stop engine
        query_engine.stop().await;
        assert!(!*query_engine.is_running.read().await);
    }

    #[tokio::test]
    async fn test_sql_query_execution() {
        let mut query_engine = SimpleQueryEngine::new("test-engine".to_string());
        query_engine.start().await;

        let query = QueryType::Sql("SELECT * FROM metrics".to_string());
        let result = query_engine.execute_query(query).await.unwrap();

        assert_eq!(result.status, QueryStatus::Success);
        assert_eq!(result.row_count, 5); // Mock data returns 5 rows
        assert!(result.execution_time_ms >= 0);
    }

    #[tokio::test]
    async fn test_json_query_execution() {
        let mut query_engine = SimpleQueryEngine::new("test-engine".to_string());
        query_engine.start().await;

        let query = QueryType::Json(r#"{"query": "metrics"}"#.to_string());
        let result = query_engine.execute_query(query).await.unwrap();

        assert_eq!(result.status, QueryStatus::Success);
        assert_eq!(result.row_count, 3); // Mock data returns 3 rows
        assert!(result.execution_time_ms >= 0);
    }

    #[tokio::test]
    async fn test_graphql_query_execution() {
        let mut query_engine = SimpleQueryEngine::new("test-engine".to_string());
        query_engine.start().await;

        let query = QueryType::GraphQL("query { metrics }".to_string());
        let result = query_engine.execute_query(query).await.unwrap();

        assert_eq!(result.status, QueryStatus::Success);
        assert_eq!(result.row_count, 4); // Mock data returns 4 rows
        assert!(result.execution_time_ms >= 0);
    }

    #[tokio::test]
    async fn test_query_engine_stats() {
        let mut query_engine = SimpleQueryEngine::new("test-engine".to_string());
        query_engine.start().await;

        // Execute a query to update stats
        let query = QueryType::Sql("SELECT * FROM metrics".to_string());
        query_engine.execute_query(query).await.unwrap();

        let stats = query_engine.get_stats().await;
        assert_eq!(stats.total_queries, 1);
        assert_eq!(stats.successful_queries, 1);
        assert_eq!(stats.failed_queries, 0);
        assert!(stats.avg_execution_time_ms >= 0.0);
        assert!(stats.last_query_time.is_some());
    }

    #[tokio::test]
    async fn test_query_aware_receiver_creation() {
        let query_engine = Arc::new(SimpleQueryEngine::new("test-engine".to_string()));
        let receiver = QueryAwareReceiver::new("test-receiver".to_string(), query_engine);

        assert_eq!(receiver.get_received_queries_count().await, 0);
    }

    #[tokio::test]
    async fn test_query_aware_receiver_query_handling() {
        let query_engine = Arc::new(SimpleQueryEngine::new("test-engine".to_string()));
        let receiver = QueryAwareReceiver::new("test-receiver".to_string(), query_engine);

        // Add a query
        let query = QueryType::Sql("SELECT * FROM metrics".to_string());
        receiver.add_query(query).await;

        assert_eq!(receiver.get_received_queries_count().await, 1);
    }

    #[tokio::test]
    async fn test_query_aware_exporter_creation() {
        let query_engine = Arc::new(SimpleQueryEngine::new("test-engine".to_string()));
        let exporter = QueryAwareExporter::new("test-exporter".to_string(), query_engine);

        assert_eq!(exporter.name(), "test-exporter");
        assert_eq!(exporter.version(), "1.0.0");
    }
}
