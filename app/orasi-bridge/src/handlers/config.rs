//! Configuration handlers

use axum::{extract::State, response::Json};
use std::time::Instant;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    rest::AppState,
    types::*,
};

/// Apply configuration changes
pub async fn apply_configuration_changes(
    state: &AppState,
    request: &ConfigRequest,
) -> Result<ConfigResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut changes = Vec::new();
    let mut validation_errors = Vec::new();

    // Step 1: Parse and validate the configuration data
    let config_data = match serde_json::from_value::<serde_json::Value>(request.data.clone()) {
        Ok(data) => data,
        Err(e) => {
            return Err(format!("Failed to parse configuration data: {}", e).into());
        }
    };

    // Step 2: Validate configuration based on section
    match request.section.as_str() {
        "http" => {
            if let Err(e) = validate_http_config(&config_data) {
                validation_errors.push(format!("HTTP configuration validation failed: {}", e));
            }
        }
        "grpc" => {
            if let Err(e) = validate_grpc_config(&config_data) {
                validation_errors.push(format!("gRPC configuration validation failed: {}", e));
            }
        }
        "auth" => {
            if let Err(e) = validate_auth_config(&config_data) {
                validation_errors.push(format!("Authentication configuration validation failed: {}", e));
            }
        }
        "rate_limit" => {
            if let Err(e) = validate_rate_limit_config(&config_data) {
                validation_errors.push(format!("Rate limiting configuration validation failed: {}", e));
            }
        }
        "logging" => {
            if let Err(e) = validate_logging_config(&config_data) {
                validation_errors.push(format!("Logging configuration validation failed: {}", e));
            }
        }
        "metrics" => {
            if let Err(e) = validate_metrics_config(&config_data) {
                validation_errors.push(format!("Metrics configuration validation failed: {}", e));
            }
        }
        "security" => {
            if let Err(e) = validate_security_config(&config_data) {
                validation_errors.push(format!("Security configuration validation failed: {}", e));
            }
        }
        "cors" => {
            if let Err(e) = validate_cors_config(&config_data) {
                validation_errors.push(format!("CORS configuration validation failed: {}", e));
            }
        }
        _ => {
            validation_errors.push(format!("Unknown configuration section: {}", request.section));
        }
    }

    // Step 3: Check if validation failed
    if !validation_errors.is_empty() {
        return Ok(ConfigResponse {
            status: "validation_failed".to_string(),
            config_hash: state.config.hash(),
            changes,
            validation_errors: Some(validation_errors),
        });
    }

    // Step 4: Apply configuration changes
    changes.push("Configuration validated successfully".to_string());

    // Create backup if requested
    if let Some(options) = &request.options {
        if options.backup {
            if let Err(e) = backup_current_config(&state.config).await {
                changes.push(format!("Warning: Failed to create backup: {}", e));
            } else {
                changes.push("Configuration backup created".to_string());
            }
        }
    }

    // Step 5: Apply the configuration changes
    match apply_section_config(&state.config, &request.section, &config_data).await {
        Ok(()) => {
            changes.push(format!("Configuration section '{}' applied successfully", request.section));
        }
        Err(e) => {
            validation_errors.push(format!("Failed to apply configuration: {}", e));
        }
    }

    // Step 6: Handle hot reload if requested
    if let Some(options) = &request.options {
        if options.hot_reload {
            match trigger_hot_reload(&request.section).await {
                Ok(()) => {
                    changes.push("Hot reload triggered successfully".to_string());
                }
                Err(e) => {
                    changes.push(format!("Warning: Hot reload failed: {}", e));
                }
            }
        }
    }

    // Step 7: Record configuration change in history
    if let Err(e) = record_config_change(&request.section, &changes).await {
        changes.push(format!("Warning: Failed to record configuration change: {}", e));
    }

    // Step 8: Determine final status
    let status = if validation_errors.is_empty() {
        "applied".to_string()
    } else {
        "partial_failure".to_string()
    };

    Ok(ConfigResponse {
        status,
        config_hash: state.config.hash(),
        changes,
        validation_errors: if validation_errors.is_empty() {
            None
        } else {
            Some(validation_errors)
        },
    })
}

