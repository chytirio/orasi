//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Snowflake connector example
//!
//! This example demonstrates how to use the Snowflake connector to write
//! and read telemetry data from Snowflake.

use bridge_core::traits::{LakehouseConnector, LakehouseReader, LakehouseWriter};
use bridge_core::types::{MetricData, MetricType, MetricValue, MetricsBatch};
use chrono::Utc;
use lakehouse_snowflake::{SnowflakeConfig, SnowflakeConnector};
use num_cpus;
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Snowflake Connector Example");
    println!("==========================");

    // Create Snowflake configuration
    let config = SnowflakeConfig {
        connection: lakehouse_snowflake::config::SnowflakeConnectionConfig {
            account: "your-account".to_string(),
            username: "your-username".to_string(),
            password: "your-password".to_string(),
            role: Some("your-role".to_string()),
            warehouse: "COMPUTE_WH".to_string(),
            database: "OPENTELEMETRY".to_string(),
            schema: "PUBLIC".to_string(),
            connection_timeout_secs: 60,
            query_timeout_secs: 300,
        },
        warehouse: lakehouse_snowflake::config::SnowflakeWarehouseConfig {
            warehouse_name: "COMPUTE_WH".to_string(),
            warehouse_size: "X-SMALL".to_string(),
            auto_suspend_secs: 300,
            auto_resume: true,
            properties: HashMap::new(),
        },
        database: lakehouse_snowflake::config::SnowflakeDatabaseConfig {
            database_name: "OPENTELEMETRY".to_string(),
            schema_name: "PUBLIC".to_string(),
            table_format: "PARQUET".to_string(),
            copy_options: "FILE_FORMAT = (TYPE = 'PARQUET')".to_string(),
        },
        writer: lakehouse_snowflake::config::SnowflakeWriterConfig {
            batch_size: 10000,
            flush_interval_ms: 5000,
            auto_scaling: true,
            enable_clustering: false,
            clustering_keys: Vec::new(),
        },
        reader: lakehouse_snowflake::config::SnowflakeReaderConfig {
            read_batch_size: 10000,
            enable_result_caching: true,
            cache_timeout_secs: 300,
            enable_query_acceleration: false,
            read_timeout_secs: 300,
        },
        schema: lakehouse_snowflake::config::SnowflakeSchemaConfig {
            enable_schema_evolution: true,
            validation_mode: "lenient".to_string(),
            enable_column_mapping: false,
            column_mapping_mode: "name".to_string(),
        },
        performance: lakehouse_snowflake::config::SnowflakePerformanceConfig {
            enable_parallel_processing: true,
            parallel_threads: num_cpus::get(),
            enable_memory_optimization: true,
            memory_limit_mb: 1000,
        },
        security: lakehouse_snowflake::config::SnowflakeSecurityConfig {
            enable_encryption_at_rest: true,
            enable_encryption_in_transit: true,
            enable_access_control: true,
            access_control_mode: Some("rbac".to_string()),
        },
    };

    println!("Connecting to Snowflake...");

    // Connect to Snowflake
    let connector = SnowflakeConnector::connect(config).await?;
    println!("✅ Connected to Snowflake successfully!");

    // Get writer and reader handles
    let writer = connector.writer().await?;
    let reader = connector.reader().await?;

    println!("✅ Got writer and reader handles");

    // Create sample metrics data
    let metrics_batch = MetricsBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        metrics: vec![
            MetricData {
                name: "cpu_usage".to_string(),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(75.5),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("service".to_string(), "web-server".to_string());
                    labels.insert("instance".to_string(), "web-01".to_string());
                    labels
                },
                timestamp: Utc::now(),
            },
            MetricData {
                name: "http_requests_total".to_string(),
                description: Some("Total HTTP requests".to_string()),
                unit: Some("requests".to_string()),
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1234.0),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("service".to_string(), "web-server".to_string());
                    labels.insert("method".to_string(), "GET".to_string());
                    labels.insert("status".to_string(), "200".to_string());
                    labels
                },
                timestamp: Utc::now(),
            },
        ],
        metadata: HashMap::new(),
    };

    println!("Writing metrics to Snowflake...");

    // Write metrics to Snowflake
    let write_result = writer.write_metrics(metrics_batch).await?;
    println!(
        "✅ Successfully wrote {} metrics records to Snowflake",
        write_result.records_written
    );

    // Create a simple query
    let query = bridge_core::types::MetricsQuery {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        filters: vec![bridge_core::types::Filter {
            field: "service".to_string(),
            operator: bridge_core::types::FilterOperator::Equals,
            value: bridge_core::types::FilterValue::String("web-server".to_string()),
        }],
        aggregations: vec![bridge_core::types::Aggregation {
            field: "value".to_string(),
            function: bridge_core::types::AggregationFunction::Average,
            alias: Some("avg_cpu_usage".to_string()),
            parameters: None,
        }],
        time_range: bridge_core::types::TimeRange {
            start: Utc::now() - chrono::Duration::hours(1),
            end: Utc::now(),
        },
        limit: Some(100),
        offset: Some(0),
        metadata: HashMap::new(),
    };

    println!("Querying metrics from Snowflake...");

    // Query metrics from Snowflake
    let query_result = reader.query_metrics(query).await?;
    println!(
        "✅ Successfully queried {} metrics records from Snowflake",
        query_result.data.len()
    );

    // Get connector statistics
    let stats = connector.get_stats().await?;
    println!("📊 Connector Statistics:");
    println!("   - Total connections: {}", stats.total_connections);
    println!("   - Active connections: {}", stats.active_connections);
    println!("   - Total writes: {}", stats.total_writes);
    println!("   - Total reads: {}", stats.total_reads);
    println!("   - Error count: {}", stats.error_count);

    // Health check
    let is_healthy = connector.health_check().await?;
    println!(
        "🏥 Connector health: {}",
        if is_healthy {
            "✅ Healthy"
        } else {
            "❌ Unhealthy"
        }
    );

    // Shutdown the connector
    connector.shutdown().await?;
    println!("✅ Snowflake connector shutdown completed");

    Ok(())
}
