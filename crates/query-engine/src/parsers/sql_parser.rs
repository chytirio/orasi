//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! SQL parser implementation
//!
//! This module provides a SQL parser for parsing SQL queries.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlparser::ast::{
    self, Expr, FunctionArg, Ident, ObjectName, Query, Select, SelectItem, SetExpr, Statement,
    TableFactor, TableWithJoins, Value,
};
use sqlparser::dialect::{
    BigQueryDialect, Dialect, GenericDialect, MySqlDialect, PostgreSqlDialect, SnowflakeDialect,
};
use sqlparser::parser::Parser;
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

/// SQL parser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlParserConfig {
    /// Parser name
    pub name: String,

    /// Parser version
    pub version: String,

    /// SQL dialect
    pub dialect: SqlDialect,

    /// Enable strict mode
    pub strict_mode: bool,

    /// Enable case sensitivity
    pub case_sensitive: bool,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// SQL dialect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SqlDialect {
    Standard,
    PostgreSQL,
    MySQL,
    SQLite,
    BigQuery,
    Snowflake,
    Custom(String),
}

impl SqlParserConfig {
    /// Create new SQL parser configuration
    pub fn new() -> Self {
        Self {
            name: "sql".to_string(),
            version: "1.0.0".to_string(),
            dialect: SqlDialect::Standard,
            strict_mode: false,
            case_sensitive: false,
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl ParserConfig for SqlParserConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "SQL parser name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// SQL parser implementation
pub struct SqlParser {
    config: SqlParserConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ParserStats>>,
}

impl SqlParser {
    /// Create new SQL parser
    pub async fn new(config: &dyn ParserConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<SqlParserConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration(
                    "Invalid SQL parser configuration".to_string(),
                )
            })?
            .clone();

        config.validate().await?;

        let stats = ParserStats {
            total_queries: 0,
            successful_parses: 0,
            failed_parses: 0,
            avg_parse_time_ms: 0.0,
            last_parse_time: None,
        };

        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        })
    }

    /// Get the appropriate SQL dialect
    fn get_dialect(&self) -> &'static dyn Dialect {
        static GENERIC: GenericDialect = GenericDialect {};
        static POSTGRES: PostgreSqlDialect = PostgreSqlDialect {};
        static MYSQL: MySqlDialect = MySqlDialect {};
        static BIGQUERY: BigQueryDialect = BigQueryDialect {};
        static SNOWFLAKE: SnowflakeDialect = SnowflakeDialect {};

        match self.config.dialect {
            SqlDialect::Standard => &GENERIC,
            SqlDialect::PostgreSQL => &POSTGRES,
            SqlDialect::MySQL => &MYSQL,
            SqlDialect::BigQuery => &BIGQUERY,
            SqlDialect::Snowflake => &SNOWFLAKE,
            SqlDialect::SQLite => &GENERIC, // SQLite uses generic dialect
            SqlDialect::Custom(_) => &GENERIC, // Custom uses generic dialect
        }
    }

    /// Parse SQL query into AST
    async fn parse_sql(&self, query: &str) -> BridgeResult<QueryAst> {
        let start_time = std::time::Instant::now();

        info!("Parsing SQL query: {}", query);

        // Get the appropriate dialect
        let dialect = self.get_dialect();

        // Parse the SQL query
        let ast_statements = Parser::parse_sql(dialect, query)
            .map_err(|e| bridge_core::BridgeError::query(format!("SQL parsing failed: {}", e)))?;

        if ast_statements.is_empty() {
            return Err(bridge_core::BridgeError::query(
                "No SQL statements found in query".to_string(),
            ));
        }

        // Convert the first statement to our AST format
        let root = self.convert_statement_to_ast_node(&ast_statements[0])?;

        // Calculate node count and depth
        let node_count = self.count_nodes(&root);
        let depth = self.calculate_depth(&root);

        let ast = QueryAst {
            root,
            node_count,
            depth,
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

        info!("SQL query parsed in {}ms", parsing_time);

        Ok(ast)
    }

    /// Convert SQL statement to AST node
    fn convert_statement_to_ast_node(&self, statement: &Statement) -> BridgeResult<AstNode> {
        match statement {
            Statement::Query(query) => self.convert_query_to_ast_node(query),
            Statement::Insert {
                table,
                columns,
                source,
                ..
            } => {
                let mut children = vec![];

                // Add table node
                children.push(AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(format!("{:?}", table)),
                    children: vec![],
                    metadata: HashMap::new(),
                });

                // Add columns if specified
                if !columns.is_empty() {
                    let column_children = columns
                        .iter()
                        .map(|col| AstNode {
                            node_type: NodeType::Identifier,
                            value: Some(format!("{:?}", col)),
                            children: vec![],
                            metadata: HashMap::new(),
                        })
                        .collect();

                    children.push(AstNode {
                        node_type: NodeType::Other("columns".to_string()),
                        value: None,
                        children: column_children,
                        metadata: HashMap::new(),
                    });
                }

                // Add source if specified
                children.push(self.convert_query_to_ast_node(source)?);

                Ok(AstNode {
                    node_type: NodeType::Insert,
                    value: None,
                    children,
                    metadata: HashMap::new(),
                })
            }
            Statement::Update {
                table,
                assignments,
                selection,
                ..
            } => {
                let mut children = vec![];

                // Add table node
                children.push(AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(format!("{:?}", table)),
                    children: vec![],
                    metadata: HashMap::new(),
                });

                // Add assignments
                let assignment_children = assignments
                    .iter()
                    .map(|assignment| AstNode {
                        node_type: NodeType::Expression,
                        value: Some(format!("{:?}", assignment)),
                        children: vec![],
                        metadata: HashMap::new(),
                    })
                    .collect();

                children.push(AstNode {
                    node_type: NodeType::Other("assignments".to_string()),
                    value: None,
                    children: assignment_children,
                    metadata: HashMap::new(),
                });

                // Add where clause if specified
                if let Some(expr) = selection {
                    children.push(self.convert_expr_to_ast_node(expr)?);
                }

                Ok(AstNode {
                    node_type: NodeType::Update,
                    value: None,
                    children,
                    metadata: HashMap::new(),
                })
            }
            Statement::Delete {
                tables,
                from,
                selection,
                ..
            } => {
                let mut children = vec![];

                // Add table nodes
                for table in tables {
                    children.push(AstNode {
                        node_type: NodeType::Identifier,
                        value: Some(format!("{:?}", table)),
                        children: vec![],
                        metadata: HashMap::new(),
                    });
                }

                // Add from clause if present
                if !from.is_empty() {
                    let from_children = from
                        .iter()
                        .map(|table_with_joins| {
                            self.convert_table_with_joins_to_ast_node(table_with_joins)
                        })
                        .collect::<BridgeResult<Vec<_>>>()?;

                    children.push(AstNode {
                        node_type: NodeType::From,
                        value: None,
                        children: from_children,
                        metadata: HashMap::new(),
                    });
                }

                // Add where clause if specified
                if let Some(expr) = selection {
                    children.push(self.convert_expr_to_ast_node(expr)?);
                }

                Ok(AstNode {
                    node_type: NodeType::Delete,
                    value: None,
                    children,
                    metadata: HashMap::new(),
                })
            }
            Statement::CreateTable { name, columns, .. } => {
                let mut children = vec![];

                // Add table name
                children.push(AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(format!("{:?}", name)),
                    children: vec![],
                    metadata: HashMap::new(),
                });

                // Add columns
                let column_children = columns
                    .iter()
                    .map(|col_def| AstNode {
                        node_type: NodeType::Identifier,
                        value: Some(format!("{:?}", col_def)),
                        children: vec![],
                        metadata: HashMap::new(),
                    })
                    .collect();

                children.push(AstNode {
                    node_type: NodeType::Other("columns".to_string()),
                    value: None,
                    children: column_children,
                    metadata: HashMap::new(),
                });

                Ok(AstNode {
                    node_type: NodeType::Create,
                    value: None,
                    children,
                    metadata: HashMap::new(),
                })
            }
            Statement::Drop {
                object_type, names, ..
            } => {
                let children = names
                    .iter()
                    .map(|name| AstNode {
                        node_type: NodeType::Identifier,
                        value: Some(format!("{:?}", name)),
                        children: vec![],
                        metadata: HashMap::new(),
                    })
                    .collect();

                Ok(AstNode {
                    node_type: NodeType::Drop,
                    value: Some(format!("DROP {}", object_type)),
                    children,
                    metadata: HashMap::new(),
                })
            }
            _ => Ok(AstNode {
                node_type: NodeType::Statement,
                value: Some(format!("{:?}", statement)),
                children: vec![],
                metadata: HashMap::new(),
            }),
        }
    }

    /// Convert SQL query to AST node
    fn convert_query_to_ast_node(&self, query: &Query) -> BridgeResult<AstNode> {
        let mut children = vec![];

        match &*query.body {
            SetExpr::Select(select) => {
                children.push(self.convert_select_to_ast_node(select)?);
            }
            SetExpr::Query(subquery) => {
                children.push(self.convert_query_to_ast_node(subquery)?);
            }
            SetExpr::SetOperation {
                op, left, right, ..
            } => {
                children.push(self.convert_set_expr_to_ast_node(left)?);
                children.push(AstNode {
                    node_type: NodeType::Operator,
                    value: Some(format!("{:?}", op)),
                    children: vec![],
                    metadata: HashMap::new(),
                });
                children.push(self.convert_set_expr_to_ast_node(right)?);
            }
            _ => {
                children.push(AstNode {
                    node_type: NodeType::Other("unknown_set_expr".to_string()),
                    value: Some(format!("{:?}", query.body)),
                    children: vec![],
                    metadata: HashMap::new(),
                });
            }
        }

        // Add order by if present
        if !query.order_by.is_empty() {
            let order_children = query
                .order_by
                .iter()
                .map(|order| AstNode {
                    node_type: NodeType::OrderBy,
                    value: Some(format!("{} {:?}", order.expr, order.asc)),
                    children: vec![],
                    metadata: HashMap::new(),
                })
                .collect();

            children.push(AstNode {
                node_type: NodeType::OrderBy,
                value: None,
                children: order_children,
                metadata: HashMap::new(),
            });
        }

        // Add limit if present
        if let Some(limit) = &query.limit {
            children.push(AstNode {
                node_type: NodeType::Limit,
                value: Some(limit.to_string()),
                children: vec![],
                metadata: HashMap::new(),
            });
        }

        // Add offset if present
        if let Some(offset) = &query.offset {
            children.push(AstNode {
                node_type: NodeType::Offset,
                value: Some(offset.to_string()),
                children: vec![],
                metadata: HashMap::new(),
            });
        }

        Ok(AstNode {
            node_type: NodeType::Statement,
            value: None,
            children,
            metadata: HashMap::new(),
        })
    }

    /// Convert SELECT statement to AST node
    fn convert_select_to_ast_node(&self, select: &Select) -> BridgeResult<AstNode> {
        let mut children = vec![];

        // Convert select items
        let select_children = select
            .projection
            .iter()
            .map(|item| match item {
                SelectItem::UnnamedExpr(expr) => AstNode {
                    node_type: NodeType::Expression,
                    value: Some(expr.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                SelectItem::ExprWithAlias { expr, alias } => AstNode {
                    node_type: NodeType::Expression,
                    value: Some(format!("{} AS {}", expr, alias)),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                SelectItem::QualifiedWildcard(qualifier, _) => AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(format!("{:?}.*", qualifier)),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                SelectItem::Wildcard(_) => AstNode {
                    node_type: NodeType::Identifier,
                    value: Some("*".to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
            })
            .collect();

        children.push(AstNode {
            node_type: NodeType::Select,
            value: None,
            children: select_children,
            metadata: HashMap::new(),
        });

        // Convert from clause
        if !select.from.is_empty() {
            let from_children = select
                .from
                .iter()
                .map(|table_with_joins| self.convert_table_with_joins_to_ast_node(table_with_joins))
                .collect::<BridgeResult<Vec<_>>>()?;

            children.push(AstNode {
                node_type: NodeType::From,
                value: None,
                children: from_children,
                metadata: HashMap::new(),
            });
        }

        // Convert where clause
        if let Some(expr) = &select.selection {
            children.push(AstNode {
                node_type: NodeType::Where,
                value: None,
                children: vec![self.convert_expr_to_ast_node(expr)?],
                metadata: HashMap::new(),
            });
        }

        // Convert group by clause
        match &select.group_by {
            ast::GroupByExpr::Expressions(exprs) => {
                if !exprs.is_empty() {
                    let group_children = exprs
                        .iter()
                        .map(|expr| self.convert_expr_to_ast_node(expr))
                        .collect::<BridgeResult<Vec<_>>>()?;

                    children.push(AstNode {
                        node_type: NodeType::GroupBy,
                        value: None,
                        children: group_children,
                        metadata: HashMap::new(),
                    });
                }
            }
            _ => {
                // Handle other group by expressions if needed
            }
        }

        // Convert having clause
        if let Some(expr) = &select.having {
            children.push(AstNode {
                node_type: NodeType::Other("having".to_string()),
                value: None,
                children: vec![self.convert_expr_to_ast_node(expr)?],
                metadata: HashMap::new(),
            });
        }

        Ok(AstNode {
            node_type: NodeType::Select,
            value: None,
            children,
            metadata: HashMap::new(),
        })
    }

    /// Convert table with joins to AST node
    fn convert_table_with_joins_to_ast_node(
        &self,
        table_with_joins: &TableWithJoins,
    ) -> BridgeResult<AstNode> {
        let mut children = vec![];

        // Convert the main table
        children.push(self.convert_table_factor_to_ast_node(&table_with_joins.relation)?);

        // Convert joins
        for join in &table_with_joins.joins {
            children.push(AstNode {
                node_type: NodeType::Other("join".to_string()),
                value: Some(format!("{:?} JOIN", join.join_operator)),
                children: vec![self.convert_table_factor_to_ast_node(&join.relation)?],
                metadata: HashMap::new(),
            });
        }

        Ok(AstNode {
            node_type: NodeType::From,
            value: None,
            children,
            metadata: HashMap::new(),
        })
    }

    /// Convert table factor to AST node
    fn convert_table_factor_to_ast_node(
        &self,
        table_factor: &TableFactor,
    ) -> BridgeResult<AstNode> {
        match table_factor {
            TableFactor::Table { name, alias, .. } => {
                let mut children = vec![AstNode {
                    node_type: NodeType::Identifier,
                    value: Some(format!("{:?}", name)),
                    children: vec![],
                    metadata: HashMap::new(),
                }];

                if let Some(alias) = alias {
                    children.push(AstNode {
                        node_type: NodeType::Identifier,
                        value: Some(alias.name.to_string()),
                        children: vec![],
                        metadata: HashMap::new(),
                    });
                }

                Ok(AstNode {
                    node_type: NodeType::Identifier,
                    value: None,
                    children,
                    metadata: HashMap::new(),
                })
            }
            TableFactor::Derived {
                subquery, alias, ..
            } => {
                let mut children = vec![self.convert_query_to_ast_node(subquery)?];

                if let Some(alias) = alias {
                    children.push(AstNode {
                        node_type: NodeType::Identifier,
                        value: Some(alias.name.to_string()),
                        children: vec![],
                        metadata: HashMap::new(),
                    });
                }

                Ok(AstNode {
                    node_type: NodeType::Other("subquery".to_string()),
                    value: None,
                    children,
                    metadata: HashMap::new(),
                })
            }
            _ => Ok(AstNode {
                node_type: NodeType::Other("table_factor".to_string()),
                value: Some(format!("{:?}", table_factor)),
                children: vec![],
                metadata: HashMap::new(),
            }),
        }
    }

    /// Convert expression to AST node
    fn convert_expr_to_ast_node(&self, expr: &Expr) -> BridgeResult<AstNode> {
        match expr {
            Expr::Identifier(ident) => Ok(AstNode {
                node_type: NodeType::Identifier,
                value: Some(ident.to_string()),
                children: vec![],
                metadata: HashMap::new(),
            }),
            Expr::Value(value) => Ok(AstNode {
                node_type: NodeType::Literal,
                value: Some(format!("{:?}", value)),
                children: vec![],
                metadata: HashMap::new(),
            }),
            Expr::BinaryOp { left, op, right } => Ok(AstNode {
                node_type: NodeType::Expression,
                value: Some(format!("{:?}", op)),
                children: vec![
                    self.convert_expr_to_ast_node(left)?,
                    self.convert_expr_to_ast_node(right)?,
                ],
                metadata: HashMap::new(),
            }),
            Expr::Function(func) => {
                let args = func
                    .args
                    .iter()
                    .map(|arg| match arg {
                        FunctionArg::Named { name, arg } => AstNode {
                            node_type: NodeType::Identifier,
                            value: Some(format!("{}: {:?}", name, arg)),
                            children: vec![],
                            metadata: HashMap::new(),
                        },
                        FunctionArg::Unnamed(arg) => AstNode {
                            node_type: NodeType::Expression,
                            value: Some(format!("{:?}", arg)),
                            children: vec![],
                            metadata: HashMap::new(),
                        },
                    })
                    .collect();

                Ok(AstNode {
                    node_type: NodeType::Function,
                    value: Some(func.name.to_string()),
                    children: args,
                    metadata: HashMap::new(),
                })
            }
            _ => Ok(AstNode {
                node_type: NodeType::Expression,
                value: Some(format!("{:?}", expr)),
                children: vec![],
                metadata: HashMap::new(),
            }),
        }
    }

    /// Convert set expression to AST node
    fn convert_set_expr_to_ast_node(&self, set_expr: &SetExpr) -> BridgeResult<AstNode> {
        match set_expr {
            SetExpr::Select(select) => self.convert_select_to_ast_node(select),
            SetExpr::Query(query) => self.convert_query_to_ast_node(query),
            _ => Ok(AstNode {
                node_type: NodeType::Other("set_expr".to_string()),
                value: Some(format!("{:?}", set_expr)),
                children: vec![],
                metadata: HashMap::new(),
            }),
        }
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
        if node.children.is_empty() {
            1
        } else {
            1 + node
                .children
                .iter()
                .map(|child| self.calculate_depth(child))
                .max()
                .unwrap_or(0)
        }
    }

    /// Determine query type from AST
    fn determine_query_type(&self, ast: &QueryAst) -> QueryType {
        // Analyze the AST to determine the query type
        if let Some(_select_node) = self.find_node_by_type(&ast.root, &NodeType::Select) {
            QueryType::Select
        } else if let Some(_insert_node) = self.find_node_by_type(&ast.root, &NodeType::Insert) {
            QueryType::Insert
        } else if let Some(_update_node) = self.find_node_by_type(&ast.root, &NodeType::Update) {
            QueryType::Update
        } else if let Some(_delete_node) = self.find_node_by_type(&ast.root, &NodeType::Delete) {
            QueryType::Delete
        } else if let Some(_create_node) = self.find_node_by_type(&ast.root, &NodeType::Create) {
            QueryType::Create
        } else if let Some(_drop_node) = self.find_node_by_type(&ast.root, &NodeType::Drop) {
            QueryType::Drop
        } else if let Some(_alter_node) = self.find_node_by_type(&ast.root, &NodeType::Alter) {
            QueryType::Alter
        } else {
            QueryType::Unknown
        }
    }

    /// Find node by type in AST
    fn find_node_by_type<'a>(
        &self,
        node: &'a AstNode,
        node_type: &NodeType,
    ) -> Option<&'a AstNode> {
        if node.node_type == *node_type {
            return Some(node);
        }

        for child in &node.children {
            if let Some(found) = self.find_node_by_type(child, node_type) {
                return Some(found);
            }
        }

        None
    }

    /// Validate SQL query
    async fn validate_sql(&self, query: &str) -> BridgeResult<ValidationResult> {
        let start_time = std::time::Instant::now();

        info!("Validating SQL query: {}", query);

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic validation
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

        // Try to parse the query to validate syntax
        let dialect = self.get_dialect();
        match Parser::parse_sql(dialect, query) {
            Ok(statements) => {
                if statements.is_empty() {
                    errors.push(ValidationError {
                        code: "NO_STATEMENTS".to_string(),
                        message: "No SQL statements found in query".to_string(),
                        location: ErrorLocation {
                            line: 1,
                            column: 1,
                            offset: 0,
                        },
                        severity: ValidationSeverity::Error,
                    });
                } else if statements.len() > 1 {
                    warnings.push(ValidationWarning {
                        code: "MULTIPLE_STATEMENTS".to_string(),
                        message:
                            "Multiple SQL statements detected, only the first will be processed"
                                .to_string(),
                        location: None,
                        severity: ValidationSeverity::Warning,
                    });
                }

                // Additional semantic validation based on statement type
                if let Some(first_stmt) = statements.first() {
                    self.validate_statement_semantics(first_stmt, &mut errors, &mut warnings);
                }
            }
            Err(parse_error) => {
                errors.push(ValidationError {
                    code: "SYNTAX_ERROR".to_string(),
                    message: format!("SQL syntax error: {}", parse_error),
                    location: ErrorLocation {
                        line: 1,
                        column: 1,
                        offset: 0,
                    },
                    severity: ValidationSeverity::Error,
                });
            }
        }

        // Check for common SQL keywords to ensure it looks like SQL
        let query_lower = query.to_lowercase();
        let sql_keywords = [
            "select", "insert", "update", "delete", "create", "drop", "alter", "from", "where",
            "group", "order", "having", "join", "union",
        ];

        let has_sql_keywords = sql_keywords
            .iter()
            .any(|keyword| query_lower.contains(keyword));
        if !has_sql_keywords {
            warnings.push(ValidationWarning {
                code: "POTENTIAL_INVALID_SQL".to_string(),
                message: "Query may not be a valid SQL statement".to_string(),
                location: None,
                severity: ValidationSeverity::Warning,
            });
        }

        // Strict mode validation
        if self.config.strict_mode {
            self.validate_strict_mode(query, &mut errors, &mut warnings);
        }

        let validation_time = start_time.elapsed().as_millis() as u64;

        let result = ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        };

        info!("SQL query validation completed in {}ms", validation_time);

        Ok(result)
    }

    /// Validate statement semantics
    fn validate_statement_semantics(
        &self,
        statement: &Statement,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        match statement {
            Statement::Query(query) => {
                self.validate_query_semantics(query, errors, warnings);
            }
            Statement::Insert { .. } => {
                // Insert-specific validation could be added here
            }
            Statement::Update { .. } => {
                // Update-specific validation could be added here
            }
            Statement::Delete { .. } => {
                // Delete-specific validation could be added here
            }
            Statement::CreateTable { .. } => {
                // Create table-specific validation could be added here
            }
            Statement::Drop { .. } => {
                // Drop-specific validation could be added here
            }
            _ => {
                // Other statement types
            }
        }
    }

    /// Validate query semantics
    fn validate_query_semantics(
        &self,
        query: &Query,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        match &*query.body {
            SetExpr::Select(select) => {
                self.validate_select_semantics(select, errors, warnings);
            }
            _ => {
                // Other set expression types
            }
        }
    }

    /// Validate SELECT statement semantics
    fn validate_select_semantics(
        &self,
        select: &Select,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        // Check if SELECT has FROM clause
        if select.from.is_empty() {
            warnings.push(ValidationWarning {
                code: "NO_FROM_CLAUSE".to_string(),
                message: "SELECT statement without FROM clause".to_string(),
                location: None,
                severity: ValidationSeverity::Warning,
            });
        }

        // Check for GROUP BY without aggregate functions
        match &select.group_by {
            ast::GroupByExpr::Expressions(exprs) => {
                if !exprs.is_empty() {
                    let has_aggregates = select.projection.iter().any(|item| match item {
                        SelectItem::UnnamedExpr(expr) => self.is_aggregate_function(expr),
                        SelectItem::ExprWithAlias { expr, .. } => self.is_aggregate_function(expr),
                        _ => false,
                    });

                    if !has_aggregates {
                        warnings.push(ValidationWarning {
                            code: "GROUP_BY_WITHOUT_AGGREGATES".to_string(),
                            message: "GROUP BY clause without aggregate functions".to_string(),
                            location: None,
                            severity: ValidationSeverity::Warning,
                        });
                    }
                }
            }
            _ => {
                // Handle other group by expressions if needed
            }
        }

        // Check for HAVING without GROUP BY
        if select.having.is_some() {
            match &select.group_by {
                ast::GroupByExpr::Expressions(exprs) => {
                    if exprs.is_empty() {
                        warnings.push(ValidationWarning {
                            code: "HAVING_WITHOUT_GROUP_BY".to_string(),
                            message: "HAVING clause without GROUP BY clause".to_string(),
                            location: None,
                            severity: ValidationSeverity::Warning,
                        });
                    }
                }
                _ => {
                    // Handle other cases
                }
            }
        }
    }

    /// Check if expression is an aggregate function
    fn is_aggregate_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Function(func) => {
                let func_name = func.name.to_string().to_lowercase();
                matches!(
                    func_name.as_str(),
                    "count" | "sum" | "avg" | "min" | "max" | "stddev" | "variance"
                )
            }
            _ => false,
        }
    }

    /// Validate strict mode rules
    fn validate_strict_mode(
        &self,
        query: &str,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        // Check for potential SQL injection patterns
        let suspicious_patterns = [
            ("--", "SQL comment detected"),
            ("/*", "SQL block comment detected"),
            ("xp_", "Extended stored procedure detected"),
            ("sp_", "Stored procedure detected"),
        ];

        for (pattern, message) in suspicious_patterns.iter() {
            if query.to_lowercase().contains(pattern) {
                warnings.push(ValidationWarning {
                    code: "SUSPICIOUS_PATTERN".to_string(),
                    message: message.to_string(),
                    location: None,
                    severity: ValidationSeverity::Warning,
                });
            }
        }

        // Check for case sensitivity if configured
        if self.config.case_sensitive {
            // Additional case-sensitive validation could be added here
        }
    }
}

