//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating DataFusion-based querying across multiple data sources
//!
//! This example shows how DataFusion can be used to query:
//! - In-memory telemetry streams
//! - Delta Lake tables
//! - S3 Parquet files
//! - And combine them all with SQL

use bridge_core::types::{MetricData, MetricType, MetricValue};
use bridge_core::types::{TelemetryData, TelemetryRecord, TelemetryType};
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use query_engine::executors::datafusion_executor::{DataFusionConfig, DataFusionExecutor};
use query_engine::parsers::{ParsedQuery, QueryAst};
use query_engine::QueryExecutor;
use std::collections::HashMap;
use tracing::{error, info};
use uuid::Uuid;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting DataFusion query example");

    // Create DataFusion executor
    let config = DataFusionConfig::new();
    let mut executor = DataFusionExecutor::new(config);
    executor.init().await?;

    // Create sample telemetry data
    let telemetry_batch = create_sample_telemetry_batch().await?;

    // Register telemetry batch as a table in DataFusion
    executor.register_batch(&telemetry_batch).await?;

    // Example 1: Simple SQL query on telemetry data
    info!("Example 1: Simple SQL query on telemetry data");
    let sql_query = "
        SELECT 
            record_type,
            COUNT(*) as count,
            AVG(CAST(value AS DOUBLE)) as avg_value
        FROM telemetry_batch_{}
        GROUP BY record_type
        ORDER BY count DESC
    "
    .replace("{}", &telemetry_batch.id.to_string());

    let result = executor.execute_sql(&sql_query).await?;
    info!("Query result: {} rows", result.data.len());
    for row in &result.data {
        info!("Row: {:?}", row.data);
    }

    // Example 2: Time-based analysis
    info!("Example 2: Time-based analysis");
    let time_query = "
        SELECT 
            DATE_TRUNC('minute', timestamp) as minute,
            record_type,
            COUNT(*) as count
        FROM telemetry_batch_{}
        WHERE timestamp >= NOW() - INTERVAL '1 hour'
        GROUP BY DATE_TRUNC('minute', timestamp), record_type
        ORDER BY minute DESC
    "
    .replace("{}", &telemetry_batch.id.to_string());

    let time_result = executor.execute_sql(&time_query).await?;
    info!("Time analysis result: {} rows", time_result.data.len());

    // Example 3: Complex aggregation with window functions
    info!("Example 3: Complex aggregation with window functions");
    let window_query = "
        SELECT 
            record_type,
            value,
            timestamp,
            ROW_NUMBER() OVER (PARTITION BY record_type ORDER BY timestamp) as row_num,
            LAG(value) OVER (PARTITION BY record_type ORDER BY timestamp) as prev_value
        FROM telemetry_batch_{}
        WHERE record_type = 'Metric'
        ORDER BY timestamp
    "
    .replace("{}", &telemetry_batch.id.to_string());

    let window_result = executor.execute_sql(&window_query).await?;
    info!("Window function result: {} rows", window_result.data.len());

    // Example 4: Demonstrate how this would work with Delta Lake
    info!("Example 4: Delta Lake integration (conceptual)");
    info!("In a real implementation, you would:");
    info!("1. Register Delta Lake tables using DataFusion's Delta Lake connector");
    info!("2. Query across both in-memory streams and persistent Delta Lake tables");
    info!("3. Join telemetry streams with historical data");

    // Example SQL that would work with Delta Lake integration:
    let delta_lake_query = "
        -- This would work when Delta Lake tables are registered
        SELECT 
            s.record_type,
            s.value as stream_value,
            h.avg_value as historical_avg,
            (s.value - h.avg_value) as deviation
        FROM telemetry_batch_{} s
        JOIN delta_lake_metrics h ON s.record_type = h.record_type
        WHERE s.timestamp >= NOW() - INTERVAL '5 minutes'
        ORDER BY deviation DESC
    "
    .replace("{}", &telemetry_batch.id.to_string());

    info!("Delta Lake integration query would look like:");
    info!("{}", delta_lake_query);

    // Example 5: Show how to extend with custom functions
    info!("Example 5: Custom functions for telemetry analysis");
    info!("DataFusion allows registering custom UDFs for:");
    info!("- Anomaly detection algorithms");
    info!("- Telemetry-specific aggregations");
    info!("- Time series analysis functions");
    info!("- Statistical analysis");

    // Get executor statistics
    let stats = executor.get_stats().await?;
    info!("Executor statistics:");
    info!("  Total queries: {}", stats.total_queries);
    info!(
        "  Average execution time: {:.2}ms",
        stats.avg_execution_time_ms
    );
    info!("  Error count: {}", stats.error_count);

    info!("DataFusion query example completed successfully");
    Ok(())
}

/// Create sample telemetry batch for demonstration
async fn create_sample_telemetry_batch() -> BridgeResult<TelemetryBatch> {
    let mut records = Vec::new();

    // Create sample metrics
    for i in 0..100 {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now() - chrono::Duration::minutes(i as i64),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("cpu_usage_{}", i % 5),
                description: Some(format!("CPU usage metric {}", i)),
                unit: Some("%".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(50.0 + (i as f64 * 0.5) % 50.0),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("host".to_string(), format!("host-{}", i % 3));
                    labels.insert("service".to_string(), format!("service-{}", i % 2));
                    labels
                },
                timestamp: Utc::now() - chrono::Duration::minutes(i as i64),
            }),
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("environment".to_string(), "production".to_string());
                attrs.insert("version".to_string(), "1.0.0".to_string());
                attrs
            },
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        records.push(record);
    }

    // Create sample traces
    for i in 0..50 {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now() - chrono::Duration::minutes(i as i64),
            record_type: TelemetryType::Trace,
            data: TelemetryData::Trace(bridge_core::types::TraceData {
                trace_id: format!("trace-{}", i),
                span_id: format!("span-{}", i),
                parent_span_id: None,
                name: format!("http_request_{}", i % 3),
                kind: bridge_core::types::SpanKind::Server,
                start_time: Utc::now() - chrono::Duration::minutes(i as i64),
                end_time: Some(
                    Utc::now() - chrono::Duration::minutes(i as i64)
                        + chrono::Duration::milliseconds(100),
                ),
                duration_ns: Some(100_000_000), // 100ms
                status: bridge_core::types::SpanStatus {
                    code: bridge_core::types::StatusCode::Ok,
                    message: None,
                },
                attributes: {
                    let mut attrs = HashMap::new();
                    attrs.insert("http.method".to_string(), "GET".to_string());
                    attrs.insert("http.status_code".to_string(), "200".to_string());
                    attrs
                },
                events: Vec::new(),
                links: Vec::new(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        records.push(record);
    }

    // Create sample logs
    for i in 0..75 {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now() - chrono::Duration::minutes(i as i64),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(bridge_core::types::LogData {
                timestamp: Utc::now() - chrono::Duration::minutes(i as i64),
                level: bridge_core::types::LogLevel::Info,
                message: format!("Application log message {}", i),
                attributes: {
                    let mut attrs = HashMap::new();
                    attrs.insert("logger".to_string(), "app".to_string());
                    attrs.insert("thread".to_string(), format!("thread-{}", i % 4));
                    attrs
                },
                body: None,
                severity_number: Some(9),
                severity_text: Some("INFO".to_string()),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        records.push(record);
    }

    Ok(TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "datafusion_example".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    })
}