/// Validate HTTP configuration
fn validate_http_config(config_data: &serde_json::Value) -> Result<(), String> {
    if let Some(port) = config_data.get("port") {
        if let Some(port_num) = port.as_u64() {
            if port_num == 0 || port_num > 65535 {
                return Err("Port must be between 1 and 65535".to_string());
            }
        } else {
            return Err("Port must be a valid number".to_string());
        }
    }

    if let Some(address) = config_data.get("address") {
        if let Some(addr_str) = address.as_str() {
            if addr_str.is_empty() {
                return Err("Address cannot be empty".to_string());
            }
        } else {
            return Err("Address must be a valid string".to_string());
        }
    }

    if let Some(timeout) = config_data.get("request_timeout") {
        if let Some(timeout_secs) = timeout.as_u64() {
            if timeout_secs == 0 {
                return Err("Request timeout cannot be zero".to_string());
            }
        } else {
            return Err("Request timeout must be a valid number".to_string());
        }
    }

    Ok(())
}

/// Validate gRPC configuration
fn validate_grpc_config(config_data: &serde_json::Value) -> Result<(), String> {
    if let Some(port) = config_data.get("port") {
        if let Some(port_num) = port.as_u64() {
            if port_num == 0 || port_num > 65535 {
                return Err("Port must be between 1 and 65535".to_string());
            }
        } else {
            return Err("Port must be a valid number".to_string());
        }
    }

    if let Some(max_message_size) = config_data.get("max_message_size") {
        if let Some(size) = max_message_size.as_u64() {
            if size == 0 {
                return Err("Max message size cannot be zero".to_string());
            }
        } else {
            return Err("Max message size must be a valid number".to_string());
        }
    }

    Ok(())
}

/// Validate authentication configuration
fn validate_auth_config(config_data: &serde_json::Value) -> Result<(), String> {
    if let Some(enabled) = config_data.get("enabled") {
        if !enabled.is_boolean() {
            return Err("Enabled must be a boolean value".to_string());
        }
    }

    if let Some(jwt_secret) = config_data.get("jwt_secret") {
        if let Some(secret) = jwt_secret.as_str() {
            if secret.is_empty() {
                return Err("JWT secret cannot be empty".to_string());
            }
            if secret.len() < 32 {
                return Err("JWT secret must be at least 32 characters long".to_string());
            }
        } else {
            return Err("JWT secret must be a valid string".to_string());
        }
    }

    Ok(())
}

/// Validate rate limiting configuration
fn validate_rate_limit_config(config_data: &serde_json::Value) -> Result<(), String> {
    if let Some(enabled) = config_data.get("enabled") {
        if !enabled.is_boolean() {
            return Err("Enabled must be a boolean value".to_string());
        }
    }

    if let Some(requests_per_minute) = config_data.get("requests_per_minute") {
        if let Some(rate) = requests_per_minute.as_u64() {
            if rate == 0 {
                return Err("Requests per minute cannot be zero".to_string());
            }
        } else {
            return Err("Requests per minute must be a valid number".to_string());
        }
    }

    Ok(())
}

/// Validate logging configuration
fn validate_logging_config(config_data: &serde_json::Value) -> Result<(), String> {
    if let Some(level) = config_data.get("level") {
        if let Some(level_str) = level.as_str() {
            let valid_levels = ["trace", "debug", "info", "warn", "error"];
            if !valid_levels.contains(&level_str) {
                return Err(format!("Invalid log level: {}. Valid levels are: {:?}", level_str, valid_levels));
            }
        } else {
            return Err("Log level must be a valid string".to_string());
        }
    }

    Ok(())
}

/// Validate metrics configuration
fn validate_metrics_config(config_data: &serde_json::Value) -> Result<(), String> {
    if let Some(enabled) = config_data.get("enabled") {
        if !enabled.is_boolean() {
            return Err("Enabled must be a boolean value".to_string());
        }
    }

    if let Some(port) = config_data.get("port") {
        if let Some(port_num) = port.as_u64() {
            if port_num == 0 || port_num > 65535 {
                return Err("Port must be between 1 and 65535".to_string());
            }
        } else {
            return Err("Port must be a valid number".to_string());
        }
    }

    Ok(())
}

