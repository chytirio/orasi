//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Real Delta Lake Integration Example
//!
//! This example demonstrates real Delta Lake integration including:
//! - Creating Delta Lake tables with real data
//! - Schema discovery from actual Delta tables
//! - Query execution against real Delta Lake tables
//! - Performance monitoring and optimization

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
use std::path::Path;
use tempfile::TempDir;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Real Delta Lake Integration Example");

    // Create a temporary directory for Delta Lake tables
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let delta_base_path = temp_dir.path().to_str().unwrap();

    info!("Using temporary Delta Lake directory: {}", delta_base_path);

    // Setup the query engine with real Delta Lake integration
    let query_engine = setup_real_delta_lake_query_engine(delta_base_path).await?;

    // Create sample Delta Lake tables with real data
    create_sample_delta_tables(delta_base_path).await?;

    // Run comprehensive examples with real Delta Lake tables
    run_real_schema_discovery_example(&query_engine).await?;
    run_real_query_execution_example(&query_engine).await?;
    run_real_performance_monitoring_example(&query_engine).await?;
    run_real_telemetry_functions_example(&query_engine).await?;

    info!("Real Delta Lake Integration Example completed successfully");
    Ok(())
}

/// Setup query engine with real Delta Lake integration
async fn setup_real_delta_lake_query_engine(delta_base_path: &str) -> BridgeResult<RealDeltaLakeQueryEngine> {
    info!("Setting up real Delta Lake integrated query engine...");

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

    // 8. Setup real Delta Lake data sources
    setup_real_delta_lake_sources(&mut data_source_manager, delta_base_path).await?;
    info!("✓ Real Delta Lake data sources configured");

    Ok(RealDeltaLakeQueryEngine {
        sql_parser,
        optimizer_manager,
        execution_engine,
        function_manager,
        query_cache,
        data_source_manager,
        source_manager,
        delta_base_path: delta_base_path.to_string(),
    })
}

/// Setup real Delta Lake data sources
async fn setup_real_delta_lake_sources(
    data_source_manager: &mut DataSourceManager,
    delta_base_path: &str,
) -> BridgeResult<()> {
    info!("Setting up real Delta Lake data sources...");

    // Create multiple Delta Lake data sources for different telemetry types
    let delta_sources = vec![
        ("telemetry_traces", "traces"),
        ("telemetry_metrics", "metrics"),
        ("telemetry_logs", "logs"),
    ];

    for (source_name, table_name) in delta_sources {
        let table_path = format!("{}/{}", delta_base_path, table_name);
        let config = DeltaLakeDataSourceConfig::new(
            table_path.clone(),
            table_name.to_string(),
        );

        let mut delta_source = DeltaLakeDataSource::new(&config).await?;
        delta_source.init().await?;

        data_source_manager.add_source(source_name.to_string(), Box::new(delta_source));
        info!("✓ Added real Delta Lake source: {} -> {}", source_name, table_path);
    }

    Ok(())
}

/// Create sample Delta Lake tables with real data
async fn create_sample_delta_tables(delta_base_path: &str) -> BridgeResult<()> {
    info!("Creating sample Delta Lake tables with real data...");

    // Create traces table
    create_traces_delta_table(delta_base_path).await?;
    
    // Create metrics table
    create_metrics_delta_table(delta_base_path).await?;
    
    // Create logs table
    create_logs_delta_table(delta_base_path).await?;

    info!("✓ Sample Delta Lake tables created successfully");
    Ok(())
}

