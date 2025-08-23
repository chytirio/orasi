//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Advanced Query Engine Example
//!
//! This example demonstrates the advanced features of the query engine including:
//! - SQL query optimization
//! - Telemetry-specific functions
//! - Query result caching
//! - Performance monitoring

use query_engine::{
    cache::{CacheConfig, QueryCache},
    executors::{DataFusionExecutor, DataFusionConfig, ExecutionEngine, QueryExecutor, QueryResult},
    functions::{FunctionManager, TelemetryFunctionConfig, TelemetryFunctions},
    optimizers::{OptimizerManager, SqlOptimizer, SqlOptimizerConfig},
    parsers::{ParsedQuery, QueryAst, QueryParser, SqlParser},
    BridgeResult,
};
use bridge_core::types::{TelemetryData, TelemetryRecord, TelemetryType};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Advanced Query Engine Example");

    // Initialize components
    let query_engine = setup_query_engine().await?;

    // Run examples
    run_basic_query_example(&query_engine).await?;
    run_optimization_example(&query_engine).await?;
    run_telemetry_functions_example(&query_engine).await?;
    run_caching_example(&query_engine).await?;
    run_performance_monitoring_example(&query_engine).await?;

    info!("Advanced Query Engine Example completed successfully");
    Ok(())
}

/// Setup the complete query engine with all components
async fn setup_query_engine() -> BridgeResult<QueryEngineComponents> {
    info!("Setting up query engine components...");

    // 1. Initialize SQL Parser
    let sql_parser = SqlParser::new();
    info!("✓ SQL Parser initialized");

    // 2. Initialize SQL Optimizer
    let optimizer_config = SqlOptimizerConfig::new();
    let mut optimizer_manager = OptimizerManager::new();
    let sql_optimizer = SqlOptimizer::new(optimizer_config);
    optimizer_manager.add_optimizer("sql".to_string(), Box::new(sql_optimizer));
    info!("✓ SQL Optimizer initialized");

    // 3. Initialize DataFusion Executor
    let executor_config = DataFusionConfig::new();
    let mut execution_engine = ExecutionEngine::new();
    let datafusion_executor = DataFusionExecutor::new(executor_config);
    execution_engine.add_executor("datafusion".to_string(), Box::new(datafusion_executor));
    info!("✓ DataFusion Executor initialized");

    // 4. Initialize Telemetry Functions
    let function_config = TelemetryFunctionConfig::new();
    let mut function_manager = FunctionManager::new();
    let telemetry_functions = TelemetryFunctions::new(function_config);
    function_manager.add_function("telemetry".to_string(), Box::new(telemetry_functions));
    info!("✓ Telemetry Functions initialized");

    // 5. Initialize Query Cache
    let cache_config = CacheConfig::new();
    let query_cache = QueryCache::new(cache_config);
    info!("✓ Query Cache initialized");

    Ok(QueryEngineComponents {
        sql_parser,
        optimizer_manager,
        execution_engine,
        function_manager,
        query_cache,
    })
}

/// Query engine components
struct QueryEngineComponents {
    sql_parser: SqlParser,
    optimizer_manager: OptimizerManager,
    execution_engine: ExecutionEngine,
    function_manager: FunctionManager,
    query_cache: QueryCache,
}

/// Example 1: Basic query execution
async fn run_basic_query_example(components: &QueryEngineComponents) -> BridgeResult<()> {
    info!("=== Running Basic Query Example ===");

    // Create a simple SQL query
    let sql = "SELECT service_name, COUNT(*) as request_count FROM telemetry_data GROUP BY service_name";

    // Parse the query
    let parsed_query = components.sql_parser.parse(sql).await?;
    info!("✓ Query parsed successfully");

    // Execute the query
    let result = components
        .execution_engine
        .execute_query("datafusion", parsed_query)
        .await?;

    info!("✓ Query executed successfully");
    info!("Result: {} rows returned", result.data.len());

    Ok(())
}

