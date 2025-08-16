//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.com>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating Arrow IPC serialization/deserialization for Kafka messages
//!
//! This example shows how to:
//! 1. Create a TelemetryBatch with sample data
//! 2. Serialize it to Arrow IPC format
//! 3. Deserialize it back to TelemetryBatch
//! 4. Verify the roundtrip works correctly

use bridge_core::types::{TelemetryBatch, TelemetryData, TelemetryRecord, TelemetryType};
use chrono::Utc;
use std::collections::HashMap;
use streaming_processor::arrow_utils::{deserialize_from_arrow_ipc, serialize_to_arrow_ipc};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Arrow IPC Kafka Serialization Example");
    println!("========================================");

    // Create a sample TelemetryBatch with different types of records
    let sample_batch = create_sample_batch();

    println!("📊 Created sample batch with {} records", sample_batch.size);
    println!("   - Source: {}", sample_batch.source);
    println!("   - Batch ID: {}", sample_batch.id);

    // Display the records
    for (i, record) in sample_batch.records.iter().enumerate() {
        println!(
            "   Record {}: {:?} - {:?}",
            i, record.record_type, record.id
        );
    }

    // Serialize to Arrow IPC format
    println!("\n🔄 Serializing to Arrow IPC format...");
    let start_time = std::time::Instant::now();
    let arrow_data = serialize_to_arrow_ipc(&sample_batch)?;
    let serialize_time = start_time.elapsed();

    println!("✅ Serialization completed in {:?}", serialize_time);
    println!("   - Arrow IPC size: {} bytes", arrow_data.len());
    println!(
        "   - Compression ratio: {:.2}x",
        (sample_batch.size * 100) as f64 / arrow_data.len() as f64
    );

    // Deserialize from Arrow IPC format
    println!("\n🔄 Deserializing from Arrow IPC format...");
    let start_time = std::time::Instant::now();
    let deserialized_batch = deserialize_from_arrow_ipc(&arrow_data)?;
    let deserialize_time = start_time.elapsed();

    println!("✅ Deserialization completed in {:?}", deserialize_time);
    println!(
        "   - Deserialized batch size: {} records",
        deserialized_batch.size
    );

    // Verify roundtrip
    println!("\n🔍 Verifying roundtrip...");
    verify_roundtrip(&sample_batch, &deserialized_batch)?;
    println!("✅ Roundtrip verification passed!");

    // Simulate Kafka message processing
    println!("\n📨 Simulating Kafka message processing...");
    simulate_kafka_processing(&sample_batch).await?;

    println!("\n🎉 Example completed successfully!");
    Ok(())
}

/// Create a sample TelemetryBatch with various record types
fn create_sample_batch() -> TelemetryBatch {
    let mut records = Vec::new();

    // Add a metric record
    records.push(TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(bridge_core::types::MetricData {
            name: "cpu_usage".to_string(),
            description: Some("CPU usage percentage".to_string()),
            unit: Some("percent".to_string()),
            metric_type: bridge_core::types::MetricType::Gauge,
            value: bridge_core::types::MetricValue::Gauge(75.5),
            labels: HashMap::from([
                ("host".to_string(), "server-01".to_string()),
                ("service".to_string(), "web-api".to_string()),
            ]),
            timestamp: Utc::now(),
        }),
        attributes: HashMap::from([
            ("environment".to_string(), "production".to_string()),
            ("version".to_string(), "1.2.3".to_string()),
        ]),
        tags: HashMap::from([
            ("team".to_string(), "platform".to_string()),
            ("region".to_string(), "us-west-2".to_string()),
        ]),
        resource: None,
        service: None,
    });

    // Add a log record
    records.push(TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        record_type: TelemetryType::Log,
        data: TelemetryData::Log(bridge_core::types::LogData {
            timestamp: Utc::now(),
            level: bridge_core::types::LogLevel::Info,
            message: "User login successful".to_string(),
            attributes: HashMap::from([
                ("user_id".to_string(), "12345".to_string()),
                ("ip_address".to_string(), "192.168.1.100".to_string()),
            ]),
            body: None,
            severity_number: Some(9),
            severity_text: Some("INFO".to_string()),
        }),
        attributes: HashMap::from([
            ("component".to_string(), "auth-service".to_string()),
            ("trace_id".to_string(), "abc123".to_string()),
        ]),
        tags: HashMap::from([
            ("service".to_string(), "auth".to_string()),
            ("level".to_string(), "info".to_string()),
        ]),
        resource: None,
        service: None,
    });

    // Add a trace record
    records.push(TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        record_type: TelemetryType::Trace,
        data: TelemetryData::Trace(bridge_core::types::TraceData {
            trace_id: "trace-123".to_string(),
            span_id: "span-456".to_string(),
            parent_span_id: Some("span-789".to_string()),
            name: "HTTP GET /api/users".to_string(),
            kind: bridge_core::types::SpanKind::Server,
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            duration_ns: Some(15000000), // 15ms
            status: bridge_core::types::SpanStatus {
                code: bridge_core::types::StatusCode::Ok,
                message: None,
            },
            attributes: HashMap::from([
                ("http.method".to_string(), "GET".to_string()),
                ("http.url".to_string(), "/api/users".to_string()),
                ("http.status_code".to_string(), "200".to_string()),
            ]),
            events: Vec::new(),
            links: Vec::new(),
        }),
        attributes: HashMap::from([
            ("service.name".to_string(), "user-service".to_string()),
            ("service.version".to_string(), "1.0.0".to_string()),
        ]),
        tags: HashMap::from([
            ("environment".to_string(), "staging".to_string()),
            ("team".to_string(), "backend".to_string()),
        ]),
        resource: None,
        service: None,
    });

    // Add an event record
    records.push(TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        record_type: TelemetryType::Event,
        data: TelemetryData::Event(bridge_core::types::EventData {
            name: "user_registered".to_string(),
            timestamp: Utc::now(),
            attributes: HashMap::from([
                ("user_id".to_string(), "67890".to_string()),
                ("email".to_string(), "user@example.com".to_string()),
            ]),
            data: None,
        }),
        attributes: HashMap::from([
            ("source".to_string(), "registration-service".to_string()),
            ("event_type".to_string(), "user_management".to_string()),
        ]),
        tags: HashMap::from([
            ("priority".to_string(), "high".to_string()),
            ("category".to_string(), "business".to_string()),
        ]),
        resource: None,
        service: None,
    });

    TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "example-source".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::from([
            ("example".to_string(), "true".to_string()),
            ("created_at".to_string(), Utc::now().to_rfc3339()),
        ]),
    }
}

