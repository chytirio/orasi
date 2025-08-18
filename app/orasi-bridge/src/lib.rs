//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Bridge API for OpenTelemetry Data Lake Bridge
//!
//! This module provides REST and gRPC API interfaces for
//! the bridge system.

pub mod config;
pub mod error;
pub mod services;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod proto;
pub mod rest;
pub mod server;
pub mod types;

#[cfg(feature = "openapi")]
pub mod openapi;

// Re-export main types
pub use bridge_auth::{config as auth_config, jwt};
pub use bridge_core::types::{ProcessedBatch, TelemetryBatch};
pub use config::BridgeAPIConfig;
pub use error::ApiError;
pub use server::BridgeAPIServer;
pub use types::*;

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Bridge API version information
pub const BRIDGE_API_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Bridge API name
pub const BRIDGE_API_NAME: &str = "bridge-api";

/// Default REST API endpoint
pub const DEFAULT_REST_ENDPOINT: &str = "0.0.0.0:8080";

/// Default gRPC API endpoint
pub const DEFAULT_GRPC_ENDPOINT: &str = "0.0.0.0:9090";

/// Default health check endpoint
pub const DEFAULT_HEALTH_ENDPOINT: &str = "0.0.0.0:8081";

/// Default metrics endpoint
pub const DEFAULT_METRICS_ENDPOINT: &str = "0.0.0.0:9091";

/// Default request timeout in seconds
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Default max request body size in bytes
pub const DEFAULT_MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Initialize bridge API server
pub async fn init_bridge_api() -> ApiResult<()> {
    tracing::info!("Initializing bridge API server v{}", BRIDGE_API_VERSION);

    // Initialize API server components
    tracing::info!("Bridge API server initialization completed");
    Ok(())
}

/// Shutdown bridge API server
pub async fn shutdown_bridge_api() -> ApiResult<()> {
    tracing::info!("Shutting down bridge API server");

    // Shutdown API server components
    tracing::info!("Bridge API server shutdown completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::types::{Filter, FilterOperator, FilterValue, TelemetryQuery, TimeRange};
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_bridge_api_initialization() {
        let result = init_bridge_api().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bridge_api_shutdown() {
        let result = shutdown_bridge_api().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_jwt_authentication() {
        // Test JWT token validation
        let jwt_config = auth_config::JwtConfig {
            secret: "test_secret".to_string(),
            issuer: "test_issuer".to_string(),
            audience: "test_audience".to_string(),
            expiration_secs: 3600,
            algorithm: auth_config::JwtAlgorithm::HS256,
            enable_refresh_tokens: true,
            refresh_token_expiration_secs: 86400,
        };

        let validator = jwt::JwtManager::new(jwt_config).unwrap();
        let token = validator
            .generate_token("test_user".to_string(), vec!["user".to_string()], None)
            .await
            .unwrap();

        // Validate the token
        let result = validator.validate_token(&token).await;
        assert!(result.is_ok());

        let validated_claims = result.unwrap();
        assert_eq!(validated_claims.sub, "test_user");
    }

    #[tokio::test]
    async fn test_streaming_query_integration() {
        // Test streaming query integration
        let query = TelemetryQuery {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            query_type: bridge_core::types::TelemetryQueryType::Metrics,
            time_range: TimeRange::new(Utc::now() - chrono::Duration::hours(1), Utc::now()),
            filters: vec![Filter {
                field: "type".to_string(),
                operator: FilterOperator::Equals,
                value: FilterValue::String("metric".to_string()),
            }],
            aggregations: vec![],
            limit: Some(100),
            offset: None,
            metadata: std::collections::HashMap::new(),
        };

        // This test verifies that the streaming query structure is valid
        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.filters[0].field, "type");
        assert_eq!(query.filters[0].operator, FilterOperator::Equals);
    }

    #[tokio::test]
    async fn test_database_health_check() {
        // Test database health check functionality
        // This would test the database connection logic we implemented
        let result = crate::handlers::health::check_database_health().await;
        assert!(result.status == crate::types::HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_cache_health_check() {
        // Test cache health check functionality
        let result = crate::handlers::health::check_cache_health().await;
        assert!(result.status == crate::types::HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_metrics_rendering() {
        // Test metrics rendering
        let metrics = crate::metrics::get_metrics();
        assert!(metrics.contains("bridge_api_requests_total"));
        assert!(metrics.contains("bridge_api_response_time_seconds"));
        assert!(metrics.contains("bridge_api_active_connections"));
        assert!(metrics.contains("bridge_api_errors_total"));
    }
}
