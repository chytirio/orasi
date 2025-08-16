//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query parsers for OpenTelemetry Data Lake Bridge
//! 
//! This module provides parsing capabilities for various query languages
//! including SQL and custom query formats.

use std::collections::HashMap;
use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod query_parser;
pub mod sql_parser;

// Re-export parser implementations
pub use query_parser::GenericQueryParser as QueryParser;
pub use sql_parser::SqlParser;

/// Parser configuration trait
#[async_trait]
pub trait ParserConfig: Send + Sync {
    /// Get parser name
    fn name(&self) -> &str;
    
    /// Get parser version
    fn version(&self) -> &str;
    
    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;
    
    /// Get as any type
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Parser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfigStruct {
    /// Parser name
    pub name: String,
    
    /// Parser version
    pub version: String,
    
    /// Parser type
    pub parser_type: ParserType,
    
    /// Parser options
    pub options: HashMap<String, String>,
}

/// Parser type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParserType {
    Sql,
    Query,
    Custom(String),
}

/// Parsed query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuery {
    /// Query ID
    pub id: Uuid,
    
    /// Query text
    pub query_text: String,
    
    /// Parsed AST
    pub ast: QueryAst,
    
    /// Parse timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Parse metadata
    pub metadata: HashMap<String, String>,
}

/// Query AST node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAst {
    /// Root node
    pub root: AstNode,
    
    /// Node count
    pub node_count: usize,
    
    /// Tree depth
    pub depth: usize,
}

/// AST node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    /// Node type
    pub node_type: NodeType,
    
    /// Node value
    pub value: Option<String>,
    
    /// Child nodes
    pub children: Vec<AstNode>,
    
    /// Node metadata
    pub metadata: HashMap<String, String>,
}

/// AST node type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Select,
    From,
    Where,
    GroupBy,
    OrderBy,
    Limit,
    Offset,
    Function,
    Identifier,
    Literal,
    Operator,
    Expression,
    Statement,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Alter,
    Other(String),
}

/// Query type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Alter,
    Unknown,
    Other(String),
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Is valid
    pub is_valid: bool,
    
    /// Validation errors
    pub errors: Vec<ValidationError>,
    
    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    
    /// Error message
    pub message: String,
    
    /// Error location
    pub location: ErrorLocation,
    
    /// Error severity
    pub severity: ValidationSeverity,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    
    /// Warning message
    pub message: String,
    
    /// Warning location
    pub location: Option<ErrorLocation>,
    
    /// Warning severity
    pub severity: ValidationSeverity,
}

/// Error location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLocation {
    /// Line number
    pub line: usize,
    
    /// Column number
    pub column: usize,
    
    /// Character offset
    pub offset: usize,
}

/// Validation severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// Parser statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserStats {
    /// Total queries parsed
    pub total_queries: u64,
    
    /// Successful parses
    pub successful_parses: u64,
    
    /// Failed parses
    pub failed_parses: u64,
    
    /// Average parse time in milliseconds
    pub avg_parse_time_ms: f64,
    
    /// Last parse timestamp
    pub last_parse_time: Option<DateTime<Utc>>,
}

/// Query parser trait
#[async_trait]
pub trait QueryParserTrait: Send + Sync {
    /// Initialize parser
    async fn init(&mut self) -> BridgeResult<()>;
    
    /// Parse query text
    async fn parse(&self, query_text: &str) -> BridgeResult<ParsedQuery>;
    
    /// Validate parsed query
    async fn validate(&self, parsed_query: &ParsedQuery) -> BridgeResult<ValidationResult>;
    
    /// Get parser name
    fn name(&self) -> &str;
    
    /// Get parser version
    fn version(&self) -> &str;
    
    /// Get parser statistics
    async fn get_stats(&self) -> BridgeResult<ParserStats>;
}

/// Parser factory for creating parser instances
pub struct ParserFactory {
    /// Available parsers
    parsers: HashMap<String, Box<dyn QueryParserTrait>>,
}

impl ParserFactory {
    /// Create a new parser factory
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }
    
    /// Register a parser
    pub fn register_parser(&mut self, name: String, parser: Box<dyn QueryParserTrait>) {
        self.parsers.insert(name, parser);
    }
    
    /// Get a parser by name
    pub fn get_parser(&self, name: &str) -> Option<&dyn QueryParserTrait> {
        self.parsers.get(name).map(|p| p.as_ref())
    }
    
    /// List available parsers
    pub fn list_parsers(&self) -> Vec<String> {
        self.parsers.keys().cloned().collect()
    }
}
