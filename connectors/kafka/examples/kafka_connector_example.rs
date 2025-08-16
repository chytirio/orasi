//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka connector example
//! 
//! This example demonstrates how to use the Kafka connector for
//! producing and consuming telemetry data.

use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use chrono::Utc;

use kafka_connector::{
    KafkaConnector, KafkaConfig, KafkaProducer, KafkaConsumer,
    RealKafkaExporter, KafkaMetrics, KafkaCluster, KafkaTopic,
    LakehouseConnector, LakehouseWriter, LakehouseReader, LakehouseExporter,
};
use bridge_core::types::{
    MetricsBatch, TracesBatch, LogsBatch, TelemetryData,
    MetricData, MetricType, MetricValue, TraceData, LogData,
    ProcessedBatch, ProcessedRecord, ProcessingStatus,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    ).unwrap();
    
    println!("🚀 Starting Apache Kafka connector example");
    
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
    
    // Example 4: Real Kafka Exporter
    println!("\n📊 Example 4: Real Kafka Exporter");
    exporter_example(&config).await?;
    
    // Example 5: Metrics Collection
    println!("\n📈 Example 5: Metrics Collection");
    metrics_example().await?;
    
    // Example 6: Cluster and Topic Management
    println!("\n🏗️  Example 6: Cluster and Topic Management");
    cluster_topic_example(&config).await?;
    
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
    let traces_batch = create_sample_traces_batch();
    let logs_batch = create_sample_logs_batch();
    
    // Write metrics
    println!("  📊 Writing metrics batch...");
    let metrics_result = producer.write_metrics(metrics_batch).await?;
    println!("  - Metrics written: {} records", metrics_result.records_written);
    println!("  - Metrics failed: {} records", metrics_result.records_failed);
    println!("  - Duration: {}ms", metrics_result.duration_ms);
    
    // Write traces
    println!("  🔍 Writing traces batch...");
    let traces_result = producer.write_traces(traces_batch).await?;
    println!("  - Traces written: {} records", traces_result.records_written);
    println!("  - Traces failed: {} records", traces_result.records_failed);
    println!("  - Duration: {}ms", traces_result.duration_ms);
    
    // Write logs
    println!("  📝 Writing logs batch...");
    let logs_result = producer.write_logs(logs_batch).await?;
    println!("  - Logs written: {} records", logs_result.records_written);
    println!("  - Logs failed: {} records", logs_result.records_failed);
    println!("  - Duration: {}ms", logs_result.duration_ms);
    
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
    
    // Create sample queries
    let metrics_query = create_sample_metrics_query();
    let traces_query = create_sample_traces_query();
    let logs_query = create_sample_logs_query();
    
    // Query metrics
    println!("  📊 Querying metrics...");
    let metrics_result = consumer.query_metrics(metrics_query).await?;
    println!("  - Metrics found: {} records", metrics_result.metrics.len());
    println!("  - Query duration: {}ms", metrics_result.duration_ms);
    
    // Query traces
    println!("  🔍 Querying traces...");
    let traces_result = consumer.query_traces(traces_query).await?;
    println!("  - Traces found: {} records", traces_result.traces.len());
    println!("  - Query duration: {}ms", traces_result.duration_ms);
    
    // Query logs
    println!("  📝 Querying logs...");
    let logs_result = consumer.query_logs(logs_query).await?;
    println!("  - Logs found: {} records", logs_result.logs.len());
    println!("  - Query duration: {}ms", logs_result.duration_ms);
    
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

async fn exporter_example(config: &KafkaConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Creating Kafka exporter...");
    
    // Create exporter
    let mut exporter = RealKafkaExporter::new(config.clone()).await?;
    
    println!("  ✅ Exporter created successfully");
    println!("  - Name: {}", exporter.name());
    println!("  - Version: {}", exporter.version());
    println!("  - Running: {}", exporter.is_running());
    
    // Initialize exporter
    exporter.initialize().await?;
    println!("  ✅ Exporter initialized");
    
    // Create sample processed batch
    let processed_batch = create_sample_processed_batch();
    
    // Export batch
    println!("  📤 Exporting processed batch...");
    let export_result = exporter.export(processed_batch).await?;
    println!("  - Export status: {:?}", export_result.status);
    println!("  - Records exported: {}", export_result.records_exported);
    println!("  - Records failed: {}", export_result.records_failed);
    println!("  - Duration: {}ms", export_result.duration_ms);
    println!("  - Errors: {}", export_result.errors.len());
    
    // Health check
    let health = exporter.health_check().await?;
    println!("  - Health check: {}", health);
    
    // Get statistics
    let stats = exporter.get_stats().await?;
    println!("  📈 Exporter Statistics:");
    println!("  - Total batches: {}", stats.total_batches);
    println!("  - Total records: {}", stats.total_records);
    println!("  - Batches per minute: {}", stats.batches_per_minute);
    println!("  - Records per minute: {}", stats.records_per_minute);
    println!("  - Average export time: {:.2}ms", stats.avg_export_time_ms);
    println!("  - Error count: {}", stats.error_count);
    
    // Shutdown exporter
    exporter.shutdown().await?;
    println!("  ✅ Exporter shutdown completed");
    
    Ok(())
}

