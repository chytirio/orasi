# Query Execution in Orasi Agent

## Overview

The Orasi Agent now supports query execution capabilities, allowing it to process and execute SQL queries against various data sources. This functionality integrates with the Orasi Query Engine to provide a unified interface for querying telemetry data, analytics, and other data sources.

## Features

### ✅ Implemented Features

- **SQL Query Execution**: Execute SQL queries using DataFusion
- **Query Engine Integration**: Full integration with the Orasi Query Engine
- **Result Conversion**: Automatic conversion of query results to JSON format
- **Error Handling**: Comprehensive error handling and reporting
- **Performance Monitoring**: Query execution time tracking
- **Configuration Management**: Configurable query capabilities
- **Multiple Data Types**: Support for various data types in query results

### 🔄 Planned Features

- **Data Source Integration**: Connect to Delta Lake, S3 Parquet, and other data sources
- **Query Optimization**: Advanced query optimization strategies
- **Caching**: Query result caching for improved performance
- **Streaming Queries**: Support for real-time streaming queries
- **Custom Functions**: Telemetry-specific SQL functions

## Configuration

### Enable Query Capabilities

Query capabilities are enabled by default in the agent configuration:

```toml
[capabilities]
query = true
max_concurrent_tasks = 10
```

### Query Engine Configuration

The query engine is automatically configured with the following settings:

```rust
QueryEngineConfig {
    enable_caching: true,
    cache_size_limit: 100 * 1024 * 1024, // 100MB
    enable_optimization: true,
    enable_streaming: false,
    data_sources: HashMap::new(),
    max_concurrent_queries: 10,
    query_timeout_seconds: 300,
    debug_logging: false,
}
```

## Usage

### Basic Query Execution

```rust
use orasi_agent::{
    config::AgentConfig,
    processing::query::QueryProcessor,
    processing::tasks::QueryTask,
    state::AgentState,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create agent configuration
    let config = AgentConfig::default();
    
    // Create agent state
    let state = Arc::new(RwLock::new(AgentState::new(&config).await?));
    
    // Create query processor
    let query_processor = QueryProcessor::new(&config, state).await?;
    
    // Create a query task
    let query_task = QueryTask {
        query: "SELECT 1 as test_column, 'hello' as greeting".to_string(),
        parameters: HashMap::new(),
        result_destination: "memory".to_string(),
    };
    
    // Execute the query
    let result = query_processor.process_query(query_task).await?;
    
    println!("Query result: {:?}", result);
    Ok(())
}
```

### Parameterized Queries

```rust
let mut parameters = HashMap::new();
parameters.insert("limit".to_string(), "10".to_string());
parameters.insert("service_name".to_string(), "api-service".to_string());

let query_task = QueryTask {
    query: "SELECT * FROM metrics WHERE service_name = '{service_name}' LIMIT {limit}".to_string(),
    parameters,
    result_destination: "memory".to_string(),
};
```

### Analytics Queries

```rust
let analytics_task = QueryTask {
    query: "SELECT 
                service_name,
                COUNT(*) as request_count,
                AVG(response_time) as avg_response_time,
                MAX(response_time) as max_response_time
            FROM metrics 
            WHERE timestamp >= NOW() - INTERVAL '1 hour'
            GROUP BY service_name
            ORDER BY request_count DESC".to_string(),
    parameters: HashMap::new(),
    result_destination: "analytics_results".to_string(),
};
```

## Query Result Format

Query results are returned in a structured JSON format:

```json
{
  "query": "SELECT * FROM metrics",
  "parameters": {},
  "result_destination": "memory",
  "rows_returned": 5,
  "execution_time_ms": 150,
  "query_id": "uuid-here",
  "status": "success",
  "result": [
    {
      "service_name": "api-service",
      "request_count": 1000,
      "response_time": 150.5
    },
    {
      "service_name": "web-service", 
      "request_count": 500,
      "response_time": 75.2
    }
  ],
  "metadata": {
    "source": "datafusion",
    "optimized": true
  },
  "execution_timestamp": "2025-01-15T10:30:00Z"
}
```

## Error Handling

The query processor provides comprehensive error handling:

### Common Error Types

- **Configuration Errors**: Query engine not initialized
- **Invalid Input**: Empty queries or missing parameters
- **Processing Errors**: Query execution failures
- **Capability Errors**: Query capabilities not enabled

### Error Response Format

```json
{
  "success": false,
  "error": "Query execution failed: Table 'metrics' not found",
  "data": null,
  "processing_time_ms": 0
}
```

## Performance Considerations

### Query Optimization

