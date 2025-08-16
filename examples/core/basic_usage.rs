//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Basic usage example for the OpenTelemetry Data Lake Bridge
//!
//! This example demonstrates the minimal working functionality of the bridge.

use bridge_core::{get_bridge_status, init_bridge, shutdown_bridge, BridgeStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("🚀 Starting OpenTelemetry Data Lake Bridge Example");
    println!("==================================================");

    // Initialize the bridge
    println!("📡 Initializing bridge...");
    init_bridge().await?;
    println!("✅ Bridge initialized successfully!");

    // Get bridge status
    println!("\n📊 Getting bridge status...");
    let status: BridgeStatus = get_bridge_status().await?;
    println!("✅ Bridge Status:");
    println!("   Version: {}", status.version);
    println!("   Name: {}", status.name);
    println!("   Status: {}", status.status);
    println!("   Uptime: {} seconds", status.uptime_seconds);
    println!("   Active Components: {:?}", status.components);

    // Simulate some work
    println!("\n⚡ Simulating bridge operations...");
    for i in 1..=3 {
        println!("   Processing batch {}...", i);
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    println!("✅ Bridge operations completed!");

    // Shutdown the bridge
    println!("\n🛑 Shutting down bridge...");
    shutdown_bridge().await?;
    println!("✅ Bridge shutdown completed!");

    println!("\n🎉 Example completed successfully!");
    println!("==================================================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_functionality() {
        // Test initialization
        let init_result = init_bridge().await;
        assert!(init_result.is_ok());

        // Test status
        let status_result = get_bridge_status().await;
        assert!(status_result.is_ok());

        let status = status_result.unwrap();
        assert_eq!(status.name, "orasi");
        assert_eq!(status.status, "running");

        // Test shutdown
        let shutdown_result = shutdown_bridge().await;
        assert!(shutdown_result.is_ok());
    }
}