/// Example 2: Query optimization
async fn run_optimization_example(components: &QueryEngineComponents) -> BridgeResult<()> {
    info!("=== Running Query Optimization Example ===");

    // Create a complex query that can benefit from optimization
    let sql = "
        SELECT 
            service_name,
            COUNT(*) as request_count,
            AVG(duration_ms) as avg_duration
        FROM telemetry_data 
        WHERE timestamp >= NOW() - INTERVAL '1 hour'
            AND status_code >= 400
        GROUP BY service_name
        HAVING COUNT(*) > 10
        ORDER BY request_count DESC
        LIMIT 10
    ";

    // Parse the query
    let parsed_query = components.sql_parser.parse(sql).await?;
    info!("✓ Original query parsed");

    // Optimize the query
    let optimized_query = components
        .optimizer_manager
        .optimize_query("sql", parsed_query)
        .await?;
    info!("✓ Query optimized");

    // Execute the optimized query
    let result = components
        .execution_engine
        .execute_query("datafusion", optimized_query)
        .await?;

    info!("✓ Optimized query executed successfully");
    info!("Result: {} rows returned", result.data.len());

    // Get optimizer statistics
    let optimizer_stats = components.optimizer_manager.get_stats().await?;
    for (name, stats) in optimizer_stats {
        info!("Optimizer '{}': {} queries optimized, avg time: {:.2}ms", 
              name, stats.total_queries, stats.avg_optimization_time_ms);
    }

    Ok(())
}

/// Example 3: Telemetry functions
async fn run_telemetry_functions_example(components: &QueryEngineComponents) -> BridgeResult<()> {
    info!("=== Running Telemetry Functions Example ===");

    // Example 1: Calculate error rate
    let error_rate_args = vec![
        query_engine::functions::FunctionValue::Integer(25),
        query_engine::functions::FunctionValue::Integer(1000),
    ];

    let error_rate_result = components
        .function_manager
        .execute_function("telemetry", vec![
            query_engine::functions::FunctionValue::String("calculate_error_rate".to_string()),
            error_rate_args[0].clone(),
            error_rate_args[1].clone(),
        ])
        .await?;

    if let query_engine::functions::FunctionValue::Float(rate) = error_rate_result.value {
        info!("✓ Error rate calculated: {:.2}%", rate);
    }

    // Example 2: Extract service name from resource attributes
    let mut resource_attrs = HashMap::new();
    resource_attrs.insert("service.name".to_string(), 
                         query_engine::functions::FunctionValue::String("user-service".to_string()));

    let service_name_result = components
        .function_manager
        .execute_function("telemetry", vec![
            query_engine::functions::FunctionValue::String("extract_service_name".to_string()),
            query_engine::functions::FunctionValue::Object(resource_attrs),
        ])
        .await?;

    if let query_engine::functions::FunctionValue::String(service_name) = service_name_result.value {
        info!("✓ Service name extracted: {}", service_name);
    }

    // Example 3: Get service dependencies
    let dependencies_result = components
        .function_manager
        .execute_function("telemetry", vec![
            query_engine::functions::FunctionValue::String("get_service_dependencies".to_string()),
            query_engine::functions::FunctionValue::String("user-service".to_string()),
            query_engine::functions::FunctionValue::String("1 hour".to_string()),
        ])
        .await?;

    if let query_engine::functions::FunctionValue::Object(deps) = dependencies_result.value {
        info!("✓ Service dependencies retrieved: {:?}", deps);
    }

    Ok(())
}

/// Example 4: Query caching
async fn run_caching_example(components: &QueryEngineComponents) -> BridgeResult<()> {
    info!("=== Running Query Caching Example ===");

    // Create a query to cache
    let sql = "SELECT service_name, COUNT(*) as request_count FROM telemetry_data WHERE timestamp >= NOW() - INTERVAL '1 hour' GROUP BY service_name";
    let parsed_query = components.sql_parser.parse(sql).await?;

    // First execution (cache miss)
    let start_time = std::time::Instant::now();
    let result1 = components
        .execution_engine
        .execute_query("datafusion", parsed_query.clone())
        .await?;
    let first_execution_time = start_time.elapsed();

    // Cache the result
    components.query_cache.put(&parsed_query, result1.clone()).await?;
    info!("✓ Query result cached");

    // Second execution (should be cache hit)
    let start_time = std::time::Instant::now();
    let cached_result = components.query_cache.get(&parsed_query).await?;
    let cache_lookup_time = start_time.elapsed();

    if let Some(result2) = cached_result {
        info!("✓ Cache hit! Retrieved result in {:?}", cache_lookup_time);
        info!("Original execution time: {:?}", first_execution_time);
        info!("Cache lookup time: {:?}", cache_lookup_time);
        info!("Speedup: {:.2}x", first_execution_time.as_micros() as f64 / cache_lookup_time.as_micros() as f64);
    } else {
        warn!("Cache miss on second lookup");
    }

    // Get cache statistics
    let cache_stats = components.query_cache.get_stats().await?;
    info!("Cache stats: {} hits, {} misses, {:.2}% hit rate", 
          cache_stats.hits, cache_stats.misses, cache_stats.hit_rate * 100.0);

    Ok(())
}

