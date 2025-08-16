//! API error handling
//!
//! This module contains the API error types and error handling logic.

use crate::error::SchemaRegistryError;

/// API error
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Bad request
    #[error("Bad request: {message}")]
    BadRequest { message: String },

    /// Not found
    #[error("Not found: {message}")]
    NotFound { message: String },

    /// Internal server error
    #[error("Internal server error: {message}")]
    Internal { message: String },

    /// Validation error
    #[error("Validation error: {message}")]
    Validation { message: String },
}

impl ApiError {
    /// Create a bad request error
    pub fn bad_request(message: &str) -> Self {
        Self::BadRequest {
            message: message.to_string(),
        }
    }

    /// Create a not found error
    pub fn not_found(message: &str) -> Self {
        Self::NotFound {
            message: message.to_string(),
        }
    }

    /// Create an internal error
    pub fn internal(message: &str) -> Self {
        Self::Internal {
            message: message.to_string(),
        }
    }

    /// Create a validation error
    pub fn validation(message: &str) -> Self {
        Self::Validation {
            message: message.to_string(),
        }
    }

    /// Convert from registry error
    pub fn from_registry_error(error: SchemaRegistryError) -> Self {
        match error {
            SchemaRegistryError::Config { message } => Self::BadRequest { message },
            SchemaRegistryError::Validation { message } => Self::Validation { message },
            SchemaRegistryError::SchemaNotFound(fingerprint) => Self::NotFound {
                message: format!("Schema not found: {}", fingerprint),
            },
            SchemaRegistryError::SchemaAlreadyExists(fingerprint) => Self::BadRequest {
                message: format!("Schema already exists: {}", fingerprint),
            },
            SchemaRegistryError::InvalidRequest { message } => Self::BadRequest { message },
            SchemaRegistryError::Authentication { message } => Self::BadRequest { message },
            SchemaRegistryError::Authorization { message } => Self::BadRequest { message },
            SchemaRegistryError::RateLimit { message } => Self::BadRequest { message },
            SchemaRegistryError::Timeout { message } => Self::BadRequest { message },
            SchemaRegistryError::ServiceUnavailable { message } => Self::Internal { message },
            SchemaRegistryError::Network { message } => Self::Internal { message },
            SchemaRegistryError::Storage { message } => Self::Internal { message },
            SchemaRegistryError::Serialization { message } => Self::BadRequest { message },
            SchemaRegistryError::Deserialization { message } => Self::BadRequest { message },
            SchemaRegistryError::ValidationFailed(errors) => Self::Validation {
                message: format!("Validation failed: {:?}", errors),
            },
            SchemaRegistryError::InvalidSchemaFormat { format } => Self::BadRequest {
                message: format!("Invalid schema format: {}", format),
            },
            SchemaRegistryError::Internal { message } => Self::Internal { message },
            SchemaRegistryError::ResourceNotFound { resource } => Self::NotFound {
                message: format!("Resource not found: {}", resource),
            },
            SchemaRegistryError::ResourceAlreadyExists { resource } => Self::BadRequest {
                message: format!("Resource already exists: {}", resource),
            },
        }
    }
}

#[cfg(feature = "http")]
impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::BadRequest { ref message } => {
                (axum::http::StatusCode::BAD_REQUEST, message.clone())
            }
            ApiError::NotFound { ref message } => {
                (axum::http::StatusCode::NOT_FOUND, message.clone())
            }
            ApiError::Internal { ref message } => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                message.clone(),
            ),
            ApiError::Validation { ref message } => {
                (axum::http::StatusCode::BAD_REQUEST, message.clone())
            }
        };

        let body = axum::response::Json(crate::api::responses::ErrorResponse {
            error: self.to_string(),
            message,
            status_code: status.as_u16(),
        });

        (status, body).into_response()
    }
}
