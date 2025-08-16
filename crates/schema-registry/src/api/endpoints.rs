//! API endpoint handlers
//!
//! This module contains the API endpoint handlers for the schema registry.

use crate::registry::SchemaRegistryManager;
use crate::schema::SchemaSearchCriteria;
use std::sync::Arc;

use super::{error::ApiError, requests::*, responses::*};

#[cfg(feature = "http")]
use axum::{
    extract::{Path, Query, State},
    response::Json,
};

#[cfg(not(feature = "http"))]
use serde_json::Value as Json;

#[cfg(not(feature = "http"))]
type Path<T> = T;

#[cfg(not(feature = "http"))]
type Query<T> = T;

#[cfg(not(feature = "http"))]
type State<T> = T;

/// Health check endpoint
pub async fn health_check(
    State(registry): State<Arc<SchemaRegistryManager>>,
) -> Result<Json<HealthResponse>, ApiError> {
    let healthy = registry
        .health_check()
        .await
        .map_err(|e| ApiError::internal(&e.to_string()))?;

    let state = registry.get_state().await;

    Ok(Json(HealthResponse {
        status: if healthy {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
        timestamp: chrono::Utc::now(),
        initialized: state.initialized,
        last_health_check: state.last_health_check,
    }))
}

/// Register schema endpoint
pub async fn register_schema(
    State(registry): State<Arc<SchemaRegistryManager>>,
    Json(request): Json<RegisterSchemaRequest>,
) -> Result<Json<RegisterSchemaResponse>, ApiError> {
    let schema = crate::schema::Schema::new(
        request.name,
        request.version,
        request.schema_type,
        request.content,
        request.format,
    );

    let fingerprint = schema.fingerprint.clone();
    let version = registry
        .register_schema(schema)
        .await
        .map_err(|e| ApiError::from_registry_error(e))?;

    Ok(Json(RegisterSchemaResponse {
        fingerprint,
        version: version.to_string(),
        message: "Schema registered successfully".to_string(),
    }))
}

/// List schemas endpoint
pub async fn list_schemas(
    State(registry): State<Arc<SchemaRegistryManager>>,
    Query(criteria): Query<SchemaSearchCriteria>,
) -> Result<Json<ListSchemasResponse>, ApiError> {
    let schemas = registry
        .search_schemas(&criteria)
        .await
        .map_err(|e| ApiError::from_registry_error(e))?;

    let total_count = schemas.len();
    Ok(Json(ListSchemasResponse {
        schemas,
        total_count,
    }))
}

/// Get schema endpoint
pub async fn get_schema(
    State(registry): State<Arc<SchemaRegistryManager>>,
    Path(fingerprint): Path<String>,
) -> Result<Json<GetSchemaResponse>, ApiError> {
    let schema = registry
        .get_schema(&fingerprint)
        .await
        .map_err(|e| ApiError::from_registry_error(e))?
        .ok_or_else(|| ApiError::not_found(&format!("Schema not found: {}", fingerprint)))?;

    Ok(Json(GetSchemaResponse { schema }))
}

/// Delete schema endpoint
pub async fn delete_schema(
    State(registry): State<Arc<SchemaRegistryManager>>,
    Path(fingerprint): Path<String>,
) -> Result<Json<DeleteSchemaResponse>, ApiError> {
    let deleted = registry
        .delete_schema(&fingerprint)
        .await
        .map_err(|e| ApiError::from_registry_error(e))?;

    if !deleted {
        return Err(ApiError::not_found(&format!(
            "Schema not found: {}",
            fingerprint
        )));
    }

    Ok(Json(DeleteSchemaResponse {
        message: "Schema deleted successfully".to_string(),
    }))
}

/// Validate data endpoint
pub async fn validate_data(
    State(registry): State<Arc<SchemaRegistryManager>>,
    Path(fingerprint): Path<String>,
    Json(request): Json<ValidateDataRequest>,
) -> Result<Json<ValidateDataResponse>, ApiError> {
    let validation_result = registry
        .validate_data(&fingerprint, &request.data)
        .await
        .map_err(|e| ApiError::from_registry_error(e))?;

    Ok(Json(ValidateDataResponse {
        valid: validation_result.is_valid(),
        status: format!("{:?}", validation_result.status),
        error_count: validation_result.error_count(),
        warning_count: validation_result.warning_count(),
        errors: validation_result.errors,
        warnings: validation_result.warnings,
    }))
}

/// Validate schema endpoint
pub async fn validate_schema(
    State(registry): State<Arc<SchemaRegistryManager>>,
    Json(request): Json<ValidateSchemaRequest>,
) -> Result<Json<ValidateSchemaResponse>, ApiError> {
    let validation_result = registry
        .validate_schema(&request.schema)
        .await
        .map_err(|e| ApiError::from_registry_error(e))?;

    Ok(Json(ValidateSchemaResponse {
        valid: validation_result.is_valid(),
        status: format!("{:?}", validation_result.status),
        error_count: validation_result.error_count(),
        warning_count: validation_result.warning_count(),
        errors: validation_result.errors,
        warnings: validation_result.warnings,
    }))
}

/// Evolve schema endpoint
pub async fn evolve_schema(
    State(registry): State<Arc<SchemaRegistryManager>>,
    Json(request): Json<EvolveSchemaRequest>,
) -> Result<Json<EvolveSchemaResponse>, ApiError> {
    let evolved_schema = registry
        .evolve_schema(&request.schema)
        .await
        .map_err(|e| ApiError::from_registry_error(e))?;

    Ok(Json(EvolveSchemaResponse {
        fingerprint: evolved_schema.fingerprint,
        version: evolved_schema.version.to_string(),
        message: "Schema evolved successfully".to_string(),
    }))
}

/// Get metrics endpoint
pub async fn get_metrics(
    State(registry): State<Arc<SchemaRegistryManager>>,
) -> Result<Json<MetricsResponse>, ApiError> {
    let metrics = registry
        .get_metrics()
        .await
        .map_err(|e| ApiError::from_registry_error(e))?;

    Ok(Json(MetricsResponse { metrics }))
}

/// Get stats endpoint
pub async fn get_stats(
    State(registry): State<Arc<SchemaRegistryManager>>,
) -> Result<Json<StatsResponse>, ApiError> {
    let stats = registry
        .get_stats()
        .await
        .map_err(|e| ApiError::from_registry_error(e))?;

    Ok(Json(StatsResponse { stats }))
}
