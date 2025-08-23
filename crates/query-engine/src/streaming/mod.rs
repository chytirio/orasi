//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming queries for real-time data processing
//!
//! This module provides streaming query capabilities that enable
//! real-time data processing and continuous query execution.

pub mod continuous_queries;
pub mod stream_executor;
pub mod stream_optimizer;
pub mod stream_parser;
pub mod stream_processor;
pub mod windowing;

// Re-export main types
pub use continuous_queries::{ContinuousQuery, ContinuousQueryConfig, ContinuousQueryManager};
pub use stream_executor::{StreamExecutor, StreamExecutorConfig, StreamQueryResult};
pub use stream_optimizer::{StreamOptimizer, StreamOptimizerConfig};
pub use stream_parser::{StreamQueryParser, StreamQueryParserConfig};
pub use stream_processor::{StreamProcessor, StreamProcessorConfig};
pub use windowing::{TimeWindow, WindowConfig, WindowManager, WindowType};

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Streaming query configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingQueryConfig {
    /// Enable streaming queries
    pub enable_streaming: bool,

    /// Maximum number of concurrent streaming queries
    pub max_concurrent_queries: usize,

    /// Default window size in milliseconds
    pub default_window_ms: u64,

    /// Enable backpressure handling
    pub enable_backpressure: bool,

    /// Backpressure threshold (percentage)
    pub backpressure_threshold: u8,

    /// Enable query result caching
    pub enable_caching: bool,

    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,

    /// Enable query result streaming
    pub enable_result_streaming: bool,

    /// Maximum result buffer size
    pub max_result_buffer_size: usize,
}

impl Default for StreamingQueryConfig {
    fn default() -> Self {
        Self {
            enable_streaming: true,
            max_concurrent_queries: 100,
            default_window_ms: 60000, // 1 minute
            enable_backpressure: true,
            backpressure_threshold: 80,
            enable_caching: true,
            cache_ttl_secs: 300, // 5 minutes
            enable_result_streaming: true,
            max_result_buffer_size: 10000,
        }
    }
}

/// Streaming query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingQueryResult {
    /// Query ID
    pub query_id: Uuid,

    /// Result timestamp
    pub timestamp: DateTime<Utc>,

    /// Result data
    pub data: Vec<StreamingQueryRow>,

    /// Result metadata
    pub metadata: HashMap<String, String>,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Window information
    pub window_info: Option<WindowInfo>,
}

/// Streaming query row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingQueryRow {
    /// Row ID
    pub id: Uuid,

    /// Row data
    pub data: HashMap<String, StreamingQueryValue>,

    /// Row metadata
    pub metadata: HashMap<String, String>,

    /// Row timestamp
    pub timestamp: DateTime<Utc>,
}

/// Streaming query value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamingQueryValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Array(Vec<StreamingQueryValue>),
    Object(HashMap<String, StreamingQueryValue>),
    Timestamp(DateTime<Utc>),
}

/// Window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    /// Window start time
    pub start_time: DateTime<Utc>,

    /// Window end time
    pub end_time: DateTime<Utc>,

    /// Window type
    pub window_type: WindowType,

    /// Window size in milliseconds
    pub window_size_ms: u64,
}

/// Streaming query statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingQueryStats {
    /// Total streaming queries
    pub total_queries: u64,

    /// Active streaming queries
    pub active_queries: u64,

    /// Queries per minute
    pub queries_per_minute: u64,

    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,

    /// Total results generated
    pub total_results: u64,

    /// Results per minute
    pub results_per_minute: u64,

    /// Error count
    pub error_count: u64,

    /// Last query time
    pub last_query_time: Option<DateTime<Utc>>,

    /// Last result time
    pub last_result_time: Option<DateTime<Utc>>,
}

/// Streaming query trait
#[async_trait]
pub trait StreamingQuery: Send + Sync {
    /// Get query ID
    fn id(&self) -> Uuid;

    /// Get query name
    fn name(&self) -> &str;

    /// Get query SQL
    fn sql(&self) -> &str;

    /// Get query configuration
    fn config(&self) -> &StreamingQueryConfig;

    /// Execute streaming query
    async fn execute(&self) -> BridgeResult<StreamingQueryResult>;

    /// Stop streaming query
    async fn stop(&self) -> BridgeResult<()>;

    /// Get query statistics
    async fn get_stats(&self) -> BridgeResult<StreamingQueryStats>;

    /// Check if query is active
    fn is_active(&self) -> bool;
}

/// Streaming query manager trait
#[async_trait]
pub trait StreamingQueryManager: Send + Sync {
    /// Register a streaming query
    async fn register_query(&mut self, query: Box<dyn StreamingQuery>) -> BridgeResult<()>;

    /// Unregister a streaming query
    async fn unregister_query(&mut self, query_id: Uuid) -> BridgeResult<()>;

    /// Get all active queries
    async fn get_active_queries(&self) -> BridgeResult<Vec<Box<dyn StreamingQuery>>>;

    /// Get query by ID
    async fn get_query(&self, query_id: Uuid) -> BridgeResult<Option<Box<dyn StreamingQuery>>>;

    /// Get manager statistics
    async fn get_stats(&self) -> BridgeResult<StreamingQueryStats>;

    /// Stop all queries
    async fn stop_all(&mut self) -> BridgeResult<()>;
}
