//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Stream parser for streaming queries
//!
//! This module provides parsing capabilities for streaming queries.

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
    StreamingQueryConfig, StreamingQueryResult, StreamingQueryRow, StreamingQueryStats,
    StreamingQueryValue, WindowInfo, WindowType,
};
use crate::parsers::{AstNode, NodeType, ParsedQuery, QueryAst, QueryType, ValidationResult};

/// Stream query parser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamQueryParserConfig {
    /// Parser name
    pub name: String,

    /// Parser version
    pub version: String,

    /// Enable parsing
    pub enable_parsing: bool,

    /// Enable streaming-specific parsing
    pub enable_streaming_parsing: bool,

    /// Enable window parsing
    pub enable_window_parsing: bool,

    /// Enable continuous query parsing
    pub enable_continuous_parsing: bool,

    /// Enable backpressure parsing
    pub enable_backpressure_parsing: bool,

    /// Maximum query length
    pub max_query_length: usize,

    /// Enable query validation
    pub enable_validation: bool,

    /// Enable query optimization hints
    pub enable_optimization_hints: bool,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for StreamQueryParserConfig {
    fn default() -> Self {
        Self {
            name: "stream_query_parser".to_string(),
            version: "1.0.0".to_string(),
            enable_parsing: true,
            enable_streaming_parsing: true,
            enable_window_parsing: true,
            enable_continuous_parsing: true,
            enable_backpressure_parsing: true,
            max_query_length: 10000,
            enable_validation: true,
            enable_optimization_hints: true,
            additional_config: HashMap::new(),
        }
    }
}

/// Stream parsing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamParsingStats {
    /// Total queries parsed
    pub total_queries: u64,

    /// Successful parses
    pub successful_parses: u64,

    /// Failed parses
    pub failed_parses: u64,

    /// Average parse time in milliseconds
    pub avg_parse_time_ms: f64,

    /// Total parse time in milliseconds
    pub total_parse_time_ms: u64,

    /// Streaming queries parsed
    pub streaming_queries: u64,

    /// Window queries parsed
    pub window_queries: u64,

    /// Continuous queries parsed
    pub continuous_queries: u64,

    /// Last parse timestamp
    pub last_parse_time: Option<DateTime<Utc>>,
}

/// Stream query parsing result
#[derive(Debug, Clone)]
pub struct StreamParsingResult {
    /// Parsed query
    pub parsed_query: ParsedQuery,

    /// Streaming configuration
    pub streaming_config: Option<StreamingQueryConfig>,

    /// Window configuration
    pub window_config: Option<WindowConfig>,

    /// Continuous query configuration
    pub continuous_config: Option<ContinuousQueryConfig>,

    /// Backpressure configuration
    pub backpressure_config: Option<BackpressureConfig>,

    /// Parsing statistics
    pub stats: StreamParsingStats,

    /// Parsing metadata
    pub metadata: HashMap<String, String>,
}

/// Window configuration for streaming queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window type
    pub window_type: WindowType,

    /// Window size in milliseconds
    pub window_size_ms: u64,

    /// Window slide in milliseconds
    pub window_slide_ms: Option<u64>,

    /// Session timeout in milliseconds
    pub session_timeout_ms: Option<u64>,

    /// Enable watermarking
    pub enable_watermarking: bool,

    /// Watermark delay in milliseconds
    pub watermark_delay_ms: Option<u64>,
}

/// Continuous query configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuousQueryConfig {
    /// Execution interval in milliseconds
    pub execution_interval_ms: u64,

    /// Maximum execution time in seconds
    pub max_execution_time_secs: u64,

    /// Enable result streaming
    pub enable_result_streaming: bool,

    /// Result buffer size
    pub result_buffer_size: usize,

    /// Enable query optimization
    pub enable_optimization: bool,
}

/// Backpressure configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureConfig {
    /// Enable backpressure handling
    pub enable_backpressure: bool,

    /// Backpressure threshold (percentage)
    pub backpressure_threshold: u8,

    /// Backpressure strategy
    pub strategy: BackpressureStrategy,

    /// Buffer size
    pub buffer_size: usize,
}