async fn metrics_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Creating Kafka metrics collector...");
    
    // Create metrics collector
    let metrics = KafkaMetrics::new();
    
    println!("  ✅ Metrics collector created successfully");
    
    // Simulate some operations
    println!("  📊 Recording metrics operations...");
    
    // Record produce operations
    metrics.record_produce(100, 10240, 50); // 100 messages, 10KB, 50ms
    metrics.record_produce(200, 20480, 75); // 200 messages, 20KB, 75ms
    metrics.record_produce(150, 15360, 60); // 150 messages, 15KB, 60ms
    
    // Record consume operations
    metrics.record_consume(80, 8192, 40);   // 80 messages, 8KB, 40ms
    metrics.record_consume(120, 12288, 55); // 120 messages, 12KB, 55ms
    
    // Record some errors
    metrics.record_produce_error();
    metrics.record_consume_error();
    
    // Get metrics snapshot
    let snapshot = metrics.get_snapshot();
    println!("  📈 Metrics Snapshot:");
    println!("  - Total produces: {}", snapshot.total_produces);
    println!("  - Total consumes: {}", snapshot.total_consumes);
    println!("  - Messages produced: {}", snapshot.total_messages_produced);
    println!("  - Messages consumed: {}", snapshot.total_messages_consumed);
    println!("  - Bytes produced: {}", snapshot.total_bytes_produced);
    println!("  - Bytes consumed: {}", snapshot.total_bytes_consumed);
    println!("  - Produce errors: {}", snapshot.produce_errors);
    println!("  - Consume errors: {}", snapshot.consume_errors);
    println!("  - Average produce time: {}ms", snapshot.avg_produce_time_ms);
    println!("  - Average consume time: {}ms", snapshot.avg_consume_time_ms);
    
    Ok(())
}

async fn cluster_topic_example(config: &KafkaConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Creating Kafka cluster and topic managers...");
    
    // Create cluster configuration
    let cluster_config = kafka_connector::cluster::KafkaClusterConfig {
        name: "example-cluster".to_string(),
        bootstrap_servers: config.bootstrap_servers().to_string(),
        brokers: vec![
            kafka_connector::cluster::KafkaBroker {
                id: 1,
                host: "localhost".to_string(),
                port: 9092,
                rack: Some("rack-1".to_string()),
            }
        ],
        settings: HashMap::new(),
    };
    
    // Create cluster instance
    let mut cluster = KafkaCluster::new(cluster_config);
    println!("  ✅ Cluster created successfully");
    println!("  - Name: {}", cluster.config().name);
    println!("  - Bootstrap servers: {}", cluster.config().bootstrap_servers);
    println!("  - Brokers: {}", cluster.config().brokers.len());
    
    // Initialize cluster
    cluster.initialize().await?;
    println!("  ✅ Cluster initialized successfully");
    
    // Create topic configuration
    let topic_config = kafka_connector::topic::KafkaTopicConfig {
        name: config.topic_name().to_string(),
        partitions: config.partitions(),
        replication_factor: config.replication_factor(),
        config: HashMap::new(),
    };
    
    // Create topic instance
    let mut topic = KafkaTopic::new(topic_config);
    println!("  ✅ Topic created successfully");
    println!("  - Name: {}", topic.config().name);
    println!("  - Partitions: {}", topic.config().partitions);
    println!("  - Replication factor: {}", topic.config().replication_factor);
    
    // Initialize topic
    topic.initialize().await?;
    println!("  ✅ Topic initialized successfully");
    
    // Check if topic exists
    let exists = topic.exists().await?;
    println!("  - Topic exists: {}", exists);
    
    // Get topic metadata
    if let Some(metadata) = topic.metadata() {
        println!("  📊 Topic Metadata:");
        println!("  - Total messages: {}", metadata.total_messages);
        println!("  - Total size: {} bytes", metadata.total_size_bytes);
        println!("  - Last updated: {}", metadata.last_updated);
    }
    
    Ok(())
}

