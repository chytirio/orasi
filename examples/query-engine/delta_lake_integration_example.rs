//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake Integration Example
//!
//! This example demonstrates the integration between the query engine
//! and Delta Lake data sources, including:
//! - Delta Lake data source setup and configuration
//! - Schema discovery and validation
//! - Query execution against Delta Lake tables
//! - Performance monitoring and optimization
//! - Integration with telemetry functions

use query_engine::{
    cache::{CacheConfig, QueryCache},
    executors::{DataFusionExecutor, DataFusionConfig, ExecutionEngine, QueryExecutor, QueryResult},
    functions::{FunctionManager, TelemetryFunctionConfig, TelemetryFunctions},
    optimizers::{OptimizerManager, SqlOptimizer, SqlOptimizerConfig},
    parsers::{ParsedQuery, QueryAst, QueryParser, SqlParser},
    sources::{
        DataSource, DataSourceConfig, DataSourceManager, DataSourceResult, DataSourceStats,
        DeltaLakeDataSource, DeltaLakeDataSourceConfig, SourceManagerConfig, SourceManagerImpl,
    },
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

    info!("Starting Delta Lake Integration Example");

    // Setup the complete query engine with Delta Lake integration
    let query_engine = setup_delta_lake_query_engine().await?;

    // Run comprehensive examples
    run_delta_lake_setup_example(&query_engine).await?;
    run_schema_discovery_example(&query_engine).await?;
    run_query_execution_example(&query_engine).await?;
    run_telemetry_functions_example(&query_engine).await?;
    run_performance_monitoring_example(&query_engine).await?;
    run_multi_source_query_example(&query_engine).await?;

    info!("Delta Lake Integration Example completed successfully");
    Ok(())
}

/// Complete query engine setup with Delta Lake integration
async fn setup_delta_lake_query_engine() -> BridgeResult<DeltaLakeQueryEngine> {
    info!("Setting up Delta Lake integrated query engine...");

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

    // 6. Initialize Data Source Manager
    let mut data_source_manager = DataSourceManager::new();
    info!("✓ Data Source Manager initialized");

    // 7. Initialize Source Manager
    let source_manager_config = SourceManagerConfig::new();
    let source_manager = SourceManagerImpl::new(source_manager_config);
    info!("✓ Source Manager initialized");

    // 8. Setup Delta Lake data sources
    setup_delta_lake_sources(&mut data_source_manager).await?;
    info!("✓ Delta Lake data sources configured");

    Ok(DeltaLakeQueryEngine {
        sql_parser,
        optimizer_manager,
        execution_engine,
        function_manager,
        query_cache,
        data_source_manager,
        source_manager,
    })
}

/// Setup Delta Lake data sources
async fn setup_delta_lake_sources(
    data_source_manager: &mut DataSourceManager,
) -> BridgeResult<()> {
    info!("Setting up Delta Lake data sources...");

    // Create multiple Delta Lake data sources for different telemetry types
    let delta_sources = vec![
        ("telemetry_traces", "/data/delta/traces", "traces"),
        ("telemetry_metrics", "/data/delta/metrics", "metrics"),
        ("telemetry_logs", "/data/delta/logs", "logs"),
    ];

    for (source_name, table_path, table_name) in delta_sources {
        let config = DeltaLakeDataSourceConfig::new(
            table_path.to_string(),
            table_name.to_string(),
        );

        let mut delta_source = DeltaLakeDataSource::new(&config).await?;
        delta_source.init().await?;

        data_source_manager.add_source(source_name.to_string(), Box::new(delta_source));
        info!("✓ Added Delta Lake source: {} -> {}/{}", source_name, table_path, table_name);
    }

    Ok(())
}

/// Complete query engine with Delta Lake integration
struct DeltaLakeQueryEngine {
    sql_parser: SqlParser,
    optimizer_manager: OptimizerManager,
    execution_engine: ExecutionEngine,
    function_manager: FunctionManager,
    query_cache: QueryCache,
    data_source_manager: DataSourceManager,
    source_manager: SourceManagerImpl,
}

/// Example 1: Delta Lake Setup and Configuration
async fn run_delta_lake_setup_example(engine: &DeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Delta Lake Setup Example ===");

    // Register Delta Lake sources in the source manager
    let metadata = HashMap::new();
    
    engine.source_manager.register_source(
        "telemetry_traces".to_string(),
        "delta_lake".to_string(),
        r#"{"table_path": "/data/delta/traces", "table_name": "traces"}"#.to_string(),
        metadata.clone(),
    ).await?;

    engine.source_manager.register_source(
        "telemetry_metrics".to_string(),
        "delta_lake".to_string(),
        r#"{"table_path": "/data/delta/metrics", "table_name": "metrics"}"#.to_string(),
        metadata.clone(),
    ).await?;

    engine.source_manager.register_source(
        "telemetry_logs".to_string(),
        "delta_lake".to_string(),
        r#"{"table_path": "/data/delta/logs", "table_name": "logs"}"#.to_string(),
        metadata,
    ).await?;

    // List registered sources
    let sources = engine.source_manager.list_sources().await?;
    info!("Registered sources:");
    for source in sources {
        info!("  - {} (type: {}) - Healthy: {}", 
              source.name, source.source_type, source.is_healthy);
    }

    // Perform health checks
    let health_results = engine.source_manager.health_check_all_sources().await?;
    info!("Health check results:");
    for (source_name, is_healthy) in health_results {
        info!("  - {}: {}", source_name, if is_healthy { "✓ Healthy" } else { "✗ Unhealthy" });
    }

    Ok(())
}

