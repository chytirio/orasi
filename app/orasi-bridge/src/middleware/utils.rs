//! Utility types and functions for middleware

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use chrono;

use crate::{config::BridgeAPIConfig, rest::AppState};

/// Request context
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Request ID
    pub request_id: String,

    /// Start time
    pub start_time: Instant,

    /// User ID (if authenticated)
    pub user_id: Option<String>,

    /// Request metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
            user_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// Error handling middleware
pub async fn error_handling_middleware(
    State(_state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    next.run(request).await
}

/// Request ID middleware
pub async fn request_id_middleware(
    State(_state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Generate request ID if not present
    if request.extensions().get::<RequestContext>().is_none() {
        let context = RequestContext::new();
        request.extensions_mut().insert(context);
    }

    let response = next.run(request).await;

    // Add request ID to response headers
    let mut response = response;
    if let Some(context) = response.extensions().get::<RequestContext>() {
        let request_id = context.request_id.clone();
        response
            .headers_mut()
            .insert("X-Request-ID", request_id.parse().unwrap());
    }

    response
}

/// Request timeout middleware
pub async fn timeout_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let timeout = state.config.http.request_timeout;

    match tokio::time::timeout(timeout, next.run(request)).await {
        Ok(response) => response,
        Err(_) => Response::builder()
            .status(408)
            .body(axum::body::Body::from("Request timeout"))
            .unwrap(),
    }
}

/// Request size limit middleware
pub async fn size_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let max_size = state.config.http.max_request_size;

    // Check content length header
    if let Some(content_length) = request.headers().get("content-length") {
        if let Ok(size) = content_length.to_str().unwrap_or("0").parse::<usize>() {
            if size > max_size {
                return Response::builder()
                    .status(413)
                    .body(axum::body::Body::from(format!(
                        "Request body too large: {} bytes (max: {} bytes)",
                        size, max_size
                    )))
                    .unwrap();
            }
        }
    }

    next.run(request).await
}

/// Compression middleware
pub fn compression_middleware(
    config: &BridgeAPIConfig,
) -> tower_http::compression::CompressionLayer {
    if config.http.enable_compression {
        tower_http::compression::CompressionLayer::new()
    } else {
        // Return a no-op compression layer when compression is disabled
        tower_http::compression::CompressionLayer::new()
    }
}

/// Keep-alive middleware
pub fn keep_alive_middleware(config: &BridgeAPIConfig) -> tower_http::limit::RequestBodyLimitLayer {
    if config.http.enable_keep_alive {
        tower_http::limit::RequestBodyLimitLayer::new(config.http.max_request_size)
    } else {
        tower_http::limit::RequestBodyLimitLayer::new(0)
    }
}

/// Health check result
#[derive(Debug)]
pub(crate) struct HealthCheckResult {
    pub is_healthy: bool,
    pub details: std::collections::HashMap<String, String>,
}

/// Perform lightweight health checks
pub(crate) async fn perform_health_checks(state: &AppState) -> HealthCheckResult {
    let mut details = std::collections::HashMap::new();
    let mut is_healthy = true;

    // Check bridge core health
    match bridge_core::get_bridge_status().await {
        Ok(status) => {
            if status.status != "running" {
                is_healthy = false;
                details.insert(
                    "bridge_core".to_string(),
                    format!("Status: {}", status.status),
                );
            } else {
                details.insert("bridge_core".to_string(), "healthy".to_string());
            }
        }
        Err(e) => {
            is_healthy = false;
            details.insert("bridge_core".to_string(), format!("Error: {}", e));
        }
    }

    // Check if metrics are accessible (basic availability check)
    let metrics_string = crate::metrics::get_metrics();
    if metrics_string.is_empty() {
        is_healthy = false;
        details.insert(
            "metrics".to_string(),
            "Error: No metrics available".to_string(),
        );
    } else {
        details.insert("metrics".to_string(), "healthy".to_string());
    }

    // Check configuration validity
    if let Err(e) = state.config.validate() {
        is_healthy = false;
        details.insert("configuration".to_string(), format!("Error: {}", e));
    } else {
        details.insert("configuration".to_string(), "valid".to_string());
    }

    // Add timestamp
    details.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());

    HealthCheckResult {
        is_healthy,
        details,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BridgeAPIConfig;
    use crate::metrics::ApiMetrics;

    #[tokio::test]
    async fn test_perform_health_checks() {
        let config = BridgeAPIConfig::default();
        let metrics = ApiMetrics::new();
        let state = AppState { config, metrics };

        let result = perform_health_checks(&state).await;

        // Should be healthy with default configuration
        assert!(result.is_healthy);
        assert!(result.details.contains_key("bridge_core"));
        assert!(result.details.contains_key("metrics"));
        assert!(result.details.contains_key("configuration"));
        assert!(result.details.contains_key("timestamp"));
    }
}