// Helper functions to create sample data

fn create_sample_metrics_batch() -> MetricsBatch {
    let mut labels = HashMap::new();
    labels.insert("service".to_string(), "example-service".to_string());
    labels.insert("environment".to_string(), "production".to_string());
    
    MetricsBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        metrics: vec![
            MetricData {
                name: "cpu_usage".to_string(),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(75.5),
                timestamp: Utc::now(),
                labels: labels.clone(),
            },
            MetricData {
                name: "memory_usage".to_string(),
                description: Some("Memory usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(60.2),
                timestamp: Utc::now(),
                labels: labels.clone(),
            },
            MetricData {
                name: "request_count".to_string(),
                description: Some("Total request count".to_string()),
                unit: Some("requests".to_string()),
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1000.0),
                timestamp: Utc::now(),
                labels,
            },
        ],
        metadata: HashMap::new(),
    }
}

fn create_sample_traces_batch() -> TracesBatch {
    TracesBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        traces: vec![
            TraceData {
                trace_id: Uuid::new_v4(),
                span_id: Uuid::new_v4(),
                parent_span_id: None,
                name: "example-operation".to_string(),
                kind: "internal".to_string(),
                start_time: Utc::now(),
                end_time: Utc::now(),
                attributes: HashMap::new(),
                events: vec![],
                links: vec![],
                status: "ok".to_string(),
                status_message: None,
            }
        ],
        metadata: HashMap::new(),
    }
}

fn create_sample_logs_batch() -> LogsBatch {
    LogsBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        logs: vec![
            LogData {
                timestamp: Utc::now(),
                level: "INFO".to_string(),
                message: "Example log message".to_string(),
                attributes: HashMap::new(),
                resource: None,
                trace_id: None,
                span_id: None,
            }
        ],
        metadata: HashMap::new(),
    }
}

fn create_sample_processed_batch() -> ProcessedBatch {
    let mut labels = HashMap::new();
    labels.insert("service".to_string(), "example-service".to_string());
    
    ProcessedBatch {
        original_batch_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        status: ProcessingStatus::Success,
        records: vec![
            ProcessedRecord {
                original_id: Uuid::new_v4(),
                status: ProcessingStatus::Success,
                transformed_data: Some(TelemetryData::Metric(MetricData {
                    name: "processed_metric".to_string(),
                    description: Some("Processed metric example".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: MetricType::Counter,
                    value: MetricValue::Counter(42.0),
                    timestamp: Utc::now(),
                    labels,
                })),
                metadata: HashMap::new(),
                errors: vec![],
            }
        ],
        metadata: HashMap::new(),
        errors: vec![],
    }
}

fn create_sample_metrics_query() -> bridge_core::types::MetricsQuery {
    bridge_core::types::MetricsQuery {
        id: uuid::Uuid::new_v4(),
        filters: vec![],
        aggregations: vec![],
        time_range: bridge_core::types::TimeRange {
            start: chrono::Utc::now() - chrono::Duration::hours(1),
            end: chrono::Utc::now(),
        },
        limit: Some(100),
        offset: Some(0),
        metadata: std::collections::HashMap::new(),
        timestamp: chrono::Utc::now(),
    }
}

fn create_sample_traces_query() -> bridge_core::types::TracesQuery {
    bridge_core::types::TracesQuery {
        id: uuid::Uuid::new_v4(),
        filters: vec![],
        time_range: bridge_core::types::TimeRange {
            start: chrono::Utc::now() - chrono::Duration::hours(1),
            end: chrono::Utc::now(),
        },
        limit: Some(100),
        offset: Some(0),
        metadata: std::collections::HashMap::new(),
        timestamp: chrono::Utc::now(),
        operation_name: None,
        service_name: None,
        trace_id: None,
        span_id: None,
        parent_span_id: None,
    }
}

fn create_sample_logs_query() -> bridge_core::types::LogsQuery {
    bridge_core::types::LogsQuery {
        id: uuid::Uuid::new_v4(),
        filters: vec![],
        time_range: bridge_core::types::TimeRange {
            start: chrono::Utc::now() - chrono::Duration::hours(1),
            end: chrono::Utc::now(),
        },
        limit: Some(100),
        offset: Some(0),
        metadata: std::collections::HashMap::new(),
        timestamp: chrono::Utc::now(),
        log_level: None,
        service_name: None,
    }
}
