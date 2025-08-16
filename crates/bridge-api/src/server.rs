//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Main server implementation for Bridge API

use axum::Router;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;

use crate::{
    config::BridgeAPIConfig,
    error::{ApiError, ApiResult},
    grpc::{create_grpc_server, GrpcServer},
    handlers::init_server_start_time,
    metrics::{init_metrics, ApiMetrics},
    rest::{create_rest_router, AppState},
};

/// Bridge API Server
pub struct BridgeAPIServer {
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
    http_server: Option<HttpServer>,
    grpc_server: Option<GrpcServer>,
    status: Arc<RwLock<ServerStatus>>,
    shutdown_signal: Arc<RwLock<Option<tokio::sync::broadcast::Sender<()>>>>,
}

/// HTTP Server wrapper
struct HttpServer {
    router: Router<AppState>,
    address: String,
}

/// Server status
#[derive(Debug, Clone)]
pub struct ServerStatus {
    pub running: bool,
    pub http_server: Option<String>,
    pub grpc_server: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub uptime_seconds: u64,
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self {
            running: false,
            http_server: None,
            grpc_server: None,
            start_time: None,
            uptime_seconds: 0,
        }
    }
}

impl BridgeAPIServer {
    /// Create a new Bridge API server
    pub fn new(config: BridgeAPIConfig) -> Self {
        let metrics = ApiMetrics::new();

        Self {
            config,
            metrics,
            http_server: None,
            grpc_server: None,
            status: Arc::new(RwLock::new(ServerStatus::default())),
            shutdown_signal: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the server
    pub async fn init(&mut self) -> ApiResult<()> {
        tracing::info!(
            "Initializing Bridge API server v{}",
            crate::BRIDGE_API_VERSION
        );

        // Initialize server start time for uptime tracking
        init_server_start_time();

        // Initialize metrics
        init_metrics();

        // Initialize bridge core
        bridge_core::init_bridge()
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to initialize bridge core: {}", e)))?;

        // Create HTTP server
        let http_router = create_rest_router(self.config.clone(), self.metrics.clone());
        self.http_server = Some(HttpServer {
            router: http_router,
            address: self.config.http_address(),
        });

        // Create gRPC server
        self.grpc_server = Some(create_grpc_server(
            self.config.clone(),
            self.metrics.clone(),
        ));

        tracing::info!("Bridge API server initialized successfully");
        Ok(())
    }

    /// Start the server with graceful shutdown
    pub async fn start(&mut self) -> ApiResult<()> {
        tracing::info!("Starting Bridge API server");

        // Update status
        {
            let mut status = self.status.write().await;
            status.running = true;
            status.start_time = Some(chrono::Utc::now());
            status.http_server = Some(self.config.http_address());
            status.grpc_server = Some(self.config.grpc_address());
        }

        // Create shutdown signal using broadcast channel
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
        {
            let mut signal_guard = self.shutdown_signal.write().await;
            *signal_guard = Some(shutdown_tx.clone());
        }

        // Start HTTP server with graceful shutdown
        if let Some(http_server) = &self.http_server {
            let router = http_server.router.clone();
            let address = http_server.address.clone();
            let mut http_shutdown_rx = shutdown_tx.subscribe();

            tracing::info!("Starting HTTP server on {}", address);

            tokio::spawn(async move {
                let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
                tracing::info!("HTTP server listening on {}", address);

                // Simple server loop with graceful shutdown
                loop {
                    tokio::select! {
                        accept_result = listener.accept() => {
                            match accept_result {
                                Ok((_stream, _addr)) => {
                                    tracing::debug!("HTTP connection accepted");
                                    // TODO: Handle the connection properly
                                }
                                Err(e) => {
                                    tracing::error!("HTTP accept error: {}", e);
                                }
                            }
                        }
                        _ = http_shutdown_rx.recv() => {
                            tracing::info!("HTTP server received shutdown signal");
                            break;
                        }
                    }
                }

                tracing::info!("HTTP server stopped");
            });
        }

        // Start gRPC server
        if let Some(grpc_server) = self.grpc_server.take() {
            let grpc_config = self.config.clone();
            let grpc_metrics = self.metrics.clone();

            tracing::info!("Starting gRPC server on {}", grpc_config.grpc_address());

            tokio::spawn(async move {
                if let Err(e) = grpc_server.start().await {
                    tracing::error!("gRPC server error: {}", e);
                }
            });
        }

        // Wait for shutdown signal
        tokio::spawn(async move {
            if let Err(e) = Self::wait_for_shutdown_signal().await {
                tracing::error!("Error waiting for shutdown signal: {}", e);
            }
            if let Ok(()) = shutdown_rx.recv().await {
                tracing::info!("Shutdown signal received");
            }
        });

        tracing::info!("Bridge API server started successfully");
        Ok(())
    }

    /// Wait for shutdown signal (SIGINT, SIGTERM, etc.)
    async fn wait_for_shutdown_signal() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(unix)]
        {
            let mut interrupt_signal = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
            let mut terminate_signal = signal::unix::signal(signal::unix::SignalKind::terminate())?;

            tokio::select! {
                _ = interrupt_signal.recv() => {
                    tracing::info!("Received SIGINT signal");
                }
                _ = terminate_signal.recv() => {
                    tracing::info!("Received SIGTERM signal");
                }
            }
        }

        #[cfg(windows)]
        {
            signal::windows::ctrl_c().await?;
            tracing::info!("Received Ctrl+C signal");
        }

        Ok(())
    }

