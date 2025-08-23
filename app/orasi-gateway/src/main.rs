//! Main binary for Orasi Gateway

use orasi_gateway::{
    config::GatewayConfig, discovery::ServiceDiscovery, error::GatewayError,
    gateway::rate_limiter::GatewayRateLimiter, gateway::OrasiGateway, http::HttpServer,
    load_balancer::LoadBalancer, routing::proxy::Proxy, routing::Router, GATEWAY_NAME,
    GATEWAY_VERSION,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting {} v{}", GATEWAY_NAME, GATEWAY_VERSION);

    // Load configuration
    let config = load_config().await?;
    info!("Configuration loaded successfully");

    // Initialize gateway
    let gateway = Arc::new(RwLock::new(OrasiGateway::new(config.clone()).await?));
    info!("Gateway initialized successfully");

    // Start gateway
    {
        let mut gateway_guard = gateway.write().await;
        gateway_guard.start().await?;
    }
    info!("Gateway started successfully");

    // Initialize components
    let router = Arc::new(Router::new(&config, gateway.read().await.get_state().await).await?);
    let load_balancer =
        Arc::new(LoadBalancer::new(&config, gateway.read().await.get_state().await).await?);
    let proxy = Arc::new(Proxy::new(&config, gateway.read().await.get_state().await).await?);
    let rate_limiter = Arc::new(GatewayRateLimiter::new(&config).await?);

    // Start components
    router.start().await?;
    load_balancer.start().await?;
    proxy.start().await?;
    info!("All components started successfully");

    // Create HTTP server
    let http_server = HttpServer::new(
        config.clone(),
        gateway.clone(),
        router.clone(),
        load_balancer.clone(),
        proxy.clone(),
        rate_limiter.clone(),
    );

    let app = http_server.create_router();

    // Start HTTP server
    let addr: std::net::SocketAddr = config.gateway_endpoint.parse()?;
    info!("Starting HTTP server on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

    if let Err(e) = server.await {
        error!("HTTP server error: {}", e);
    }

    // Shutdown components
    info!("Shutting down components...");
    router.stop().await?;
    load_balancer.stop().await?;
    proxy.stop().await?;

    // Shutdown gateway
    {
        let gateway_guard = gateway.write().await;
        gateway_guard.shutdown().await?;
    }

    info!("Gateway shutdown completed");
    Ok(())
}

/// Load configuration from file or environment
async fn load_config() -> Result<GatewayConfig, GatewayError> {
    // Try to load from environment variables first
    if let Ok(config) = load_config_from_env() {
        return Ok(config);
    }

    // Try to load from config file
    if let Ok(config) = load_config_from_file().await {
        return Ok(config);
    }

    // Use default configuration
    warn!("No configuration found, using defaults");
    Ok(GatewayConfig::default())
}

/// Load configuration from environment variables
fn load_config_from_env() -> Result<GatewayConfig, GatewayError> {
    let gateway_id =
        std::env::var("GATEWAY_ID").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

    let gateway_endpoint =
        std::env::var("GATEWAY_ENDPOINT").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    let health_endpoint =
        std::env::var("HEALTH_ENDPOINT").unwrap_or_else(|_| "0.0.0.0:8081".to_string());

    let metrics_endpoint =
        std::env::var("METRICS_ENDPOINT").unwrap_or_else(|_| "0.0.0.0:9090".to_string());

    let admin_endpoint =
        std::env::var("ADMIN_ENDPOINT").unwrap_or_else(|_| "0.0.0.0:8082".to_string());

    Ok(GatewayConfig {
        gateway_id,
        gateway_endpoint,
        health_endpoint,
        metrics_endpoint,
        admin_endpoint,
        service_discovery: orasi_gateway::config::ServiceDiscoveryConfig::default(),
        load_balancing: orasi_gateway::config::LoadBalancingConfig::default(),
        routing: orasi_gateway::config::RoutingConfig::default(),
        security: orasi_gateway::config::SecurityConfig::default(),
        rate_limiting: orasi_gateway::config::RateLimitingConfig::default(),
        tls: orasi_gateway::config::TlsConfig::default(),
    })
}

/// Load configuration from file
async fn load_config_from_file() -> Result<GatewayConfig, GatewayError> {
    let config_path =
        std::env::var("GATEWAY_CONFIG").unwrap_or_else(|_| "config/gateway.toml".to_string());

    let config_content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
        GatewayError::ConfigurationError(format!("Failed to read config file: {}", e))
    })?;

    let config: GatewayConfig = toml::from_str(&config_content).map_err(|e| {
        GatewayError::ConfigurationError(format!("Failed to parse config file: {}", e))
    })?;

    Ok(config)
}

/// Graceful shutdown signal handler
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
            info!("Ctrl+C received, shutting down");
        }
        _ = terminate => {
            info!("SIGTERM received, shutting down");
        }
    }

    info!("Shutdown signal received");
}
