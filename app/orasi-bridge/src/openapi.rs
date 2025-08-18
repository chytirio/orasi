//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OpenAPI documentation for Bridge API

use axum::{routing::get, Router};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::types::*;

/// OpenAPI specification for Bridge API
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::health::health_live_handler,
        crate::handlers::health::health_ready_handler,
        crate::handlers::health::status_handler,
        crate::handlers::telemetry::otlp_traces_handler,
        crate::handlers::telemetry::otlp_metrics_handler,
        crate::handlers::telemetry::otlp_logs_handler,
        crate::handlers::telemetry::telemetry_ingestion_handler,
        crate::handlers::query::query_handler,
        crate::handlers::query::query_handler,
        crate::handlers::query::query_handler,
        crate::handlers::analytics::analytics_handler,
    ),
    components(
        schemas(
            ApiResponse<HealthResponse>,
            ApiResponse<StatusResponse>,
            ApiResponse<QueryResponse>,
            ApiResponse<AnalyticsResponse>,
            HealthResponse,
            HealthStatus,
            ComponentHealth,
            ResponseMetadata,
            StatusResponse,
            QueryRequest,
            QueryResponse,
            AnalyticsRequest,
            AnalyticsResponse,
            TelemetryBatch,
            crate::error::ApiError,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "status", description = "Status and monitoring endpoints"),
        (name = "telemetry", description = "Telemetry ingestion endpoints"),
        (name = "query", description = "Data query endpoints"),
        (name = "analytics", description = "Analytics endpoints"),
    ),
    info(
        title = "Bridge API",
        description = "REST/gRPC API server for OpenTelemetry Data Lake Bridge",
        version = "0.1.0",
        contact(
            name = "Cory Parent",
            email = "goedelsoup@gmail.com"
        ),
        license(
            name = "Apache-2.0",
            url = "https://www.apache.org/licenses/LICENSE-2.0"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Development server"),
        (url = "https://api.bridge.example.com", description = "Production server")
    )
)]
pub struct ApiDoc;

/// Create OpenAPI documentation router
pub fn create_openapi_router() -> Router<crate::rest::AppState> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}

/// Get OpenAPI specification as JSON
pub fn get_openapi_spec() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap()
}
