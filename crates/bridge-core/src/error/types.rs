//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error types for the OpenTelemetry Data Lake Bridge
//!
//! This module provides the main error types and structures used throughout
//! the bridge.

use std::error::Error as StdError;
use thiserror::Error;

/// Result type for bridge operations
pub type BridgeResult<T> = Result<T, BridgeError>;

/// Main error type for the bridge
#[derive(Error, Debug)]
pub enum BridgeError {
    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// OpenTelemetry ingestion errors
    #[error("OpenTelemetry ingestion error: {message}")]
    Ingestion {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Data lakehouse connector errors
    #[error("Lakehouse connector error: {message}")]
    Lakehouse {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Export errors
    #[error("Export error: {message}")]
    Export {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Query processing errors
    #[error("Query processing error: {message}")]
    Query {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Stream processing errors
    #[error("Stream processing error: {message}")]
    Stream {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Data processing errors
    #[error("Processing error: {message}")]
    Processing {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Schema registry errors
    #[error("Schema registry error: {message}")]
    Schema {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Plugin interface errors
    #[error("Plugin interface error: {message}")]
    Plugin {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Network and communication errors
    #[error("Network error: {message}")]
    Network {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Authentication and authorization errors
    #[error("Authentication error: {message}")]
    Authentication {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Data validation errors
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Timeout errors
    #[error("Timeout error: {message}")]
    Timeout {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Resource exhaustion errors
    #[error("Resource exhaustion: {message}")]
    ResourceExhaustion {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Circuit breaker errors
    #[error("Circuit breaker open: {message}")]
    CircuitBreaker {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Internal errors
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Unknown errors
    #[error("Unknown error: {message}")]
    Unknown {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
}

impl BridgeError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        BridgeError::Configuration {
            message: message.into(),
            source: None,
        }
    }

    /// Create a configuration error with source
    pub fn configuration_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Configuration {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an ingestion error
    pub fn ingestion(message: impl Into<String>) -> Self {
        BridgeError::Ingestion {
            message: message.into(),
            source: None,
        }
    }

    /// Create an ingestion error with source
    pub fn ingestion_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Ingestion {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a lakehouse error
    pub fn lakehouse(message: impl Into<String>) -> Self {
        BridgeError::Lakehouse {
            message: message.into(),
            source: None,
        }
    }

    /// Create a lakehouse error with source
    pub fn lakehouse_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Lakehouse {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an export error
    pub fn export(message: impl Into<String>) -> Self {
        BridgeError::Export {
            message: message.into(),
            source: None,
        }
    }

    /// Create an export error with source
    pub fn export_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Export {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a query error
    pub fn query(message: impl Into<String>) -> Self {
        BridgeError::Query {
            message: message.into(),
            source: None,
        }
    }

    /// Create a query error with source
    pub fn query_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Query {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a stream error
    pub fn stream(message: impl Into<String>) -> Self {
        BridgeError::Stream {
            message: message.into(),
            source: None,
        }
    }

    /// Create a processing error
    pub fn processing(message: impl Into<String>) -> Self {
        BridgeError::Processing {
            message: message.into(),
            source: None,
        }
    }

    /// Create a stream error with source
    pub fn stream_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Stream {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a schema error
    pub fn schema(message: impl Into<String>) -> Self {
        BridgeError::Schema {
            message: message.into(),
            source: None,
        }
    }

    /// Create a schema error with source
    pub fn schema_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Schema {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a plugin error
    pub fn plugin(message: impl Into<String>) -> Self {
        BridgeError::Plugin {
            message: message.into(),
            source: None,
        }
    }

    /// Create a plugin error with source
    pub fn plugin_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Plugin {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        BridgeError::Network {
            message: message.into(),
            source: None,
        }
    }

    /// Create a network error with source
    pub fn network_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Network {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        BridgeError::Authentication {
            message: message.into(),
            source: None,
        }
    }

    /// Create an authentication error with source
    pub fn authentication_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Authentication {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        BridgeError::Validation {
            message: message.into(),
            source: None,
        }
    }

    /// Create a validation error with source
    pub fn validation_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Validation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        BridgeError::Serialization {
            message: message.into(),
            source: None,
        }
    }

    /// Create a serialization error with source
    pub fn serialization_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Serialization {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        BridgeError::Timeout {
            message: message.into(),
            source: None,
        }
    }

    /// Create a timeout error with source
    pub fn timeout_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Timeout {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a resource exhaustion error
    pub fn resource_exhaustion(message: impl Into<String>) -> Self {
        BridgeError::ResourceExhaustion {
            message: message.into(),
            source: None,
        }
    }

    /// Create a resource exhaustion error with source
    pub fn resource_exhaustion_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::ResourceExhaustion {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a circuit breaker error
    pub fn circuit_breaker(message: impl Into<String>) -> Self {
        BridgeError::CircuitBreaker {
            message: message.into(),
            source: None,
        }
    }

    /// Create a circuit breaker error with source
    pub fn circuit_breaker_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::CircuitBreaker {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        BridgeError::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create an internal error with source
    pub fn internal_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        BridgeError::Unknown {
            message: message.into(),
            source: None,
        }
    }

    /// Create an unknown error with source
    pub fn unknown_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        BridgeError::Unknown {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            BridgeError::Network { .. }
                | BridgeError::Timeout { .. }
                | BridgeError::ResourceExhaustion { .. }
                | BridgeError::CircuitBreaker { .. }
        )
    }

    /// Check if the error is transient
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            BridgeError::Network { .. }
                | BridgeError::Timeout { .. }
                | BridgeError::ResourceExhaustion { .. }
        )
    }

    /// Check if the error is permanent
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            BridgeError::Configuration { .. }
                | BridgeError::Validation { .. }
                | BridgeError::Authentication { .. }
        )
    }

    /// Get error context for logging
    pub fn context(&self) -> crate::error::ErrorContext {
        crate::error::ErrorContext {
            error_type: self.error_type(),
            retryable: self.is_retryable(),
            transient: self.is_transient(),
            permanent: self.is_permanent(),
        }
    }

    /// Get the error type as a string
    pub fn error_type(&self) -> &'static str {
        match self {
            BridgeError::Configuration { .. } => "Configuration",
            BridgeError::Ingestion { .. } => "Ingestion",
            BridgeError::Lakehouse { .. } => "Lakehouse",
            BridgeError::Export { .. } => "Export",
            BridgeError::Query { .. } => "Query",
            BridgeError::Stream { .. } => "Stream",
            BridgeError::Processing { .. } => "Processing",
            BridgeError::Schema { .. } => "Schema",
            BridgeError::Plugin { .. } => "Plugin",
            BridgeError::Network { .. } => "Network",
            BridgeError::Authentication { .. } => "Authentication",
            BridgeError::Validation { .. } => "Validation",
            BridgeError::Serialization { .. } => "Serialization",
            BridgeError::Timeout { .. } => "Timeout",
            BridgeError::ResourceExhaustion { .. } => "ResourceExhaustion",
            BridgeError::CircuitBreaker { .. } => "CircuitBreaker",
            BridgeError::Internal { .. } => "Internal",
            BridgeError::Unknown { .. } => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let config_err = BridgeError::configuration("Invalid config");
        assert!(matches!(config_err, BridgeError::Configuration { .. }));
        assert!(!config_err.is_retryable());
        assert!(config_err.is_permanent());

        let network_err = BridgeError::network("Connection failed");
        assert!(matches!(network_err, BridgeError::Network { .. }));
        assert!(network_err.is_retryable());
        assert!(network_err.is_transient());
    }

    #[test]
    fn test_error_context() {
        let err = BridgeError::timeout("Operation timed out");
        let context = err.context();
        assert_eq!(context.error_type, "Timeout");
        assert!(context.retryable);
        assert!(context.transient);
        assert!(!context.permanent);
    }
}
