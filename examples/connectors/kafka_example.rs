//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka connector example
//! 
//! This example demonstrates how to use the Apache Kafka connector
//! to export telemetry data to Kafka topics.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use bridge_core::traits::{LakehouseConnector, LakehouseWriter, LakehouseReader};
use bridge_core::types::{TelemetryBatch, MetricsBatch, MetricData, MetricType, MetricValue, TracesBatch, TraceData, LogsBatch, LogData, TelemetryRecord, TelemetryType, TelemetryData};
use bridge_core::error::BridgeResult;

use kafka_connector::{KafkaConnector, KafkaConfig};

/// Mock telemetry data generator for testing
struct MockTelemetryGenerator {
    batch_count: usize,
    max_batches: usize,
}

impl MockTelemetryGenerator {
    fn new(max_batches: usize) -> Self {
        Self {
            batch_count: 0,
            max_batches,
        }
    }

    fn generate_metrics_batch(&mut self) -> MetricsBatch {
        self.batch_count += 1;
        
        let metrics = vec![
            MetricData {
                name: format!("cpu_usage_batch_{}", self.batch_count),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("%".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(75.5 + (self.batch_count as f64 * 0.1)),
                labels: [("service".to_string(), "web-server".to_string())].into_iter().collect(),
                timestamp: chrono::Utc::now(),
            },
            MetricData {
                name: format!("memory_usage_batch_{}", self.batch_count),
                description: Some("Memory usage percentage".to_string()),
                unit: Some("%".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(60.2 + (self.batch_count as f64 * 0.05)),
                labels: [("service".to_string(), "web-server".to_string())].into_iter().collect(),
                timestamp: chrono::Utc::now(),
            },
        ];

        MetricsBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            metrics,
            metadata: [("source".to_string(), "mock_generator".to_string())].into_iter().collect(),
        }
    }

    fn generate_traces_batch(&mut self) -> TracesBatch {
        self.batch_count += 1;
        
        let traces = vec![
            TraceData {
                trace_id: format!("trace_{}", self.batch_count),
                span_id: format!("span_{}", self.batch_count),
                parent_span_id: None,
                name: format!("http_request_batch_{}", self.batch_count),
                kind: "server".to_string(),
                start_time: chrono::Utc::now(),
                end_time: chrono::Utc::now(),
                attributes: [("http.method".to_string(), "GET".to_string())].into_iter().collect(),
                events: vec![],
                links: vec![],
                status: "ok".to_string(),
                status_message: None,
            },
        ];

        TracesBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            traces,
            metadata: [("source".to_string(), "mock_generator".to_string())].into_iter().collect(),
        }
    }

