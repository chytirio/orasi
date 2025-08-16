//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Kafka Connector Example
//! 
//! This example demonstrates how to use the Kafka connector with the
//! OpenTelemetry Data Lake Bridge architecture.

use std::collections::HashMap;
use std::sync::Arc;
use tokio;
use tracing::{info, error, warn};
use bridge_core::traits::{LakehouseConnector, LakehouseWriter, LakehouseReader};
use bridge_core::types::{
    TelemetryBatch, TelemetryRecord, TelemetryType, TelemetryData,
    MetricsBatch, TracesBatch, LogsBatch,
    MetricData, TraceData, LogData,
    MetricsQuery, TracesQuery, LogsQuery,
    ResourceInfo, ServiceInfo
};
use kafka_connector::{KafkaConnector, KafkaConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting Kafka Connector Example");
    
    // Create Kafka configuration
    let config = KafkaConfig {
        bootstrap_servers: vec!["localhost:9092".to_string()],
        topic_name: "telemetry-data".to_string(),
        producer_config: HashMap::new(),
        consumer_config: HashMap::new(),
        security_config: None,
    };
    
    info!("Connecting to Kafka with config: {:?}", config);
    
    // Connect to Kafka
    let connector = KafkaConnector::connect(config).await?;
    info!("Successfully connected to Kafka");
    
    // Get writer and reader handles
    let writer = connector.writer().await?;
    let reader = connector.reader().await?;
    
    // Create sample telemetry data
    let telemetry_batch = create_sample_telemetry_batch();
    info!("Created sample telemetry batch with {} records", telemetry_batch.total_records());
    
    // Write telemetry data to Kafka
    info!("Writing telemetry batch to Kafka...");
    let write_result = writer.write_batch(telemetry_batch).await?;
    info!("Successfully wrote {} records to Kafka", write_result.records_written);
    
    // Flush to ensure data is written
    writer.flush().await?;
    info!("Flushed writer");
    
    // Query data from Kafka
    info!("Querying data from Kafka...");
    
    // Query metrics
    let metrics_query = MetricsQuery {
        time_range: None,
        filters: vec![],
        limit: Some(10),
    };
    let metrics_result = reader.query_metrics(metrics_query).await?;
    info!("Retrieved {} metrics records", metrics_result.total_count);
    
    // Query traces
    let traces_query = TracesQuery {
        time_range: None,
        filters: vec![],
        limit: Some(10),
    };
    let traces_result = reader.query_traces(traces_query).await?;
    info!("Retrieved {} trace records", traces_result.total_count);
    
    // Query logs
    let logs_query = LogsQuery {
        time_range: None,
        filters: vec![],
        limit: Some(10),
    };
    let logs_result = reader.query_logs(logs_query).await?;
    info!("Retrieved {} log records", logs_result.total_count);
    
    // Get connector statistics
    let stats = connector.get_stats().await?;
    info!("Connector stats: {:?}", stats);
    
    // Perform health check
    let is_healthy = connector.health_check().await?;
    info!("Connector health check: {}", is_healthy);
    
    // Get writer and reader statistics
    let writer_stats = writer.get_stats().await?;
    let reader_stats = reader.get_stats().await?;
    info!("Writer stats: {:?}", writer_stats);
    info!("Reader stats: {:?}", reader_stats);
    
    // Close writer and reader
    writer.close().await?;
    reader.close().await?;
    info!("Closed writer and reader");
    
    // Shutdown connector
    connector.shutdown().await?;
    info!("Shutdown connector");
    
    info!("Kafka Connector Example completed successfully");
    Ok(())
}

/// Create a sample telemetry batch for testing
fn create_sample_telemetry_batch() -> TelemetryBatch {
    use chrono::Utc;
    use uuid::Uuid;
    
    let mut records = Vec::new();
    
    // Create sample metrics
    for i in 0..5 {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("cpu_usage_{}", i),
                description: Some(format!("CPU usage for service {}", i)),
                unit: Some("percent".to_string()),
                value: 75.5 + (i as f64 * 5.0),
                attributes: HashMap::new(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: Some(ResourceInfo {
                service_name: format!("service-{}", i),
                service_version: "1.0.0".to_string(),
                environment: "production".to_string(),
                attributes: HashMap::new(),
            }),
            service: Some(ServiceInfo {
                name: format!("service-{}", i),
                version: "1.0.0".to_string(),
                namespace: "default".to_string(),
                instance_id: format!("instance-{}", i),
            }),
        };
        records.push(record);
    }
    
    // Create sample traces
    for i in 0..3 {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Trace,
            data: TelemetryData::Trace(TraceData {
                trace_id: format!("trace-{}", i),
                span_id: format!("span-{}", i),
                parent_span_id: if i > 0 { Some(format!("span-{}", i - 1)) } else { None },
                name: format!("http_request_{}", i),
                kind: "server".to_string(),
                status: "ok".to_string(),
                start_time: Utc::now(),
                end_time: Utc::now(),
                duration_ms: 100.0 + (i as f64 * 50.0),
                attributes: HashMap::new(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: Some(ResourceInfo {
                service_name: format!("service-{}", i),
                service_version: "1.0.0".to_string(),
                environment: "production".to_string(),
                attributes: HashMap::new(),
            }),
            service: Some(ServiceInfo {
                name: format!("service-{}", i),
                version: "1.0.0".to_string(),
                namespace: "default".to_string(),
                instance_id: format!("instance-{}", i),
            }),
        };
        records.push(record);
    }
    
    // Create sample logs
    for i in 0..4 {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                level: if i % 2 == 0 { "info".to_string() } else { "warn".to_string() },
                message: format!("Sample log message {}", i),
                timestamp: Utc::now(),
                attributes: HashMap::new(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: Some(ResourceInfo {
                service_name: format!("service-{}", i),
                service_version: "1.0.0".to_string(),
                environment: "production".to_string(),
                attributes: HashMap::new(),
            }),
            service: Some(ServiceInfo {
                name: format!("service-{}", i),
                version: "1.0.0".to_string(),
                namespace: "default".to_string(),
                instance_id: format!("instance-{}", i),
            }),
        };
        records.push(record);
    }
    
    TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "kafka-example".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_kafka_connector_example() {
        // This test would require a running Kafka instance
        // For now, we'll just test that the code compiles
        let batch = create_sample_telemetry_batch();
        assert_eq!(batch.total_records(), 12); // 5 metrics + 3 traces + 4 logs
    }
}