/// Backpressure strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackpressureStrategy {
    DropOldest,
    DropNewest,
    Block,
    Throttle,
}

/// Stream query parser trait
#[async_trait]
pub trait StreamQueryParser: Send + Sync {
    /// Parse a streaming query
    async fn parse(&self, query_text: &str) -> BridgeResult<StreamParsingResult>;

    /// Parse with streaming configuration
    async fn parse_with_config(
        &self,
        query_text: &str,
        config: &StreamingQueryConfig,
    ) -> BridgeResult<StreamParsingResult>;

    /// Validate parsed streaming query
    async fn validate(&self, result: &StreamParsingResult) -> BridgeResult<ValidationResult>;

    /// Get parser statistics
    async fn get_stats(&self) -> BridgeResult<StreamParsingStats>;

    /// Start parser
    async fn start(&mut self) -> BridgeResult<()>;

    /// Stop parser
    async fn stop(&mut self) -> BridgeResult<()>;

    /// Check if parser is running
    fn is_running(&self) -> bool;
}

/// Default stream query parser implementation
pub struct DefaultStreamQueryParser {
    config: StreamQueryParserConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<StreamParsingStats>>,
    start_time: Option<DateTime<Utc>>,
}

impl DefaultStreamQueryParser {
    /// Create a new stream query parser
    pub fn new(config: StreamQueryParserConfig) -> Self {
        Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(StreamParsingStats {
                total_queries: 0,
                successful_parses: 0,
                failed_parses: 0,
                avg_parse_time_ms: 0.0,
                total_parse_time_ms: 0,
                streaming_queries: 0,
                window_queries: 0,
                continuous_queries: 0,
                last_parse_time: None,
            })),
            start_time: None,
        }
    }

    /// Parse query text into AST
    async fn parse_query_ast(&self, query_text: &str) -> BridgeResult<QueryAst> {
        let start_time = std::time::Instant::now();

        info!("Parsing stream query: {}", query_text);

        // Basic query parsing - create AST from query text
        let root = self.create_ast_from_query(query_text)?;

        let ast = QueryAst {
            root: root.clone(),
            node_count: self.count_nodes(&root),
            depth: self.calculate_depth(&root),
        };

        let parse_time = start_time.elapsed().as_millis() as u64;
        info!("Query AST created in {}ms", parse_time);

        Ok(ast)
    }

    /// Create AST from query text
    fn create_ast_from_query(&self, query_text: &str) -> BridgeResult<AstNode> {
        // Simple parsing logic - in a real implementation, this would use a proper parser
        let query_lower = query_text.to_lowercase();

        let mut children = Vec::new();

        // Parse SELECT clause
        if query_lower.contains("select") {
            children.push(AstNode {
                node_type: NodeType::Select,
                value: None,
                children: self.parse_select_clause(query_text)?,
                metadata: HashMap::new(),
            });
        }

        // Parse FROM clause
        if query_lower.contains("from") {
            children.push(AstNode {
                node_type: NodeType::From,
                value: None,
                children: self.parse_from_clause(query_text)?,
                metadata: HashMap::new(),
            });
        }

        // Parse WHERE clause
        if query_lower.contains("where") {
            children.push(AstNode {
                node_type: NodeType::Where,
                value: None,
                children: self.parse_where_clause(query_text)?,
                metadata: HashMap::new(),
            });
        }

        // Parse GROUP BY clause
        if query_lower.contains("group by") {
            children.push(AstNode {
                node_type: NodeType::GroupBy,
                value: None,
                children: self.parse_group_by_clause(query_text)?,
                metadata: HashMap::new(),
            });
        }

        // Parse ORDER BY clause
        if query_lower.contains("order by") {
            children.push(AstNode {
                node_type: NodeType::OrderBy,
                value: None,
                children: self.parse_order_by_clause(query_text)?,
                metadata: HashMap::new(),
            });
        }

        Ok(AstNode {
            node_type: NodeType::Statement,
            value: Some(query_text.to_string()),
            children,
            metadata: HashMap::new(),
        })
    }

    /// Parse SELECT clause
    fn parse_select_clause(&self, query_text: &str) -> BridgeResult<Vec<AstNode>> {
        let mut children = Vec::new();

        // Simple parsing - extract columns from SELECT
        if let Some(select_part) = query_text.split("from").next() {
            if let Some(columns_part) = select_part.split("select").nth(1) {
                let columns = columns_part.trim().split(',').collect::<Vec<_>>();
                for column in columns {
                    children.push(AstNode {
                        node_type: NodeType::Identifier,
                        value: Some(column.trim().to_string()),
                        children: vec![],
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Ok(children)
    }

    /// Parse FROM clause
    fn parse_from_clause(&self, query_text: &str) -> BridgeResult<Vec<AstNode>> {
        let mut children = Vec::new();

        // Simple parsing - extract table from FROM
        if let Some(from_part) = query_text.split("where").next() {
            if let Some(table_part) = from_part.split("from").nth(1) {
                let table = table_part
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("table");
                children.push(AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(table.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(children)
    }

    /// Parse WHERE clause
    fn parse_where_clause(&self, query_text: &str) -> BridgeResult<Vec<AstNode>> {
        let mut children = Vec::new();

        // Simple parsing - extract conditions from WHERE
        if let Some(where_part) = query_text.split("group by").next() {
            if let Some(condition_part) = where_part.split("where").nth(1) {
                let conditions = condition_part.trim().split("and").collect::<Vec<_>>();
                for condition in conditions {
                    children.push(AstNode {
                        node_type: NodeType::Expression,
                        value: Some(condition.trim().to_string()),
                        children: vec![],
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Ok(children)
    }

    /// Parse GROUP BY clause
    fn parse_group_by_clause(&self, query_text: &str) -> BridgeResult<Vec<AstNode>> {
        let mut children = Vec::new();

        // Simple parsing - extract grouping columns
        if let Some(group_part) = query_text.split("order by").next() {
            if let Some(columns_part) = group_part.split("group by").nth(1) {
                let columns = columns_part.trim().split(',').collect::<Vec<_>>();
                for column in columns {
                    children.push(AstNode {
                        node_type: NodeType::Identifier,
                        value: Some(column.trim().to_string()),
                        children: vec![],
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Ok(children)
    }

    /// Parse ORDER BY clause
    fn parse_order_by_clause(&self, query_text: &str) -> BridgeResult<Vec<AstNode>> {
        let mut children = Vec::new();

        // Simple parsing - extract ordering columns
        if let Some(columns_part) = query_text.split("order by").nth(1) {
            let columns = columns_part.trim().split(',').collect::<Vec<_>>();
            for column in columns {
                children.push(AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(column.trim().to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(children)
    }

    /// Count nodes in AST
    fn count_nodes(&self, node: &AstNode) -> usize {
        1 + node
            .children
            .iter()
            .map(|child| self.count_nodes(child))
            .sum::<usize>()
    }

    /// Calculate depth of AST
    fn calculate_depth(&self, node: &AstNode) -> usize {
        1 + node
            .children
            .iter()
            .map(|child| self.calculate_depth(child))
            .max()
            .unwrap_or(0)
    }

    /// Extract streaming configuration from query
    fn extract_streaming_config(&self, query_text: &str) -> Option<StreamingQueryConfig> {
        let query_lower = query_text.to_lowercase();

        // Check for streaming keywords
        if query_lower.contains("stream") || query_lower.contains("continuous") {
            Some(StreamingQueryConfig {
                enable_streaming: true,
                max_concurrent_queries: 100,
                default_window_ms: 60000,
                enable_backpressure: true,
                backpressure_threshold: 80,
                enable_caching: true,
                cache_ttl_secs: 300,
                enable_result_streaming: true,
                max_result_buffer_size: 10000,
            })
        } else {
            None
        }
    }

    /// Extract window configuration from query
    fn extract_window_config(&self, query_text: &str) -> Option<WindowConfig> {
        let query_lower = query_text.to_lowercase();

        // Check for window keywords
        if query_lower.contains("window")
            || query_lower.contains("tumble")
            || query_lower.contains("hop")
        {
            let window_type = if query_lower.contains("tumble") {
                WindowType::Tumbling
            } else if query_lower.contains("hop") {
                WindowType::Sliding
            } else {
                WindowType::Time
            };

            Some(WindowConfig {
                window_type,
                window_size_ms: 60000, // Default 1 minute
                window_slide_ms: None,
                session_timeout_ms: None,
                enable_watermarking: true,
                watermark_delay_ms: Some(5000), // Default 5 seconds
            })
        } else {
            None
        }
    }

    /// Extract continuous query configuration from query
    fn extract_continuous_config(&self, query_text: &str) -> Option<ContinuousQueryConfig> {
        let query_lower = query_text.to_lowercase();

        // Check for continuous query keywords
        if query_lower.contains("continuous") || query_lower.contains("every") {
            Some(ContinuousQueryConfig {
                execution_interval_ms: 60000, // Default 1 minute
                max_execution_time_secs: 300, // Default 5 minutes
                enable_result_streaming: true,
                result_buffer_size: 1000,
                enable_optimization: true,
            })
        } else {
            None
        }
    }

    /// Extract backpressure configuration from query
    fn extract_backpressure_config(&self, query_text: &str) -> Option<BackpressureConfig> {
        let query_lower = query_text.to_lowercase();

        // Check for backpressure keywords
        if query_lower.contains("backpressure") || query_lower.contains("buffer") {
            Some(BackpressureConfig {
                enable_backpressure: true,
                backpressure_threshold: 80,
                strategy: BackpressureStrategy::DropOldest,
                buffer_size: 1000,
            })
        } else {
            None
        }
    }

    /// Update parsing statistics
    async fn update_stats(
        &self,
        parse_time_ms: u64,
        is_streaming: bool,
        has_window: bool,
        is_continuous: bool,
    ) {
        let mut stats = self.stats.write().await;

        stats.total_queries += 1;
        stats.successful_parses += 1;
        stats.total_parse_time_ms += parse_time_ms;
        stats.last_parse_time = Some(Utc::now());

        // Calculate average parse time
        if stats.successful_parses > 0 {
            stats.avg_parse_time_ms =
                stats.total_parse_time_ms as f64 / stats.successful_parses as f64;
        }

        // Update specific counters
        if is_streaming {
            stats.streaming_queries += 1;
        }
        if has_window {
            stats.window_queries += 1;
        }
        if is_continuous {
            stats.continuous_queries += 1;
        }
    }

    /// Validate streaming query
    async fn validate_streaming_query(
        &self,
        result: &StreamParsingResult,
    ) -> BridgeResult<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic validation
        if result.parsed_query.query_text.is_empty() {
            errors.push(crate::parsers::ValidationError {
                code: "EMPTY_QUERY".to_string(),
                message: "Query text cannot be empty".to_string(),
                location: crate::parsers::ErrorLocation {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
                severity: crate::parsers::ValidationSeverity::Error,
            });
        }

        // Check query length
        if result.parsed_query.query_text.len() > self.config.max_query_length {
            errors.push(crate::parsers::ValidationError {
                code: "QUERY_TOO_LONG".to_string(),
                message: format!(
                    "Query exceeds maximum length of {}",
                    self.config.max_query_length
                ),
                location: crate::parsers::ErrorLocation {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
                severity: crate::parsers::ValidationSeverity::Error,
            });
        }

        // Validate streaming configuration
        if let Some(streaming_config) = &result.streaming_config {
            if streaming_config.max_concurrent_queries == 0 {
                warnings.push(crate::parsers::ValidationWarning {
                    code: "ZERO_CONCURRENT_QUERIES".to_string(),
                    message: "Maximum concurrent queries is set to 0".to_string(),
                    location: None,
                    severity: crate::parsers::ValidationSeverity::Warning,
                });
            }
        }

        // Validate window configuration
        if let Some(window_config) = &result.window_config {
            if window_config.window_size_ms == 0 {
                errors.push(crate::parsers::ValidationError {
                    code: "ZERO_WINDOW_SIZE".to_string(),
                    message: "Window size cannot be zero".to_string(),
                    location: crate::parsers::ErrorLocation {
                        line: 1,
                        column: 1,
                        offset: 0,
                    },
                    severity: crate::parsers::ValidationSeverity::Error,
                });
            }
        }

        Ok(crate::parsers::ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }
}

#[async_trait]
impl StreamQueryParser for DefaultStreamQueryParser {
    async fn parse(&self, query_text: &str) -> BridgeResult<StreamParsingResult> {
        if !self.is_running() {
            return Err(bridge_core::BridgeError::query(
                "Stream parser is not running".to_string(),
            ));
        }

        let start_time = std::time::Instant::now();

        // Parse query into AST
        let ast = self.parse_query_ast(query_text).await?;

        // Extract streaming-specific configurations
        let streaming_config = self.extract_streaming_config(query_text);
        let window_config = self.extract_window_config(query_text);
        let continuous_config = self.extract_continuous_config(query_text);
        let backpressure_config = self.extract_backpressure_config(query_text);

        // Create parsed query
        let ast_clone = ast.clone();
        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: query_text.to_string(),
            ast,
            timestamp: Utc::now(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("parser".to_string(), self.config.name.clone());
                metadata.insert("version".to_string(), self.config.version.clone());
                metadata.insert(
                    "is_streaming".to_string(),
                    streaming_config.is_some().to_string(),
                );
                metadata.insert(
                    "has_window".to_string(),
                    window_config.is_some().to_string(),
                );
                metadata.insert(
                    "is_continuous".to_string(),
                    continuous_config.is_some().to_string(),
                );
                metadata
            },
        };

        let parse_time = start_time.elapsed().as_millis() as u64;

        // Update statistics
        self.update_stats(
            parse_time,
            streaming_config.is_some(),
            window_config.is_some(),
            continuous_config.is_some(),
        )
        .await;

        // Create parsing result
        let result = StreamParsingResult {
            parsed_query,
            streaming_config,
            window_config,
            continuous_config,
            backpressure_config,
            stats: self.stats.read().await.clone(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("parse_time_ms".to_string(), parse_time.to_string());
                metadata.insert("query_length".to_string(), query_text.len().to_string());
                metadata.insert(
                    "ast_node_count".to_string(),
                    ast_clone.node_count.to_string(),
                );
                metadata.insert("ast_depth".to_string(), ast_clone.depth.to_string());
                metadata
            },
        };

        info!("Stream query parsed successfully in {}ms", parse_time);
        Ok(result)
    }

    async fn parse_with_config(
        &self,
        query_text: &str,
        config: &StreamingQueryConfig,
    ) -> BridgeResult<StreamParsingResult> {
        let mut result = self.parse(query_text).await?;

        // Override with provided configuration
        result.streaming_config = Some(config.clone());

        Ok(result)
    }

    async fn validate(&self, result: &StreamParsingResult) -> BridgeResult<ValidationResult> {
        if !self.is_running() {
            return Err(bridge_core::BridgeError::query(
                "Stream parser is not running".to_string(),
            ));
        }

        self.validate_streaming_query(result).await
    }

    async fn get_stats(&self) -> BridgeResult<StreamParsingStats> {
        Ok(self.stats.read().await.clone())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        if *self.is_running.read().await {
            warn!("Stream parser is already running");
            return Ok(());
        }

        info!("Starting stream parser: {}", self.config.name);

        // Set running state
        {
            let mut running = self.is_running.write().await;
            *running = true;
        }

        self.start_time = Some(Utc::now());

        info!("Stream parser started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        if !*self.is_running.read().await {
            warn!("Stream parser is not running");
            return Ok(());
        }

        info!("Stopping stream parser: {}", self.config.name);

        // Set running state to false
        {
            let mut running = self.is_running.write().await;
            *running = false;
        }

        info!("Stream parser stopped successfully");
        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.is_running.blocking_read()
    }
}
