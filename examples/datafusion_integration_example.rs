//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! DataFusion Integration Example
//!
//! This example demonstrates the complete DataFusion integration with the Orasi query engine,
//! showing how to execute SQL queries against telemetry data using DataFusion.

use bridge_core::{
    types::{
        MetricData, MetricType, MetricValue, TelemetryBatch, TelemetryData, TelemetryRecord,
        TelemetryType,
    },
    BridgeResult,
};
use chrono::Utc;
use query_engine::{
    executors::datafusion_executor::{DataFusionConfig, DataFusionExecutor},
    DataSourceManager, DeltaLakeDataSourceConfig, QueryEngineConfig, QueryExecutor,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting DataFusion Integration Example");

    // Create DataFusion executor configuration
    let config = DataFusionConfig::new().with_delta_table(
        "telemetry_data".to_string(),
        "/path/to/delta/table".to_string(),
    );

    // Create and initialize DataFusion executor
    let mut executor = DataFusionExecutor::new(config);
    executor.init().await?;
    info!("DataFusion executor initialized successfully");

    // Create sample telemetry data
    let telemetry_batch = create_sample_telemetry_batch();
    info!(
        "Created sample telemetry batch with {} records",
        telemetry_batch.records.len()
    );

    // Register telemetry batch with DataFusion
    executor.register_batch(&telemetry_batch).await?;
    info!("Registered telemetry batch with DataFusion");

    // Example 1: Simple SELECT query
    info!("=== Example 1: Simple SELECT Query ===");
    let simple_query = "SELECT * FROM telemetry_batch_* LIMIT 5";
    let result = executor.execute_sql(simple_query).await?;
    info!("Query executed successfully:");
    info!("  - Rows returned: {}", result.data.len());
    info!("  - Execution time: {}ms", result.execution_time_ms);
    info!(
        "  - Query type: {}",
        result
            .metadata
            .get("query_type")
            .unwrap_or(&"unknown".to_string())
    );

    // Example 2: Aggregation query
    info!("=== Example 2: Aggregation Query ===");
    let agg_query =
        "SELECT record_type, COUNT(*) as count FROM telemetry_batch_* GROUP BY record_type";
    let agg_result = executor.execute_sql(agg_query).await?;
    info!("Aggregation query executed successfully:");
    info!("  - Rows returned: {}", agg_result.data.len());
    info!("  - Execution time: {}ms", agg_result.execution_time_ms);

    // Example 3: Filtered query
    info!("=== Example 3: Filtered Query ===");
    let filter_query = "SELECT * FROM telemetry_batch_* WHERE record_type = 'Metric' LIMIT 3";
    let filter_result = executor.execute_sql(filter_query).await?;
    info!("Filtered query executed successfully:");
    info!("  - Rows returned: {}", filter_result.data.len());
    info!("  - Execution time: {}ms", filter_result.execution_time_ms);

    // Example 4: Query plan generation
    info!("=== Example 4: Query Plan Generation ===");
    let plan_query = "SELECT record_type, COUNT(*) FROM telemetry_batch_* GROUP BY record_type";
    let plan = executor.get_query_plan(plan_query).await?;
    info!("Query plan generated:");
    info!("{}", plan);

    // Example 5: Optimized query plan
    info!("=== Example 5: Optimized Query Plan ===");
    let optimized_plan = executor.get_optimized_query_plan(plan_query).await?;
    info!("Optimized query plan:");
    info!("{}", optimized_plan);

    // Example 6: Execute query against specific Delta Lake table
    info!("=== Example 6: Delta Lake Table Query ===");
    let delta_query = "SELECT * FROM telemetry_data LIMIT 10";
    let delta_result = executor
        .execute_delta_lake_query("telemetry_data", delta_query)
        .await;
    match delta_result {
        Ok(result) => {
            info!("Delta Lake query executed successfully:");
            info!("  - Rows returned: {}", result.data.len());
            info!("  - Execution time: {}ms", result.execution_time_ms);
        }
        Err(e) => {
            info!("Delta Lake query failed (expected with mock data): {}", e);
        }
    }

    // Example 7: Get executor statistics
    info!("=== Example 7: Executor Statistics ===");
    let stats = executor.get_stats().await?;
    info!("Executor statistics:");
    info!("  - Total queries: {}", stats.total_queries);
    info!(
        "  - Average execution time: {:.2}ms",
        stats.avg_execution_time_ms
    );
    info!("  - Error count: {}", stats.error_count);
    info!("  - Last execution: {:?}", stats.last_execution_time);

    // Example 8: Complex query with multiple operations
    info!("=== Example 8: Complex Query ===");
    let complex_query = r#"
        SELECT 
            record_type,
            COUNT(*) as total_records,
            AVG(CAST(value AS DOUBLE)) as avg_value
        FROM telemetry_batch_*
        WHERE record_type IN ('Metric', 'Trace')
        GROUP BY record_type
        ORDER BY total_records DESC
    "#;
    let complex_result = executor.execute_sql(complex_query).await?;
    info!("Complex query executed successfully:");
    info!("  - Rows returned: {}", complex_result.data.len());
    info!("  - Execution time: {}ms", complex_result.execution_time_ms);

    // Display some sample results
    info!("=== Sample Query Results ===");
    for (i, row) in complex_result.data.iter().take(3).enumerate() {
        info!("Row {}: {:?}", i + 1, row.data);
    }

    info!("DataFusion Integration Example completed successfully!");
    Ok(())
}

