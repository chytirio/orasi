# SQL Parser

The SQL parser module provides comprehensive SQL parsing and validation capabilities for the Orasi query engine.

## Features

- **Multi-dialect Support**: Supports Standard SQL, PostgreSQL, MySQL, SQLite, BigQuery, and Snowflake dialects
- **AST Generation**: Converts SQL queries into Abstract Syntax Trees (AST) for analysis and processing
- **Query Type Detection**: Automatically identifies query types (SELECT, INSERT, UPDATE, DELETE, etc.)
- **Semantic Validation**: Validates SQL syntax and provides semantic analysis
- **Strict Mode**: Configurable strict validation for security and compliance
- **Performance Metrics**: Tracks parsing performance and statistics

## Usage

### Basic Usage

```rust
use query_engine::parsers::{SqlParser, SqlParserConfig, ParserConfig};

// Create configuration
let config = SqlParserConfig {
    name: "my_sql_parser".to_string(),
    version: "1.0.0".to_string(),
    dialect: SqlDialect::Standard,
    strict_mode: false,
    case_sensitive: false,
    additional_config: HashMap::new(),
};

// Create and initialize parser
let mut parser = SqlParser::new(&config).await?;
parser.init().await?;

// Parse a query
let parsed_query = parser.parse("SELECT * FROM users WHERE age > 18").await?;

// Validate the query
let validation_result = parser.validate(&parsed_query).await?;
```

### Supported SQL Dialects

- `SqlDialect::Standard` - Standard SQL
- `SqlDialect::PostgreSQL` - PostgreSQL-specific syntax
- `SqlDialect::MySQL` - MySQL-specific syntax
- `SqlDialect::SQLite` - SQLite-specific syntax
- `SqlDialect::BigQuery` - Google BigQuery syntax
- `SqlDialect::Snowflake` - Snowflake-specific syntax
- `SqlDialect::Custom(String)` - Custom dialect

### Query Types

The parser automatically detects the following query types:

- `QueryType::Select` - SELECT queries
- `QueryType::Insert` - INSERT queries
- `QueryType::Update` - UPDATE queries
- `QueryType::Delete` - DELETE queries
- `QueryType::Create` - CREATE TABLE queries
- `QueryType::Drop` - DROP TABLE queries
- `QueryType::Alter` - ALTER TABLE queries
- `QueryType::Unknown` - Unknown or unsupported queries

### AST Structure

The parser generates an Abstract Syntax Tree with the following node types:

- `NodeType::Select` - SELECT clause
- `NodeType::From` - FROM clause
- `NodeType::Where` - WHERE clause
- `NodeType::GroupBy` - GROUP BY clause
- `NodeType::OrderBy` - ORDER BY clause
- `NodeType::Limit` - LIMIT clause
- `NodeType::Offset` - OFFSET clause
- `NodeType::Function` - Function calls
- `NodeType::Identifier` - Identifiers (table names, column names)
- `NodeType::Literal` - Literal values
- `NodeType::Operator` - Operators
- `NodeType::Expression` - Expressions
- `NodeType::Statement` - SQL statements

### Validation Features

#### Syntax Validation
- Validates SQL syntax using the sqlparser crate
- Provides detailed error messages with line and column information
- Supports multiple SQL dialects

#### Semantic Validation
- Checks for common SQL anti-patterns
- Validates GROUP BY usage with aggregate functions
- Ensures HAVING clauses are used with GROUP BY
- Warns about SELECT statements without FROM clauses

#### Security Validation (Strict Mode)
- Detects potential SQL injection patterns
- Warns about suspicious SQL comments
- Identifies stored procedure calls
- Validates case sensitivity requirements

### Configuration Options

- `strict_mode`: Enables strict validation for security and compliance
- `case_sensitive`: Enforces case sensitivity in identifiers
- `dialect`: Specifies the SQL dialect for parsing
- `additional_config`: Custom configuration parameters

### Performance Monitoring

The parser tracks the following metrics:

- Total queries processed
- Successful vs failed parses
- Average parsing time
- Last parse timestamp

### Example

See `examples/sql_parser_example.rs` for a complete working example.

## Dependencies

- `sqlparser` - SQL parsing library
- `bridge-core` - Core bridge functionality
- `tokio` - Async runtime
- `tracing` - Logging and observability

## Error Handling

The parser provides comprehensive error handling with:

- Detailed error messages
- Error location information (line, column, offset)
- Error severity levels (Error, Warning, Info)
- Error codes for programmatic handling

## Future Enhancements

- Support for additional SQL dialects
- Enhanced semantic validation rules
- Query optimization suggestions
- Schema-aware validation
- Performance optimization recommendations