#[async_trait]
impl QueryParserTrait for SqlParser {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing SQL parser");

        // Validate configuration
        self.config.validate().await?;

        // Set running state
        {
            let mut is_running = self.is_running.write().await;
            *is_running = true;
        }

        info!("SQL parser initialized");
        Ok(())
    }

    async fn parse(&self, query: &str) -> BridgeResult<ParsedQuery> {
        if !self.is_running().await {
            return Err(bridge_core::BridgeError::query(
                "SQL parser is not running".to_string(),
            ));
        }

        let ast = self.parse_sql(query).await?;
        let query_type = self.determine_query_type(&ast);

        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: query.to_string(),
            ast,
            timestamp: Utc::now(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("dialect".to_string(), format!("{:?}", self.config.dialect));
                metadata.insert(
                    "strict_mode".to_string(),
                    self.config.strict_mode.to_string(),
                );
                metadata.insert(
                    "case_sensitive".to_string(),
                    self.config.case_sensitive.to_string(),
                );
                metadata.insert("query_type".to_string(), format!("{:?}", query_type));
                metadata
            },
        };

        Ok(parsed_query)
    }

    async fn validate(&self, parsed_query: &ParsedQuery) -> BridgeResult<ValidationResult> {
        if !self.is_running().await {
            return Err(bridge_core::BridgeError::query(
                "SQL parser is not running".to_string(),
            ));
        }

        self.validate_sql(&parsed_query.query_text).await
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

impl SqlParser {
    /// Get SQL parser configuration
    pub fn get_config(&self) -> &SqlParserConfig {
        &self.config
    }

    /// Check if parser is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }
}
