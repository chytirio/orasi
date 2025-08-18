use bridge_api::{BridgeAPIConfig, BridgeAPIServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a simple configuration
    let config = BridgeAPIConfig::default();

    // Create and initialize the server
    let mut server = BridgeAPIServer::new(config);
    server.init().await?;

    // Start the server
    server.start().await?;

    // Keep the server running
    tokio::signal::ctrl_c().await?;

    // Shutdown gracefully
    server.stop().await?;

    Ok(())
}
