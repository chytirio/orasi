use orasi_gateway::{
    config::GatewayConfig,
    gateway::health::HealthChecker,
    types::{HealthState, HealthStatus},
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Starting Orasi Gateway Health Checker Example");

    // Create gateway configuration
    let config = GatewayConfig::default();
    println!("Gateway ID: {}", config.gateway_id);

    // Create health checker
    let health_checker = HealthChecker::new(&config).await?;
    println!("Health checker created successfully");

    // Start health checker
    health_checker.start().await?;
    println!("Health checker started");

    // Wait a bit for the first health check to complete
    sleep(Duration::from_secs(2)).await;

    // Check health status
    let health_status = health_checker.check_health().await?;
    println!("Current health status: {:?}", health_status.status);
    println!("Health details:");
    for (key, value) in &health_status.details {
        println!("  {}: {}", key, value);
    }

    // Wait a bit more to see multiple health checks
    sleep(Duration::from_secs(3)).await;

    // Check health status again
    let health_status = health_checker.check_health().await?;
    println!("Updated health status: {:?}", health_status.status);

    // Stop health checker
    health_checker.stop().await?;
    println!("Health checker stopped");

    println!("Example completed successfully");
    Ok(())
}
