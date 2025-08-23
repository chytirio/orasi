//! Basic example demonstrating Orasi Gateway functionality

use axum::serve;
use orasi_gateway::{
    config::GatewayConfig, error::GatewayError,
    gateway::rate_limiter::GatewayRateLimiter as RateLimiter, gateway::OrasiGateway,
    http::HttpServer, load_balancer::LoadBalancer, routing::proxy::Proxy, routing::Router,
    types::*, GATEWAY_NAME, GATEWAY_VERSION,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting {} v{} example", GATEWAY_NAME, GATEWAY_VERSION);

    // Create configuration
    let config = create_example_config();

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
    let gateway_state = gateway.read().await.get_state().await;
    let router = Arc::new(Router::new(&config, gateway_state.clone()).await?);
    let load_balancer = Arc::new(LoadBalancer::new(&config, gateway_state.clone()).await?);
    let proxy = Arc::new(Proxy::new(&config, gateway_state.clone()).await?);
    let rate_limiter = Arc::new(RateLimiter::new(&config).await?);

    // Start components
    router.start().await?;
    load_balancer.start().await?;
    proxy.start().await?;
    info!("All components started successfully");

    // Add example routes
    add_example_routes(&router).await?;

    // Demonstrate routing
    demonstrate_routing(&router).await?;

    // Demonstrate load balancing
    demonstrate_load_balancing(&load_balancer).await?;

    // Demonstrate rate limiting
    demonstrate_rate_limiting(&rate_limiter).await?;

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
    let addr: std::net::SocketAddr = "127.0.0.1:8080".parse()?;
    info!("Starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = serve(listener, app).with_graceful_shutdown(shutdown_signal());

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
        let mut gateway_guard = gateway.write().await;
        gateway_guard.shutdown().await?;
    }

    info!("Gateway example completed");
    Ok(())
}

/// Create example configuration
fn create_example_config() -> GatewayConfig {
    GatewayConfig {
        gateway_id: "example-gateway".to_string(),
        gateway_endpoint: "127.0.0.1:8080".to_string(),
        health_endpoint: "127.0.0.1:8081".to_string(),
        metrics_endpoint: "127.0.0.1:9090".to_string(),
        admin_endpoint: "127.0.0.1:8082".to_string(),
        service_discovery: orasi_gateway::config::ServiceDiscoveryConfig::default(),
        load_balancing: orasi_gateway::config::LoadBalancingConfig::default(),
        routing: orasi_gateway::config::RoutingConfig::default(),
        security: orasi_gateway::config::SecurityConfig::default(),
        rate_limiting: orasi_gateway::config::RateLimitingConfig::default(),
        tls: orasi_gateway::config::TlsConfig::default(),
    }
}

/// Add example routes
async fn add_example_routes(router: &Arc<Router>) -> Result<(), GatewayError> {
    // Add API route
    let api_route = Route {
        path: "/api/v1/*".to_string(),
        method: "*".to_string(),
        service_name: "bridge-api".to_string(),
        priority: 100,
        rules: vec![],
        metadata: std::collections::HashMap::new(),
    };
    router.add_route(api_route).await?;

    // Add agent route
    let agent_route = Route {
        path: "/agent/*".to_string(),
        method: "*".to_string(),
        service_name: "orasi-agent".to_string(),
        priority: 90,
        rules: vec![],
        metadata: std::collections::HashMap::new(),
    };
    router.add_route(agent_route).await?;

    // Add health route
    let health_route = Route {
        path: "/health".to_string(),
        method: "GET".to_string(),
        service_name: "gateway-health".to_string(),
        priority: 200,
        rules: vec![],
        metadata: std::collections::HashMap::new(),
    };
    router.add_route(health_route).await?;

    info!("Added example routes");
    Ok(())
}

/// Demonstrate routing functionality
async fn demonstrate_routing(router: &Arc<Router>) -> Result<(), GatewayError> {
    info!("Demonstrating routing functionality...");

    // Create test requests
    let test_requests = vec![
        create_request_context("GET", "/api/v1/health", "api-client"),
        create_request_context("POST", "/api/v1/data", "api-client"),
        create_request_context("GET", "/agent/status", "agent-client"),
        create_request_context("GET", "/health", "health-client"),
        create_request_context("GET", "/unknown/path", "unknown-client"),
    ];

    for request in test_requests {
        let method = request.method.clone();
        let path = request.path.clone();
        match router.route_request(request).await {
            Ok(route_match) => {
                info!(
                    "Route matched: {} {} -> service in metadata",
                    route_match.method, route_match.path
                );
            }
            Err(e) => {
                info!("Route not found: {} {} - {}", method, path, e);
            }
        }
    }

    Ok(())
}

/// Demonstrate load balancing functionality
async fn demonstrate_load_balancing(load_balancer: &Arc<LoadBalancer>) -> Result<(), GatewayError> {
    info!("Demonstrating load balancing functionality...");

    let services = vec!["bridge-api", "orasi-agent"];

    for service in services {
        for i in 0..5 {
            match load_balancer.select_endpoint(service, None).await {
                Ok(endpoint) => {
                    info!(
                        "Selected endpoint {} for service {}: {}",
                        i + 1,
                        service,
                        endpoint.url
                    );
                }
                Err(e) => {
                    error!("Failed to select endpoint for service {}: {}", service, e);
                }
            }
        }
    }

    Ok(())
}

/// Demonstrate rate limiting functionality
async fn demonstrate_rate_limiting(rate_limiter: &Arc<RateLimiter>) -> Result<(), GatewayError> {
    info!("Demonstrating rate limiting functionality...");

    let clients = vec!["client-1", "client-2", "client-3"];
    let endpoints = vec!["/api/v1/health", "/api/v1/data", "/agent/status"];

    for client in &clients {
        for endpoint in &endpoints {
            for i in 0..10 {
                let allowed = rate_limiter.check_rate_limit(client, endpoint).await?;
                if allowed {
                    info!(
                        "Request {} allowed for client {} on endpoint {}",
                        i + 1,
                        client,
                        endpoint
                    );
                } else {
                    info!(
                        "Request {} rate limited for client {} on endpoint {}",
                        i + 1,
                        client,
                        endpoint
                    );
                    break;
                }
            }
        }
    }

    // Get rate limit statistics
    let stats = rate_limiter.get_stats().await;
    info!("Rate limit statistics: {:?}", stats);

    Ok(())
}

/// Create request context for testing
fn create_request_context(method: &str, path: &str, client_id: &str) -> RequestContext {
    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Client-ID".to_string(), client_id.to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    RequestContext {
        request_id: uuid::Uuid::new_v4().to_string(),
        client_ip: "127.0.0.1".to_string(),
        user_agent: Some("example-client".to_string()),
        method: method.to_string(),
        path: path.to_string(),
        headers,
        query_params: std::collections::HashMap::new(),
        body: None,
        metadata: std::collections::HashMap::new(),
    }
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
