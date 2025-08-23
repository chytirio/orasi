//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Basic bridge example demonstrating the core functionality
//!
//! This example shows how to:
//! 1. Initialize the bridge system
//! 2. Create and configure receivers
//! 3. Start the system
//! 4. Monitor health and metrics
//! 5. Shutdown gracefully

use bridge_core::{
    config::BridgeConfig,
    error::BridgeResult,
    receivers::{mock::MockReceiverConfig, MockReceiver},
    BridgeSystem, TelemetryReceiver,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting basic bridge example");

    // Create bridge configuration
    let config = BridgeConfig::default();
    info!("Using default bridge configuration");

    // Create bridge system
    let bridge_system = BridgeSystem::new(config).await?;
    info!("Bridge system created");

    // Initialize the system
    bridge_system.init().await?;
    info!("Bridge system initialized");

    // Start the system
    bridge_system.start().await?;
    info!("Bridge system started");

    // Create a mock receiver for demonstration
    let mock_config = MockReceiverConfig {
        auto_generate: true,
        generation_interval_ms: 2000, // Generate data every 2 seconds
        records_per_batch: 5,
        simulate_errors: false,
        error_probability: 0.0,
    };

    let mock_receiver = Arc::new(MockReceiver::new(mock_config));
    info!("Mock receiver created");

    // Start the mock receiver
    mock_receiver.start().await?;
    info!("Mock receiver started");

    // Add the receiver to the health checker
    bridge_system
        .health_checker
        .add_health_check(mock_receiver.clone())
        .await?;
    info!("Mock receiver added to health monitoring");

    // Main loop - run for 30 seconds
    let start_time = std::time::Instant::now();
    let duration = Duration::from_secs(30);

    info!("Running bridge system for 30 seconds...");

    while start_time.elapsed() < duration {
        // Get bridge status
        let status = bridge_system.get_status().await;
        info!(
            "Bridge status: {} (uptime: {}s, components: {})",
            status.status,
            status.uptime_seconds,
            status.components.len()
        );

        // Get receiver stats
        let receiver_stats = mock_receiver.get_stats().await?;
        info!(
            "Receiver stats: {} records, {} bytes, {} errors",
            receiver_stats.total_records, receiver_stats.total_bytes, receiver_stats.error_count
        );

        // Receive some data
        match mock_receiver.receive().await {
            Ok(batch) => {
                info!(
                    "Received batch {} with {} records from {}",
                    batch.id,
                    batch.records.len(),
                    batch.source
                );

                // Update metrics
                bridge_system
                    .metrics
                    .record_processed_records(batch.records.len() as u64)
                    .await;
                bridge_system.metrics.record_processed_batches(1).await;
            }
            Err(e) => {
                warn!("Error receiving data: {}", e);
                bridge_system.metrics.record_ingestion_errors(1).await;
            }
        }

        // Sleep for 5 seconds
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    info!("Shutting down bridge system...");

    // Stop the mock receiver
    mock_receiver.stop().await?;
    info!("Mock receiver stopped");

    // Stop the bridge system
    bridge_system.stop().await?;
    info!("Bridge system stopped");

    info!("Basic bridge example completed successfully");
    Ok(())
}
