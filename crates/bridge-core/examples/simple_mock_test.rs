//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Simple Mock Test
//!
//! This example tests the mock receiver in isolation.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

use bridge_core::{
    receivers::{mock::MockReceiverConfig, MockReceiver},
    TelemetryReceiver,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting simple mock test");

    // Create mock receiver configuration
    let mock_receiver_config = MockReceiverConfig {
        auto_generate: true,
        generation_interval_ms: 1000, // 1 second
        records_per_batch: 5,
        simulate_errors: false,
        error_probability: 0.0,
    };

    let mock_receiver = Arc::new(MockReceiver::new(mock_receiver_config));

    info!("Mock receiver created");

    // Start the mock receiver
    mock_receiver.start().await?;
    info!("Mock receiver started");

    // Check health
    let health = mock_receiver.health_check().await?;
    info!("Mock receiver health: {}", health);

    // Wait a bit for auto-generation to start
    sleep(Duration::from_millis(2000)).await;

    // Try to receive data
    for i in 0..5 {
        match mock_receiver.receive().await {
            Ok(batch) => {
                info!("Received batch {} with {} records", i, batch.records.len());
            }
            Err(e) => {
                warn!("Error receiving batch {}: {}", i, e);
            }
        }
        sleep(Duration::from_millis(500)).await;
    }

    // Stop the mock receiver
    mock_receiver.stop().await?;
    info!("Mock receiver stopped");

    info!("Simple mock test completed successfully");
    Ok(())
}
