//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling for the Schema Registry
//!
//! This module provides error types and result aliases for the schema registry.

use std::fmt;
use thiserror::Error;

/// Result type for schema registry operations
pub type SchemaRegistryResult<T> = Result<T, SchemaRegistryError>;

/// Schema Registry error types
#[derive(Error, Debug)]
pub enum SchemaRegistryError {
    /// Configuration error
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Storage error
    #[error("Storage error: {message}")]
    Storage { message: String },

    /// Validation error
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Schema not found
    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    /// Schema already exists
    #[error("Schema already exists: {0}")]
    SchemaAlreadyExists(String),

    /// Invalid schema format
    #[error("Invalid schema format: {format}")]
    InvalidSchemaFormat { format: String },

    /// Schema validation failed
    #[error("Schema validation failed: {0:?}")]
    ValidationFailed(Vec<String>),

    /// Serialization error
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// Deserialization error
    #[error("Deserialization error: {message}")]
    Deserialization { message: String },

    /// Network error
    #[error("Network error: {message}")]
    Network { message: String },

    /// Authentication error
    #[error("Authentication error: {message}")]
    Authentication { message: String },

    /// Authorization error
    #[error("Authorization error: {message}")]
    Authorization { message: String },

    /// Rate limiting error
    #[error("Rate limiting error: {message}")]
    RateLimit { message: String },

    /// Internal error
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Timeout error
    #[error("Timeout error: {message}")]
    Timeout { message: String },

    /// Resource not found
    #[error("Resource not found: {resource}")]
    ResourceNotFound { resource: String },

    /// Resource already exists
    #[error("Resource already exists: {resource}")]
    ResourceAlreadyExists { resource: String },

    /// Invalid request
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    /// Service unavailable
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },
}

impl SchemaRegistryError {
    /// Create a configuration error
    pub fn config(message: &str) -> Self {
        Self::Config {
            message: message.to_string(),
        }
    }

    /// Create a configuration error with details
    pub fn config_with_details(message: &str, details: String) -> Self {
        Self::Config {
            message: format!("{}: {}", message, details),
        }
    }

    /// Create a storage error
    pub fn storage(message: &str) -> Self {
        Self::Storage {
            message: message.to_string(),
        }
    }

    /// Create a validation error
    pub fn validation(message: &str) -> Self {
        Self::Validation {
            message: message.to_string(),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: &str) -> Self {
        Self::Serialization {
            message: message.to_string(),
        }
    }

    /// Create a deserialization error
    pub fn deserialization(message: &str) -> Self {
        Self::Deserialization {
            message: message.to_string(),
        }
    }

    /// Create a network error
    pub fn network(message: &str) -> Self {
        Self::Network {
            message: message.to_string(),
        }
    }

    /// Create an authentication error
    pub fn authentication(message: &str) -> Self {
        Self::Authentication {
            message: message.to_string(),
        }
    }

    /// Create an authorization error
    pub fn authorization(message: &str) -> Self {
        Self::Authorization {
            message: message.to_string(),
        }
    }

    /// Create a rate limiting error
    pub fn rate_limit(message: &str) -> Self {
        Self::RateLimit {
            message: message.to_string(),
        }
    }

    /// Create an internal error
    pub fn internal(message: &str) -> Self {
        Self::Internal {
            message: message.to_string(),
        }
    }

    /// Create a timeout error
    pub fn timeout(message: &str) -> Self {
        Self::Timeout {
            message: message.to_string(),
        }
    }

    /// Create an invalid request error
    pub fn invalid_request(message: &str) -> Self {
        Self::InvalidRequest {
            message: message.to_string(),
        }
    }

    /// Create a service unavailable error
    pub fn service_unavailable(message: &str) -> Self {
        Self::ServiceUnavailable {
            message: message.to_string(),
        }
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network { .. } | Self::Timeout { .. } | Self::ServiceUnavailable { .. }
        )
    }

    /// Get the error code for HTTP responses
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::Config { .. } => 400,
            Self::Validation { .. } => 400,
            Self::InvalidRequest { .. } => 400,
            Self::SchemaNotFound { .. } => 404,
            Self::ResourceNotFound { .. } => 404,
            Self::SchemaAlreadyExists { .. } => 409,
            Self::ResourceAlreadyExists { .. } => 409,
            Self::Authentication { .. } => 401,
            Self::Authorization { .. } => 403,
            Self::RateLimit { .. } => 429,
            Self::Timeout { .. } => 408,
            Self::ServiceUnavailable { .. } => 503,
            Self::Network { .. } => 502,
            Self::Storage { .. } => 500,
            Self::Serialization { .. } => 500,
            Self::Deserialization { .. } => 500,
            Self::ValidationFailed { .. } => 400,
            Self::InvalidSchemaFormat { .. } => 400,
            Self::Internal { .. } => 500,
        }
    }
}

