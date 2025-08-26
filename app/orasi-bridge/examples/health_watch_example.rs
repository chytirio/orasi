//! Example demonstrating health watch streaming functionality

use std::time::Duration;
use tonic::{Request, Status};
use tracing;

use bridge_api::proto::{
    health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
    HealthCheckResponse,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Connect to the health service
    let mut client = HealthClient::connect("http://[::1]:50051").await?;

    println!("Connected to health service");
    println!("Starting health watch for 'bridge-api' service...");

    // Create health watch request
    let request = Request::new(HealthCheckRequest {
        service: "bridge-api".to_string(),
    });

    // Start watching health status
    let mut stream = client.watch(request).await?.into_inner();

    let mut update_count = 0;
    const MAX_UPDATES: u32 = 10;

    // Process health status updates
    while let Some(result) = stream.message().await? {
        match result {
            Ok(response) => {
                update_count += 1;
                let status = ServingStatus::from_i32(response.status)
                    .unwrap_or(ServingStatus::Unknown);

                println!(
                    "Health update #{}: Service status = {:?}",
                    update_count, status
                );

                // Stop after receiving enough updates
                if update_count >= MAX_UPDATES {
                    println!("Received {} health updates, stopping watch", MAX_UPDATES);
                    break;
                }
            }
            Err(status) => {
                eprintln!("Health watch error: {}", status);
                break;
            }
        }
    }

    println!("Health watch completed");
    Ok(())
}
