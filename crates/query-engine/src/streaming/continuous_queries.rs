//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Continuous queries for real-time data processing
//!
//! This module provides continuous query capabilities that enable
//! long-running queries that process data as it arrives.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    StreamingQuery, StreamingQueryConfig, StreamingQueryManager, StreamingQueryResult,
    StreamingQueryRow, StreamingQueryStats, StreamingQueryValue, WindowInfo, WindowType,
};
use crate::executors::QueryExecutor;
use crate::parsers::ParsedQuery;

/// Continuous query configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuousQueryConfig {
    /// Query name
    pub name: String,

    /// Query SQL
    pub sql: String,

    /// Execution interval in milliseconds
    pub execution_interval_ms: u64,

    /// Window configuration
    pub window_config: Option<WindowConfig>,

    /// Enable result streaming
    pub enable_result_streaming: bool,

    /// Result stream buffer size
    pub result_buffer_size: usize,

    /// Maximum execution time in seconds
    pub max_execution_time_secs: u64,

    /// Enable query optimization
    pub enable_optimization: bool,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window type
    pub window_type: WindowType,

    /// Window size in milliseconds
    pub window_size_ms: u64,

    /// Window slide in milliseconds
    pub window_slide_ms: u64,

    /// Enable window watermarking
    pub enable_watermarking: bool,

    /// Watermark delay in milliseconds
    pub watermark_delay_ms: u64,
}

/// Continuous query implementation
pub struct ContinuousQuery {
    id: Uuid,
    config: ContinuousQueryConfig,
    streaming_config: StreamingQueryConfig,
    executor: Arc<dyn QueryExecutor>,
    is_active: Arc<RwLock<bool>>,
    stats: Arc<RwLock<StreamingQueryStats>>,
    result_sender: Option<mpsc::Sender<StreamingQueryResult>>,
    result_receiver: Option<mpsc::Receiver<StreamingQueryResult>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ContinuousQuery {
    /// Create a new continuous query
    pub fn new(
        config: ContinuousQueryConfig,
        streaming_config: StreamingQueryConfig,
        executor: Arc<dyn QueryExecutor>,
    ) -> Self {
        let (result_sender, result_receiver) = if config.enable_result_streaming {
            let (tx, rx) = mpsc::channel(config.result_buffer_size);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        let stats = StreamingQueryStats {
            total_queries: 0,
            active_queries: 0,
            queries_per_minute: 0,
            avg_processing_time_ms: 0.0,
            total_results: 0,
            results_per_minute: 0,
            error_count: 0,
            last_query_time: None,
            last_result_time: None,
        };

        Self {
            id: Uuid::new_v4(),
            config,
            streaming_config,
            executor,
            is_active: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            result_sender,
            result_receiver,
            task_handle: None,
        }
    }

    /// Start the continuous query
    pub async fn start(&mut self) -> BridgeResult<()> {
        {
            let mut is_active = self.is_active.write().await;
            if *is_active {
                return Ok(());
            }
            *is_active = true;
        }

        info!("Starting continuous query: {}", self.config.name);

        let id = self.id;
        let config = self.config.clone();
        let streaming_config = self.streaming_config.clone();
        let executor = self.executor.clone();
        let is_active = self.is_active.clone();
        let stats = self.stats.clone();
        let result_sender = self.result_sender.clone();

        let task_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(config.execution_interval_ms));

            while {
                let active = is_active.read().await;
                *active
            } {
                interval.tick().await;

                if let Err(e) = Self::execute_query_cycle(
                    &config,
                    &streaming_config,
                    &executor,
                    &stats,
                    &result_sender,
                )
                .await
                {
                    error!("Error executing continuous query {}: {}", id, e);

                    {
                        let mut stats = stats.write().await;
                        stats.error_count += 1;
                    }
                }
            }
        });

        self.task_handle = Some(task_handle);

