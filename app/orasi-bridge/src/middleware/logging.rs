//! Logging and metrics middleware

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use uuid::Uuid;

use super::utils::RequestContext;
use crate::{config::BridgeAPIConfig, rest::AppState};

/// Logging middleware
pub async fn logging_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Extract request ID from context or generate new one
    let request_id = request
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.request_id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Log request
    if state.config.logging.enable_request_logging {
        tracing::info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            "Incoming request"
        );
    }

    // Process request
    let response = next.run(request).await;

    // Log response
    if state.config.logging.enable_response_logging {
        let duration = start_time.elapsed();
        let status = response.status();

        tracing::info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = duration.as_millis(),
            "Request completed"
        );
    }

    response
}

/// Metrics middleware
pub async fn metrics_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = request.method().as_str().to_string();
    let path = request.uri().path().to_string();

    // Increment active connections
    state.metrics.increment_active_connections();

    // Process request
    let response = next.run(request).await;

    // Decrement active connections
    state.metrics.decrement_active_connections();

    // Record metrics
    let duration = start_time.elapsed();
    let status_code = response.status().as_u16();

    state.metrics.record_request(&method, &path, status_code);
    state.metrics.record_response_time(&method, &path, duration);

    // Record error if status code indicates error
    if status_code >= 400 {
        let error_type = if status_code >= 500 {
            "server_error"
        } else {
            "client_error"
        };
        state.metrics.record_error(error_type, &method, &path);
    }

    response
}