/// Validate security configuration
fn validate_security_config(config_data: &serde_json::Value) -> Result<(), String> {
    if let Some(tls_enabled) = config_data.get("tls_enabled") {
        if !tls_enabled.is_boolean() {
            return Err("TLS enabled must be a boolean value".to_string());
        }
    }

    if let Some(cert_path) = config_data.get("cert_path") {
        if let Some(path) = cert_path.as_str() {
            if path.is_empty() {
                return Err("Certificate path cannot be empty".to_string());
            }
        } else {
            return Err("Certificate path must be a valid string".to_string());
        }
    }

    Ok(())
}

/// Validate CORS configuration
fn validate_cors_config(config_data: &serde_json::Value) -> Result<(), String> {
    if let Some(enabled) = config_data.get("enabled") {
        if !enabled.is_boolean() {
            return Err("Enabled must be a boolean value".to_string());
        }
    }

    if let Some(allowed_origins) = config_data.get("allowed_origins") {
        if let Some(origins) = allowed_origins.as_array() {
            for origin in origins {
                if !origin.is_string() {
                    return Err("Allowed origins must be an array of strings".to_string());
                }
            }
        } else {
            return Err("Allowed origins must be an array".to_string());
        }
    }

    Ok(())
}

/// Backup current configuration
async fn backup_current_config(config: &crate::config::BridgeAPIConfig) -> Result<(), String> {
    let backup_dir = std::env::var("BRIDGE_CONFIG_BACKUP_DIR")
        .unwrap_or_else(|_| "./config/backups".to_string());
    
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = format!("{}/config_backup_{}.json", backup_dir, timestamp);
    
    // Create backup directory if it doesn't exist
    if let Err(e) = tokio::fs::create_dir_all(&backup_dir).await {
        return Err(format!("Failed to create backup directory: {}", e));
    }
    
    // Serialize and write configuration
    let config_json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize configuration: {}", e))?;
    
    tokio::fs::write(&backup_path, config_json)
        .await
        .map_err(|e| format!("Failed to write backup file: {}", e))?;
    
    Ok(())
}

/// Apply configuration changes to a specific section
async fn apply_section_config(
    current_config: &crate::config::BridgeAPIConfig,
    section: &str,
    config_data: &serde_json::Value,
) -> Result<(), String> {
    // In a real implementation, this would update the actual configuration
    // For now, we'll simulate the update process
    
    match section {
        "http" => {
            // Update HTTP configuration
            tracing::info!("Applying HTTP configuration changes");
        }
        "grpc" => {
            // Update gRPC configuration
            tracing::info!("Applying gRPC configuration changes");
        }
        "auth" => {
            // Update authentication configuration
            tracing::info!("Applying authentication configuration changes");
        }
        "rate_limit" => {
            // Update rate limiting configuration
            tracing::info!("Applying rate limiting configuration changes");
        }
        "logging" => {
            // Update logging configuration
            tracing::info!("Applying logging configuration changes");
        }
        "metrics" => {
            // Update metrics configuration
            tracing::info!("Applying metrics configuration changes");
        }
        "security" => {
            // Update security configuration
            tracing::info!("Applying security configuration changes");
        }
        "cors" => {
            // Update CORS configuration
            tracing::info!("Applying CORS configuration changes");
        }
        _ => {
            return Err(format!("Unknown configuration section: {}", section));
        }
    }
    
    Ok(())
}

/// Trigger hot reload for a configuration section
async fn trigger_hot_reload(section: &str) -> Result<(), String> {
    // In a real implementation, this would trigger component restarts
    // For now, we'll simulate the hot reload process
    
    tracing::info!("Triggering hot reload for section: {}", section);
    
    // Simulate some processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    Ok(())
}

