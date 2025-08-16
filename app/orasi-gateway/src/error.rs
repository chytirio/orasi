//! Error types for Orasi Gateway

use thiserror::Error;

/// Error type for gateway operations
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Service discovery error: {0}")]
    ServiceDiscovery(String),

    #[error("Load balancing error: {0}")]
    LoadBalancing(String),

    #[error("Routing error: {0}")]
    Routing(String),

    #[error("Proxy error: {0}")]
    Proxy(String),

    #[error("Rate limiting error: {0}")]
    RateLimiting(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Health check error: {0}")]
    Health(String),

    #[error("Metrics error: {0}")]
    Metrics(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Circuit breaker error: {0}")]
    CircuitBreaker(String),

    #[error("Retry error: {0}")]
    Retry(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Shutdown error: {0}")]
    Shutdown(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<std::io::Error> for GatewayError {
    fn from(err: std::io::Error) -> Self {
        GatewayError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for GatewayError {
    fn from(err: serde_json::Error) -> Self {
        GatewayError::Serialization(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for GatewayError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        GatewayError::Timeout(err.to_string())
    }
}

impl From<reqwest::Error> for GatewayError {
    fn from(err: reqwest::Error) -> Self {
        GatewayError::Network(err.to_string())
    }
}