/// Create a sample telemetry batch for testing
fn create_sample_telemetry_batch() -> TelemetryBatch {
    let mut records = Vec::new();

    // Create metric records
    for i in 0..10 {
        records.push(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("cpu_usage_{}", i),
                description: Some(format!("CPU usage for service {}", i)),
                unit: Some("percent".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(50.0 + (i as f64 * 5.0)),
                labels: HashMap::from([
                    ("service".to_string(), format!("service-{}", i)),
                    ("environment".to_string(), "production".to_string()),
                ]),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        });
    }

    // Create trace records
    for i in 0..5 {
        records.push(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Trace,
            data: TelemetryData::Trace(bridge_core::types::TraceData {
                trace_id: Uuid::new_v4().to_string(),
                span_id: Uuid::new_v4().to_string(),
                parent_span_id: None,
                name: "datafusion_query".to_string(),
                kind: bridge_core::types::SpanKind::Internal,
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                duration_ns: Some(100_000_000),
                attributes: HashMap::new(),
                events: vec![],
                links: vec![],
                status: bridge_core::types::SpanStatus {
                    code: bridge_core::types::StatusCode::Ok,
                    message: Some("Query executed successfully".to_string()),
                },
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        });
    }

    // Create log records
    for i in 0..3 {
        records.push(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(bridge_core::types::LogData {
                timestamp: Utc::now(),
                level: bridge_core::types::LogLevel::Info,
                message: format!("Log message {}", i),
                attributes: HashMap::new(),
                body: Some(format!("Log message {}", i)),
                severity_number: Some(1),
                severity_text: Some("INFO".to_string()),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        });
    }

    TelemetryBatch {
        id: Uuid::new_v4(),
        source: "example_service".to_string(),
        timestamp: Utc::now(),
        size: records.len(),
        records,
        metadata: HashMap::from([
            ("example".to_string(), "datafusion_integration".to_string()),
            ("version".to_string(), "1.0.0".to_string()),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_datafusion_integration_example() {
        // This test verifies that the example can run without errors
        let result = main().await;
        assert!(
            result.is_ok(),
            "Example should run successfully: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_sample_telemetry_batch_creation() {
        let batch = create_sample_telemetry_batch();
        assert_eq!(batch.records.len(), 18); // 10 metrics + 5 traces + 3 logs
        assert_eq!(batch.source, "example_service");
        assert!(!batch.records.is_empty());
    }
}