/// Example 2: Schema Discovery
async fn run_schema_discovery_example(engine: &DeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Schema Discovery Example ===");

    let source_names = vec!["telemetry_traces", "telemetry_metrics", "telemetry_logs"];

    for source_name in source_names {
        info!("Discovering schema for source: {}", source_name);

        if let Some(source) = engine.data_source_manager.get_source(source_name) {
            match source.get_schema().await {
                Ok(schema) => {
                    info!("✓ Schema discovered for {}:", source_name);
                    info!("  - Schema ID: {}", schema.id);
                    info!("  - Schema Name: {}", schema.name);
                    info!("  - Schema Version: {}", schema.version);
                    info!("  - Columns: {}", schema.columns.len());
                    info!("  - Partition Columns: {:?}", schema.partition_columns);
                    
                    // Display column details
                    for column in &schema.columns {
                        info!("    - {}: {:?} (nullable: {})", 
                              column.name, column.data_type, column.nullable);
                    }
                }
                Err(e) => {
                    warn!("✗ Failed to discover schema for {}: {}", source_name, e);
                }
            }
        } else {
            warn!("✗ Source not found: {}", source_name);
        }
    }

    Ok(())
}

/// Example 3: Query Execution
async fn run_query_execution_example(engine: &DeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Query Execution Example ===");

    // Example queries for different Delta Lake tables
    let queries = vec![
        ("telemetry_traces", "SELECT service_name, COUNT(*) as trace_count FROM telemetry_traces GROUP BY service_name ORDER BY trace_count DESC LIMIT 10"),
        ("telemetry_metrics", "SELECT service_name, AVG(duration_ms) as avg_duration FROM telemetry_metrics WHERE timestamp >= NOW() - INTERVAL '1 hour' GROUP BY service_name"),
        ("telemetry_logs", "SELECT service_name, COUNT(*) as error_count FROM telemetry_logs WHERE status_code >= 400 GROUP BY service_name"),
    ];

    for (source_name, query) in queries {
        info!("Executing query on {}: {}", source_name, query);

        let start_time = std::time::Instant::now();
        
        match engine.data_source_manager.execute_query(source_name, query).await {
            Ok(result) => {
                let execution_time = start_time.elapsed();
                info!("✓ Query executed successfully:");
                info!("  - Execution time: {:?}", execution_time);
                info!("  - Rows returned: {}", result.data.len());
                info!("  - Query ID: {}", result.query_id);
                
                // Display first few rows
                for (i, row) in result.data.iter().take(3).enumerate() {
                    info!("  - Row {}: {:?}", i + 1, row.data);
                }
            }
            Err(e) => {
                warn!("✗ Query failed: {}", e);
            }
        }
    }

    Ok(())
}

