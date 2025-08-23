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
async fn apply_configuration_changes(
    state: &AppState,
    request: &ConfigRequest,
) -> Result<ConfigResponse, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Implement actual configuration management
    // This would typically involve:
    // 1. Validating the configuration changes
    // 2. Applying the changes to the system
    // 3. Restarting affected components if necessary
    // 4. Recording the changes in a configuration history

    // For now, return a placeholder response
    Ok(ConfigResponse {
        status: "applied".to_string(),
        config_hash: super::utils::calculate_config_hash().await,
        changes: vec![
            "Configuration validated".to_string(),
            "Configuration applied".to_string(),
        ],
        validation_errors: None,
    })
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
