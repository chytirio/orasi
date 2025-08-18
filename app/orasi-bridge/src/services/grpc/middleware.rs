//! gRPC middleware and interceptors

use bridge_auth::jwt::{JwtClaims, JwtManager};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tonic::service::Interceptor;
use tonic::{Request, Response, Status};
use tracing::{debug, warn};

use crate::metrics::ApiMetrics;
use bridge_auth::config::JwtConfig;

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
pub async fn grpc_logging_middleware(_request: Request<()>) -> Result<Response<()>, Status> {
    let method = _request
        .metadata()
        .get("grpc-method")
        .unwrap_or(&"unknown".parse().unwrap())
        .to_str()
        .unwrap_or("unknown")
        .to_string();
    let start_time = std::time::Instant::now();

    tracing::debug!("gRPC request: {}", method);

    // TODO: Implement actual request processing
    let response = Response::new(());

    let duration = start_time.elapsed();
    tracing::debug!("gRPC response: {} ({}ms)", method, duration.as_millis());

    Ok(response)
}

/// gRPC middleware for metrics
pub async fn grpc_metrics_middleware(
    _request: Request<()>,
    metrics: ApiMetrics,
) -> Result<Response<()>, Status> {
    let method = _request
        .metadata()
        .get("grpc-method")
        .unwrap_or(&"unknown".parse().unwrap())
        .to_str()
        .unwrap_or("unknown")
        .to_string();
    let start_time = std::time::Instant::now();

    // Increment request counter
    metrics.record_request("grpc", &method, 200);

    // TODO: Implement actual request processing
    let response = Response::new(());

    let duration = start_time.elapsed();

    // Record response time
    metrics.record_response_time("grpc", &method, duration);

    Ok(response)
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
