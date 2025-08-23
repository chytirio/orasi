//! Orasi Agent main binary

use orasi_agent::{agent::OrasiAgent, config::AgentConfig, error::AgentError, http::HttpServer};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Orasi Agent v{}", orasi_agent::AGENT_VERSION);

    // Load configuration
    let config = load_config()?;
    info!("Configuration loaded successfully");

    // Create agent
    let agent = Arc::new(RwLock::new(OrasiAgent::new(config.clone()).await?));
    info!("Agent created successfully");

    // Start agent
    {
        let mut agent = agent.write().await;
        agent.start().await?;
    }
    info!("Agent started successfully");

    // Create HTTP server
    let http_server = HttpServer::new(
        config.clone(),
        agent.clone(),
        agent.read().await.get_health_checker(),
        agent.read().await.get_metrics_collector(),
    );

    let app = http_server.create_router();
    let addr: std::net::SocketAddr = config.agent_endpoint.parse()?;

    info!("Starting HTTP server on {}", addr);

    // Start HTTP server
    let listener = TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app);

    // Handle shutdown signals
    let graceful_shutdown = server.with_graceful_shutdown(shutdown_signal());

    // Run server
    if let Err(e) = graceful_shutdown.await {
        error!("HTTP server error: {}", e);
    }

    // Shutdown agent
    info!("Shutting down agent");
    {
        let agent = agent.read().await;
        agent.shutdown().await?;
    }

    info!("Orasi Agent shutdown completed");
    Ok(())
}

/// Load configuration from environment or file
fn load_config() -> Result<AgentConfig, AgentError> {
    // Try to load from environment variables first
    if let Ok(agent_id) = std::env::var("ORASI_AGENT_ID") {
        let config = AgentConfig {
            agent_id,
            agent_endpoint: std::env::var("ORASI_AGENT_ENDPOINT")
                .unwrap_or_else(|_| "0.0.0.0:8082".to_string()),
            health_endpoint: std::env::var("ORASI_HEALTH_ENDPOINT")
                .unwrap_or_else(|_| "0.0.0.0:8083".to_string()),
            metrics_endpoint: std::env::var("ORASI_METRICS_ENDPOINT")
                .unwrap_or_else(|_| "0.0.0.0:9092".to_string()),
            cluster_endpoint: std::env::var("ORASI_CLUSTER_ENDPOINT")
                .unwrap_or_else(|_| "0.0.0.0:8084".to_string()),
            ..Default::default()
        };
        return Ok(config);
    }

    // Try to load from config file
    let config_path =
        std::env::var("ORASI_CONFIG_PATH").unwrap_or_else(|_| "config/agent.toml".to_string());

    if let Ok(config_content) = std::fs::read_to_string(&config_path) {
        match toml::from_str::<AgentConfig>(&config_content) {
            Ok(config) => return Ok(config),
            Err(e) => warn!("Failed to parse config file: {}", e),
        }
    }

    // Use default configuration
    info!("Using default configuration");
    Ok(AgentConfig::default())
}

/// Handle shutdown signals
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        }
        _ = terminate => {
            info!("Received terminate signal");
        }
    }

    info!("Shutdown signal received");
}
