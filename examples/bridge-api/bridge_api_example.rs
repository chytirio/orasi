//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example usage of Bridge API

use bridge_api::{BridgeAPIConfig, run_server, run_minimal_server, ServerBuilder};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Bridge API Example");
    println!("==================");

    // Example 1: Run with default configuration
    println!("\n1. Running with default configuration...");
    run_default_example().await?;

    // Example 2: Run with custom configuration
    println!("\n2. Running with custom configuration...");
    run_custom_example().await?;

    // Example 3: Run minimal server
    println!("\n3. Running minimal server...");
    run_minimal_example().await?;

    // Example 4: Run with server builder
    println!("\n4. Running with server builder...");
    run_builder_example().await?;

    Ok(())
}

/// Example 1: Run with default configuration
async fn run_default_example() -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeAPIConfig::default();
    
    println!("  HTTP Server: {}", config.http_address());
    println!("  gRPC Server: {}", config.grpc_address());
    println!("  Metrics: {}", config.metrics_address());
    
    // Note: This would run indefinitely, so we'll just show the config
    println!("  Configuration loaded successfully");
    
    Ok(())
}

/// Example 2: Run with custom configuration
async fn run_custom_example() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = BridgeAPIConfig::default();
    
    // Customize configuration
    config.http.port = 8081;
    config.grpc.port = 9091;
    config.metrics.port = 9092;
    
    // Disable authentication for development
    config.auth.enabled = false;
    
    // Enable verbose logging
    config.logging.level = "debug".to_string();
    config.logging.enable_request_logging = true;
    config.logging.enable_response_logging = true;
    
    println!("  Custom HTTP Server: {}", config.http_address());
    println!("  Custom gRPC Server: {}", config.grpc_address());
    println!("  Custom Metrics: {}", config.metrics_address());
    println!("  Authentication: {}", if config.auth.enabled { "enabled" } else { "disabled" });
    println!("  Log Level: {}", config.logging.level);
    
    // Note: This would run indefinitely, so we'll just show the config
    println!("  Custom configuration loaded successfully");
    
    Ok(())
}

/// Example 3: Run minimal server
async fn run_minimal_example() -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeAPIConfig::default();
    
    println!("  Minimal server configuration:");
    println!("  - Health checks only");
    println!("  - Metrics endpoint");
    println!("  - No authentication");
    println!("  - No CORS");
    println!("  - No rate limiting");
    
    // Note: This would run indefinitely, so we'll just show the config
    println!("  Minimal configuration loaded successfully");
    
    Ok(())
}

/// Example 4: Run with server builder
async fn run_builder_example() -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeAPIConfig::default();
    
    let server = ServerBuilder::new(config)
        .disable_auth()
        .disable_cors()
        .disable_rate_limit()
        .build();
    
    println!("  Server builder configuration:");
    println!("  - Authentication: disabled");
    println!("  - CORS: disabled");
    println!("  - Rate limiting: disabled");
    println!("  - HTTP: enabled");
    println!("  - gRPC: enabled");
    println!("  - Metrics: enabled");
    
    // Note: This would run indefinitely, so we'll just show the config
    println!("  Server builder configuration loaded successfully");
    
    Ok(())
}

/// Example 5: Load configuration from file
async fn load_config_from_file() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n5. Loading configuration from file...");
    
    // Try to load from config file
    match BridgeAPIConfig::from_file("config/bridge-api.toml") {
        Ok(config) => {
            println!("  Configuration loaded from file successfully");
            println!("  HTTP Server: {}", config.http_address());
            println!("  gRPC Server: {}", config.grpc_address());
            println!("  Metrics: {}", config.metrics_address());
        }
        Err(e) => {
            println!("  Failed to load configuration from file: {}", e);
            println!("  Using default configuration");
        }
    }
    
    Ok(())
}

/// Example 6: Create sample configuration file
fn create_sample_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n6. Creating sample configuration file...");
    
    let config = BridgeAPIConfig::default();
    let config_toml = toml::to_string_pretty(&config)?;
    
    std::fs::write("config/bridge-api-sample.toml", config_toml)?;
    println!("  Sample configuration file created: config/bridge-api-sample.toml");
    
    Ok(())
}