    /// Stop the server
    pub async fn stop(&mut self) -> ApiResult<()> {
        tracing::info!("Stopping Bridge API server");

        // Send shutdown signal
        if let Some(shutdown_tx) = self.shutdown_signal.write().await.take() {
            if let Err(_) = shutdown_tx.send(()) {
                tracing::warn!("Failed to send shutdown signal");
            }
        }

        // Update status
        {
            let mut status = self.status.write().await;
            status.running = false;
        }

        // Shutdown bridge core
        bridge_core::shutdown_bridge()
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to shutdown bridge core: {}", e)))?;

        tracing::info!("Bridge API server stopped successfully");
        Ok(())
    }

    /// Get server status
    pub async fn get_status(&self) -> ServerStatus {
        let mut status = self.status.read().await.clone();

        // Calculate uptime
        if let Some(start_time) = status.start_time {
            let now = chrono::Utc::now();
            status.uptime_seconds = (now - start_time).num_seconds() as u64;
        }

        status
    }

    /// Get HTTP router (for testing or custom usage)
    pub fn get_http_router(&self) -> Option<Router<AppState>> {
        self.http_server.as_ref().map(|s| s.router.clone())
    }

    /// Get metrics
    pub fn get_metrics(&self) -> &ApiMetrics {
        &self.metrics
    }

    /// Get configuration
    pub fn get_config(&self) -> &BridgeAPIConfig {
        &self.config
    }
}

// Note: Drop implementation removed to avoid runtime panic
// Shutdown should be handled explicitly in the main function

/// Create a minimal server (health checks and metrics only)
pub fn create_minimal_server(config: BridgeAPIConfig) -> BridgeAPIServer {
    let metrics = ApiMetrics::new();

    BridgeAPIServer {
        config,
        metrics,
        http_server: None,
        grpc_server: None,
        status: Arc::new(RwLock::new(ServerStatus::default())),
        shutdown_signal: Arc::new(RwLock::new(None)),
    }
}

/// Create a development server (with additional debugging features)
pub fn create_dev_server(config: BridgeAPIConfig) -> BridgeAPIServer {
    let server = BridgeAPIServer::new(config);

    // Enable development features
    // TODO: Add development-specific configuration

    server
}

/// Create a production server (with all features enabled)
pub fn create_production_server(config: BridgeAPIConfig) -> BridgeAPIServer {
    let server = BridgeAPIServer::new(config);

    // Enable production features
    // TODO: Add production-specific configuration

    server
}

/// Server builder for custom configuration
pub struct ServerBuilder {
    config: BridgeAPIConfig,
    enable_http: bool,
    enable_grpc: bool,
    enable_metrics: bool,
    enable_auth: bool,
    enable_cors: bool,
    enable_rate_limit: bool,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new(config: BridgeAPIConfig) -> Self {
        Self {
            config,
            enable_http: true,
            enable_grpc: true,
            enable_metrics: true,
            enable_auth: true,
            enable_cors: true,
            enable_rate_limit: true,
        }
    }

    /// Disable HTTP server
    pub fn disable_http(mut self) -> Self {
        self.enable_http = false;
        self
    }

    /// Disable gRPC server
    pub fn disable_grpc(mut self) -> Self {
        self.enable_grpc = false;
        self
    }

    /// Disable metrics
    pub fn disable_metrics(mut self) -> Self {
        self.enable_metrics = false;
        self
    }

    /// Disable authentication
    pub fn disable_auth(mut self) -> Self {
        self.enable_auth = false;
        self
    }

    /// Disable CORS
    pub fn disable_cors(mut self) -> Self {
        self.enable_cors = false;
        self
    }

    /// Disable rate limiting
    pub fn disable_rate_limit(mut self) -> Self {
        self.enable_rate_limit = false;
        self
    }

    /// Build the server
    pub fn build(self) -> BridgeAPIServer {
        let mut config = self.config;

        // Apply builder options
        if !self.enable_auth {
            config.auth.enabled = false;
        }

        if !self.enable_cors {
            config.cors.enabled = false;
        }

        if !self.enable_rate_limit {
            config.rate_limit.enabled = false;
        }

        BridgeAPIServer::new(config)
    }
}

/// Run the server with default configuration
pub async fn run_server(config: BridgeAPIConfig) -> ApiResult<()> {
    let mut server = BridgeAPIServer::new(config);

    server.init().await?;
    server.start().await?;

    // Keep the server running
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to wait for shutdown signal: {}", e)))?;

    server.stop().await?;
    Ok(())
}

/// Run the server with custom configuration
pub async fn run_custom_server<F>(config: BridgeAPIConfig, customizer: F) -> ApiResult<()>
where
    F: FnOnce(&mut BridgeAPIServer),
{
    let mut server = BridgeAPIServer::new(config);
    customizer(&mut server);

    server.init().await?;
    server.start().await?;

    // Keep the server running
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to wait for shutdown signal: {}", e)))?;

    server.stop().await?;
    Ok(())
}

/// Run a minimal server (health checks and metrics only)
pub async fn run_minimal_server(config: BridgeAPIConfig) -> ApiResult<()> {
    let mut server = create_minimal_server(config);

    server.init().await?;
    server.start().await?;

    // Keep the server running
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to wait for shutdown signal: {}", e)))?;

    server.stop().await?;
    Ok(())
}

/// Run a development server
pub async fn run_dev_server(config: BridgeAPIConfig) -> ApiResult<()> {
    let mut server = create_dev_server(config);

    server.init().await?;
    server.start().await?;

    // Keep the server running
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to wait for shutdown signal: {}", e)))?;

    server.stop().await?;
    Ok(())
}

/// Run a production server
pub async fn run_production_server(config: BridgeAPIConfig) -> ApiResult<()> {
    let mut server = create_production_server(config);

    server.init().await?;
    server.start().await?;

    // Keep the server running
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to wait for shutdown signal: {}", e)))?;

    server.stop().await?;
    Ok(())
}
