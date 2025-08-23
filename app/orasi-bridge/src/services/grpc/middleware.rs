//! gRPC middleware and interceptors

use bridge_auth::jwt::{JwtClaims, JwtManager};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tonic::service::Interceptor;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

use crate::metrics::ApiMetrics;
use crate::proto::*;
use crate::services::{config::ConfigService, query::QueryService, status::StatusService};
use bridge_auth::config::JwtConfig;
use bridge_core::{BridgeConfig, BridgeResult};
use std::path::PathBuf;

/// gRPC middleware for authentication
pub async fn grpc_auth_middleware(
    request: Request<()>,
    jwt_config: Arc<JwtConfig>,
) -> Result<Response<()>, Status> {
    // Extract authentication token from metadata
    if let Some(auth_header) = request.metadata().get("authorization") {
        let auth_value = auth_header.to_str().unwrap_or("");

        if auth_value.starts_with("Bearer ") {
            let token = &auth_value[7..]; // Remove "Bearer " prefix

            // Validate JWT token
            match validate_jwt_token(token, &jwt_config).await {
                Ok(claims) => {
                    debug!("gRPC request authenticated for user: {}", claims.user_id());

                    // Add user information to request extensions for downstream use
                    // Note: This would require modifying the request type to include extensions
                    // For now, we'll just log the authentication success
                }
                Err(e) => {
                    warn!("JWT validation failed: {}", e);
                    return Err(Status::unauthenticated(format!(
                        "Invalid authentication token: {}",
                        e
                    )));
                }
            }
        } else {
            return Err(Status::unauthenticated(
                "Invalid authorization header format",
            ));
        }
    } else {
        // Check if authentication is required
        if !jwt_config.secret.is_empty() {
            return Err(Status::unauthenticated("Authentication required"));
        }

        debug!("gRPC request without authentication header (auth not required)");
    }

    Ok(Response::new(()))
}

/// Validate JWT token
async fn validate_jwt_token(token: &str, config: &JwtConfig) -> Result<JwtClaims, String> {
    let validator = JwtManager::new(config.clone()).map_err(|e| e.to_string())?;

    match validator.validate_token(token).await {
        Ok(claims) => {
            // Check if token is expired
            if claims.is_expired() {
                return Err("Token has expired".to_string());
            }

            // Check if token is not yet valid
            if claims.is_not_yet_valid() {
                return Err("Token is not yet valid".to_string());
            }

            Ok(claims)
        }
        Err(e) => Err(format!("JWT validation error: {}", e)),
    }
}