impl From<std::io::Error> for SchemaRegistryError {
    fn from(err: std::io::Error) -> Self {
        Self::Storage {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for SchemaRegistryError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            message: err.to_string(),
        }
    }
}

impl From<serde_yaml::Error> for SchemaRegistryError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Serialization {
            message: err.to_string(),
        }
    }
}

impl From<config::ConfigError> for SchemaRegistryError {
    fn from(err: config::ConfigError) -> Self {
        Self::Config {
            message: err.to_string(),
        }
    }
}

impl From<sqlx::Error> for SchemaRegistryError {
    fn from(err: sqlx::Error) -> Self {
        Self::Storage {
            message: err.to_string(),
        }
    }
}

impl From<redis::RedisError> for SchemaRegistryError {
    fn from(err: redis::RedisError) -> Self {
        Self::Storage {
            message: err.to_string(),
        }
    }
}

#[cfg(feature = "http")]
impl From<axum::http::StatusCode> for SchemaRegistryError {
    fn from(status: axum::http::StatusCode) -> Self {
        match status.as_u16() {
            400 => Self::InvalidRequest {
                message: "Bad request".to_string(),
            },
            401 => Self::Authentication {
                message: "Unauthorized".to_string(),
            },
            403 => Self::Authorization {
                message: "Forbidden".to_string(),
            },
            404 => Self::ResourceNotFound {
                resource: "Resource".to_string(),
            },
            408 => Self::Timeout {
                message: "Request timeout".to_string(),
            },
            429 => Self::RateLimit {
                message: "Too many requests".to_string(),
            },
            500 => Self::Internal {
                message: "Internal server error".to_string(),
            },
            502 => Self::Network {
                message: "Bad gateway".to_string(),
            },
            503 => Self::ServiceUnavailable {
                message: "Service unavailable".to_string(),
            },
            _ => Self::Internal {
                message: format!("HTTP error: {}", status),
            },
        }
    }
}

/// Error context for additional information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Error details
    pub details: Option<String>,
    /// Error timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Error correlation ID
    pub correlation_id: Option<String>,
    /// Error stack trace
    pub stack_trace: Option<String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
            timestamp: chrono::Utc::now(),
            correlation_id: None,
            stack_trace: None,
        }
    }

    /// Add details to the error context
    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    /// Add correlation ID to the error context
    pub fn with_correlation_id(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }

    /// Add stack trace to the error context
    pub fn with_stack_trace(mut self, stack_trace: &str) -> Self {
        self.stack_trace = Some(stack_trace.to_string());
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;

        if let Some(details) = &self.details {
            write!(f, ": {}", details)?;
        }

        if let Some(correlation_id) = &self.correlation_id {
            write!(f, " (correlation_id: {})", correlation_id)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = SchemaRegistryError::config("test error");
        assert!(matches!(error, SchemaRegistryError::Config { .. }));

        let error = SchemaRegistryError::storage("storage error");
        assert!(matches!(error, SchemaRegistryError::Storage { .. }));

        let error = SchemaRegistryError::validation("validation error");
        assert!(matches!(error, SchemaRegistryError::Validation { .. }));
    }

    #[test]
    fn test_error_retryable() {
        let error = SchemaRegistryError::Network {
            message: "network error".to_string(),
        };
        assert!(error.is_retryable());

        let error = SchemaRegistryError::Config {
            message: "config error".to_string(),
        };
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_http_status_codes() {
        let error = SchemaRegistryError::Config {
            message: "config error".to_string(),
        };
        assert_eq!(error.http_status_code(), 400);

        let error = SchemaRegistryError::SchemaNotFound("test".to_string());
        assert_eq!(error.http_status_code(), 404);

        let error = SchemaRegistryError::Internal {
            message: "internal error".to_string(),
        };
        assert_eq!(error.http_status_code(), 500);
    }

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("TEST_ERROR", "test message")
            .with_details("test details")
            .with_correlation_id("test-correlation-id");

        assert_eq!(context.code, "TEST_ERROR");
        assert_eq!(context.message, "test message");
        assert_eq!(context.details, Some("test details".to_string()));
        assert_eq!(
            context.correlation_id,
            Some("test-correlation-id".to_string())
        );
    }
}