/// Create traces Delta Lake table
async fn create_traces_delta_table(delta_base_path: &str) -> BridgeResult<()> {
    use deltalake::operations::write::WriteBuilder;
    use deltalake::DeltaTable;
    use arrow::array::*;
    use arrow::datatypes::{Field, Schema, DataType};
    use arrow::record_batch::RecordBatch;

    let table_path = format!("{}/traces", delta_base_path);
    info!("Creating traces Delta Lake table at: {}", table_path);

    // Create Arrow schema for traces
    let schema = Schema::new(vec![
        Field::new("timestamp", DataType::Timestamp(arrow::datatypes::TimeUnit::Second, None), false),
        Field::new("service_name", DataType::Utf8, false),
        Field::new("operation_name", DataType::Utf8, false),
        Field::new("duration_ms", DataType::Int64, true),
        Field::new("status_code", DataType::Int32, true),
        Field::new("trace_id", DataType::Utf8, true),
        Field::new("span_id", DataType::Utf8, true),
    ]);

    // Create sample data
    let timestamps = vec![
        Utc.timestamp_opt(1640995200, 0).unwrap(), // 2022-01-01 00:00:00
        Utc.timestamp_opt(1640995260, 0).unwrap(), // 2022-01-01 00:01:00
        Utc.timestamp_opt(1640995320, 0).unwrap(), // 2022-01-01 00:02:00
    ];

    let timestamp_array = TimestampSecondArray::from_vec(
        timestamps.iter().map(|t| t.timestamp()).collect(),
        None,
    );

    let service_names = StringArray::from(vec!["user-service", "auth-service", "payment-service"]);
    let operation_names = StringArray::from(vec!["GET /api/users", "POST /api/auth", "POST /api/payment"]);
    let durations = Int64Array::from(vec![150, 250, 300]);
    let status_codes = Int32Array::from(vec![200, 200, 500]);
    let trace_ids = StringArray::from(vec!["trace-123", "trace-456", "trace-789"]);
    let span_ids = StringArray::from(vec!["span-123", "span-456", "span-789"]);

    // Create record batch
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(timestamp_array),
            Arc::new(service_names),
            Arc::new(operation_names),
            Arc::new(durations),
            Arc::new(status_codes),
            Arc::new(trace_ids),
            Arc::new(span_ids),
        ],
    )?;

    // Create Delta table
    let table = WriteBuilder::new()
        .with_location(&table_path)
        .with_table_name("traces")
        .with_schema(schema)
        .with_partition_columns(vec!["service_name"])
        .build()
        .await?;

    // Write data
    let table = WriteBuilder::for_table(&table)
        .with_columns(vec!["timestamp", "service_name", "operation_name", "duration_ms", "status_code", "trace_id", "span_id"])
        .with_record_batch(batch)
        .build()
        .await?;

    info!("✓ Traces Delta Lake table created with {} rows", 3);
    Ok(())
}

/// Create metrics Delta Lake table
async fn create_metrics_delta_table(delta_base_path: &str) -> BridgeResult<()> {
    use deltalake::operations::write::WriteBuilder;
    use deltalake::DeltaTable;
    use arrow::array::*;
    use arrow::datatypes::{Field, Schema, DataType};
    use arrow::record_batch::RecordBatch;

    let table_path = format!("{}/metrics", delta_base_path);
    info!("Creating metrics Delta Lake table at: {}", table_path);

    // Create Arrow schema for metrics
    let schema = Schema::new(vec![
        Field::new("timestamp", DataType::Timestamp(arrow::datatypes::TimeUnit::Second, None), false),
        Field::new("service_name", DataType::Utf8, false),
        Field::new("metric_name", DataType::Utf8, false),
        Field::new("metric_value", DataType::Float64, false),
        Field::new("metric_unit", DataType::Utf8, true),
    ]);

    // Create sample data
    let timestamps = vec![
        Utc.timestamp_opt(1640995200, 0).unwrap(),
        Utc.timestamp_opt(1640995260, 0).unwrap(),
        Utc.timestamp_opt(1640995320, 0).unwrap(),
    ];

    let timestamp_array = TimestampSecondArray::from_vec(
        timestamps.iter().map(|t| t.timestamp()).collect(),
        None,
    );

    let service_names = StringArray::from(vec!["user-service", "auth-service", "payment-service"]);
    let metric_names = StringArray::from(vec!["cpu_usage", "memory_usage", "request_rate"]);
    let metric_values = Float64Array::from(vec![75.5, 82.3, 95.1]);
    let metric_units = StringArray::from(vec!["percent", "percent", "requests/sec"]);

    // Create record batch
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(timestamp_array),
            Arc::new(service_names),
            Arc::new(metric_names),
            Arc::new(metric_values),
            Arc::new(metric_units),
        ],
    )?;

    // Create Delta table
    let table = WriteBuilder::new()
        .with_location(&table_path)
        .with_table_name("metrics")
        .with_schema(schema)
        .with_partition_columns(vec!["service_name"])
        .build()
        .await?;

    // Write data
    let table = WriteBuilder::for_table(&table)
        .with_columns(vec!["timestamp", "service_name", "metric_name", "metric_value", "metric_unit"])
        .with_record_batch(batch)
        .build()
        .await?;

    info!("✓ Metrics Delta Lake table created with {} rows", 3);
    Ok(())
}

