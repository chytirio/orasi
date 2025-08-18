//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! REST API routing for Bridge API

use axum::{
    extract::State,
    middleware,
    routing::{get, post, put},
    Router,
};
use tower_http::trace::TraceLayer;

use crate::{
    config::BridgeAPIConfig, error::ApiError, handlers::*, metrics::ApiMetrics, middleware::*,
};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: BridgeAPIConfig,
    pub metrics: ApiMetrics,
}

/// Create the REST API router
pub fn create_rest_router(config: BridgeAPIConfig, metrics: ApiMetrics) -> Router<AppState> {
    let state = AppState { config, metrics };

    // Create the main router with shared state
    let app = Router::new()
        // Root endpoint
        .route("/", get(root_handler))
        // Health check endpoints
        .route("/health/live", get(health_live_handler))
        .route("/health/ready", get(health_ready_handler))
        // Metrics endpoint
        .route("/metrics", get(metrics_handler))
        // API v1 routes
        .nest("/api/v1", create_api_v1_router())
        // OTLP endpoints
        .nest("/v1", create_otlp_router())
        // Add middleware
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_id_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            logging_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            timeout_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            size_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security_headers_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            health_check_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            error_handling_middleware,
        ))
        .layer(cors_middleware(&state.config))
        .layer(compression_middleware(&state.config))
        .layer(keep_alive_middleware(&state.config))
        .layer(TraceLayer::new_for_http())
        // Add shared state
        .with_state(state)
        // Fallback handler for 404
        .fallback(not_found_handler);

    app
}

/// Create API v1 router
fn create_api_v1_router() -> Router<AppState> {
    Router::new()
        // Status endpoints
        .route("/status", get(status_handler))
        // Telemetry endpoints
        .route("/telemetry/batch", post(telemetry_ingestion_handler))
        .route("/telemetry/stream", post(telemetry_ingestion_handler))
        // Query endpoints
        .route("/query/metrics", post(query_handler))
        .route("/query/traces", post(query_handler))
        .route("/query/logs", post(query_handler))
        .route("/query/analytics", post(query_handler))
        .route("/query/schema", get(query_handler))
        .route("/query/capabilities", get(query_handler))
        // Analytics endpoints
        .route("/analytics/workflow", post(analytics_handler))
        .route("/analytics/agent", post(analytics_handler))
        .route("/analytics/multi-repo", post(analytics_handler))
        .route("/analytics/insights", get(analytics_handler))
        .route("/analytics/trends", get(analytics_handler))
        .route("/analytics/alerts", post(analytics_handler))
        // Configuration endpoints
        .route("/config", get(get_config_handler))
        .route("/config", put(update_config_handler))
        .route("/config/validate", post(validate_config_handler))
        // Component management endpoints
        .route("/components", get(list_components_handler))
        .route(
            "/components/:name/status",
            get(get_component_status_handler),
        )
        .route("/components/:name/restart", post(restart_component_handler))
        .route("/components/restart", post(restart_components_handler))
        // Plugin endpoints
        .route("/plugin/capabilities", get(plugin_capabilities_handler))
        .route("/plugin/query", post(plugin_query_handler))
        .route("/plugin/stream", get(plugin_query_handler))
        .route("/plugin/analytics", post(plugin_query_handler))
}

/// Create OTLP router
fn create_otlp_router() -> Router<AppState> {
    Router::new()
        .route("/traces", post(otlp_traces_handler))
        .route("/metrics", post(otlp_metrics_handler))
        .route("/logs", post(otlp_logs_handler))
}

/// Create a simple health check router (for basic health checks)
pub fn create_health_router() -> Router<AppState> {
    Router::new()
        .route("/health/live", get(health_live_handler))
        .route("/health/ready", get(health_ready_handler))
        .route("/metrics", get(metrics_handler))
}

/// Create a metrics-only router (for metrics scraping)
pub fn create_metrics_router() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics_handler))
}

