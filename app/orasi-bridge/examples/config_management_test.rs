//! Configuration management test example
//! 
//! This example demonstrates how to use the configuration management
//! functionality in the Bridge API.

use axum::Json;
use serde_json::json;
use uuid::Uuid;

use bridge_api::{
    config::BridgeAPIConfig,
    handlers::config::apply_configuration_changes,
    rest::AppState,
    types::{ConfigRequest, ConfigOptions},
    metrics::ApiMetrics,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Bridge API Configuration Management Test");
    println!("===========================================");

    // Create a default configuration
    let config = BridgeAPIConfig::default();
    let metrics = ApiMetrics::new();
    let state = AppState { config, metrics };

    // Test 1: Valid HTTP configuration
    println!("\n📋 Test 1: Valid HTTP Configuration");
    let http_config_request = ConfigRequest {
        section: "http".to_string(),
        data: json!({
            "port": 9090,
            "address": "0.0.0.0",
            "request_timeout": 60
        }),
        options: Some(ConfigOptions {
            validate: true,
            hot_reload: true,
            backup: true,
        }),
    };

    match apply_configuration_changes(&state, &http_config_request).await {
        Ok(response) => {
            println!("✅ HTTP config applied successfully");
            println!("   Status: {}", response.status);
            println!("   Changes: {:?}", response.changes);
            if let Some(errors) = response.validation_errors {
                println!("   Errors: {:?}", errors);
            }
        }
        Err(e) => {
            println!("❌ HTTP config failed: {}", e);
        }
    }

    // Test 2: Invalid HTTP configuration (invalid port)
    println!("\n📋 Test 2: Invalid HTTP Configuration (Invalid Port)");
    let invalid_http_config_request = ConfigRequest {
        section: "http".to_string(),
        data: json!({
            "port": 0,  // Invalid port
            "address": "0.0.0.0"
        }),
        options: Some(ConfigOptions {
            validate: true,
            hot_reload: false,
            backup: false,
        }),
    };

    match apply_configuration_changes(&state, &invalid_http_config_request).await {
        Ok(response) => {
            println!("📊 Invalid HTTP config response:");
            println!("   Status: {}", response.status);
            if let Some(errors) = response.validation_errors {
                println!("   Validation errors: {:?}", errors);
            }
        }
        Err(e) => {
            println!("❌ Invalid HTTP config failed: {}", e);
        }
    }

    // Test 3: Valid gRPC configuration
    println!("\n📋 Test 3: Valid gRPC Configuration");
    let grpc_config_request = ConfigRequest {
        section: "grpc".to_string(),
        data: json!({
            "port": 9091,
            "address": "0.0.0.0",
            "max_message_size": 1048576
        }),
        options: Some(ConfigOptions {
            validate: true,
            hot_reload: true,
            backup: true,
        }),
    };

    match apply_configuration_changes(&state, &grpc_config_request).await {
        Ok(response) => {
            println!("✅ gRPC config applied successfully");
            println!("   Status: {}", response.status);
            println!("   Changes: {:?}", response.changes);
        }
        Err(e) => {
            println!("❌ gRPC config failed: {}", e);
        }
    }

    // Test 4: Valid authentication configuration
    println!("\n📋 Test 4: Valid Authentication Configuration");
    let auth_config_request = ConfigRequest {
        section: "auth".to_string(),
        data: json!({
            "enabled": true,
            "jwt_secret": "this_is_a_very_long_secret_key_for_jwt_signing_32_chars"
        }),
        options: Some(ConfigOptions {
            validate: true,
            hot_reload: true,
            backup: true,
        }),
    };

    match apply_configuration_changes(&state, &auth_config_request).await {
        Ok(response) => {
            println!("✅ Auth config applied successfully");
            println!("   Status: {}", response.status);
            println!("   Changes: {:?}", response.changes);
        }
        Err(e) => {
            println!("❌ Auth config failed: {}", e);
        }
    }

    // Test 5: Invalid authentication configuration (short secret)
    println!("\n📋 Test 5: Invalid Authentication Configuration (Short Secret)");
    let invalid_auth_config_request = ConfigRequest {
        section: "auth".to_string(),
        data: json!({
            "enabled": true,
            "jwt_secret": "short"  // Too short
        }),
        options: Some(ConfigOptions {
            validate: true,
            hot_reload: false,
            backup: false,
        }),
    };

    match apply_configuration_changes(&state, &invalid_auth_config_request).await {
        Ok(response) => {
            println!("📊 Invalid auth config response:");
            println!("   Status: {}", response.status);
            if let Some(errors) = response.validation_errors {
                println!("   Validation errors: {:?}", errors);
            }
        }
        Err(e) => {
            println!("❌ Invalid auth config failed: {}", e);
        }
    }

    // Test 6: Valid logging configuration
    println!("\n📋 Test 6: Valid Logging Configuration");
    let logging_config_request = ConfigRequest {
        section: "logging".to_string(),
        data: json!({
            "level": "info"
        }),
        options: Some(ConfigOptions {
            validate: true,
            hot_reload: true,
            backup: false,
        }),
    };

    match apply_configuration_changes(&state, &logging_config_request).await {
        Ok(response) => {
            println!("✅ Logging config applied successfully");
            println!("   Status: {}", response.status);
            println!("   Changes: {:?}", response.changes);
        }
        Err(e) => {
            println!("❌ Logging config failed: {}", e);
        }
    }

    // Test 7: Invalid logging configuration (invalid level)
    println!("\n📋 Test 7: Invalid Logging Configuration (Invalid Level)");
    let invalid_logging_config_request = ConfigRequest {
        section: "logging".to_string(),
        data: json!({
            "level": "invalid_level"
        }),
        options: Some(ConfigOptions {
            validate: true,
            hot_reload: false,
            backup: false,
        }),
    };

    match apply_configuration_changes(&state, &invalid_logging_config_request).await {
        Ok(response) => {
            println!("📊 Invalid logging config response:");
            println!("   Status: {}", response.status);
            if let Some(errors) = response.validation_errors {
                println!("   Validation errors: {:?}", errors);
            }
        }
        Err(e) => {
            println!("❌ Invalid logging config failed: {}", e);
        }
    }

    // Test 8: Unknown configuration section
    println!("\n📋 Test 8: Unknown Configuration Section");
    let unknown_config_request = ConfigRequest {
        section: "unknown_section".to_string(),
        data: json!({
            "some_field": "some_value"
        }),
        options: Some(ConfigOptions {
            validate: true,
            hot_reload: false,
            backup: false,
        }),
    };

    match apply_configuration_changes(&state, &unknown_config_request).await {
        Ok(response) => {
            println!("📊 Unknown section response:");
            println!("   Status: {}", response.status);
            if let Some(errors) = response.validation_errors {
                println!("   Validation errors: {:?}", errors);
            }
        }
        Err(e) => {
            println!("❌ Unknown section failed: {}", e);
        }
    }

    println!("\n✅ Configuration management test completed!");
    println!("Check the ./config/backups and ./config/history directories for generated files.");

    Ok(())
}
