//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Complete streaming processor integration example
//!
//! This example demonstrates:
//! - OAuth configuration mapping between different crates
//! - Proper API error type handling
//! - Complete streaming processor integration
//! - Configuration validation and error handling

use bridge_api::error::{ApiError, EnhancedErrorResponse, ErrorContext};
use bridge_auth::config::{OAuthConfig, OAuthConfigConverter, OAuthProviderConfig};
use bridge_core::BridgeResult;
use std::collections::HashMap;
use streaming_processor::{
    config::{
        ConnectionConfig, MetricsConfig, ProcessingConfig, ProcessorConfig, ProcessorType,
        SecurityConfig, SinkConfig, SinkType, SourceConfig, SourceType, StateConfig,
    },
    StreamingProcessor, StreamingProcessorConfig,
};
use tracing::info;

/// Example OAuth provider configurations
fn create_oauth_providers() -> HashMap<String, OAuthProviderConfig> {
    let mut providers = HashMap::new();

    // Google OAuth provider
    providers.insert(
        "google".to_string(),
        OAuthProviderConfig::new(
            "your-google-client-id".to_string(),
            "your-google-client-secret".to_string(),
            "https://accounts.google.com/o/oauth2/auth".to_string(),
            "https://oauth2.googleapis.com/token".to_string(),
            "https://www.googleapis.com/oauth2/v2/userinfo".to_string(),
            vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
        ),
    );

    // GitHub OAuth provider
    providers.insert(
        "github".to_string(),
        OAuthProviderConfig::new(
            "your-github-client-id".to_string(),
            "your-github-client-secret".to_string(),
            "https://github.com/login/oauth/authorize".to_string(),
            "https://github.com/login/oauth/access_token".to_string(),
            "https://api.github.com/user".to_string(),
            vec!["read:user".to_string(), "user:email".to_string()],
        ),
    );

    providers
}

/// Create a complete streaming processor configuration
fn create_streaming_processor_config() -> StreamingProcessorConfig {
    let mut sources = HashMap::new();

    // HTTP source configuration
    sources.insert(
        "http_source".to_string(),
        SourceConfig {
            source_type: SourceType::Http,
            name: "http_source".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert(
                    "endpoint_url".to_string(),
                    serde_json::json!("https://httpbin.org/json"),
                );
                config.insert("method".to_string(), serde_json::json!("GET"));
                config.insert("polling_interval_ms".to_string(), serde_json::json!(5000));
                config
            },
            auth: None,
            connection: ConnectionConfig::default(),
        },
    );

    // Kafka source configuration
    sources.insert(
        "kafka_source".to_string(),
        SourceConfig {
            source_type: SourceType::Kafka,
            name: "kafka_source".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert(
                    "bootstrap_servers".to_string(),
                    serde_json::json!("localhost:9092"),
                );
                config.insert("topic".to_string(), serde_json::json!("telemetry-data"));
                config.insert(
                    "group_id".to_string(),
                    serde_json::json!("streaming-processor"),
                );
                config
            },
            auth: None,
            connection: ConnectionConfig::default(),
        },
    );

    let processors = vec![
        // Filter processor
        ProcessorConfig {
            processor_type: ProcessorType::Filter,
            name: "filter_processor".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert(
                    "filter_expression".to_string(),
                    serde_json::json!("service_name == 'web-service'"),
                );
                config
            },
            order: 1,
        },
        // Transform processor
        ProcessorConfig {
            processor_type: ProcessorType::Transform,
            name: "transform_processor".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert("transform_rules".to_string(), serde_json::json!(vec![
                    serde_json::json!({"field": "timestamp", "operation": "format", "value": "ISO8601"}),
                    serde_json::json!({"field": "level", "operation": "uppercase"})
                ]));
                config
            },
            order: 2,
        },
        // Aggregate processor
        ProcessorConfig {
            processor_type: ProcessorType::Aggregate,
            name: "aggregate_processor".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert("window_size_ms".to_string(), serde_json::json!(60000));
                config.insert(
                    "aggregation_fields".to_string(),
                    serde_json::json!(vec!["count", "sum", "avg"]),
                );
                config
            },
            order: 3,
        },
    ];

    let mut sinks = HashMap::new();

    // HTTP sink configuration
    sinks.insert(
        "http_sink".to_string(),
        SinkConfig {
            sink_type: SinkType::Http,
            name: "http_sink".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert(
                    "endpoint_url".to_string(),
                    serde_json::json!("https://api.example.com/telemetry"),
                );
                config.insert("method".to_string(), serde_json::json!("POST"));
                config.insert("batch_size".to_string(), serde_json::json!(100));
                config
            },
            auth: Some(streaming_processor::config::AuthConfig {
                auth_type: streaming_processor::config::AuthType::ApiKey,
                credentials: {
                    let mut creds = HashMap::new();
                    creds.insert("api_key".to_string(), "your-api-key".to_string());
                    creds
                },
            }),
            connection: ConnectionConfig::default(),
        },
    );

    // File sink configuration
    sinks.insert(
        "file_sink".to_string(),
        SinkConfig {
            sink_type: SinkType::File,
            name: "file_sink".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert(
                    "file_path".to_string(),
                    serde_json::json!("/tmp/processed-data.json"),
                );
                config.insert("format".to_string(), serde_json::json!("json"));
                config.insert("rotation_size_mb".to_string(), serde_json::json!(100));
                config
            },
            auth: None,
            connection: ConnectionConfig::default(),
        },
    );

    StreamingProcessorConfig {
        name: "comprehensive-streaming-processor".to_string(),
        version: "1.0.0".to_string(),
        sources,
        processors,
        sinks,
        processing: ProcessingConfig {
            batch_size: 1000,
            buffer_size: 10000,
            timeout: std::time::Duration::from_secs(30),
            enable_parallel: true,
            num_workers: 4,
            enable_backpressure: true,
            backpressure_threshold: 80,
        },
        state: StateConfig::default(),
        metrics: MetricsConfig::default(),
        security: SecurityConfig::default(),
    }
}

