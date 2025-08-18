//! Health check middleware

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum::http::StatusCode;
use serde_json;

use crate::rest::AppState;
use super::utils::perform_health_checks;

/// Health check middleware
pub async fn health_check_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();

    // Skip middleware for health check endpoints
    if path == "/health/live" || path == "/health/ready" || path == "/metrics" {
        return next.run(request).await;
    }

    // Perform lightweight health checks
    let health_status = perform_health_checks(&state).await;

    // If health checks fail, return 503 Service Unavailable
    if !health_status.is_healthy {
        return Response::builder()
            .status(503)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::json!({
                    "error": "Service temporarily unavailable",
                    "reason": "Health check failed",
                    "details": health_status.details,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
                .to_string(),
            ))
            .unwrap();
    }

    next.run(request).await
}