/// Verify that the roundtrip serialization/deserialization works correctly
fn verify_roundtrip(
    original: &TelemetryBatch,
    deserialized: &TelemetryBatch,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check basic properties
    assert_eq!(original.id, deserialized.id, "Batch ID mismatch");
    assert_eq!(original.source, deserialized.source, "Source mismatch");
    assert_eq!(original.size, deserialized.size, "Size mismatch");
    assert_eq!(
        original.records.len(),
        deserialized.records.len(),
        "Record count mismatch"
    );

    // Check each record
    for (i, (orig_record, deser_record)) in original
        .records
        .iter()
        .zip(deserialized.records.iter())
        .enumerate()
    {
        assert_eq!(orig_record.id, deser_record.id, "Record {} ID mismatch", i);
        assert_eq!(
            orig_record.record_type, deser_record.record_type,
            "Record {} type mismatch",
            i
        );

        // Check data based on type
        match (&orig_record.data, &deser_record.data) {
            (TelemetryData::Metric(orig_metric), TelemetryData::Metric(deser_metric)) => {
                assert_eq!(
                    orig_metric.name, deser_metric.name,
                    "Record {} metric name mismatch",
                    i
                );
                // Note: We're only checking basic fields for brevity
            }
            (TelemetryData::Log(orig_log), TelemetryData::Log(deser_log)) => {
                assert_eq!(
                    orig_log.message, deser_log.message,
                    "Record {} log message mismatch",
                    i
                );
                assert_eq!(
                    orig_log.level, deser_log.level,
                    "Record {} log level mismatch",
                    i
                );
            }
            (TelemetryData::Trace(orig_trace), TelemetryData::Trace(deser_trace)) => {
                assert_eq!(
                    orig_trace.name, deser_trace.name,
                    "Record {} trace name mismatch",
                    i
                );
                assert_eq!(
                    orig_trace.duration_ns, deser_trace.duration_ns,
                    "Record {} trace duration mismatch",
                    i
                );
            }
            (TelemetryData::Event(orig_event), TelemetryData::Event(deser_event)) => {
                assert_eq!(
                    orig_event.name, deser_event.name,
                    "Record {} event name mismatch",
                    i
                );
            }
            _ => {
                return Err(format!("Record {} data type mismatch", i).into());
            }
        }
    }

    println!(
        "   ✅ All {} records verified successfully",
        original.records.len()
    );
    Ok(())
}

/// Simulate Kafka message processing
async fn simulate_kafka_processing(
    batch: &TelemetryBatch,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("   📤 Serializing for Kafka producer...");
    let kafka_message = serialize_to_arrow_ipc(batch)?;
    println!("   📦 Kafka message size: {} bytes", kafka_message.len());

    println!("   📥 Deserializing from Kafka consumer...");
    let received_batch = deserialize_from_arrow_ipc(&kafka_message)?;
    println!("   ✅ Received batch with {} records", received_batch.size);

    // Verify the Kafka roundtrip
    verify_roundtrip(batch, &received_batch)?;

    println!("   🎯 Kafka processing simulation completed successfully!");
    Ok(())
}
