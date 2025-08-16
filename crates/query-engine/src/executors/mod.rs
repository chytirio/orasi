//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query executors for the bridge
//!
//! This module provides executor implementations for query execution
//! including query execution engines and result handling.

pub mod datafusion_executor;

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
