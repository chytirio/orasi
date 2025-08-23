//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Stream executor for streaming queries
//!
//! This module provides stream execution capabilities for processing
//! streaming queries with real-time data.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    StreamingQueryConfig, StreamingQueryResult, StreamingQueryStats, WindowInfo, WindowType,
};
use crate::executors::QueryExecutor;
use crate::parsers::ParsedQuery;

/// Stream executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamExecutorConfig {
    /// Executor name
    pub name: String,

    /// Executor version
    pub version: String,

    /// Maximum concurrent streams
    pub max_concurrent_streams: usize,

    /// Stream buffer size
    pub stream_buffer_size: usize,

    /// Enable backpressure handling
    pub enable_backpressure: bool,

    /// Backpressure threshold (percentage)
    pub backpressure_threshold: u8,

    /// Enable stream monitoring
    pub enable_monitoring: bool,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for StreamExecutorConfig {
    fn default() -> Self {
        Self {
            name: "stream_executor".to_string(),
            version: "1.0.0".to_string(),
            max_concurrent_streams: 100,
            stream_buffer_size: 10000,
            enable_backpressure: true,
            backpressure_threshold: 80,
            enable_monitoring: true,
            additional_config: HashMap::new(),
        }
    }
}

/// Stream query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamQueryResult {
    /// Query ID
    pub query_id: Uuid,

    /// Result timestamp
    pub timestamp: DateTime<Utc>,

    /// Result data
    pub data: Vec<StreamQueryRow>,

    /// Result metadata
    pub metadata: HashMap<String, String>,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Stream information
    pub stream_info: Option<StreamInfo>,
}

/// Stream query row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamQueryRow {
    /// Row ID
    pub id: Uuid,

    /// Row data
    pub data: HashMap<String, StreamQueryValue>,

    /// Row metadata
    pub metadata: HashMap<String, String>,

    /// Row timestamp
    pub timestamp: DateTime<Utc>,
}

/// Stream query value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamQueryValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Array(Vec<StreamQueryValue>),
    Object(HashMap<String, StreamQueryValue>),
    Timestamp(DateTime<Utc>),
}

/// Stream information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    /// Stream ID
    pub stream_id: Uuid,

    /// Stream name
    pub stream_name: String,

    /// Stream type
    pub stream_type: String,

    /// Window information
    pub window_info: Option<WindowInfo>,

    /// Stream metadata
    pub metadata: HashMap<String, String>,
}

/// Stream executor trait
#[async_trait]
pub trait StreamExecutor: Send + Sync {
    /// Initialize the executor
    async fn init(&mut self) -> BridgeResult<()>;

    /// Execute a streaming query
    async fn execute_stream_query(
        &self,
        query: ParsedQuery,
        config: &StreamingQueryConfig,
    ) -> BridgeResult<StreamQueryResult>;

    /// Get executor statistics
    async fn get_stats(&self) -> BridgeResult<StreamExecutorStats>;

    /// Get executor name
    fn name(&self) -> &str;

    /// Get executor version
    fn version(&self) -> &str;
}

/// Stream executor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamExecutorStats {
    /// Executor name
    pub executor: String,

    /// Total stream queries
    pub total_stream_queries: u64,

    /// Active stream queries
    pub active_stream_queries: u64,

    /// Stream queries per minute
    pub stream_queries_per_minute: u64,

    /// Total stream results
    pub total_stream_results: u64,

    /// Stream results per minute
    pub stream_results_per_minute: u64,

    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last stream query time
    pub last_stream_query_time: Option<DateTime<Utc>>,

    /// Last stream result time
    pub last_stream_result_time: Option<DateTime<Utc>>,
}

/// Default stream executor implementation
pub struct DefaultStreamExecutor {
    config: StreamExecutorConfig,
    query_executor: Arc<dyn QueryExecutor>,
    stats: Arc<RwLock<StreamExecutorStats>>,
    is_initialized: bool,
}

impl DefaultStreamExecutor {
    /// Create a new stream executor
    pub fn new(config: StreamExecutorConfig, query_executor: Arc<dyn QueryExecutor>) -> Self {
        let stats = StreamExecutorStats {
            executor: config.name.clone(),
            total_stream_queries: 0,
            active_stream_queries: 0,
            stream_queries_per_minute: 0,
            total_stream_results: 0,
            stream_results_per_minute: 0,
            avg_processing_time_ms: 0.0,
            error_count: 0,
            last_stream_query_time: None,
            last_stream_result_time: None,
        };

        Self {
            config,
            query_executor,
            stats: Arc::new(RwLock::new(stats)),
            is_initialized: false,
        }
    }
}