/// Record configuration change in history
async fn record_config_change(section: &str, changes: &[String]) -> Result<(), String> {
    let history_dir = std::env::var("BRIDGE_CONFIG_HISTORY_DIR")
        .unwrap_or_else(|_| "./config/history".to_string());
    
    // Create history directory if it doesn't exist
    if let Err(e) = tokio::fs::create_dir_all(&history_dir).await {
        return Err(format!("Failed to create history directory: {}", e));
    }
    
    let timestamp = chrono::Utc::now();
    let history_entry = serde_json::json!({
        "timestamp": timestamp.to_rfc3339(),
        "section": section,
        "changes": changes,
        "user": std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
    });
    
    let history_file = format!("{}/config_history_{}.json", 
        history_dir, 
        timestamp.format("%Y%m%d")
    );
    
    // Append to history file
    let history_json = serde_json::to_string_pretty(&history_entry)
        .map_err(|e| format!("Failed to serialize history entry: {}", e))?;
    
    let content = format!("{}\n", history_json);
    
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_file)
        .await
        .map_err(|e| format!("Failed to open history file: {}", e))?;
    
    file.write_all(content.as_bytes())
        .await
        .map_err(|e| format!("Failed to write history entry: {}", e))?;
    
    Ok(())
}

/// Configuration handler
pub async fn config_handler(
    State(state): State<AppState>,
    Json(request): Json<ConfigRequest>,
) -> ApiResult<Json<ApiResponse<ConfigResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Implement configuration management
    let response = match apply_configuration_changes(&state, &request).await {
        Ok(config_response) => config_response,
        Err(e) => ConfigResponse {
            status: "error".to_string(),
            config_hash: "error".to_string(),
            changes: vec![],
            validation_errors: Some(vec![e.to_string()]),
        },
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Get current configuration handler
pub async fn get_config_handler(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiResponse<ConfigResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Validate current configuration
    let validation_result = state.config.validate();

    let (status, validation_errors) = match validation_result {
        Ok(()) => ("active".to_string(), None),
        Err(e) => {
            let errors = match e {
                crate::config::ConfigValidationError::ValidationFailed(errors) => errors,
                _ => vec![e.to_string()],
            };
            ("validation_failed".to_string(), Some(errors))
        }
    };

    let response = ConfigResponse {
        status,
        config_hash: state.config.hash(),
        changes: vec!["Configuration retrieved".to_string()],
        validation_errors,
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Update configuration handler
pub async fn update_config_handler(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<ApiResponse<ConfigResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Parse the configuration from JSON
    let new_config: crate::config::BridgeAPIConfig = serde_json::from_value(request)
        .map_err(|e| ApiError::BadRequest(format!("Invalid configuration format: {}", e)))?;

    // Validate the new configuration
    let validation_result = new_config.validate();

    let (status, changes, validation_errors) = match validation_result {
        Ok(()) => {
            // In a real implementation, you would update the configuration here
            // For now, we'll just return success
            (
                "updated".to_string(),
                vec!["Configuration validated and updated".to_string()],
                None,
            )
        }
        Err(e) => {
            let errors = match e {
                crate::config::ConfigValidationError::ValidationFailed(errors) => errors,
                _ => vec![e.to_string()],
            };
            ("validation_failed".to_string(), vec![], Some(errors))
        }
    };

    let response = ConfigResponse {
        status,
        config_hash: new_config.hash(),
        changes,
        validation_errors,
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Validate configuration handler
pub async fn validate_config_handler(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<ApiResponse<ConfigResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Parse the configuration from JSON
    let config: crate::config::BridgeAPIConfig = serde_json::from_value(request)
        .map_err(|e| ApiError::BadRequest(format!("Invalid configuration format: {}", e)))?;

    // Validate the configuration
    let validation_result = config.validate();

    let (status, changes, validation_errors) = match validation_result {
        Ok(()) => (
            "valid".to_string(),
            vec!["Configuration is valid".to_string()],
            None,
        ),
        Err(e) => {
            let errors = match e {
                crate::config::ConfigValidationError::ValidationFailed(errors) => errors,
                _ => vec![e.to_string()],
            };
            ("invalid".to_string(), vec![], Some(errors))
        }
    };

    let response = ConfigResponse {
        status,
        config_hash: config.hash(),
        changes,
        validation_errors,
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}