        info!("Continuous query started: {}", self.config.name);
        Ok(())
    }

    /// Execute a single query cycle
    async fn execute_query_cycle(
        config: &ContinuousQueryConfig,
        streaming_config: &StreamingQueryConfig,
        executor: &Arc<dyn QueryExecutor>,
        stats: &Arc<RwLock<StreamingQueryStats>>,
        result_sender: &Option<mpsc::Sender<StreamingQueryResult>>,
    ) -> BridgeResult<()> {
        let start_time = std::time::Instant::now();

        // Parse the query
        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: config.sql.clone(),
            ast: crate::parsers::QueryAst {
                root: crate::parsers::AstNode {
                    node_type: crate::parsers::NodeType::Statement,
                    value: Some(config.sql.clone()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                node_count: 1,
                depth: 1,
            },
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Execute the query
        let query_result = executor.execute(parsed_query).await?;

        // Convert to streaming result
        let streaming_result = StreamingQueryResult {
            query_id: query_result.query_id,
            timestamp: Utc::now(),
            data: query_result
                .data
                .into_iter()
                .map(|row| {
                    // Convert to StreamingQueryRow
                    StreamingQueryRow {
                        id: Uuid::new_v4(),
                        data: row.data.into_iter().map(|(k, v)| {
                            (k, match v {
                                crate::executors::QueryValue::String(s) => StreamingQueryValue::String(s),
                                crate::executors::QueryValue::Integer(i) => StreamingQueryValue::Integer(i),
                                crate::executors::QueryValue::Float(f) => StreamingQueryValue::Float(f),
                                crate::executors::QueryValue::Boolean(b) => StreamingQueryValue::Boolean(b),
                                crate::executors::QueryValue::Null => StreamingQueryValue::Null,
                                crate::executors::QueryValue::Array(arr) => {
                                    StreamingQueryValue::Array(arr.into_iter().map(|v| {
                                        match v {
                                            crate::executors::QueryValue::String(s) => StreamingQueryValue::String(s),
                                            crate::executors::QueryValue::Integer(i) => StreamingQueryValue::Integer(i),
                                            crate::executors::QueryValue::Float(f) => StreamingQueryValue::Float(f),
                                            crate::executors::QueryValue::Boolean(b) => StreamingQueryValue::Boolean(b),
                                            crate::executors::QueryValue::Null => StreamingQueryValue::Null,
                                            _ => StreamingQueryValue::String(format!("{:?}", v)),
                                        }
                                    }).collect())
                                }
                                crate::executors::QueryValue::Object(obj) => {
                                    StreamingQueryValue::Object(obj.into_iter().map(|(k, v)| {
                                        (k, match v {
                                            crate::executors::QueryValue::String(s) => StreamingQueryValue::String(s),
                                            crate::executors::QueryValue::Integer(i) => StreamingQueryValue::Integer(i),
                                            crate::executors::QueryValue::Float(f) => StreamingQueryValue::Float(f),
                                            crate::executors::QueryValue::Boolean(b) => StreamingQueryValue::Boolean(b),
                                            crate::executors::QueryValue::Null => StreamingQueryValue::Null,
                                            _ => StreamingQueryValue::String(format!("{:?}", v)),
                                        })
                                    }).collect())
                                }
                            })
                        }).collect(),
                        metadata: HashMap::new(),
                        timestamp: Utc::now(),
                    }
                })
                .collect::<Vec<StreamingQueryRow>>(),
            metadata: query_result.metadata,
            processing_time_ms: query_result.execution_time_ms,
            window_info: config.window_config.as_ref().map(|wc| WindowInfo {
                start_time: Utc::now() - chrono::Duration::milliseconds(wc.window_size_ms as i64),
                end_time: Utc::now(),
                window_type: wc.window_type.clone(),
                window_size_ms: wc.window_size_ms,
            }),
        };

        let processing_time = start_time.elapsed().as_millis() as u64;

        // Update statistics
        {
            let mut stats = stats.write().await;
            stats.total_queries += 1;
            stats.total_results += streaming_result.data.len() as u64;
            stats.avg_processing_time_ms = (stats.avg_processing_time_ms
                * (stats.total_queries - 1) as f64
                + processing_time as f64)
                / stats.total_queries as f64;
            stats.last_query_time = Some(Utc::now());
            stats.last_result_time = Some(Utc::now());
        }

        // Send result if streaming is enabled
        if let Some(sender) = result_sender {
            if let Err(e) = sender.send(streaming_result).await {
                error!("Failed to send streaming result: {}", e);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl StreamingQuery for ContinuousQuery {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn sql(&self) -> &str {
        &self.config.sql
    }

    fn config(&self) -> &StreamingQueryConfig {
        &self.streaming_config
    }

    async fn execute(&self) -> BridgeResult<StreamingQueryResult> {
        // For continuous queries, we return the last result or create a placeholder
        // The actual execution happens in the background task
        let result_data = vec![]; // Placeholder for now
        Ok(StreamingQueryResult {
            query_id: self.id,
            data: result_data,
            metadata: HashMap::new(),
            processing_time_ms: 0,
            timestamp: Utc::now(),
            window_info: None,
        })
    }

    async fn stop(&self) -> BridgeResult<()> {
        {
            let mut is_active = self.is_active.write().await;
            *is_active = false;
        }

        if let Some(task_handle) = &self.task_handle {
            task_handle.abort();
        }

        info!("Continuous query stopped: {}", self.config.name);
        Ok(())
    }

    async fn get_stats(&self) -> BridgeResult<StreamingQueryStats> {
        Ok(self.stats.read().await.clone())
    }

    fn is_active(&self) -> bool {
        // This is a simplified check - in practice, we'd want to check the task status
        true
    }
}

/// Continuous query manager implementation
pub struct ContinuousQueryManager {
    queries: Arc<RwLock<HashMap<Uuid, Box<dyn StreamingQuery>>>>,
    stats: Arc<RwLock<StreamingQueryStats>>,
}

impl ContinuousQueryManager {
    /// Create a new continuous query manager
    pub fn new() -> Self {
        let stats = StreamingQueryStats {
            total_queries: 0,
            active_queries: 0,
            queries_per_minute: 0,
            avg_processing_time_ms: 0.0,
            total_results: 0,
            results_per_minute: 0,
            error_count: 0,
            last_query_time: None,
            last_result_time: None,
        };

        Self {
            queries: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(stats)),
        }
    }
}

#[async_trait]
impl StreamingQueryManager for ContinuousQueryManager {
    async fn register_query(&mut self, query: Box<dyn StreamingQuery>) -> BridgeResult<()> {
        let query_id = query.id();
        let query_name = query.name().to_string();

        {
            let mut queries = self.queries.write().await;
            queries.insert(query_id, query);
        }

        {
            let mut stats = self.stats.write().await;
            stats.total_queries += 1;
            stats.active_queries += 1;
        }

        info!("Registered continuous query: {} ({})", query_name, query_id);
        Ok(())
    }

    async fn unregister_query(&mut self, query_id: Uuid) -> BridgeResult<()> {
        let query_name = {
            let queries = self.queries.read().await;
            queries
                .get(&query_id)
                .map(|q| q.name().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        };

        {
            let mut queries = self.queries.write().await;
            if let Some(query) = queries.remove(&query_id) {
                query.stop().await?;
            }
        }

        {
            let mut stats = self.stats.write().await;
            stats.active_queries = stats.active_queries.saturating_sub(1);
        }

        info!(
            "Unregistered continuous query: {} ({})",
            query_name, query_id
        );
        Ok(())
    }

    async fn get_active_queries(&self) -> BridgeResult<Vec<Box<dyn StreamingQuery>>> {
        let queries = self.queries.read().await;
        // Since we can't clone Box<dyn StreamingQuery>, we'll return empty for now
        // In a real implementation, you might want to return query metadata instead
        Ok(vec![])
    }

    async fn get_query(&self, query_id: Uuid) -> BridgeResult<Option<Box<dyn StreamingQuery>>> {
        let queries = self.queries.read().await;
        // Since we can't clone Box<dyn StreamingQuery>, we'll return None for now
        // In a real implementation, you might want to return query metadata instead
        Ok(None)
    }

    async fn get_stats(&self) -> BridgeResult<StreamingQueryStats> {
        Ok(self.stats.read().await.clone())
    }

    async fn stop_all(&mut self) -> BridgeResult<()> {
        let query_ids: Vec<Uuid> = {
            let queries = self.queries.read().await;
            queries.keys().cloned().collect()
        };

        for query_id in query_ids {
            self.unregister_query(query_id).await?;
        }

        info!("Stopped all continuous queries");
        Ok(())
    }
}