/// Demonstrate OAuth configuration mapping
fn demonstrate_oauth_mapping() -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating OAuth configuration mapping...");

    // Create auth crate OAuth configuration
    let auth_oauth_config = OAuthConfig {
        enabled: true,
        providers: create_oauth_providers(),
        callback_url: "http://localhost:8080/auth/callback".to_string(),
        state_timeout_secs: 300,
    };

    // Convert to bridge-api format
    let bridge_api_oauth_config = auth_oauth_config.to_bridge_api_format();

    info!("Converted OAuth config to bridge-api format:");
    info!("  Client ID: {}", bridge_api_oauth_config.client_id);
    info!(
        "  Authorization URL: {}",
        bridge_api_oauth_config.authorization_url
    );
    info!("  Token URL: {}", bridge_api_oauth_config.token_url);

    // Convert back to auth format
    let converted_back = OAuthConfig::from_bridge_api_format(&bridge_api_oauth_config);

    info!("Converted back to auth format:");
    info!("  Enabled: {}", converted_back.enabled);
    info!("  Providers count: {}", converted_back.providers.len());
    info!("  Callback URL: {}", converted_back.callback_url);

    Ok(())
}

/// Demonstrate API error handling
fn demonstrate_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating API error handling...");

    // Create different types of errors
    let errors = vec![
        ApiError::BadRequest("Invalid request format".to_string()),
        ApiError::Unauthorized("Invalid credentials".to_string()),
        ApiError::Forbidden("Insufficient permissions".to_string()),
        ApiError::NotFound("Resource not found".to_string()),
        ApiError::Validation("Validation failed".to_string()),
        ApiError::RateLimitExceeded("Rate limit exceeded".to_string()),
        ApiError::Internal("Internal server error".to_string()),
    ];

    for error in errors {
        let enhanced_response = EnhancedErrorResponse::new(&error, Some("req-123".to_string()));

        info!(
            "Error: {} -> Status: {}, Category: {}, Severity: {:?}",
            error.error_code(),
            error.status_code(),
            enhanced_response.category,
            enhanced_response.severity
        );

        if let Some(suggested_action) = &enhanced_response.suggested_action {
            info!("  Suggested action: {}", suggested_action);
        }
    }

    // Demonstrate error context
    let error_context = ErrorContext::new()
        .with_request_id("req-456".to_string())
        .with_user_id("user-123".to_string())
        .with_endpoint("/api/v1/streaming".to_string())
        .with_method("POST".to_string());

    info!("Error context: {:?}", error_context);

    Ok(())
}

/// Demonstrate streaming processor integration
async fn demonstrate_streaming_processor() -> BridgeResult<()> {
    info!("Demonstrating streaming processor integration...");

    // Create configuration
    let config = create_streaming_processor_config();

    // Validate configuration
    config.validate()?;
    info!("Configuration validation passed");

    // Create streaming processor
    let mut processor = StreamingProcessor::new(config).await?;
    info!("Streaming processor created successfully");

    // Start processor
    processor.start().await?;
    info!("Streaming processor started successfully");

    // Get statistics
    let stats = processor.get_stats().await;
    info!("Processor statistics: {:?}", stats);

    // Simulate some processing time
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Stop processor
    processor.stop().await?;
    info!("Streaming processor stopped successfully");

    Ok(())
}

/// Demonstrate complete integration with error handling
async fn demonstrate_complete_integration() -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating complete integration...");

    // 1. OAuth Configuration Mapping
    demonstrate_oauth_mapping()?;

    // 2. API Error Handling
    demonstrate_error_handling()?;

    // 3. Streaming Processor Integration
    demonstrate_streaming_processor().await?;

    info!("Complete integration demonstration finished successfully");
    Ok(())
}

/// Main function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting comprehensive streaming processor integration example");

    // Demonstrate complete integration
    demonstrate_complete_integration().await?;

    info!("Example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_configuration_mapping() {
        let auth_config = OAuthConfig {
            enabled: true,
            providers: create_oauth_providers(),
            callback_url: "http://localhost:8080/auth/callback".to_string(),
            state_timeout_secs: 300,
        };

        let bridge_api_config = auth_config.to_bridge_api_format();
        let converted_back = OAuthConfig::from_bridge_api_format(&bridge_api_config);

        assert!(converted_back.enabled);
        assert_eq!(
            converted_back.callback_url,
            "http://localhost:8080/auth/callback"
        );
        assert_eq!(converted_back.state_timeout_secs, 300);
    }

    #[test]
    fn test_error_handling() {
        let error = ApiError::BadRequest("Test error".to_string());
        let response = EnhancedErrorResponse::new(&error, Some("test-request".to_string()));

        assert_eq!(response.code, "BAD_REQUEST");
        assert_eq!(response.category, "client");
        assert!(matches!(response.severity, ErrorSeverity::Low));
    }

    #[tokio::test]
    async fn test_streaming_processor_config() {
        let config = create_streaming_processor_config();
        assert!(config.validate().is_ok());
        assert_eq!(config.name, "comprehensive-streaming-processor");
        assert!(!config.sources.is_empty());
        assert!(!config.processors.is_empty());
        assert!(!config.sinks.is_empty());
    }
}
