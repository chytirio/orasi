//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Orasi Gateway for routing traffic and servicing external requests
//!
//! This module provides the gateway functionality for the Orasi distributed system,
//! handling external traffic routing, load balancing, and API services.

pub mod config;
pub mod error;
pub mod gateway;
pub mod health;
pub mod load_balancer;
pub mod metrics;
pub mod proxy;
pub mod rate_limiter;
pub mod routing;
pub mod service_discovery;
pub mod tls;
pub mod types;

// Re-export main types
pub use gateway::OrasiGateway;
pub use config::GatewayConfig;
pub use error::GatewayError;
pub use types::*;

/// Result type for gateway operations
pub type GatewayResult<T> = Result<T, GatewayError>;

/// Gateway version information
pub const GATEWAY_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Gateway name
pub const GATEWAY_NAME: &str = "orasi-gateway";

/// Default gateway endpoint
pub const DEFAULT_GATEWAY_ENDPOINT: &str = "0.0.0.0:8080";

/// Default health check endpoint
pub const DEFAULT_HEALTH_ENDPOINT: &str = "0.0.0.0:8081";

/// Default metrics endpoint
pub const DEFAULT_METRICS_ENDPOINT: &str = "0.0.0.0:9090";

/// Default admin endpoint
pub const DEFAULT_ADMIN_ENDPOINT: &str = "0.0.0.0:8082";

/// Default request timeout in seconds
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Default max request body size in bytes
pub const DEFAULT_MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Default rate limit requests per second
pub const DEFAULT_RATE_LIMIT_RPS: u32 = 1000;

/// Initialize orasi gateway
pub async fn init_gateway(config: GatewayConfig) -> GatewayResult<OrasiGateway> {
    tracing::info!("Initializing Orasi gateway v{}", GATEWAY_VERSION);
    
    let gateway = OrasiGateway::new(config).await?;
    tracing::info!("Orasi gateway initialization completed");
    
    Ok(gateway)
}

/// Shutdown orasi gateway
pub async fn shutdown_gateway(gateway: OrasiGateway) -> GatewayResult<()> {
    tracing::info!("Shutting down Orasi gateway");
    
    gateway.shutdown().await?;
    tracing::info!("Orasi gateway shutdown completed");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_initialization() {
        let config = GatewayConfig::default();
        let result = init_gateway(config).await;
        assert!(result.is_ok());
    }
}