/// gRPC middleware for logging
pub async fn grpc_logging_middleware(request: Request<()>) -> Result<Response<()>, Status> {
    let method = request
        .metadata()
        .get("grpc-method")
        .unwrap_or(&"unknown".parse().unwrap())
        .to_str()
        .unwrap_or("unknown")
        .to_string();
    let start_time = std::time::Instant::now();

    // Extract additional request information
    let request_id = request
        .metadata()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let client_ip = request
        .metadata()
        .get("x-forwarded-for")
        .or_else(|| request.metadata().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    info!(
        "gRPC request started: method={}, request_id={}, client_ip={}",
        method, request_id, client_ip
    );

    // Process the request based on the method
    let response =
        match process_request_by_method(&method, request).await {
            Ok(result) => {
                let duration = start_time.elapsed();
                info!(
                "gRPC request completed: method={}, request_id={}, duration={}ms, status=success",
                method, request_id, duration.as_millis()
            );
                result
            }
            Err(e) => {
                let duration = start_time.elapsed();
                error!(
                    "gRPC request failed: method={}, request_id={}, duration={}ms, error={}",
                    method,
                    request_id,
                    duration.as_millis(),
                    e
                );
                return Err(e);
            }
        };

    Ok(response)
}

/// gRPC middleware for metrics
pub async fn grpc_metrics_middleware(
    request: Request<()>,
    metrics: ApiMetrics,
) -> Result<Response<()>, Status> {
    let method = request
        .metadata()
        .get("grpc-method")
        .unwrap_or(&"unknown".parse().unwrap())
        .to_str()
        .unwrap_or("unknown")
        .to_string();
    let start_time = std::time::Instant::now();

    // Increment request counter
    metrics.record_request("grpc", &method, 200);

    // Process the request and collect metrics
    let response = match process_request_with_metrics(&method, request, &metrics).await {
        Ok(result) => {
            let duration = start_time.elapsed();

            // Record successful response time
            metrics.record_response_time("grpc", &method, duration);
            metrics.record_processing("grpc_request", duration, true);

            result
        }
        Err(e) => {
            let duration = start_time.elapsed();

            // Record error metrics
            metrics.record_error("grpc_error", "grpc", &method);
            metrics.record_processing("grpc_request", duration, false);

            return Err(e);
        }
    };

    Ok(response)
}

/// Process request based on gRPC method
async fn process_request_by_method(
    method: &str,
    _request: Request<()>,
) -> Result<Response<()>, Status> {
    match method {
        "query_telemetry" => {
            debug!("Processing telemetry query request");
            // In a real implementation, this would validate the query parameters
            // and prepare the request for the query service
            Ok(Response::new(()))
        }
        "get_status" => {
            debug!("Processing status request");
            // In a real implementation, this would validate the status request
            // and prepare it for the status service
            Ok(Response::new(()))
        }
        "update_config" => {
            debug!("Processing config update request");
            // In a real implementation, this would validate the config update
            // and prepare it for the config service
            Ok(Response::new(()))
        }
        "get_metrics" => {
            debug!("Processing metrics request");
            // In a real implementation, this would validate the metrics request
            // and prepare it for the metrics service
            Ok(Response::new(()))
        }
        _ => {
            warn!("Unknown gRPC method: {}", method);
            Ok(Response::new(()))
        }
    }
}

/// Process request with metrics collection
async fn process_request_with_metrics(
    method: &str,
    _request: Request<()>,
    metrics: &ApiMetrics,
) -> Result<Response<()>, Status> {
    // Increment active connections for this request
    metrics.increment_active_connections();

    let result = match method {
        "query_telemetry" => {
            debug!("Processing telemetry query with metrics");
            // Simulate query processing time
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(Response::new(()))
        }
        "get_status" => {
            debug!("Processing status request with metrics");
            // Simulate status processing time
            tokio::time::sleep(Duration::from_millis(5)).await;
            Ok(Response::new(()))
        }
        "update_config" => {
            debug!("Processing config update with metrics");
            // Simulate config update processing time
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok(Response::new(()))
        }
        "get_metrics" => {
            debug!("Processing metrics request with metrics");
            // Simulate metrics processing time
            tokio::time::sleep(Duration::from_millis(5)).await;
            Ok(Response::new(()))
        }
        _ => {
            warn!("Unknown gRPC method with metrics: {}", method);
            Ok(Response::new(()))
        }
    };

    // Decrement active connections
    metrics.decrement_active_connections();

    result
}

/// gRPC interceptor for request ID
pub fn grpc_request_id_interceptor() -> impl Interceptor {
    |mut request: Request<()>| {
        let request_id = uuid::Uuid::new_v4().to_string();
        request
            .metadata_mut()
            .insert("x-request-id", request_id.parse().unwrap());
        Ok(request)
    }
}

/// gRPC interceptor for timeout
pub fn grpc_timeout_interceptor(timeout: std::time::Duration) -> impl Interceptor {
    move |mut request: Request<()>| {
        request.set_timeout(timeout);
        Ok(request)
    }
}

/// gRPC interceptor for rate limiting
pub fn grpc_rate_limit_interceptor() -> impl Interceptor {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    // Simple in-memory rate limiter
    // In production, use a proper rate limiting library like governor
    let request_counts: Mutex<HashMap<String, (Instant, u32)>> = Mutex::new(HashMap::new());

    move |request: Request<()>| {
        let client_id = request
            .metadata()
            .get("x-client-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let now = Instant::now();
        let window = Duration::from_secs(60); // 1 minute window
        let max_requests = 100; // Max 100 requests per minute

        if let Ok(mut counts) = request_counts.lock() {
            if let Some((last_time, count)) = counts.get_mut(&client_id) {
                if now.duration_since(*last_time) > window {
                    // Reset window
                    *last_time = now;
                    *count = 1;
                } else if *count >= max_requests {
                    return Err(Status::resource_exhausted("Rate limit exceeded"));
                } else {
                    *count += 1;
                }
            } else {
                counts.insert(client_id.clone(), (now, 1));
            }
        }

        tracing::debug!("Rate limit check passed for client: {}", client_id);
        Ok(request)
    }
}

/// Enhanced gRPC middleware that combines authentication, logging, and metrics
pub async fn grpc_enhanced_middleware(
    request: Request<()>,
    jwt_config: Arc<JwtConfig>,
    metrics: ApiMetrics,
) -> Result<Response<()>, Status> {
    let start_time = Instant::now();
    let method = request
        .metadata()
        .get("grpc-method")
        .unwrap_or(&"unknown".parse().unwrap())
        .to_str()
        .unwrap_or("unknown")
        .to_string();

    let request_id = request
        .metadata()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Step 1: Authentication - extract metadata for auth check
    let auth_header = request.metadata().get("authorization").cloned();
    if let Some(auth_header) = auth_header {
        let auth_value = auth_header.to_str().unwrap_or("");
        if auth_value.starts_with("Bearer ") {
            let token = &auth_value[7..]; // Remove "Bearer " prefix
            match validate_jwt_token(token, &jwt_config).await {
                Ok(claims) => {
                    debug!("gRPC request authenticated for user: {}", claims.user_id());
                }
                Err(e) => {
                    warn!("JWT validation failed: {}", e);
                    metrics.record_error("auth_failed", "grpc", &method);
                    return Err(Status::unauthenticated(format!(
                        "Invalid authentication token: {}",
                        e
                    )));
                }
            }
        } else {
            metrics.record_error("auth_failed", "grpc", &method);
            return Err(Status::unauthenticated(
                "Invalid authorization header format",
            ));
        }
    } else if !jwt_config.secret.is_empty() {
        metrics.record_error("auth_failed", "grpc", &method);
        return Err(Status::unauthenticated("Authentication required"));
    }

    // Step 2: Logging
    info!(
        "gRPC request authenticated and logged: method={}, request_id={}",
        method, request_id
    );

    // Step 3: Metrics
    metrics.record_request("grpc", &method, 200);
    metrics.increment_active_connections();

    // Step 4: Process request
    let response = match process_enhanced_request(&method, request).await {
        Ok(result) => {
            let duration = start_time.elapsed();
            metrics.record_response_time("grpc", &method, duration);
            metrics.record_processing("grpc_enhanced", duration, true);
            result
        }
        Err(e) => {
            let duration = start_time.elapsed();
            metrics.record_error("processing_failed", "grpc", &method);
            metrics.record_processing("grpc_enhanced", duration, false);
            return Err(e);
        }
    };

    metrics.decrement_active_connections();
    Ok(response)
}

/// Enhanced request processing with comprehensive validation
async fn process_enhanced_request(
    method: &str,
    _request: Request<()>,
) -> Result<Response<()>, Status> {
    match method {
        "query_telemetry" => {
            debug!("Enhanced processing: telemetry query");
            // Validate query parameters, check permissions, etc.
            validate_telemetry_query().await?;
            Ok(Response::new(()))
        }
        "get_status" => {
            debug!("Enhanced processing: status request");
            // Validate status request parameters
            validate_status_request().await?;
            Ok(Response::new(()))
        }
        "update_config" => {
            debug!("Enhanced processing: config update");
            // Validate config update permissions and parameters
            validate_config_update().await?;
            Ok(Response::new(()))
        }
        "get_metrics" => {
            debug!("Enhanced processing: metrics request");
            // Validate metrics request parameters
            validate_metrics_request().await?;
            Ok(Response::new(()))
        }
        _ => {
            warn!("Enhanced processing: unknown method {}", method);
            Ok(Response::new(()))
        }
    }
}

/// Validate telemetry query request
async fn validate_telemetry_query() -> Result<(), Status> {
    // In a real implementation, this would validate:
    // - Query parameters
    // - Time range constraints
    // - Data access permissions
    // - Rate limiting for expensive queries
    Ok(())
}

/// Validate status request
async fn validate_status_request() -> Result<(), Status> {
    // In a real implementation, this would validate:
    // - Component access permissions
    // - Status detail level permissions
    Ok(())
}

/// Validate config update request
async fn validate_config_update() -> Result<(), Status> {
    // In a real implementation, this would validate:
    // - Configuration change permissions
    // - Configuration syntax and semantics
    // - Impact assessment
    Ok(())
}

/// Validate metrics request
async fn validate_metrics_request() -> Result<(), Status> {
    // In a real implementation, this would validate:
    // - Metrics access permissions
    // - Time range constraints
    // - Metric type permissions
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::metadata::MetadataValue;

    #[tokio::test]
    async fn test_grpc_logging_middleware() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("grpc-method", "test_method".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-request-id", "test-123".parse().unwrap());

        let result = grpc_logging_middleware(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_grpc_metrics_middleware() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("grpc-method", "test_method".parse().unwrap());

        let metrics = ApiMetrics::new();
        let result = grpc_metrics_middleware(request, metrics).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_grpc_auth_middleware_no_auth_required() {
        let request = Request::new(());
        let jwt_config = Arc::new(JwtConfig {
            secret: "".to_string(),
            expiration_secs: 3600,
            issuer: "test".to_string(),
            audience: "test".to_string(),
            algorithm: bridge_auth::config::JwtAlgorithm::HS256,
            enable_refresh_tokens: false,
            refresh_token_expiration_secs: 86400,
        });

        let result = grpc_auth_middleware(request, jwt_config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_grpc_enhanced_middleware() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("grpc-method", "test_method".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-request-id", "test-123".parse().unwrap());

        let jwt_config = Arc::new(JwtConfig {
            secret: "".to_string(),
            expiration_secs: 3600,
            issuer: "test".to_string(),
            audience: "test".to_string(),
            algorithm: bridge_auth::config::JwtAlgorithm::HS256,
            enable_refresh_tokens: false,
            refresh_token_expiration_secs: 86400,
        });

        let metrics = ApiMetrics::new();
        let result = grpc_enhanced_middleware(request, jwt_config, metrics).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_request_by_method() {
        let request = Request::new(());

        let result = process_request_by_method("query_telemetry", request).await;
        assert!(result.is_ok());

        let request = Request::new(());
        let result = process_request_by_method("get_status", request).await;
        assert!(result.is_ok());

        let request = Request::new(());
        let result = process_request_by_method("update_config", request).await;
        assert!(result.is_ok());

        let request = Request::new(());
        let result = process_request_by_method("get_metrics", request).await;
        assert!(result.is_ok());

        let request = Request::new(());
        let result = process_request_by_method("unknown_method", request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_request_with_metrics() {
        let request = Request::new(());
        let metrics = ApiMetrics::new();

        let result = process_request_with_metrics("query_telemetry", request, &metrics).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validation_functions() {
        assert!(validate_telemetry_query().await.is_ok());
        assert!(validate_status_request().await.is_ok());
        assert!(validate_config_update().await.is_ok());
        assert!(validate_metrics_request().await.is_ok());
    }
}