/// Create logs Delta Lake table
async fn create_logs_delta_table(delta_base_path: &str) -> BridgeResult<()> {
    use deltalake::operations::write::WriteBuilder;
    use deltalake::DeltaTable;
    use arrow::array::*;
    use arrow::datatypes::{Field, Schema, DataType};
    use arrow::record_batch::RecordBatch;

    let table_path = format!("{}/logs", delta_base_path);
    info!("Creating logs Delta Lake table at: {}", table_path);

    // Create Arrow schema for logs
    let schema = Schema::new(vec![
        Field::new("timestamp", DataType::Timestamp(arrow::datatypes::TimeUnit::Second, None), false),
        Field::new("service_name", DataType::Utf8, false),
        Field::new("log_level", DataType::Utf8, false),
        Field::new("message", DataType::Utf8, false),
        Field::new("status_code", DataType::Int32, true),
    ]);

    // Create sample data
    let timestamps = vec![
        Utc.timestamp_opt(1640995200, 0).unwrap(),
        Utc.timestamp_opt(1640995260, 0).unwrap(),
        Utc.timestamp_opt(1640995320, 0).unwrap(),
    ];

    let timestamp_array = TimestampSecondArray::from_vec(
        timestamps.iter().map(|t| t.timestamp()).collect(),
        None,
    );

    let service_names = StringArray::from(vec!["user-service", "auth-service", "payment-service"]);
    let log_levels = StringArray::from(vec!["INFO", "WARN", "ERROR"]);
    let messages = StringArray::from(vec![
        "User request processed successfully",
        "Authentication timeout",
        "Payment processing failed",
    ]);
    let status_codes = Int32Array::from(vec![200, 408, 500]);

    // Create record batch
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(timestamp_array),
            Arc::new(service_names),
            Arc::new(log_levels),
            Arc::new(messages),
            Arc::new(status_codes),
        ],
    )?;

    // Create Delta table
    let table = WriteBuilder::new()
        .with_location(&table_path)
        .with_table_name("logs")
        .with_schema(schema)
        .with_partition_columns(vec!["service_name"])
        .build()
        .await?;

    // Write data
    let table = WriteBuilder::for_table(&table)
        .with_columns(vec!["timestamp", "service_name", "log_level", "message", "status_code"])
        .with_record_batch(batch)
        .build()
        .await?;

    info!("✓ Logs Delta Lake table created with {} rows", 3);
    Ok(())
}

/// Complete query engine with real Delta Lake integration
struct RealDeltaLakeQueryEngine {
    sql_parser: SqlParser,
    optimizer_manager: OptimizerManager,
    execution_engine: ExecutionEngine,
    function_manager: FunctionManager,
    query_cache: QueryCache,
    data_source_manager: DataSourceManager,
    source_manager: SourceManagerImpl,
    delta_base_path: String,
}

