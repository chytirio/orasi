//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating OTLP receiver functionality
//!
//! This example shows how to use the OTLP receiver to ingest OpenTelemetry data.

use bridge_core::receivers::otlp::{OtlpReceiver, OtlpReceiverConfig};
use bridge_core::TelemetryReceiver;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting OTLP receiver test...");

    // Create OTLP receiver configuration
    let config = OtlpReceiverConfig {
        endpoint: "127.0.0.1:4317".to_string(),
        max_concurrent_connections: 10,
        buffer_size: 1000,
        enable_tls: false,
        tls_cert_path: None,
        tls_key_path: None,
        enable_compression: true,
        request_timeout_seconds: 30,
    };

    // Create OTLP receiver
    let receiver = Arc::new(OtlpReceiver::new(config));
    println!("Created OTLP receiver");

    // Start the receiver
    receiver.start().await?;
    println!("Started OTLP receiver on {}", receiver.config.endpoint);

    // Check health
    let health = receiver.health_check().await?;
    println!("Receiver health: {}", health);

    // Get initial stats
    let stats = receiver.get_stats().await?;
    println!("Initial stats: {:?}", stats);

    // Wait a bit for potential incoming data
    println!("Waiting for incoming OTLP data...");
    sleep(Duration::from_secs(5)).await;

    // Try to receive data
    let batch = receiver.receive().await?;
    println!("Received batch: {:?}", batch);

    // Get updated stats
    let stats = receiver.get_stats().await?;
    println!("Updated stats: {:?}", stats);

    // Stop the receiver
    receiver.stop().await?;
    println!("Stopped OTLP receiver");

    println!("OTLP receiver test completed successfully!");
    Ok(())
}
