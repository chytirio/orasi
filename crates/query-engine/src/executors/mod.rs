//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query executors for the bridge
//!
//! This module provides executor implementations for query execution
//! including query execution engines and result handling.

pub mod datafusion_executor;

/// Mock executor for testing and development
pub struct MockExecutor {
    name: String,
    version: String,
    stats: Arc<RwLock<ExecutorStats>>,
}

impl MockExecutor {
    /// Create a new mock executor
    pub fn new() -> Self {
        Self {
            name: "mock_executor".to_string(),
            version: "1.0.0".to_string(),
            stats: Arc::new(RwLock::new(ExecutorStats {
                executor: "mock_executor".to_string(),
                total_queries: 0,
                queries_per_minute: 0,
                total_execution_time_ms: 0,
                avg_execution_time_ms: 0.0,
                error_count: 0,
                last_execution_time: None,
                is_executing: false,
            })),
        }
    }
}

#[async_trait]
impl QueryExecutor for MockExecutor {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing Mock Executor");
        Ok(())
    }

    async fn execute(&self, query: ParsedQuery) -> BridgeResult<QueryResult> {
        let start_time = std::time::Instant::now();

        // Simulate some processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create mock result data
        let mock_data = vec![
            QueryRow {
                id: Uuid::new_v4(),
                data: HashMap::from([
                    ("service".to_string(), QueryValue::String("web".to_string())),
                    ("response_time".to_string(), QueryValue::Float(150.0)),
                    ("status".to_string(), QueryValue::String("200".to_string())),
                ]),
                metadata: HashMap::new(),
            },
            QueryRow {
                id: Uuid::new_v4(),
                data: HashMap::from([
                    ("service".to_string(), QueryValue::String("api".to_string())),
                    ("response_time".to_string(), QueryValue::Float(200.0)),
                    ("status".to_string(), QueryValue::String("200".to_string())),
                ]),
                metadata: HashMap::new(),
            },
        ];

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_queries += 1;
            stats.total_execution_time_ms += execution_time;
            stats.avg_execution_time_ms =
                stats.total_execution_time_ms as f64 / stats.total_queries as f64;
            stats.last_execution_time = Some(Utc::now());
        }

        let result = QueryResult {
            id: Uuid::new_v4(),
            query_id: query.id,
            status: ExecutionStatus::Success,
            data: mock_data,
            metadata: HashMap::from([
                ("executor".to_string(), self.name.clone()),
                ("query_type".to_string(), "mock".to_string()),
            ]),
            execution_time_ms: execution_time,
            execution_timestamp: Utc::now(),
        };

        Ok(result)
    }

    async fn get_stats(&self) -> BridgeResult<ExecutorStats> {
        Ok(self.stats.read().await.clone())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }
}

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::parsers::{ParsedQuery, QueryAst};

/// Executor configuration trait
#[async_trait]
pub trait ExecutorConfig: Send + Sync {
    /// Get executor name
    fn name(&self) -> &str;

    /// Get executor version
    fn version(&self) -> &str;

    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;

    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Query executor trait for executing queries
#[async_trait]
pub trait QueryExecutor: Send + Sync {
    /// Initialize the executor
    async fn init(&mut self) -> BridgeResult<()>;

    /// Execute query
    async fn execute(&self, query: ParsedQuery) -> BridgeResult<QueryResult>;

    /// Get executor statistics
    async fn get_stats(&self) -> BridgeResult<ExecutorStats>;

    /// Get executor name
    fn name(&self) -> &str;

    /// Get executor version
    fn version(&self) -> &str;
}

/// Query result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Result ID
    pub id: Uuid,

    /// Query ID
    pub query_id: Uuid,

    /// Execution status
    pub status: ExecutionStatus,

    /// Result data
    pub data: Vec<QueryRow>,

    /// Result metadata
    pub metadata: HashMap<String, String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Execution timestamp
    pub execution_timestamp: DateTime<Utc>,
}

/// Query row structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRow {
    /// Row ID
    pub id: Uuid,

    /// Row data
    pub data: HashMap<String, QueryValue>,

    /// Row metadata
    pub metadata: HashMap<String, String>,
}

/// Query value structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Array(Vec<QueryValue>),
    Object(HashMap<String, QueryValue>),
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    Error(String),
    Timeout,
    Cancelled,
}

/// Executor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorStats {
    /// Executor name
    pub executor: String,

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

    /// Executor status
    pub is_executing: bool,
}

/// Execution engine for managing query execution
pub struct ExecutionEngine {
    executors: HashMap<String, Box<dyn QueryExecutor>>,
    is_running: Arc<RwLock<bool>>,
}

impl ExecutionEngine {
    /// Create new execution engine
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add executor
    pub fn add_executor(&mut self, name: String, executor: Box<dyn QueryExecutor>) {
        self.executors.insert(name, executor);
    }

    /// Remove executor
    pub fn remove_executor(&mut self, name: &str) -> Option<Box<dyn QueryExecutor>> {
        self.executors.remove(name)
    }

    /// Get executor
    pub fn get_executor(&self, name: &str) -> Option<&dyn QueryExecutor> {
        self.executors.get(name).map(|e| e.as_ref())
    }

    /// Get all executor names
    pub fn get_executor_names(&self) -> Vec<String> {
        self.executors.keys().cloned().collect()
    }

    /// Execute query with specified executor
    pub async fn execute_query(
        &self,
        executor_name: &str,
        query: ParsedQuery,
    ) -> BridgeResult<QueryResult> {
        if let Some(executor) = self.get_executor(executor_name) {
            executor.execute(query).await
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "Executor not found: {}",
                executor_name
            )))
        }
    }

    /// Get executor statistics
    pub async fn get_stats(&self) -> BridgeResult<HashMap<String, ExecutorStats>> {
        let mut stats = HashMap::new();

        for (name, executor) in &self.executors {
            match executor.get_stats().await {
                Ok(executor_stats) => {
                    stats.insert(name.clone(), executor_stats);
                }
                Err(e) => {
                    error!("Failed to get stats for executor {}: {}", name, e);
                    return Err(e);
                }
            }
        }

        Ok(stats)
    }
}

/// Executor factory for creating executors
pub struct ExecutorFactory;

impl ExecutorFactory {
    /// Create an executor based on configuration
    pub async fn create_executor(
        config: &dyn ExecutorConfig,
    ) -> BridgeResult<Box<dyn QueryExecutor>> {
        match config.name() {
            "datafusion" => {
                // Create DataFusion executor
                let datafusion_config = datafusion_executor::DataFusionConfig::new();
                let mut executor = datafusion_executor::DataFusionExecutor::new(datafusion_config);
                executor.init().await?;
                Ok(Box::new(executor))
            }
            "sql" => {
                // Use DataFusion as the SQL executor
                let datafusion_config = datafusion_executor::DataFusionConfig::new();
                let mut executor = datafusion_executor::DataFusionExecutor::new(datafusion_config);
                executor.init().await?;
                Ok(Box::new(executor))
            }
            "query" => {
                // Use DataFusion as the generic query executor
                let datafusion_config = datafusion_executor::DataFusionConfig::new();
                let mut executor = datafusion_executor::DataFusionExecutor::new(datafusion_config);
                executor.init().await?;
                Ok(Box::new(executor))
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported executor: {}",
                config.name()
            ))),
        }
    }
}
