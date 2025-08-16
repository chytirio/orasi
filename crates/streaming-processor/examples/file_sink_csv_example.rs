//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! File sink CSV example
//!
//! This example demonstrates how to use the file sink to write telemetry data to CSV files.

use bridge_core::types::{LogData, LogLevel, TelemetryData, TelemetryRecord, TelemetryType};
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::Utc;
use std::collections::HashMap;
use streaming_processor::sinks::file_sink::{FileFormat, FileSinkConfig};
use streaming_processor::sinks::{FileSink, StreamSink};
use uuid::Uuid;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("File Sink CSV Example");
    println!("=====================");

    // Create file sink configuration for CSV
    let config = FileSinkConfig::new("output/telemetry_data.csv".to_string(), FileFormat::Csv);

    // Create file sink
    let mut sink = FileSink::new(&config).await?;

    // Initialize and start the sink
    sink.init().await?;
    sink.start().await?;

    println!("File sink initialized and started");

    // Create sample telemetry records
    let records = vec![
        TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "CSV test log 1".to_string(),
                attributes: HashMap::new(),
                body: None,
                severity_number: Some(9),
                severity_text: Some("INFO".to_string()),
            }),
            attributes: HashMap::from([
                ("component".to_string(), "csv_test".to_string()),
                ("test_id".to_string(), "1".to_string()),
            ]),
            tags: HashMap::new(),
            resource: None,
            service: None,
        },
        TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: LogLevel::Warn,
                message: "CSV test log 2".to_string(),
                attributes: HashMap::new(),
                body: None,
                severity_number: Some(13),
                severity_text: Some("WARN".to_string()),
            }),
            attributes: HashMap::from([
                ("component".to_string(), "csv_test".to_string()),
                ("test_id".to_string(), "2".to_string()),
            ]),
            tags: HashMap::new(),
            resource: None,
            service: None,
        },
    ];

    // Create telemetry batch
    let batch = TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "file_sink_csv_example".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::from([
            ("example".to_string(), "true".to_string()),
            ("format".to_string(), "csv".to_string()),
        ]),
    };

    // Send batch to file sink
    println!("Sending batch to file sink...");
    sink.send(batch).await?;

    // Get statistics
    let stats = sink.get_stats().await?;
    println!("Sink statistics:");
    println!("  Total batches: {}", stats.total_batches);
    println!("  Total records: {}", stats.total_records);
    println!("  Total bytes: {}", stats.total_bytes);
    println!("  Is connected: {}", stats.is_connected);

    // Stop the sink
    sink.stop().await?;
    println!("File sink stopped");

    println!("Check the output/telemetry_data.csv file for the written data");

    Ok(())
}
