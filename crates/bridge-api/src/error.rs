//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling for Bridge API

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use bridge_auth;

/// API error types
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout error: {0}")]
    Timeout(String),
}

impl ApiError {
    /// Get HTTP status code for the error
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            ApiError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Configuration(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Authentication(_) => StatusCode::UNAUTHORIZED,
            ApiError::Authorization(_) => StatusCode::FORBIDDEN,
            ApiError::Serialization(_) => StatusCode::BAD_REQUEST,
            ApiError::Deserialization(_) => StatusCode::BAD_REQUEST,
            ApiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Network(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
        }
    }

    /// Get error code for the error
    pub fn error_code(&self) -> &'static str {
        match self {
            ApiError::Internal(_) => "INTERNAL_ERROR",
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::Unauthorized(_) => "UNAUTHORIZED",
            ApiError::Forbidden(_) => "FORBIDDEN",
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::Conflict(_) => "CONFLICT",
            ApiError::Validation(_) => "VALIDATION_ERROR",
            ApiError::RateLimitExceeded(_) => "RATE_LIMIT_EXCEEDED",
            ApiError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            ApiError::Configuration(_) => "CONFIGURATION_ERROR",
            ApiError::Authentication(_) => "AUTHENTICATION_ERROR",
            ApiError::Authorization(_) => "AUTHORIZATION_ERROR",
            ApiError::Serialization(_) => "SERIALIZATION_ERROR",
            ApiError::Deserialization(_) => "DESERIALIZATION_ERROR",
            ApiError::Database(_) => "DATABASE_ERROR",
            ApiError::Network(_) => "NETWORK_ERROR",
            ApiError::Timeout(_) => "TIMEOUT_ERROR",
        }
    }

    /// Get error message
    pub fn message(&self) -> String {
        match self {
            ApiError::Internal(msg) => msg.clone(),
            ApiError::BadRequest(msg) => msg.clone(),
            ApiError::Unauthorized(msg) => msg.clone(),
            ApiError::Forbidden(msg) => msg.clone(),
            ApiError::NotFound(msg) => msg.clone(),
            ApiError::Conflict(msg) => msg.clone(),
            ApiError::Validation(msg) => msg.clone(),
            ApiError::RateLimitExceeded(msg) => msg.clone(),
            ApiError::ServiceUnavailable(msg) => msg.clone(),
            ApiError::Configuration(msg) => msg.clone(),
            ApiError::Authentication(msg) => msg.clone(),
            ApiError::Authorization(msg) => msg.clone(),
            ApiError::Serialization(msg) => msg.clone(),
            ApiError::Deserialization(msg) => msg.clone(),
            ApiError::Database(msg) => msg.clone(),
            ApiError::Network(msg) => msg.clone(),
            ApiError::Timeout(msg) => msg.clone(),
        }
    }
}

