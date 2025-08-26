//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Generic query parser implementation
//!
//! This module provides a generic query parser for non-SQL query languages.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    AstNode, ErrorLocation, NodeType, ParsedQuery, ParserConfig, ParserStats, QueryAst,
    QueryParserTrait, QueryType, ValidationError, ValidationResult, ValidationSeverity,
    ValidationWarning,
};

/// Generic query parser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParserConfig {
    /// Parser name
    pub name: String,

    /// Parser version
    pub version: String,

    /// Query language
    pub query_language: QueryLanguage,

    /// Enable strict mode
    pub strict_mode: bool,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Query language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryLanguage {
    JSON,
    GraphQL,
    PromQL,
    InfluxQL,
    Custom(String),
}

impl QueryParserConfig {
    /// Create new query parser configuration
    pub fn new(query_language: QueryLanguage) -> Self {
        Self {
            name: "query".to_string(),
            version: "1.0.0".to_string(),
            query_language,
            strict_mode: false,
            additional_config: HashMap::new(),
        }
    }
}

impl Default for QueryParserConfig {
    fn default() -> Self {
        Self {
            name: "query".to_string(),
            version: "1.0.0".to_string(),
            query_language: QueryLanguage::JSON,
            strict_mode: false,
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl ParserConfig for QueryParserConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Query parser name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Generic query parser implementation
#[derive(Debug)]
pub struct GenericQueryParser {
    config: QueryParserConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ParserStats>>,
}

impl GenericQueryParser {
    /// Create new query parser
    pub async fn new(config: &QueryParserConfig) -> BridgeResult<Self> {
        config.validate().await?;

        let stats = ParserStats {
            total_queries: 0,
            successful_parses: 0,
            failed_parses: 0,
            avg_parse_time_ms: 0.0,
            last_parse_time: None,
        };

        Ok(Self {
            config: config.clone(),
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        })
    }

    /// Parse query into AST
    async fn parse_query(&self, query: &str) -> BridgeResult<QueryAst> {
        let start_time = std::time::Instant::now();

        info!("Parsing query: {}", query);

        let ast = match &self.config.query_language {
            QueryLanguage::JSON => self.parse_json_query(query).await?,
            QueryLanguage::GraphQL => self.parse_graphql_query(query).await?,
            QueryLanguage::PromQL => self.parse_promql_query(query).await?,
            QueryLanguage::InfluxQL => self.parse_influxql_query(query).await?,
            QueryLanguage::Custom(lang) => self.parse_custom_query(query, lang).await?,
        };

        let parsing_time = start_time.elapsed().as_millis() as u64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_queries += 1;
            stats.successful_parses += 1;
            stats.avg_parse_time_ms = (stats.avg_parse_time_ms
                * (stats.successful_parses - 1) as f64
                + parsing_time as f64)
                / stats.successful_parses as f64;
            stats.last_parse_time = Some(Utc::now());
        }

        info!("Query parsed in {}ms", parsing_time);

        Ok(ast)
    }

    /// Parse JSON query
    async fn parse_json_query(&self, query: &str) -> BridgeResult<QueryAst> {
        // Parse JSON query format (e.g., {"query": "select * from table", "filters": {...}})
        let json_value: serde_json::Value = serde_json::from_str(query)
            .map_err(|e| bridge_core::BridgeError::query(format!("Invalid JSON query: {}", e)))?;

        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "json".to_string());

        let root = AstNode {
            node_type: NodeType::Statement,
            value: Some(query.to_string()),
            children: vec![
                AstNode {
                    node_type: NodeType::Identifier,
                    value: Some("json_query".to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                AstNode {
                    node_type: NodeType::Other("object".to_string()),
                    value: Some(json_value.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
            ],
            metadata,
        };

        Ok(QueryAst {
            root,
            node_count: 3,
            depth: 2,
        })
    }

    /// Parse GraphQL query
    async fn parse_graphql_query(&self, query: &str) -> BridgeResult<QueryAst> {
        // Parse GraphQL query format
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "graphql".to_string());

        // Simple GraphQL parsing - extract operation type and fields
        let operation_type = if query.trim().starts_with("query") {
            "query"
        } else if query.trim().starts_with("mutation") {
            "mutation"
        } else if query.trim().starts_with("subscription") {
            "subscription"
        } else {
            "query" // Default to query
        };

        let root = AstNode {
            node_type: NodeType::Statement,
            value: Some(query.to_string()),
            children: vec![
                AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(operation_type.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                AstNode {
                    node_type: NodeType::Other("object".to_string()),
                    value: Some("graphql_fields".to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
            ],
            metadata,
        };

        Ok(QueryAst {
            root,
            node_count: 3,
            depth: 2,
        })
    }

    /// Parse PromQL query
    async fn parse_promql_query(&self, query: &str) -> BridgeResult<QueryAst> {
        // Parse PromQL query format (e.g., "http_requests_total{job='api'}")
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "promql".to_string());

        // Simple PromQL parsing - extract metric name and labels
        let parts: Vec<&str> = query.split('{').collect();
        let metric_name = parts[0].trim();
        let labels = if parts.len() > 1 {
            parts[1].trim_end_matches('}').trim()
        } else {
            ""
        };

        let root = AstNode {
            node_type: NodeType::Statement,
            value: Some(query.to_string()),
            children: vec![
                AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(metric_name.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                if !labels.is_empty() {
                    AstNode {
                        node_type: NodeType::Other("object".to_string()),
                        value: Some(labels.to_string()),
                        children: vec![],
                        metadata: HashMap::new(),
                    }
                } else {
                    AstNode {
                        node_type: NodeType::Other("empty".to_string()),
                        value: None,
                        children: vec![],
                        metadata: HashMap::new(),
                    }
                },
            ],
            metadata,
        };

        Ok(QueryAst {
            root,
            node_count: if labels.is_empty() { 2 } else { 3 },
            depth: 2,
        })
    }

    /// Parse InfluxQL query
    async fn parse_influxql_query(&self, query: &str) -> BridgeResult<QueryAst> {
        // Parse InfluxQL query format (e.g., "SELECT * FROM measurements WHERE time > now() - 1h")
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "influxql".to_string());

        // Simple InfluxQL parsing - extract SELECT, FROM, WHERE clauses
        let query_lower = query.to_lowercase();
        let select_part = if let Some(idx) = query_lower.find("from") {
            &query[..idx].trim()
        } else {
            query.trim()
        };

        let from_part = if let Some(from_idx) = query_lower.find("from") {
            if let Some(where_idx) = query_lower.find("where") {
                &query[from_idx..where_idx].trim()
            } else {
                &query[from_idx..].trim()
            }
        } else {
            ""
        };

        let root = AstNode {
            node_type: NodeType::Statement,
            value: Some(query.to_string()),
            children: vec![
                AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(select_part.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(from_part.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
            ],
            metadata,
        };

        Ok(QueryAst {
            root,
            node_count: 3,
            depth: 2,
        })
    }

    /// Parse custom query
    async fn parse_custom_query(&self, query: &str, language: &str) -> BridgeResult<QueryAst> {
        // Parse custom query format
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), language.to_string());

        let root = AstNode {
            node_type: NodeType::Statement,
            value: Some(query.to_string()),
            children: vec![
                AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(language.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                AstNode {
                    node_type: NodeType::Other("object".to_string()),
                    value: Some("custom_query".to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
            ],
            metadata,
        };

        Ok(QueryAst {
            root,
            node_count: 3,
            depth: 2,
        })
    }

    /// Determine query type from AST
    fn determine_query_type(&self, ast: &QueryAst) -> QueryType {
        // Analyze the AST to determine the query type based on language and structure
        match &self.config.query_language {
            QueryLanguage::JSON => self.determine_json_query_type(ast),
            QueryLanguage::GraphQL => self.determine_graphql_query_type(ast),
            QueryLanguage::PromQL => self.determine_promql_query_type(ast),
            QueryLanguage::InfluxQL => self.determine_influxql_query_type(ast),
            QueryLanguage::Custom(_) => self.determine_custom_query_type(ast),
        }
    }

    /// Determine JSON query type
    fn determine_json_query_type(&self, ast: &QueryAst) -> QueryType {
        // Analyze JSON query structure to determine type
        if let Some(value) = &ast.root.value {
            if value.contains("select") || value.contains("SELECT") {
                QueryType::Select
            } else if value.contains("insert") || value.contains("INSERT") {
                QueryType::Insert
            } else if value.contains("update") || value.contains("UPDATE") {
                QueryType::Update
            } else if value.contains("delete") || value.contains("DELETE") {
                QueryType::Delete
            } else {
                QueryType::Select // Default for JSON queries
            }
        } else {
            QueryType::Select
        }
    }

    /// Determine GraphQL query type
    fn determine_graphql_query_type(&self, ast: &QueryAst) -> QueryType {
        // Analyze GraphQL operation type
        for child in &ast.root.children {
            if let Some(value) = &child.value {
                match value.as_str() {
                    "query" => return QueryType::Select,
                    "mutation" => return QueryType::Update,
                    "subscription" => return QueryType::Select, // Treat as select for now
                    _ => continue,
                }
            }
        }
        QueryType::Select // Default for GraphQL
    }

    /// Determine PromQL query type
    fn determine_promql_query_type(&self, _ast: &QueryAst) -> QueryType {
        // PromQL queries are typically for selecting metrics
        QueryType::Select
    }

    /// Determine InfluxQL query type
    fn determine_influxql_query_type(&self, ast: &QueryAst) -> QueryType {
        // Analyze InfluxQL query structure
        if let Some(value) = &ast.root.value {
            let query_lower = value.to_lowercase();
            if query_lower.contains("select") {
                QueryType::Select
            } else if query_lower.contains("insert") {
                QueryType::Insert
            } else if query_lower.contains("update") {
                QueryType::Update
            } else if query_lower.contains("delete") {
                QueryType::Delete
            } else {
                QueryType::Select // Default for InfluxQL
            }
        } else {
            QueryType::Select
        }
    }

    /// Determine custom query type
    fn determine_custom_query_type(&self, _ast: &QueryAst) -> QueryType {
        // Default to Select for custom query languages
        QueryType::Select
    }

    /// Validate query
    async fn validate_query(&self, query: &str) -> BridgeResult<ValidationResult> {
        let start_time = std::time::Instant::now();

        info!("Validating query: {}", query);

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic validation - check for empty query
        if query.trim().is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_QUERY".to_string(),
                message: "Query cannot be empty".to_string(),
                location: ErrorLocation {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
                severity: ValidationSeverity::Error,
            });
            return Ok(ValidationResult {
                is_valid: false,
                errors,
                warnings,
            });
        }

        // Language-specific validation
        match &self.config.query_language {
            QueryLanguage::JSON => self.validate_json_query(query, &mut errors, &mut warnings).await?,
            QueryLanguage::GraphQL => self.validate_graphql_query(query, &mut errors, &mut warnings).await?,
            QueryLanguage::PromQL => self.validate_promql_query(query, &mut errors, &mut warnings).await?,
            QueryLanguage::InfluxQL => self.validate_influxql_query(query, &mut errors, &mut warnings).await?,
            QueryLanguage::Custom(lang) => self.validate_custom_query(query, lang, &mut errors, &mut warnings).await?,
        }

        // Strict mode validation
        if self.config.strict_mode {
            self.validate_strict_mode(query, &mut errors, &mut warnings).await?;
        }

        let validation_time = start_time.elapsed().as_millis() as u64;

        let result = ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        };

        info!("Query validation completed in {}ms", validation_time);

        Ok(result)
    }

    /// Validate JSON query
    async fn validate_json_query(&self, query: &str, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> BridgeResult<()> {
        // Validate JSON syntax
        match serde_json::from_str::<serde_json::Value>(query) {
            Ok(json_value) => {
                // Check for required fields
                if let Some(obj) = json_value.as_object() {
                    if !obj.contains_key("query") {
                        errors.push(ValidationError {
                            code: "MISSING_QUERY_FIELD".to_string(),
                            message: "JSON query must contain 'query' field".to_string(),
                            location: ErrorLocation {
                                line: 1,
                                column: 1,
                                offset: 0,
                            },
                            severity: ValidationSeverity::Error,
                        });
                    }
                } else {
                    errors.push(ValidationError {
                        code: "INVALID_JSON_STRUCTURE".to_string(),
                        message: "JSON query must be an object".to_string(),
                        location: ErrorLocation {
                            line: 1,
                            column: 1,
                            offset: 0,
                        },
                        severity: ValidationSeverity::Error,
                    });
                }
            }
            Err(e) => {
                errors.push(ValidationError {
                    code: "INVALID_JSON_SYNTAX".to_string(),
                    message: format!("Invalid JSON syntax: {}", e),
                    location: ErrorLocation {
                        line: 1,
                        column: 1,
                        offset: 0,
                    },
                    severity: ValidationSeverity::Error,
                });
            }
        }
        Ok(())
    }

    /// Validate GraphQL query
    async fn validate_graphql_query(&self, query: &str, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> BridgeResult<()> {
        let query_trimmed = query.trim();
        
        // Check for valid GraphQL operation types
        if !query_trimmed.starts_with("query") 
            && !query_trimmed.starts_with("mutation") 
            && !query_trimmed.starts_with("subscription") {
            errors.push(ValidationError {
                code: "INVALID_GRAPHQL_OPERATION".to_string(),
                message: "GraphQL query must start with 'query', 'mutation', or 'subscription'".to_string(),
                location: ErrorLocation {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
                severity: ValidationSeverity::Error,
            });
        }

        // Check for balanced braces
        let mut brace_count = 0;
        for (i, ch) in query.chars().enumerate() {
            match ch {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count < 0 {
                        errors.push(ValidationError {
                            code: "UNBALANCED_BRACES".to_string(),
                            message: "Unbalanced braces in GraphQL query".to_string(),
                            location: ErrorLocation {
                                line: 1,
                                column: i + 1,
                                offset: i,
                            },
                            severity: ValidationSeverity::Error,
                        });
                        break;
                    }
                }
                _ => {}
            }
        }

        if brace_count > 0 {
            errors.push(ValidationError {
                code: "UNBALANCED_BRACES".to_string(),
                message: "Unbalanced braces in GraphQL query".to_string(),
                location: ErrorLocation {
                    line: 1,
                    column: query.len(),
                    offset: query.len() - 1,
                },
                severity: ValidationSeverity::Error,
            });
        }

        Ok(())
    }

    /// Validate PromQL query
    async fn validate_promql_query(&self, query: &str, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> BridgeResult<()> {
        let query_trimmed = query.trim();
        
        // Check for valid PromQL metric name
        if query_trimmed.is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_PROMQL_QUERY".to_string(),
                message: "PromQL query cannot be empty".to_string(),
                location: ErrorLocation {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
                severity: ValidationSeverity::Error,
            });
            return Ok(());
        }

        // Check for balanced braces in labels
        let mut brace_count = 0;
        for (i, ch) in query.chars().enumerate() {
            match ch {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count < 0 {
                        errors.push(ValidationError {
                            code: "UNBALANCED_LABEL_BRACES".to_string(),
                            message: "Unbalanced braces in PromQL labels".to_string(),
                            location: ErrorLocation {
                                line: 1,
                                column: i + 1,
                                offset: i,
                            },
                            severity: ValidationSeverity::Error,
                        });
                        break;
                    }
                }
                _ => {}
            }
        }

        if brace_count > 0 {
            errors.push(ValidationError {
                code: "UNBALANCED_LABEL_BRACES".to_string(),
                message: "Unbalanced braces in PromQL labels".to_string(),
                location: ErrorLocation {
                    line: 1,
                    column: query.len(),
                    offset: query.len() - 1,
                },
                severity: ValidationSeverity::Error,
            });
        }

        Ok(())
    }

    /// Validate InfluxQL query
    async fn validate_influxql_query(&self, query: &str, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> BridgeResult<()> {
        let query_lower = query.to_lowercase();
        
        // Check for required keywords
        if !query_lower.contains("select") && !query_lower.contains("insert") {
            errors.push(ValidationError {
                code: "MISSING_OPERATION".to_string(),
                message: "InfluxQL query must contain SELECT or INSERT".to_string(),
                location: ErrorLocation {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
                severity: ValidationSeverity::Error,
            });
        }

        // Check for FROM clause in SELECT queries
        if query_lower.contains("select") && !query_lower.contains("from") {
            errors.push(ValidationError {
                code: "MISSING_FROM_CLAUSE".to_string(),
                message: "SELECT query must contain FROM clause".to_string(),
                location: ErrorLocation {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
                severity: ValidationSeverity::Error,
            });
        }

        Ok(())
    }

    /// Validate custom query
    async fn validate_custom_query(&self, query: &str, language: &str, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> BridgeResult<()> {
        // Basic validation for custom query languages
        if query.trim().is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_CUSTOM_QUERY".to_string(),
                message: format!("{} query cannot be empty", language),
                location: ErrorLocation {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
                severity: ValidationSeverity::Error,
            });
        }

        // Add a warning about custom language validation
        warnings.push(ValidationWarning {
            code: "CUSTOM_LANGUAGE_VALIDATION".to_string(),
            message: format!("Limited validation available for custom language: {}", language),
            location: Some(ErrorLocation {
                line: 1,
                column: 1,
                offset: 0,
            }),
            severity: ValidationSeverity::Warning,
        });

        Ok(())
    }

    /// Validate in strict mode
    async fn validate_strict_mode(&self, query: &str, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> BridgeResult<()> {
        // Additional strict mode validations
        
        // Check for potential SQL injection patterns
        let suspicious_patterns = [
            (";", "Multiple statements not allowed in strict mode"),
            ("--", "SQL comments not allowed in strict mode"),
            ("/*", "Block comments not allowed in strict mode"),
            ("*/", "Block comments not allowed in strict mode"),
            ("xp_", "Extended stored procedures not allowed in strict mode"),
            ("sp_", "System stored procedures not allowed in strict mode"),
        ];

        for (pattern, message) in suspicious_patterns.iter() {
            if query.to_lowercase().contains(pattern) {
                errors.push(ValidationError {
                    code: "STRICT_MODE_VIOLATION".to_string(),
                    message: message.to_string(),
                    location: ErrorLocation {
                        line: 1,
                        column: 1,
                        offset: 0,
                    },
                    severity: ValidationSeverity::Error,
                });
            }
        }

        // Check query length
        if query.len() > 10000 {
            warnings.push(ValidationWarning {
                code: "QUERY_TOO_LONG".to_string(),
                message: "Query is very long, consider breaking it down".to_string(),
                location: Some(ErrorLocation {
                    line: 1,
                    column: 1,
                    offset: 0,
                }),
                severity: ValidationSeverity::Warning,
            });
        }

        Ok(())
    }
}

#[async_trait]
impl QueryParserTrait for GenericQueryParser {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing query parser");

        // Validate configuration
        self.config.validate().await?;

        // Set running state
        {
            let mut is_running = self.is_running.write().await;
            *is_running = true;
        }

        info!("Query parser initialized");
        Ok(())
    }

    async fn parse(&self, query: &str) -> BridgeResult<ParsedQuery> {
        if !self.is_running().await {
            return Err(bridge_core::BridgeError::query(
                "Query parser is not running".to_string(),
            ));
        }

        let ast = self.parse_query(query).await?;
        let _query_type = self.determine_query_type(&ast);

        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: query.to_string(),
            ast,
            timestamp: Utc::now(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert(
                    "language".to_string(),
                    format!("{:?}", self.config.query_language),
                );
                metadata.insert(
                    "strict_mode".to_string(),
                    self.config.strict_mode.to_string(),
                );
                metadata
            },
        };

        Ok(parsed_query)
    }

    async fn validate(&self, parsed_query: &ParsedQuery) -> BridgeResult<ValidationResult> {
        if !self.is_running().await {
            return Err(bridge_core::BridgeError::query(
                "Query parser is not running".to_string(),
            ));
        }

        self.validate_query(&parsed_query.query_text).await
    }

    async fn get_stats(&self) -> BridgeResult<ParserStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn version(&self) -> &str {
        &self.config.version
    }
}

impl GenericQueryParser {
    /// Get query parser configuration
    pub fn get_config(&self) -> &QueryParserConfig {
        &self.config
    }

    /// Check if parser is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }
}