#[async_trait]
impl StreamExecutor for DefaultStreamExecutor {
    async fn init(&mut self) -> BridgeResult<()> {
        if self.is_initialized {
            return Ok(());
        }

        info!("Initializing stream executor: {}", self.config.name);

        // Initialize the underlying query executor
        // Note: This assumes the query executor is already initialized
        // In practice, you might want to initialize it here

        self.is_initialized = true;
        info!("Stream executor initialized: {}", self.config.name);
        Ok(())
    }

    async fn execute_stream_query(
        &self,
        query: ParsedQuery,
        config: &StreamingQueryConfig,
    ) -> BridgeResult<StreamQueryResult> {
        let start_time = std::time::Instant::now();

        info!("Executing stream query: {}", query.id);

        // Execute the query using the underlying query executor
        let query_result = self.query_executor.execute(query.clone()).await?;

        // Convert to stream query result
        let stream_result = StreamQueryResult {
            query_id: query_result.query_id,
            timestamp: Utc::now(),
            data: query_result
                .data
                .into_iter()
                .map(|row| StreamQueryRow {
                    id: row.id,
                    data: row
                        .data
                        .into_iter()
                        .map(|(k, v)| {
                            (
                                k,
                                match v {
                                    crate::executors::QueryValue::String(s) => {
                                        StreamQueryValue::String(s)
                                    }
                                    crate::executors::QueryValue::Integer(i) => {
                                        StreamQueryValue::Integer(i)
                                    }
                                    crate::executors::QueryValue::Float(f) => {
                                        StreamQueryValue::Float(f)
                                    }
                                    crate::executors::QueryValue::Boolean(b) => {
                                        StreamQueryValue::Boolean(b)
                                    }
                                    crate::executors::QueryValue::Null => StreamQueryValue::Null,
                                    crate::executors::QueryValue::Array(arr) => {
                                        StreamQueryValue::Array(
                                            arr.into_iter()
                                                .map(|v| match v {
                                                    crate::executors::QueryValue::String(s) => {
                                                        StreamQueryValue::String(s)
                                                    }
                                                    crate::executors::QueryValue::Integer(i) => {
                                                        StreamQueryValue::Integer(i)
                                                    }
                                                    crate::executors::QueryValue::Float(f) => {
                                                        StreamQueryValue::Float(f)
                                                    }
                                                    crate::executors::QueryValue::Boolean(b) => {
                                                        StreamQueryValue::Boolean(b)
                                                    }
                                                    crate::executors::QueryValue::Null => {
                                                        StreamQueryValue::Null
                                                    }
                                                    _ => StreamQueryValue::Null,
                                                })
                                                .collect(),
                                        )
                                    }
                                    crate::executors::QueryValue::Object(obj) => {
                                        StreamQueryValue::Object(
                                            obj.into_iter()
                                                .map(|(k, v)| {
                                                    (
                                                        k,
                                                        match v {
                                                            crate::executors::QueryValue::String(s) => {
                                                                StreamQueryValue::String(s)
                                                            }
                                                            crate::executors::QueryValue::Integer(i) => {
                                                                StreamQueryValue::Integer(i)
                                                            }
                                                            crate::executors::QueryValue::Float(f) => {
                                                                StreamQueryValue::Float(f)
                                                            }
                                                            crate::executors::QueryValue::Boolean(b) => {
                                                                StreamQueryValue::Boolean(b)
                                                            }
                                                            crate::executors::QueryValue::Null => {
                                                                StreamQueryValue::Null
                                                            }
                                                            _ => StreamQueryValue::Null,
                                                        },
                                                    )
                                                })
                                                .collect(),
                                        )
                                    }
                                },
                            )
                        })
                        .collect(),
                    metadata: row.metadata,
                    timestamp: Utc::now(),
                })
                .collect(),
            metadata: query_result.metadata,
            processing_time_ms: query_result.execution_time_ms,
            stream_info: Some(StreamInfo {
                stream_id: Uuid::new_v4(),
                stream_name: format!("stream_{}", query.id),
                stream_type: "continuous".to_string(),
                window_info: None, // Could be populated based on query analysis
                metadata: HashMap::new(),
            }),
        };

        let processing_time = start_time.elapsed().as_millis() as u64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_stream_queries += 1;
            stats.total_stream_results += stream_result.data.len() as u64;
            stats.avg_processing_time_ms = (stats.avg_processing_time_ms
                * (stats.total_stream_queries - 1) as f64
                + processing_time as f64)
                / stats.total_stream_queries as f64;
            stats.last_stream_query_time = Some(Utc::now());
            stats.last_stream_result_time = Some(Utc::now());
        }

        info!("Stream query executed in {}ms", processing_time);
        Ok(stream_result)
    }

    async fn get_stats(&self) -> BridgeResult<StreamExecutorStats> {
        Ok(self.stats.read().await.clone())
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn version(&self) -> &str {
        &self.config.version
    }
}
