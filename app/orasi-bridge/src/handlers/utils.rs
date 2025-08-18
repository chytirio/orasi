//! Utility functions for handlers

use axum::response::Json;
use serde_json;

use crate::{
    error::{ApiError, ApiResult},
    types::*,
};

/// Calculate configuration hash for change detection
pub async fn calculate_config_hash() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Get current configuration from environment or default
    let mut hasher = DefaultHasher::new();

    // Hash relevant configuration values
    let config_values = [
        std::env::var("DATABASE_URL").unwrap_or_default(),
        std::env::var("CACHE_URL").unwrap_or_default(),
        std::env::var("BRIDGE_API_VERSION")
            .unwrap_or_else(|_| crate::BRIDGE_API_VERSION.to_string()),
        std::env::var("BRIDGE_CORE_VERSION")
            .unwrap_or_else(|_| bridge_core::BRIDGE_VERSION.to_string()),
    ];

    for value in &config_values {
        value.hash(&mut hasher);
    }

    // Add timestamp to make hash unique
    chrono::Utc::now().timestamp().hash(&mut hasher);

    format!("{:x}", hasher.finish())
}

/// Metrics handler
pub async fn metrics_handler() -> ApiResult<String> {
    // Return Prometheus metrics
    Ok(crate::metrics::get_metrics())
}

/// Root handler
pub async fn root_handler() -> ApiResult<Json<serde_json::Value>> {
    let response = serde_json::json!({
        "name": crate::BRIDGE_API_NAME,
        "version": crate::BRIDGE_API_VERSION,
        "description": "OpenTelemetry Data Lake Bridge API",
        "endpoints": {
            "health": "/health/live",
            "ready": "/health/ready",
            "status": "/api/v1/status",
            "telemetry": "/api/v1/telemetry",
            "query": "/api/v1/query",
            "analytics": "/api/v1/analytics",
            "metrics": "/metrics",
        }
    });

    Ok(Json(response))
}

/// Not found handler
pub async fn not_found_handler() -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError::NotFound("Endpoint not found".to_string()))
}