/// Error response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,
    
    /// Error message
    pub message: String,
    
    /// Error details (optional)
    pub details: Option<serde_json::Value>,
    
    /// Request ID for tracking
    pub request_id: Option<String>,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error: &ApiError, request_id: Option<String>) -> Self {
        Self {
            code: error.error_code().to_string(),
            message: error.message(),
            details: None,
            request_id,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a new error response with details
    pub fn with_details(
        error: &ApiError,
        details: serde_json::Value,
        request_id: Option<String>,
    ) -> Self {
        Self {
            code: error.error_code().to_string(),
            message: error.message(),
            details: Some(details),
            request_id,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let error_response = ErrorResponse::new(&self, None);
        
        (status_code, Json(error_response)).into_response()
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::Internal(format!("IO error: {}", err))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::Serialization(format!("JSON serialization error: {}", err))
    }
}

impl From<config::ConfigError> for ApiError {
    fn from(err: config::ConfigError) -> Self {
        ApiError::Configuration(format!("Configuration error: {}", err))
    }
}

impl From<tonic::transport::Error> for ApiError {
    fn from(err: tonic::transport::Error) -> Self {
        ApiError::Network(format!("gRPC transport error: {}", err))
    }
}

impl From<tonic::Status> for ApiError {
    fn from(err: tonic::Status) -> Self {
        match err.code() {
            tonic::Code::InvalidArgument => ApiError::BadRequest(err.message().to_string()),
            tonic::Code::NotFound => ApiError::NotFound(err.message().to_string()),
            tonic::Code::AlreadyExists => ApiError::Conflict(err.message().to_string()),
            tonic::Code::PermissionDenied => ApiError::Forbidden(err.message().to_string()),
            tonic::Code::Unauthenticated => ApiError::Unauthorized(err.message().to_string()),
            tonic::Code::ResourceExhausted => ApiError::RateLimitExceeded(err.message().to_string()),
            tonic::Code::Unavailable => ApiError::ServiceUnavailable(err.message().to_string()),
            tonic::Code::DeadlineExceeded => ApiError::Timeout(err.message().to_string()),
            _ => ApiError::Internal(format!("gRPC error: {}", err)),
        }
    }
}

impl From<bridge_core::error::BridgeError> for ApiError {
    fn from(err: bridge_core::error::BridgeError) -> Self {
        match err {
            bridge_core::error::BridgeError::Configuration { message, .. } => ApiError::Configuration(message),
            bridge_core::error::BridgeError::Validation { message, .. } => ApiError::Validation(message),
            bridge_core::error::BridgeError::Authentication { message, .. } => ApiError::Authentication(message),
            bridge_core::error::BridgeError::Serialization { message, .. } => ApiError::Serialization(message),
            bridge_core::error::BridgeError::Network { message, .. } => ApiError::Network(message),
            bridge_core::error::BridgeError::Timeout { message, .. } => ApiError::Timeout(message),
            bridge_core::error::BridgeError::Internal { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::Ingestion { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::Lakehouse { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::Export { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::Query { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::Stream { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::Processing { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::Schema { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::Plugin { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::ResourceExhaustion { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::CircuitBreaker { message, .. } => ApiError::Internal(message),
            bridge_core::error::BridgeError::Unknown { message, .. } => ApiError::Internal(message),
        }
    }
}

impl From<bridge_auth::AuthError> for ApiError {
    fn from(err: bridge_auth::AuthError) -> Self {
        match err {
            bridge_auth::AuthError::InvalidCredentials(message) => ApiError::Unauthorized(message),
            bridge_auth::AuthError::AccountLocked(message) => ApiError::Forbidden(message),
            bridge_auth::AuthError::UserNotFound(message) => ApiError::NotFound(message),
            bridge_auth::AuthError::AuthenticationFailed(message) => ApiError::Unauthorized(message),
            bridge_auth::AuthError::AuthorizationFailed(message) => ApiError::Forbidden(message),
            bridge_auth::AuthError::TokenExpired(message) => ApiError::Unauthorized(message),
            bridge_auth::AuthError::TokenValidation(message) => ApiError::Unauthorized(message),
            bridge_auth::AuthError::TokenGeneration(message) => ApiError::Internal(message),
            bridge_auth::AuthError::TokenNotYetValid(message) => ApiError::Unauthorized(message),
            bridge_auth::AuthError::TokenBlacklisted(message) => ApiError::Unauthorized(message),
            bridge_auth::AuthError::RefreshDisabled(message) => ApiError::Forbidden(message),
            bridge_auth::AuthError::RateLimitExceeded(message) => ApiError::RateLimitExceeded(message),
            bridge_auth::AuthError::Internal(message) => ApiError::Internal(message),
        }
    }
}

/// Enhanced error response with more detailed information
#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedErrorResponse {
    /// Error code
    pub code: String,
    
    /// Error message
    pub message: String,
    
    /// Error details (optional)
    pub details: Option<serde_json::Value>,
    
    /// Request ID for tracking
    pub request_id: Option<String>,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Error category
    pub category: String,
    
    /// Suggested action (optional)
    pub suggested_action: Option<String>,
    
    /// Error severity
    pub severity: ErrorSeverity,
}

/// Error severity levels
#[derive(Debug, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl EnhancedErrorResponse {
    /// Create a new enhanced error response
    pub fn new(error: &ApiError, request_id: Option<String>) -> Self {
        let (category, severity, suggested_action) = match error {
            ApiError::Internal(_) => ("system", ErrorSeverity::High, Some("Contact support".to_string())),
            ApiError::BadRequest(_) => ("client", ErrorSeverity::Low, Some("Check request format".to_string())),
            ApiError::Unauthorized(_) => ("auth", ErrorSeverity::Medium, Some("Provide valid credentials".to_string())),
            ApiError::Forbidden(_) => ("auth", ErrorSeverity::Medium, Some("Check permissions".to_string())),
            ApiError::NotFound(_) => ("client", ErrorSeverity::Low, Some("Check resource path".to_string())),
            ApiError::Conflict(_) => ("client", ErrorSeverity::Medium, Some("Resolve conflict".to_string())),
            ApiError::Validation(_) => ("client", ErrorSeverity::Low, Some("Fix validation errors".to_string())),
            ApiError::RateLimitExceeded(_) => ("client", ErrorSeverity::Medium, Some("Wait and retry".to_string())),
            ApiError::ServiceUnavailable(_) => ("system", ErrorSeverity::High, Some("Retry later".to_string())),
            ApiError::Configuration(_) => ("system", ErrorSeverity::Critical, Some("Contact administrator".to_string())),
            ApiError::Authentication(_) => ("auth", ErrorSeverity::Medium, Some("Check credentials".to_string())),
            ApiError::Authorization(_) => ("auth", ErrorSeverity::Medium, Some("Check permissions".to_string())),
            ApiError::Serialization(_) => ("client", ErrorSeverity::Low, Some("Check data format".to_string())),
            ApiError::Deserialization(_) => ("client", ErrorSeverity::Low, Some("Check data format".to_string())),
            ApiError::Database(_) => ("system", ErrorSeverity::High, Some("Contact support".to_string())),
            ApiError::Network(_) => ("system", ErrorSeverity::Medium, Some("Check connectivity".to_string())),
            ApiError::Timeout(_) => ("system", ErrorSeverity::Medium, Some("Retry with longer timeout".to_string())),
        };

        Self {
            code: error.error_code().to_string(),
            message: error.message(),
            details: None,
            request_id,
            timestamp: chrono::Utc::now(),
            category: category.to_string(),
            suggested_action,
            severity,
        }
    }

    /// Create a new enhanced error response with details
    pub fn with_details(
        error: &ApiError,
        details: serde_json::Value,
        request_id: Option<String>,
    ) -> Self {
        let mut response = Self::new(error, request_id);
        response.details = Some(details);
        response
    }
}

/// Error context for better error handling
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub endpoint: Option<String>,
    pub method: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            request_id: None,
            user_id: None,
            endpoint: None,
            method: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    pub fn with_method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Helper function to create internal server error
pub fn internal_error(message: impl Into<String>) -> ApiError {
    ApiError::Internal(message.into())
}

/// Helper function to create bad request error
pub fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError::BadRequest(message.into())
}

/// Helper function to create unauthorized error
pub fn unauthorized(message: impl Into<String>) -> ApiError {
    ApiError::Unauthorized(message.into())
}

/// Helper function to create forbidden error
pub fn forbidden(message: impl Into<String>) -> ApiError {
    ApiError::Forbidden(message.into())
}

/// Helper function to create not found error
pub fn not_found(message: impl Into<String>) -> ApiError {
    ApiError::NotFound(message.into())
}

/// Helper function to create validation error
pub fn validation_error(message: impl Into<String>) -> ApiError {
    ApiError::Validation(message.into())
}

/// Helper function to create rate limit error
pub fn rate_limit_exceeded(message: impl Into<String>) -> ApiError {
    ApiError::RateLimitExceeded(message.into())
}