/// Example 5: Performance monitoring
async fn run_performance_monitoring_example(components: &QueryEngineComponents) -> BridgeResult<()> {
    info!("=== Running Performance Monitoring Example ===");

    // Execute multiple queries to generate statistics
    let queries = vec![
        "SELECT COUNT(*) FROM telemetry_data",
        "SELECT service_name, AVG(duration_ms) FROM telemetry_data GROUP BY service_name",
        "SELECT * FROM telemetry_data WHERE status_code >= 400 LIMIT 100",
    ];

    for (i, sql) in queries.iter().enumerate() {
        info!("Executing query {}: {}", i + 1, sql);
        
        let parsed_query = components.sql_parser.parse(sql).await?;
        let result = components
            .execution_engine
            .execute_query("datafusion", parsed_query)
            .await?;

        info!("Query {} completed: {} rows, {}ms", 
              i + 1, result.data.len(), result.execution_time_ms);
    }

    // Get executor statistics
    let executor_stats = components.execution_engine.get_stats().await?;
    for (name, stats) in executor_stats {
        info!("Executor '{}': {} queries, avg time: {:.2}ms, errors: {}", 
              name, stats.total_queries, stats.avg_execution_time_ms, stats.error_count);
    }

    // Get function statistics
    let function_stats = components.function_manager.get_stats();
    for (name, stats) in function_stats {
        info!("Function '{}': {} calls, avg time: {:.2}ms, errors: {}", 
              name, stats.total_calls, stats.avg_execution_time_ms, stats.error_count);
    }

    // Get cache statistics
    let cache_stats = components.query_cache.get_stats().await?;
    info!("Cache performance: {} entries, {:.2}% hit rate, {} bytes used", 
          cache_stats.total_entries, cache_stats.hit_rate * 100.0, cache_stats.total_size_bytes);

    Ok(())
}

/// Helper function to create sample telemetry data
fn create_sample_telemetry_data() -> Vec<TelemetryRecord> {
    let mut records = Vec::new();
    let now = Utc::now();

    // Create sample service names
    let services = vec!["user-service", "auth-service", "payment-service", "notification-service"];

    for i in 0..100 {
        let service_name = services[i % services.len()];
        let timestamp = now - chrono::Duration::minutes(i as i64);

        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp,
            data: TelemetryData::Trace {
                trace_id: Uuid::new_v4(),
                span_id: Uuid::new_v4(),
                parent_span_id: None,
                service_name: service_name.to_string(),
                operation_name: "http_request".to_string(),
                duration_ms: (100 + (i % 900)) as u64, // 100-1000ms
                status_code: if i % 20 == 0 { 500 } else { 200 }, // 5% error rate
                attributes: HashMap::new(),
            },
        };

        records.push(record);
    }

    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_engine_setup() {
        let result = setup_query_engine().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_basic_query_example() {
        let components = setup_query_engine().await.unwrap();
        let result = run_basic_query_example(&components).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_optimization_example() {
        let components = setup_query_engine().await.unwrap();
        let result = run_optimization_example(&components).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_telemetry_functions_example() {
        let components = setup_query_engine().await.unwrap();
        let result = run_telemetry_functions_example(&components).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_caching_example() {
        let components = setup_query_engine().await.unwrap();
        let result = run_caching_example(&components).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_performance_monitoring_example() {
        let components = setup_query_engine().await.unwrap();
        let result = run_performance_monitoring_example(&components).await;
        assert!(result.is_ok());
    }
}