/// Example 7: Test API endpoints
async fn test_api_endpoints() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n7. Testing API endpoints...");
    
    // This would typically use a test HTTP client to make requests
    // For now, just show the available endpoints
    
    println!("  Available endpoints:");
    println!("  - GET  /                    - Root endpoint");
    println!("  - GET  /health/live         - Liveness probe");
    println!("  - GET  /health/ready        - Readiness probe");
    println!("  - GET  /metrics             - Prometheus metrics");
    println!("  - GET  /api/v1/status       - Bridge status");
    println!("  - POST /api/v1/telemetry/batch - Telemetry ingestion");
    println!("  - POST /api/v1/query/metrics   - Metrics query");
    println!("  - POST /api/v1/query/traces    - Traces query");
    println!("  - POST /api/v1/query/logs      - Logs query");
    println!("  - POST /api/v1/analytics/workflow - Workflow analytics");
    println!("  - GET  /api/v1/config       - Get configuration");
    println!("  - PUT  /api/v1/config       - Update configuration");
    println!("  - GET  /api/v1/components   - List components");
    println!("  - POST /v1/traces           - OTLP traces");
    println!("  - POST /v1/metrics          - OTLP metrics");
    println!("  - POST /v1/logs             - OTLP logs");
    
    Ok(())
}

/// Example 8: Show configuration options
fn show_configuration_options() {
    println!("\n8. Configuration options:");
    
    println!("  HTTP Server:");
    println!("    - address: Server address (default: 0.0.0.0)");
    println!("    - port: Server port (default: 8080)");
    println!("    - request_timeout: Request timeout (default: 30s)");
    println!("    - max_request_size: Max request body size (default: 10MB)");
    println!("    - enable_compression: Enable response compression (default: true)");
    println!("    - enable_keep_alive: Enable keep-alive (default: true)");
    
    println!("  gRPC Server:");
    println!("    - address: Server address (default: 0.0.0.0)");
    println!("    - port: Server port (default: 9090)");
    println!("    - enable_reflection: Enable gRPC reflection (default: true)");
    println!("    - enable_health_checks: Enable health checks (default: true)");
    println!("    - max_message_size: Max message size (default: 4MB)");
    println!("    - max_concurrent_streams: Max concurrent streams (default: 1000)");
    
    println!("  Authentication:");
    println!("    - enabled: Enable authentication (default: false)");
    println!("    - auth_type: Authentication type (None, ApiKey, Jwt, OAuth)");
    println!("    - api_key: API key configuration");
    println!("    - jwt: JWT configuration");
    println!("    - oauth: OAuth configuration");
    
    println!("  CORS:");
    println!("    - enabled: Enable CORS (default: true)");
    println!("    - allowed_origins: Allowed origins (default: *)");
    println!("    - allowed_methods: Allowed HTTP methods");
    println!("    - allowed_headers: Allowed headers");
    println!("    - allow_credentials: Allow credentials (default: true)");
    println!("    - max_age: Max age (default: 1 hour)");
    
    println!("  Rate Limiting:");
    println!("    - enabled: Enable rate limiting (default: true)");
    println!("    - requests_per_second: Requests per second (default: 100)");
    println!("    - burst_size: Burst size (default: 200)");
    println!("    - window_size: Rate limit window (default: 60s)");
    
    println!("  Logging:");
    println!("    - level: Log level (default: info)");
    println!("    - enable_request_logging: Enable request logging (default: true)");
    println!("    - enable_response_logging: Enable response logging (default: false)");
    println!("    - format: Log format (Json, Text)");
    
    println!("  Metrics:");
    println!("    - enabled: Enable metrics (default: true)");
    println!("    - address: Metrics address (default: 0.0.0.0)");
    println!("    - port: Metrics port (default: 9091)");
    println!("    - path: Metrics path (default: /metrics)");
    
    println!("  Security:");
    println!("    - enable_security_headers: Enable security headers (default: true)");
    println!("    - content_security_policy: Content Security Policy");
    println!("    - strict_transport_security: Strict Transport Security");
    println!("    - x_frame_options: X-Frame-Options");
    println!("    - x_content_type_options: X-Content-Type-Options");
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_api::BridgeAPIConfig;

    #[test]
    fn test_default_config() {
        let config = BridgeAPIConfig::default();
        assert_eq!(config.http.port, 8080);
        assert_eq!(config.grpc.port, 9090);
        assert_eq!(config.metrics.port, 9091);
        assert!(!config.auth.enabled);
        assert!(config.cors.enabled);
        assert!(config.rate_limit.enabled);
    }

    #[test]
    fn test_custom_config() {
        let mut config = BridgeAPIConfig::default();
        config.http.port = 8081;
        config.grpc.port = 9091;
        config.auth.enabled = true;
        
        assert_eq!(config.http.port, 8081);
        assert_eq!(config.grpc.port, 9091);
        assert!(config.auth.enabled);
    }

    #[test]
    fn test_config_addresses() {
        let config = BridgeAPIConfig::default();
        assert_eq!(config.http_address(), "0.0.0.0:8080");
        assert_eq!(config.grpc_address(), "0.0.0.0:9090");
        assert_eq!(config.metrics_address(), "0.0.0.0:9091");
    }
}