/// Create a development router (with additional debugging endpoints)
pub fn create_dev_router(config: BridgeAPIConfig, metrics: ApiMetrics) -> Router<AppState> {
    let state = AppState { config, metrics };

    let app = Router::new()
        // Include all regular routes
        .merge(create_rest_router(
            state.config.clone(),
            state.metrics.clone(),
        ))
        // Additional development endpoints
        .route("/dev/status", get(status_handler))
        .route("/dev/config", get(config_handler))
        .route("/dev/metrics", get(metrics_handler))
        // Add development middleware
        .layer(middleware::from_fn_with_state(
            state.clone(),
            dev_logging_middleware,
        ))
        .with_state(state);

    app
}

/// Development logging middleware (more verbose)
async fn dev_logging_middleware(
    State(_state): State<AppState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, ApiError> {
    let start_time = std::time::Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();

    // Log request details
    tracing::debug!(
        method = %method,
        uri = %uri,
        headers = ?headers,
        "Development request logging"
    );

    // Process request
    let response = next.run(request).await;

    // Log response details
    let duration = start_time.elapsed();
    let status = response.status();

    tracing::debug!(
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = duration.as_millis(),
        "Development response logging"
    );

    Ok(response)
}

/// Create router with custom configuration
pub fn create_custom_router<F>(
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
    custom_routes: F,
) -> Router<AppState>
where
    F: FnOnce(Router<AppState>) -> Router<AppState>,
{
    let base_router = create_rest_router(config, metrics);
    custom_routes(base_router)
}

/// Create router with authentication disabled
pub fn create_unauthenticated_router(
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
) -> Router<AppState> {
    let mut config = config;
    config.auth.enabled = false;

    create_rest_router(config, metrics)
}

/// Create router with CORS disabled
pub fn create_no_cors_router(config: BridgeAPIConfig, metrics: ApiMetrics) -> Router<AppState> {
    let mut config = config;
    config.cors.enabled = false;

    create_rest_router(config, metrics)
}

/// Create router with rate limiting disabled
pub fn create_no_rate_limit_router(
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
) -> Router<AppState> {
    let mut config = config;
    config.rate_limit.enabled = false;

    create_rest_router(config, metrics)
}

/// Create minimal router (health checks and metrics only)
pub fn create_minimal_router(config: BridgeAPIConfig, metrics: ApiMetrics) -> Router<AppState> {
    let state = AppState { config, metrics };

    Router::new()
        .route("/health/live", get(health_live_handler))
        .route("/health/ready", get(health_ready_handler))
        .route("/metrics", get(metrics_handler))
        .route("/", get(root_handler))
        .with_state(state)
}

/// Create telemetry-only router
pub fn create_telemetry_router(config: BridgeAPIConfig, metrics: ApiMetrics) -> Router<AppState> {
    let state = AppState { config, metrics };

    Router::new()
        .route("/v1/traces", post(otlp_traces_handler))
        .route("/v1/metrics", post(otlp_metrics_handler))
        .route("/v1/logs", post(otlp_logs_handler))
        .route("/api/v1/telemetry/batch", post(telemetry_ingestion_handler))
        .route(
            "/api/v1/telemetry/stream",
            post(telemetry_ingestion_handler),
        )
        .route("/health/live", get(health_live_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

/// Create query-only router
pub fn create_query_router(config: BridgeAPIConfig, metrics: ApiMetrics) -> Router<AppState> {
    let state = AppState { config, metrics };

    Router::new()
        .route("/api/v1/query/metrics", post(query_handler))
        .route("/api/v1/query/traces", post(query_handler))
        .route("/api/v1/query/logs", post(query_handler))
        .route("/api/v1/query/analytics", post(query_handler))
        .route("/api/v1/query/schema", get(query_handler))
        .route("/api/v1/query/capabilities", get(query_handler))
        .route("/health/live", get(health_live_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

/// Create management-only router
pub fn create_management_router(config: BridgeAPIConfig, metrics: ApiMetrics) -> Router<AppState> {
    let state = AppState { config, metrics };

    Router::new()
        .route("/api/v1/status", get(status_handler))
        .route("/api/v1/config", get(config_handler))
        .route("/api/v1/config", put(config_handler))
        .route("/api/v1/components", get(component_status_handler))
        .route(
            "/api/v1/components/:name/status",
            get(component_status_handler),
        )
        .route(
            "/api/v1/components/:name/restart",
            post(component_restart_handler),
        )
        .route("/health/live", get(health_live_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}