- **Automatic Optimization**: Queries are automatically optimized by DataFusion
- **Predicate Pushdown**: Filters are pushed down to data sources when possible
- **Column Pruning**: Only requested columns are read from data sources
- **Join Optimization**: Automatic join reordering and optimization

### Caching

- **Result Caching**: Query results can be cached for improved performance
- **Configurable TTL**: Cache time-to-live is configurable
- **Memory Management**: Cache size limits prevent memory exhaustion

### Monitoring

- **Execution Time**: All queries track execution time
- **Resource Usage**: Monitor CPU and memory usage during query execution
- **Query Statistics**: Track query success rates and performance metrics

## Examples

### Running the Query Example

```bash
# Run the query example
cargo run --example query_example

# Run with debug logging
RUST_LOG=debug cargo run --example query_example
```

### Example Output

```
2025-01-15T10:30:00.000Z INFO  query_example Starting Orasi Agent Query Example
2025-01-15T10:30:00.001Z INFO  query_example Agent configuration: AgentConfig { ... }
2025-01-15T10:30:00.002Z INFO  query_example === Example 1: Simple SQL Query ===
2025-01-15T10:30:00.003Z INFO  query_example Executing SQL Query: SELECT 1 as test_column, 'hello' as greeting
2025-01-15T10:30:00.004Z INFO  query_example SQL Query completed successfully
2025-01-15T10:30:00.005Z INFO  query_example Query Result:
2025-01-15T10:30:00.006Z INFO  query_example   Task ID: SELECT 1 as test_column, 'hello' as greeting
2025-01-15T10:30:00.007Z INFO  query_example   Success: true
2025-01-15T10:30:00.008Z INFO  query_example   Processing Time: 5ms
2025-01-15T10:30:00.009Z INFO  query_example   Data: {
  "query": "SELECT 1 as test_column, 'hello' as greeting",
  "parameters": {},
  "result_destination": "memory",
  "rows_returned": 1,
  "execution_time_ms": 5,
  "query_id": "uuid-here",
  "status": "success",
  "result": [
    {
      "test_column": 1,
      "greeting": "hello"
    }
  ],
  "metadata": {},
  "execution_timestamp": "2025-01-15T10:30:00Z"
}
```

## Integration with Data Sources

The query engine supports integration with various data sources:

### Supported Data Sources

- **Delta Lake**: Apache Delta Lake tables
- **S3 Parquet**: AWS S3 Parquet files
- **Iceberg**: Apache Iceberg tables
- **In-Memory**: In-memory data structures
- **CSV Files**: Local CSV file data

### Data Source Configuration

Data sources can be configured through the query engine configuration:

```rust
let mut data_sources = HashMap::new();
data_sources.insert(
    "delta_table".to_string(),
    "s3://bucket/path/to/delta/table".to_string(),
);

let query_config = QueryEngineConfig {
    data_sources,
    // ... other configuration
};
```

## Troubleshooting

### Common Issues

1. **Query Engine Not Initialized**
   - Ensure query capabilities are enabled in configuration
   - Check that the query engine dependency is properly included

2. **Query Execution Failures**
   - Verify SQL syntax is correct
   - Check that referenced tables/data sources exist
   - Ensure proper permissions for data access

3. **Performance Issues**
   - Monitor query execution times
   - Check cache configuration
   - Review query optimization settings

### Debug Logging

Enable debug logging to troubleshoot issues:

```bash
RUST_LOG=debug cargo run --example query_example
```

## Future Enhancements

### Planned Features

- **Real-time Streaming**: Support for continuous queries
- **Advanced Analytics**: Machine learning integration
- **Query Federation**: Query across multiple data sources
- **Custom UDFs**: User-defined functions for telemetry data
- **Query Plan Visualization**: Visual query execution plans
- **Advanced Caching**: Multi-level caching strategies

### Performance Improvements

- **Parallel Execution**: Multi-threaded query execution
- **Query Compilation**: Compile queries to native code
- **Memory Optimization**: Improved memory management
- **Index Optimization**: Automatic index selection and creation

## Contributing

To contribute to the query execution functionality:

1. **Code Style**: Follow the existing code style and patterns
2. **Testing**: Add tests for new query features
3. **Documentation**: Update documentation for new capabilities
4. **Performance**: Ensure new features don't degrade performance
5. **Error Handling**: Provide comprehensive error handling

## References

- [DataFusion Documentation](https://arrow.apache.org/datafusion/)
- [SQL Reference](https://arrow.apache.org/datafusion/user-guide/sql/index.html)
- [Query Engine Architecture](https://github.com/chytirio/orasi/tree/main/crates/query-engine)
- [Agent Configuration](https://github.com/chytirio/orasi/tree/main/app/orasi-agent/src/config.rs)