/// Example 4: Telemetry Functions with Delta Lake
async fn run_telemetry_functions_example(engine: &DeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Telemetry Functions Example ===");

    // Example 1: Calculate error rate using telemetry functions
    let error_rate_args = vec![
        query_engine::functions::FunctionValue::Integer(25),
        query_engine::functions::FunctionValue::Integer(1000),
    ];

    let error_rate_result = engine
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

    let service_name_result = engine
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
    let dependencies_result = engine
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

/// Example 5: Performance Monitoring
async fn run_performance_monitoring_example(engine: &DeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Performance Monitoring Example ===");

    // Execute multiple queries to generate performance data
    let test_queries = vec![
        ("telemetry_traces", "SELECT COUNT(*) FROM telemetry_traces"),
        ("telemetry_traces", "SELECT service_name, COUNT(*) FROM telemetry_traces GROUP BY service_name"),
        ("telemetry_traces", "SELECT * FROM telemetry_traces WHERE timestamp >= NOW() - INTERVAL '1 hour' LIMIT 100"),
    ];

    for (source_name, query) in test_queries {
        info!("Executing performance test query: {}", query);
        
        let start_time = std::time::Instant::now();
        let result = engine.data_source_manager.execute_query(source_name, query).await;
        let execution_time = start_time.elapsed();

        match result {
            Ok(data_result) => {
                info!("✓ Query completed in {:?}: {} rows", execution_time, data_result.data.len());
            }
            Err(e) => {
                warn!("✗ Query failed: {}", e);
            }
        }
    }

    // Get performance statistics
    let source_stats = engine.data_source_manager.get_stats();
    info!("Data source performance statistics:");
    for (source_name, stats) in source_stats {
        info!("  - {}:", source_name);
        info!("    - Total queries: {}", stats.total_queries);
        info!("    - Avg execution time: {:.2}ms", stats.avg_execution_time_ms);
        info!("    - Total rows processed: {}", stats.total_rows_processed);
        info!("    - Error count: {}", stats.error_count);
        info!("    - Is connected: {}", stats.is_connected);
    }

    // Get function performance statistics
    let function_stats = engine.function_manager.get_stats();
    info!("Function performance statistics:");
    for (function_name, stats) in function_stats {
        info!("  - {}:", function_name);
        info!("    - Total calls: {}", stats.total_calls);
        info!("    - Avg execution time: {:.2}ms", stats.avg_execution_time_ms);
        info!("    - Error count: {}", stats.error_count);
    }

    // Get cache statistics
    let cache_stats = engine.query_cache.get_stats().await?;
    info!("Cache performance statistics:");
    info!("  - Total entries: {}", cache_stats.total_entries);
    info!("  - Hit rate: {:.2}%", cache_stats.hit_rate * 100.0);
    info!("  - Total size: {} bytes", cache_stats.total_size_bytes);
    info!("  - Evicted entries: {}", cache_stats.evicted_entries);

    Ok(())
}

/// Example 6: Multi-Source Query
async fn run_multi_source_query_example(engine: &DeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Multi-Source Query Example ===");

    // This example demonstrates how to query across multiple Delta Lake tables
    // In a real implementation, this would involve federated queries

    let sources = vec!["telemetry_traces", "telemetry_metrics", "telemetry_logs"];

    info!("Executing queries across multiple Delta Lake sources:");

    for source in sources {
        let query = format!("SELECT service_name, COUNT(*) as count FROM {} GROUP BY service_name LIMIT 5", source);
        
        info!("Querying {}: {}", source, query);
        
        match engine.data_source_manager.execute_query(source, &query).await {
            Ok(result) => {
                info!("✓ {}: {} rows returned", source, result.data.len());
                
                // Display results
                for row in result.data.iter().take(2) {
                    if let (Some(service_name), Some(count)) = (
                        row.data.get("service_name"),
                        row.data.get("count")
                    ) {
                        info!("  - {}: {}", 
                              match service_name {
                                  query_engine::sources::DataSourceValue::String(s) => s,
                                  _ => "unknown",
                              },
                              match count {
                                  query_engine::sources::DataSourceValue::Integer(i) => i.to_string(),
                                  _ => "unknown".to_string(),
                              }
                        );
                    }
                }
            }
            Err(e) => {
                warn!("✗ {}: {}", source, e);
            }
        }
    }

    // Example of a complex query that would require federation
    info!("Complex multi-source query example (conceptual):");
    info!("SELECT t.service_name, ");
    info!("       COUNT(t.trace_id) as trace_count, ");
    info!("       AVG(m.duration_ms) as avg_duration, ");
    info!("       COUNT(l.log_id) as error_count ");
    info!("FROM telemetry_traces t ");
    info!("LEFT JOIN telemetry_metrics m ON t.service_name = m.service_name ");
    info!("LEFT JOIN telemetry_logs l ON t.service_name = l.service_name AND l.status_code >= 400 ");
    info!("WHERE t.timestamp >= NOW() - INTERVAL '1 hour' ");
    info!("GROUP BY t.service_name ");
    info!("ORDER BY trace_count DESC");

    info!("Note: This query would require federated query execution across multiple Delta Lake tables");

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
    async fn test_delta_lake_query_engine_setup() {
        let result = setup_delta_lake_query_engine().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delta_lake_setup_example() {
        let engine = setup_delta_lake_query_engine().await.unwrap();
        let result = run_delta_lake_setup_example(&engine).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_schema_discovery_example() {
        let engine = setup_delta_lake_query_engine().await.unwrap();
        let result = run_schema_discovery_example(&engine).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_execution_example() {
        let engine = setup_delta_lake_query_engine().await.unwrap();
        let result = run_query_execution_example(&engine).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_telemetry_functions_example() {
        let engine = setup_delta_lake_query_engine().await.unwrap();
        let result = run_telemetry_functions_example(&engine).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_performance_monitoring_example() {
        let engine = setup_delta_lake_query_engine().await.unwrap();
        let result = run_performance_monitoring_example(&engine).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multi_source_query_example() {
        let engine = setup_delta_lake_query_engine().await.unwrap();
        let result = run_multi_source_query_example(&engine).await;
        assert!(result.is_ok());
    }
}
