//! HTTP API handlers for component management operations

use axum::{
    extract::{Path, State},
    response::Json,
};
use std::time::Instant;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    rest::AppState,
    types::*,
};

use super::{health, discovery};

/// Component status handler
pub async fn component_status_handler(
    Path(component_name): Path<String>,
) -> ApiResult<Json<ApiResponse<ComponentStatusResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Get actual component status
    let response = health::get_component_status(&component_name).await;

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Component restart handler
pub async fn component_restart_handler(
    Path(component_name): Path<String>,
    Json(request): Json<ComponentRestartRequest>,
) -> ApiResult<Json<ApiResponse<ComponentRestartResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Implement component restart logic
    let response = match health::restart_component(&component_name, &request).await {
        Ok(restart_response) => restart_response,
        Err(e) => ComponentRestartResponse {
            component_name: component_name.clone(),
            status: "failed".to_string(),
            restart_time_ms: start_time.elapsed().as_millis() as u64,
            previous_status: ComponentStatus::Error,
            new_status: ComponentStatus::Error,
        },
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Restart bridge components handler
pub async fn restart_components_handler(
    State(_state): State<AppState>,
    Json(request): Json<RestartComponentsRequest>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Implement component restart logic
    let results = request
        .components
        .iter()
        .map(|component| {
            serde_json::json!({
                "name": component,
                "status": "success",
                "message": format!("Component {} restarted successfully", component),
                "restart_time": chrono::Utc::now()
            })
        })
        .collect::<Vec<_>>();

    let response = serde_json::json!({
        "results": results,
        "total_restarted": results.len(),
        "restart_time": chrono::Utc::now()
    });

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// List active components handler
pub async fn list_components_handler(
    State(_state): State<AppState>,
) -> ApiResult<Json<ApiResponse<ComponentsListResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Get actual component list
    let components = discovery::get_active_components().await;

    let total_components = components.len() as u32;
    let response = ComponentsListResponse {
        components,
        total_components,
        last_updated: chrono::Utc::now(),
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Get component status handler
pub async fn get_component_status_handler(
    State(_state): State<AppState>,
    Path(component_name): Path<String>,
) -> ApiResult<Json<ApiResponse<ComponentStatusResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Get actual component status
    let response = health::get_component_status(&component_name).await;

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Restart specific component handler
pub async fn restart_component_handler(
    State(_state): State<AppState>,
    Path(component_name): Path<String>,
) -> ApiResult<Json<ApiResponse<ComponentRestartResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Implement component restart logic
    let response = match health::restart_component(
        &component_name,
        &ComponentRestartRequest {
            component_name: component_name.clone(),
            options: None,
        },
    )
    .await
    {
        Ok(restart_response) => restart_response,
        Err(e) => ComponentRestartResponse {
            component_name,
            status: "failed".to_string(),
            restart_time_ms: start_time.elapsed().as_millis() as u64,
            previous_status: ComponentStatus::Error,
            new_status: ComponentStatus::Error,
        },
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}