/// Example 1: Real Schema Discovery
async fn run_real_schema_discovery_example(engine: &RealDeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Real Schema Discovery Example ===");

    let source_names = vec!["telemetry_traces", "telemetry_metrics", "telemetry_logs"];

    for source_name in source_names {
        info!("Discovering real schema for source: {}", source_name);

        if let Some(source) = engine.data_source_manager.get_source(source_name) {
            match source.get_schema().await {
                Ok(schema) => {
                    info!("✓ Real schema discovered for {}:", source_name);
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

/// Example 2: Real Query Execution
async fn run_real_query_execution_example(engine: &RealDeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Real Query Execution Example ===");

    // Example queries for different Delta Lake tables
    let queries = vec![
        ("telemetry_traces", "SELECT service_name, COUNT(*) as trace_count FROM telemetry_traces GROUP BY service_name ORDER BY trace_count DESC"),
        ("telemetry_metrics", "SELECT service_name, AVG(metric_value) as avg_value FROM telemetry_metrics GROUP BY service_name"),
        ("telemetry_logs", "SELECT service_name, COUNT(*) as error_count FROM telemetry_logs WHERE status_code >= 400 GROUP BY service_name"),
    ];

    for (source_name, query) in queries {
        info!("Executing real query on {}: {}", source_name, query);

        let start_time = std::time::Instant::now();
        
        match engine.data_source_manager.execute_query(source_name, query).await {
            Ok(result) => {
                let execution_time = start_time.elapsed();
                info!("✓ Real query executed successfully:");
                info!("  - Execution time: {:?}", execution_time);
                info!("  - Rows returned: {}", result.data.len());
                info!("  - Query ID: {}", result.query_id);
                
                // Display results
                for (i, row) in result.data.iter().take(3).enumerate() {
                    info!("  - Row {}: {:?}", i + 1, row.data);
                }
            }
            Err(e) => {
                warn!("✗ Real query failed: {}", e);
            }
        }
    }

    Ok(())
}

/// Example 3: Real Performance Monitoring
async fn run_real_performance_monitoring_example(engine: &RealDeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Real Performance Monitoring Example ===");

    // Execute multiple real queries to generate performance data
    let test_queries = vec![
        ("telemetry_traces", "SELECT COUNT(*) FROM telemetry_traces"),
        ("telemetry_traces", "SELECT service_name, COUNT(*) FROM telemetry_traces GROUP BY service_name"),
        ("telemetry_metrics", "SELECT * FROM telemetry_metrics WHERE metric_value > 80"),
        ("telemetry_logs", "SELECT * FROM telemetry_logs WHERE log_level = 'ERROR'"),
    ];

    for (source_name, query) in test_queries {
        info!("Executing real performance test query: {}", query);
        
        let start_time = std::time::Instant::now();
        let result = engine.data_source_manager.execute_query(source_name, query).await;
        let execution_time = start_time.elapsed();

        match result {
            Ok(data_result) => {
                info!("✓ Real query completed in {:?}: {} rows", execution_time, data_result.data.len());
            }
            Err(e) => {
                warn!("✗ Real query failed: {}", e);
            }
        }
    }

    // Get real performance statistics
    let source_stats = engine.data_source_manager.get_stats();
    info!("Real data source performance statistics:");
    for (source_name, stats) in source_stats {
        info!("  - {}:", source_name);
        info!("    - Total queries: {}", stats.total_queries);
        info!("    - Avg execution time: {:.2}ms", stats.avg_execution_time_ms);
        info!("    - Total rows processed: {}", stats.total_rows_processed);
        info!("    - Error count: {}", stats.error_count);
        info!("    - Is connected: {}", stats.is_connected);
    }

    Ok(())
}

/// Example 4: Real Telemetry Functions
async fn run_real_telemetry_functions_example(engine: &RealDeltaLakeQueryEngine) -> BridgeResult<()> {
    info!("=== Running Real Telemetry Functions Example ===");

    // Example 1: Calculate error rate using real data
    let error_rate_args = vec![
        query_engine::functions::FunctionValue::Integer(1), // 1 error from logs
        query_engine::functions::FunctionValue::Integer(3), // 3 total logs
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
        info!("✓ Real error rate calculated: {:.2}%", rate);
    }

    // Example 2: Extract service name from real resource attributes
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
        info!("✓ Real service name extracted: {}", service_name);
    }

    // Example 3: Get service dependencies using real data
    let dependencies_result = engine
        .function_manager
        .execute_function("telemetry", vec![
            query_engine::functions::FunctionValue::String("get_service_dependencies".to_string()),
            query_engine::functions::FunctionValue::String("user-service".to_string()),
            query_engine::functions::FunctionValue::String("1 hour".to_string()),
        ])
        .await?;

    if let query_engine::functions::FunctionValue::Object(deps) = dependencies_result.value {
        info!("✓ Real service dependencies retrieved: {:?}", deps);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_real_delta_lake_query_engine_setup() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let delta_base_path = temp_dir.path().to_str().unwrap();
        
        let result = setup_real_delta_lake_query_engine(delta_base_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_sample_delta_tables() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let delta_base_path = temp_dir.path().to_str().unwrap();
        
        let result = create_sample_delta_tables(delta_base_path).await;
        assert!(result.is_ok());
    }
}
