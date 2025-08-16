//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Simple Apache Kafka connector example
//! 
//! This example demonstrates basic usage of the Kafka connector.

use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

use kafka_connector::{
    KafkaConnector, KafkaConfig, KafkaProducer, KafkaConsumer,
    LakehouseConnector, LakehouseWriter, LakehouseReader,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Simple Apache Kafka connector example");
    
    // Create Kafka configuration
    let config = KafkaConfig::new(
        "localhost:9092".to_string(),
        "telemetry-data".to_string(),
        "example-consumer-group".to_string(),
    );
    
    println!("📋 Kafka Configuration:");
    println!("  - Bootstrap servers: {}", config.bootstrap_servers());
    println!("  - Topic: {}", config.topic_name());
    println!("  - Consumer group: {}", config.group_id());
    
    // Example 1: Basic Kafka Connector
    println!("\n🔌 Example 1: Basic Kafka Connector");
    basic_kafka_connector_example(&config).await?;
    
    // Example 2: Producer Operations
    println!("\n📤 Example 2: Producer Operations");
    producer_example(&config).await?;
    
    // Example 3: Consumer Operations
    println!("\n📥 Example 3: Consumer Operations");
    consumer_example(&config).await?;
    
    println!("\n✅ All examples completed successfully!");
    Ok(())
}

async fn basic_kafka_connector_example(config: &KafkaConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Creating Kafka connector...");
    
    // Create and connect the connector
    let connector = KafkaConnector::connect(config.clone()).await?;
    
    println!("  ✅ Connector created successfully");
    println!("  - Name: {}", connector.name());
    println!("  - Version: {}", connector.version());
    println!("  - Connected: {}", connector.is_connected());
    
    // Health check
    let health = connector.health_check().await?;
    println!("  - Health check: {}", health);
    
    // Get statistics
    let stats = connector.get_stats().await?;
    println!("  - Total connections: {}", stats.total_connections);
    println!("  - Active connections: {}", stats.active_connections);
    println!("  - Total writes: {}", stats.total_writes);
    println!("  - Total reads: {}", stats.total_reads);
    
    // Shutdown
    connector.shutdown().await?;
    println!("  ✅ Connector shutdown completed");
    
    Ok(())
}

async fn producer_example(config: &KafkaConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Creating Kafka producer...");
    
    // Create producer
    let producer = KafkaProducer::new(config.clone()).await?;
    
    println!("  ✅ Producer created successfully");
    println!("  - Initialized: {}", producer.is_initialized());
    
    // Create sample metrics data
    let metrics_batch = create_sample_metrics_batch();
    
    // Write metrics
    println!("  📊 Writing metrics batch...");
    let metrics_result = producer.write_metrics(metrics_batch).await?;
    println!("  - Metrics written: {} records", metrics_result.records_written);
    println!("  - Metrics failed: {} records", metrics_result.records_failed);
    println!("  - Duration: {}ms", metrics_result.duration_ms);
    
    // Get producer statistics
    let stats = producer.get_stats().await?;
    println!("  📈 Producer Statistics:");
    println!("  - Total writes: {}", stats.total_writes);
    println!("  - Total records: {}", stats.total_records);
    println!("  - Writes per minute: {}", stats.writes_per_minute);
    println!("  - Records per minute: {}", stats.records_per_minute);
    println!("  - Average write time: {:.2}ms", stats.avg_write_time_ms);
    println!("  - Error count: {}", stats.error_count);
    
    // Close producer
    producer.close().await?;
    println!("  ✅ Producer closed successfully");
    
    Ok(())
}

async fn consumer_example(config: &KafkaConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Creating Kafka consumer...");
    
    // Create consumer
    let consumer = KafkaConsumer::new(config.clone()).await?;
    
    println!("  ✅ Consumer created successfully");
    println!("  - Initialized: {}", consumer.is_initialized());
    
    // Create sample query
    let metrics_query = create_sample_metrics_query();
    
    // Query metrics
    println!("  📊 Querying metrics...");
    let metrics_result = consumer.query_metrics(metrics_query).await?;
    println!("  - Query status: {:?}", metrics_result.status);
    println!("  - Query duration: {}ms", metrics_result.duration_ms);
    println!("  - Data records: {}", metrics_result.data.len());
    
    // Get consumer statistics
    let stats = consumer.get_stats().await?;
    println!("  📈 Consumer Statistics:");
    println!("  - Total reads: {}", stats.total_reads);
    println!("  - Total records: {}", stats.total_records);
    println!("  - Reads per minute: {}", stats.reads_per_minute);
    println!("  - Records per minute: {}", stats.records_per_minute);
    println!("  - Average read time: {:.2}ms", stats.avg_read_time_ms);
    println!("  - Error count: {}", stats.error_count);
    
    // Close consumer
    consumer.close().await?;
    println!("  ✅ Consumer closed successfully");
    
    Ok(())
}

// Helper functions to create sample data

fn create_sample_metrics_batch() -> bridge_core::types::MetricsBatch {
    let mut labels = HashMap::new();
    labels.insert("service".to_string(), "example-service".to_string());
    labels.insert("environment".to_string(), "production".to_string());
    
    bridge_core::types::MetricsBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        metrics: vec![
            bridge_core::types::MetricData {
                name: "cpu_usage".to_string(),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: bridge_core::types::MetricValue::Gauge(75.5),
                timestamp: Utc::now(),
                labels: labels.clone(),
            },
            bridge_core::types::MetricData {
                name: "memory_usage".to_string(),
                description: Some("Memory usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: bridge_core::types::MetricValue::Gauge(60.2),
                timestamp: Utc::now(),
                labels: labels.clone(),
            },
            bridge_core::types::MetricData {
                name: "request_count".to_string(),
                description: Some("Total request count".to_string()),
                unit: Some("requests".to_string()),
                metric_type: bridge_core::types::MetricType::Counter,
                value: bridge_core::types::MetricValue::Counter(1000.0),
                timestamp: Utc::now(),
                labels,
            },
        ],
        metadata: HashMap::new(),
    }
}

fn create_sample_metrics_query() -> bridge_core::types::MetricsQuery {
    bridge_core::types::MetricsQuery {
        id: Uuid::new_v4(),
        filters: vec![],
        aggregations: vec![],
        time_range: bridge_core::types::TimeRange {
            start: Utc::now() - chrono::Duration::hours(1),
            end: Utc::now(),
        },
        limit: Some(100),
        offset: Some(0),
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    }
}