    fn generate_logs_batch(&mut self) -> LogsBatch {
        self.batch_count += 1;
        
        let logs = vec![
            LogData {
                timestamp: chrono::Utc::now(),
                level: "INFO".to_string(),
                message: format!("Request processed successfully batch_{}", self.batch_count),
                attributes: [("service".to_string(), "web-server".to_string())].into_iter().collect(),
                resource: None,
            },
        ];

        LogsBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            logs,
            metadata: [("source".to_string(), "mock_generator".to_string())].into_iter().collect(),
        }
    }

    fn generate_telemetry_batch(&mut self) -> TelemetryBatch {
        self.batch_count += 1;
        
        let records = vec![
            TelemetryRecord::new(
                TelemetryType::Metric,
                TelemetryData::Metric(MetricData {
                    name: format!("custom_metric_batch_{}", self.batch_count),
                    description: Some("Custom metric".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: MetricType::Counter,
                    value: MetricValue::Counter(self.batch_count as f64),
                    labels: [("service".to_string(), "web-server".to_string())].into_iter().collect(),
                    timestamp: chrono::Utc::now(),
                }),
            ),
        ];

        TelemetryBatch::new(
            format!("mock_source_batch_{}", self.batch_count),
            records,
        )
    }

    fn has_more(&self) -> bool {
        self.batch_count < self.max_batches
    }
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting Apache Kafka connector example");

    // Create Kafka configuration
    let config = KafkaConfig::new(
        "localhost:9092".to_string(),
        "telemetry".to_string(),
        "kafka-example-producer".to_string(),
        "kafka-example-consumer".to_string(),
    );

    // Connect to Kafka
    let connector = KafkaConnector::connect(config).await?;
    info!("Connected to Kafka successfully");

    // Get writer and reader handles
    let writer = connector.writer().await?;
    let reader = connector.reader().await?;

    // Create mock data generator
    let mut generator = MockTelemetryGenerator::new(5);

    // Generate and write telemetry data
    info!("Generating and writing telemetry data to Kafka...");
    
    while generator.has_more() {
        // Generate different types of telemetry data
        let metrics_batch = generator.generate_metrics_batch();
        let traces_batch = generator.generate_traces_batch();
        let logs_batch = generator.generate_logs_batch();
        let telemetry_batch = generator.generate_telemetry_batch();

        // Write metrics
        match writer.write_metrics(metrics_batch).await {
            Ok(result) => {
                info!("Successfully wrote metrics batch: {} records", result.records_written);
            }
            Err(e) => {
                error!("Failed to write metrics batch: {}", e);
            }
        }

        // Write traces
        match writer.write_traces(traces_batch).await {
            Ok(result) => {
                info!("Successfully wrote traces batch: {} records", result.records_written);
            }
            Err(e) => {
                error!("Failed to write traces batch: {}", e);
            }
        }

        // Write logs
        match writer.write_logs(logs_batch).await {
            Ok(result) => {
                info!("Successfully wrote logs batch: {} records", result.records_written);
            }
            Err(e) => {
                error!("Failed to write logs batch: {}", e);
            }
        }

        // Write telemetry batch
        match writer.write_batch(telemetry_batch).await {
            Ok(result) => {
                info!("Successfully wrote telemetry batch: {} records", result.records_written);
            }
            Err(e) => {
                error!("Failed to write telemetry batch: {}", e);
            }
        }

        // Flush to ensure data is sent
        if let Err(e) = writer.flush().await {
            warn!("Failed to flush writer: {}", e);
        }

        // Wait before generating next batch
        sleep(Duration::from_millis(1000)).await;
    }

    // Query data from Kafka
    info!("Querying data from Kafka...");
    
    // Create a simple query
    let query = bridge_core::types::MetricsQuery {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        time_range: bridge_core::types::TimeRange::last_hours(1),
        filters: vec![],
        aggregations: vec![],
        limit: Some(10),
        offset: None,
        metadata: [("source".to_string(), "example".to_string())].into_iter().collect(),
    };

    // Query metrics
    match reader.query_metrics(query).await {
        Ok(result) => {
            info!("Successfully queried metrics: {} records", result.data.len());
            for metric in &result.data {
                info!("Metric: {} = {:?}", metric.name, metric.value);
            }
        }
        Err(e) => {
            error!("Failed to query metrics: {}", e);
        }
    }

    // Execute a custom query
    let custom_query = r#"{"query": "SELECT * FROM metrics LIMIT 5"}"#.to_string();
    match reader.execute_query(custom_query).await {
        Ok(result) => {
            info!("Custom query result: {:?}", result);
        }
        Err(e) => {
            error!("Failed to execute custom query: {}", e);
        }
    }

    // Get statistics
    info!("Getting connector statistics...");
    let stats = connector.get_stats().await?;
    info!("Connector stats: {:?}", stats);

    // Health check
    let health = connector.health_check().await?;
    info!("Connector health: {}", health);

    // Shutdown
    info!("Shutting down Kafka connector...");
    connector.shutdown().await?;
    
    info!("Apache Kafka connector example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kafka_example_components() {
        // Test Kafka configuration
        let config = KafkaConfig::new(
            "test-server:9092".to_string(),
            "test-topic".to_string(),
            "test-consumer-group".to_string(),
        );

        assert_eq!(config.topic_name(), "test-topic");
        assert_eq!(config.bootstrap_servers(), "test-server:9092");
        assert_eq!(config.group_id(), "test-consumer-group");

        // Test mock generator
        let mut generator = MockTelemetryGenerator::new(2);
        let batch1 = generator.generate_metrics_batch();
        assert!(batch1.metrics.len() > 0);
        
        let batch2 = generator.generate_traces_batch();
        assert!(batch2.traces.len() > 0);
        
        let batch3 = generator.generate_logs_batch();
        assert!(batch3.logs.len() > 0);

        let batch4 = generator.generate_telemetry_batch();
        assert!(batch4.records.len() > 0);
        
        assert!(generator.has_more());
        
        let batch5 = generator.generate_metrics_batch();
        assert!(generator.has_more());
        
        let batch6 = generator.generate_traces_batch();
        assert!(generator.has_more());

        let batch7 = generator.generate_logs_batch();
        assert!(generator.has_more());

        let batch8 = generator.generate_telemetry_batch();
        assert!(generator.has_more());
        
        let batch9 = generator.generate_metrics_batch();
        assert!(generator.has_more());
        
        let batch10 = generator.generate_traces_batch();
        assert!(generator.has_more());

        let batch11 = generator.generate_logs_batch();
        assert!(generator.has_more());

        let batch12 = generator.generate_telemetry_batch();
        assert!(generator.has_more());
        
        let batch13 = generator.generate_metrics_batch();
        assert!(!generator.has_more()); // Should return None after max_batches
    }
}
