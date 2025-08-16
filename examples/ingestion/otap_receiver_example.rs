//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTAP Receiver Example
//!
//! This example demonstrates the OTAP receiver functionality with actual data reception
//! from the OTAP protocol handler.

use bridge_core::BridgeResult;
use ingestion::{
    protocols::otap::{OtapConfig, OtapProtocol},
    receivers::otap_receiver::{OtapReceiver, OtapReceiverConfig},
};
use std::time::Duration;
use tracing::{error, info};

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting OTAP receiver example");

    // Create OTAP receiver configuration
    let config = OtapReceiverConfig::new("localhost".to_string(), 8080);
    
    // Create OTAP receiver
    let mut receiver = OtapReceiver::new(&config).await?;
    
    // Initialize the receiver
    receiver.init().await?;
    
    // Start the receiver
    receiver.start().await?;
    
    info!("OTAP receiver started successfully");

    // Get the protocol handler to simulate incoming data
    if let Some(handler) = receiver.get_protocol_handler() {
        // Try to downcast to OtapProtocol to access simulation methods
        if let Some(otap_handler) = handler.as_any().downcast_ref::<OtapProtocol>() {
            // Simulate incoming OTAP data
            info!("Simulating incoming OTAP data...");
            otap_handler.simulate_incoming_data().await?;
            
            // Wait a moment for the data to be processed
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // Receive data from the receiver
    info!("Receiving data from OTAP receiver...");
    
    for i in 0..5 {
        match receiver.receive().await {
            Ok(batch) => {
                info!(
                    "Received batch #{}: {} records from {}",
                    i + 1, batch.size, batch.source
                );
                
                // Print some details about the received data
                for (j, record) in batch.records.iter().enumerate() {
                    info!(
                        "Record {}: Type={:?}, ID={}",
                        j + 1, record.record_type, record.id
                    );
                    
                    match &record.data {
                        bridge_core::types::TelemetryData::Metric(metric) => {
                            info!("  Metric: {} = {:?}", metric.name, metric.value);
                        }
                        bridge_core::types::TelemetryData::Log(log) => {
                            info!("  Log: {} ({:?})", log.message, log.level);
                        }
                        bridge_core::types::TelemetryData::Trace(trace) => {
                            info!("  Trace: {} spans", trace.spans.len());
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error receiving data: {}", e);
            }
        }
        
        // Wait between receives
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Check receiver statistics
    match receiver.get_stats().await {
        Ok(stats) => {
            info!("Receiver statistics:");
            info!("  Total records: {}", stats.total_records);
            info!("  Total bytes: {}", stats.total_bytes);
            info!("  Error count: {}", stats.error_count);
            info!("  Last receive time: {:?}", stats.last_receive_time);
        }
        Err(e) => {
            error!("Error getting receiver stats: {}", e);
        }
    }

    // Check receiver health
    match receiver.health_check().await {
        Ok(healthy) => {
            info!("Receiver health check: {}", if healthy { "OK" } else { "FAILED" });
        }
        Err(e) => {
            error!("Error checking receiver health: {}", e);
        }
    }

    // Stop the receiver
    receiver.stop().await?;
    
    // Shutdown the receiver
    receiver.shutdown().await?;
    
    info!("OTAP receiver example completed successfully");
    
    Ok(())
}
