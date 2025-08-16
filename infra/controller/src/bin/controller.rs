//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0

//! Main controller binary for the Orasi controller

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};

use chrono::Utc;
use orasi_controller::{Controller, ControllerResult, Metrics};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

/// Application state
#[derive(Clone)]
struct AppState {
    /// Metrics collection
    metrics: Arc<Metrics>,
    /// Last event timestamp
    last_event: Arc<tokio::sync::RwLock<chrono::DateTime<Utc>>>,
}

/// Health check endpoint
async fn health_check() -> StatusCode {
    StatusCode::OK
}

/// Metrics endpoint
async fn metrics(State(_state): State<AppState>) -> String {
    "metrics_placeholder".to_string()
}

/// Root endpoint with debug information
async fn root(State(state): State<AppState>) -> Json<serde_json::Value> {
    let last_event = *state.last_event.read().await;
    Json(serde_json::json!({
        "last_event": last_event.to_rfc3339(),
        "service": "orasi-controller",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Start the HTTP server
async fn start_server(state: AppState) -> ControllerResult<()> {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/metrics", get(metrics))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| orasi_controller::ControllerError::GeneralError(e.to_string()))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| orasi_controller::ControllerError::GeneralError(e.to_string()))?;

    Ok(())
}

/// Initialize metrics
fn init_metrics() -> Arc<Metrics> {
    Arc::new(Metrics::new())
}

/// Initialize logging
fn init_logging() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,orasi_controller=debug".into());

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    init_logging();

    info!("Starting Orasi controller v{}", env!("CARGO_PKG_VERSION"));

    // Initialize metrics
    let metrics = init_metrics();

    // Create application state
    let state = AppState {
        metrics: metrics.clone(),
        last_event: Arc::new(tokio::sync::RwLock::new(Utc::now())),
    };

    // Start HTTP server in background
    let server_handle = tokio::spawn(start_server(state));

    // Create and run controller
    let controller = Controller::new(metrics);
    let controller_handle = tokio::spawn(controller.run());

    // Wait for shutdown signal
    let _ = signal::ctrl_c().await;
    info!("Received shutdown signal");

    // Graceful shutdown
    info!("Shutting down...");
    
    // Cancel server and controller
    server_handle.abort();
    controller_handle.abort();

    // Wait for tasks to finish
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        if let Err(e) = server_handle.await {
            if !e.is_cancelled() {
                error!("Server task failed: {:?}", e);
            }
        }
        if let Err(e) = controller_handle.await {
            if !e.is_cancelled() {
                error!("Controller task failed: {:?}", e);
            }
        }
    })
    .await;

    info!("Shutdown complete");
    Ok(())
}
