//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Minimal DataFusion Integration Example
//!
//! This example demonstrates the DataFusion integration with Delta Lake
//! for real query execution in the Orasi query engine, using only the
//! essential crates to avoid compilation issues.

use std::sync::Arc;
use tracing::{info, warn};

use query_engine::{
    executors::datafusion_executor::{DataFusionConfig, DataFusionExecutor},
    QueryEngineConfig, QueryExecutor,
};

use multi_tenancy::{MultiTenancyConfig, TenantManager, TenantType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Minimal DataFusion Integration Example");

    // Example 1: Multi-tenancy Setup
    info!("=== Multi-tenancy Setup ===");

    let multi_tenancy_config = MultiTenancyConfig::default();
    let storage = Arc::new(multi_tenancy::storage::MockTenantStorage::new());
    let tenant_manager = TenantManager::new(multi_tenancy_config, storage).await?;

    // Create a test tenant
    let tenant = tenant_manager
        .create_tenant(
            "test-tenant",
            "Test Tenant",
            TenantType::Small,
            Some("user-1".to_string()),
        )
        .await?;
    info!("Created tenant: {}", tenant.id);

    // Activate the tenant
    tenant_manager.activate_tenant(&tenant.id).await?;
    info!("Activated tenant: {}", tenant.id);

    // Example 2: DataFusion Executor Setup
    info!("=== DataFusion Executor Setup ===");

    // Create DataFusion configuration with Delta Lake tables
    let config = DataFusionConfig::new()
        .with_delta_table(
            "telemetry_data".to_string(),
            "/path/to/telemetry_table".to_string(),
        )
        .with_delta_table(
            "metrics_data".to_string(),
            "/path/to/metrics_table".to_string(),
        );

    // Create DataFusion executor
    let mut executor = DataFusionExecutor::new(config);
    executor.init().await?;
    info!("DataFusion executor initialized with Delta Lake tables");

    // Example 3: Execute SQL Queries
    info!("=== Executing SQL Queries ===");

    // Simple SELECT query
    let simple_query = "SELECT * FROM telemetry_data LIMIT 10";
    info!("Executing query: {}", simple_query);

    match executor.execute_sql(simple_query).await {
        Ok(result) => {
            info!("Query executed successfully!");
            info!("Result rows: {}", result.data.len());
            info!("Execution time: {}ms", result.execution_time_ms);

            // Display first few rows
            for (i, row) in result.data.iter().take(3).enumerate() {
                info!("Row {}: {:?}", i, row.data);
            }
        }
        Err(e) => {
            warn!("Query failed: {}", e);
            info!("This is expected if Delta Lake tables don't exist in the example");
        }
    }

    // Example 4: Complex SQL Query
    info!("=== Complex SQL Query ===");

    let complex_query = r#"
        SELECT
            service_name,
            COUNT(*) as request_count,
            AVG(duration_ms) as avg_duration,
            MAX(duration_ms) as max_duration
        FROM telemetry_data
        WHERE timestamp >= '2024-01-01'
        GROUP BY service_name
        ORDER BY request_count DESC
        LIMIT 5
    "#;

    info!("Executing complex query: {}", complex_query);

    match executor.execute_sql(complex_query).await {
        Ok(result) => {
            info!("Complex query executed successfully!");
            info!("Result rows: {}", result.data.len());
            info!("Execution time: {}ms", result.execution_time_ms);
        }
        Err(e) => {
            warn!("Complex query failed: {}", e);
            info!("This is expected if Delta Lake tables don't exist in the example");
        }
    }

    // Example 5: Query Plan Analysis
    info!("=== Query Plan Analysis ===");

    let query_for_plan = "SELECT service_name, COUNT(*) FROM telemetry_data GROUP BY service_name";

    match executor.get_query_plan(query_for_plan).await {
        Ok(plan) => {
            info!("Query plan generated successfully!");
            info!("Logical plan: {}", plan);
        }
        Err(e) => {
            warn!("Failed to generate query plan: {}", e);
        }
    }

    match executor.get_optimized_query_plan(query_for_plan).await {
        Ok(optimized_plan) => {
            info!("Optimized query plan generated successfully!");
            info!("Optimized plan: {}", optimized_plan);
        }
        Err(e) => {
            warn!("Failed to generate optimized query plan: {}", e);
        }
    }

    // Example 6: Performance Monitoring
    info!("=== Performance Monitoring ===");

    // Get executor statistics
    let stats = executor.get_stats().await?;
    info!("Executor statistics:");
    info!("  Total queries: {}", stats.total_queries);
    info!(
        "  Average execution time: {:.2}ms",
        stats.avg_execution_time_ms
    );
    info!("  Error count: {}", stats.error_count);
    info!("  Last execution: {:?}", stats.last_execution_time);

    // Get registered tables
    let registered_tables = executor.get_registered_tables().await;
    info!("Registered tables: {:?}", registered_tables);

    // Example 7: Error Handling
    info!("=== Error Handling ===");

    // Try to execute a query against a non-existent table
    let invalid_query = "SELECT * FROM non_existent_table LIMIT 5";
    info!("Executing invalid query: {}", invalid_query);

    match executor.execute_sql(invalid_query).await {
        Ok(result) => {
            info!(
                "Unexpected: Invalid query succeeded with {} rows",
                result.data.len()
            );
        }
        Err(e) => {
            info!("Expected error for invalid query: {}", e);
        }
    }

    // Example 8: Batch Processing
    info!("=== Batch Processing ===");

    // Execute multiple queries in sequence
    let queries = vec![
        "SELECT COUNT(*) as total_records FROM telemetry_data",
        "SELECT service_name, COUNT(*) as service_count FROM telemetry_data GROUP BY service_name",
        "SELECT AVG(duration_ms) as avg_duration FROM telemetry_data WHERE status_code = 200",
    ];

    for (i, query) in queries.iter().enumerate() {
        info!("Executing batch query {}: {}", i + 1, query);

        match executor.execute_sql(query).await {
            Ok(result) => {
                info!(
                    "Batch query {} succeeded: {} rows, {}ms",
                    i + 1,
                    result.data.len(),
                    result.execution_time_ms
                );
            }
            Err(e) => {
                warn!("Batch query {} failed: {}", i + 1, e);
            }
        }
    }

    // Example 9: Query Engine Configuration
    info!("=== Query Engine Configuration ===");

    // Create query engine configuration
    let engine_config = QueryEngineConfig {
        enable_caching: true,
        cache_size_limit: 1000,
        enable_optimization: true,
        max_execution_time_seconds: 300,
        enable_streaming: false,
        max_result_set_size: 10000,
        enable_plan_visualization: true,
        enable_performance_monitoring: true,
        data_sources: std::collections::HashMap::new(),
    };

    info!("Query engine configuration created with DataFusion integration");
    info!("  Caching enabled: {}", engine_config.enable_caching);
    info!(
        "  Optimization enabled: {}",
        engine_config.enable_optimization
    );
    info!(
        "  Plan visualization enabled: {}",
        engine_config.enable_plan_visualization
    );
    info!(
        "  Performance monitoring enabled: {}",
        engine_config.enable_performance_monitoring
    );

    // Example 10: Multi-tenancy Integration
    info!("=== Multi-tenancy Integration ===");

    // Check tenant resource usage
    let tenant_info = tenant_manager.get_tenant(&tenant.id).await?;
    info!("Tenant resource usage:");
    info!("  Tenant ID: {}", tenant_info.id);
    info!("  Tenant name: {}", tenant_info.name);
    info!("  Tenant type: {:?}", tenant_info.tenant_type);

    // Check if tenant can perform operations
    match tenant_manager
        .check_resource_request(&tenant.id, &multi_tenancy::quotas::ResourceType::Cpu, 0.5)
        .await
    {
        Ok(_) => info!("Tenant can use 0.5 CPU cores"),
        Err(e) => warn!("Tenant cannot use 0.5 CPU cores: {}", e),
    }

    info!("=== Minimal DataFusion Integration Example Completed Successfully ===");
    info!("");
    info!("Key Features Demonstrated:");
    info!("  ✅ Multi-tenancy setup and management");
    info!("  ✅ DataFusion executor initialization");
    info!("  ✅ SQL query execution");
    info!("  ✅ Complex queries with aggregations");
    info!("  ✅ Query plan analysis and optimization");
    info!("  ✅ Performance monitoring and statistics");
    info!("  ✅ Error handling and fallbacks");
    info!("  ✅ Batch processing capabilities");
    info!("  ✅ Query engine configuration");
    info!("  ✅ Multi-tenancy resource management");
    info!("");
    info!("Next Steps:");
    info!("  1. Create actual Delta Lake tables with telemetry data");
    info!("  2. Configure real table paths in the examples");
    info!("  3. Test with production data volumes");
    info!("  4. Implement additional data source integrations");
    info!("  5. Add streaming query capabilities");
    info!("  6. Integrate with real tenant storage backends");

    Ok(())
}
