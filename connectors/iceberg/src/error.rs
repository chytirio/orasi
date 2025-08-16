//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling for the Apache Iceberg connector
//!
//! This module provides structured error types with context and recovery strategies
//! for the Apache Iceberg connector.

use std::error::Error as StdError;
use std::fmt;
use thiserror::Error;

/// Result type for Iceberg operations
pub type IcebergResult<T> = Result<T, IcebergError>;

/// Main error type for the Apache Iceberg connector
#[derive(Error, Debug)]
pub enum IcebergError {
    /// Configuration errors
    #[error("Iceberg configuration error: {message}")]
    Configuration {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Connection errors
    #[error("Iceberg connection error: {message}")]
    Connection {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Table operation errors
    #[error("Iceberg table operation error: {message}")]
    TableOperation {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Schema errors
    #[error("Iceberg schema error: {message}")]
    Schema {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Catalog errors
    #[error("Iceberg catalog error: {message}")]
    Catalog {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Write operation errors
    #[error("Iceberg write error: {message}")]
    Write {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Read operation errors
    #[error("Iceberg read error: {message}")]
    Read {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Storage errors
    #[error("Iceberg storage error: {message}")]
    Storage {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Serialization/deserialization errors
    #[error("Iceberg serialization error: {message}")]
    Serialization {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Validation errors
    #[error("Iceberg validation error: {message}")]
    Validation {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Timeout errors
    #[error("Iceberg timeout error: {message}")]
    Timeout {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Internal errors
    #[error("Iceberg internal error: {message}")]
    Internal {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Unknown errors
    #[error("Iceberg unknown error: {message}")]
    Unknown {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
}

impl IcebergError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        IcebergError::Configuration {
            message: message.into(),
            source: None,
        }
    }

    /// Create a configuration error with source
    pub fn configuration_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Configuration {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a connection error
    pub fn connection(message: impl Into<String>) -> Self {
        IcebergError::Connection {
            message: message.into(),
            source: None,
        }
    }

    /// Create a connection error with source
    pub fn connection_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Connection {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a table operation error
    pub fn table_operation(message: impl Into<String>) -> Self {
        IcebergError::TableOperation {
            message: message.into(),
            source: None,
        }
    }

    /// Create a table operation error with source
    pub fn table_operation_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::TableOperation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a schema error
    pub fn schema(message: impl Into<String>) -> Self {
        IcebergError::Schema {
            message: message.into(),
            source: None,
        }
    }

    /// Create a schema error with source
    pub fn schema_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Schema {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a catalog error
    pub fn catalog(message: impl Into<String>) -> Self {
        IcebergError::Catalog {
            message: message.into(),
            source: None,
        }
    }

    /// Create a catalog error with source
    pub fn catalog_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Catalog {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a write error
    pub fn write(message: impl Into<String>) -> Self {
        IcebergError::Write {
            message: message.into(),
            source: None,
        }
    }

    /// Create a write error with source
    pub fn write_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Write {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a read error
    pub fn read(message: impl Into<String>) -> Self {
        IcebergError::Read {
            message: message.into(),
            source: None,
        }
    }

    /// Create a read error with source
    pub fn read_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Read {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a storage error
    pub fn storage(message: impl Into<String>) -> Self {
        IcebergError::Storage {
            message: message.into(),
            source: None,
        }
    }

    /// Create a storage error with source
    pub fn storage_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Storage {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        IcebergError::Serialization {
            message: message.into(),
            source: None,
        }
    }

    /// Create a serialization error with source
    pub fn serialization_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Serialization {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        IcebergError::Validation {
            message: message.into(),
            source: None,
        }
    }

    /// Create a validation error with source
    pub fn validation_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Validation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        IcebergError::Timeout {
            message: message.into(),
            source: None,
        }
    }

    /// Create a timeout error with source
    pub fn timeout_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Timeout {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        IcebergError::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create an internal error with source
    pub fn internal_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a not implemented error
    pub fn not_implemented(message: impl Into<String>) -> Self {
        IcebergError::Internal {
            message: format!("Not implemented: {}", message.into()),
            source: None,
        }
    }

    /// Create a conversion error
    pub fn conversion(message: impl Into<String>) -> Self {
        IcebergError::Serialization {
            message: format!("Conversion error: {}", message.into()),
            source: None,
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        IcebergError::Unknown {
            message: message.into(),
            source: None,
        }
    }

    /// Create an unknown error with source
    pub fn unknown_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        IcebergError::Unknown {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            IcebergError::Connection { .. }
                | IcebergError::Timeout { .. }
                | IcebergError::Storage { .. }
        )
    }

    /// Check if the error is transient
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            IcebergError::Connection { .. }
                | IcebergError::Timeout { .. }
                | IcebergError::Storage { .. }
                | IcebergError::Catalog { .. }
        )
    }

    /// Check if the error is permanent
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            IcebergError::Configuration { .. }
                | IcebergError::Schema { .. }
                | IcebergError::Validation { .. }
                | IcebergError::Serialization { .. }
        )
    }

    /// Get error context for logging
    pub fn context(&self) -> IcebergErrorContext {
        IcebergErrorContext {
            error_type: self.error_type(),
            retryable: self.is_retryable(),
            transient: self.is_transient(),
            permanent: self.is_permanent(),
        }
    }

    /// Get the error type as a string
    pub fn error_type(&self) -> &'static str {
        match self {
            IcebergError::Configuration { .. } => "configuration",
            IcebergError::Connection { .. } => "connection",
            IcebergError::TableOperation { .. } => "table_operation",
            IcebergError::Schema { .. } => "schema",
            IcebergError::Catalog { .. } => "catalog",
            IcebergError::Write { .. } => "write",
            IcebergError::Read { .. } => "read",
            IcebergError::Storage { .. } => "storage",
            IcebergError::Serialization { .. } => "serialization",
            IcebergError::Validation { .. } => "validation",
            IcebergError::Timeout { .. } => "timeout",
            IcebergError::Internal { .. } => "internal",
            IcebergError::Unknown { .. } => "unknown",
        }
    }
}

/// Error context for logging and monitoring
#[derive(Debug, Clone)]
pub struct IcebergErrorContext {
    pub error_type: &'static str,
    pub retryable: bool,
    pub transient: bool,
    pub permanent: bool,
}

impl fmt::Display for IcebergErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IcebergErrorContext {{ type: {}, retryable: {}, transient: {}, permanent: {} }}",
            self.error_type, self.retryable, self.transient, self.permanent
        )
    }
}

/// Error conversion traits for common error types
impl From<std::io::Error> for IcebergError {
    fn from(err: std::io::Error) -> Self {
        IcebergError::storage_with_source("IO error", err)
    }
}

impl From<serde_json::Error> for IcebergError {
    fn from(err: serde_json::Error) -> Self {
        IcebergError::serialization_with_source("JSON serialization error", err)
    }
}

impl From<url::ParseError> for IcebergError {
    fn from(err: url::ParseError) -> Self {
        IcebergError::configuration_with_source("URL parse error", err)
    }
}

impl From<tokio::time::error::Elapsed> for IcebergError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        IcebergError::timeout_with_source("Operation timeout", err)
    }
}

impl From<config::ConfigError> for IcebergError {
    fn from(err: config::ConfigError) -> Self {
        IcebergError::configuration_with_source("Configuration error", err)
    }
}

impl From<validator::ValidationErrors> for IcebergError {
    fn from(err: validator::ValidationErrors) -> Self {
        IcebergError::validation_with_source("Configuration validation failed", err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = IcebergError::configuration("test error");
        assert!(matches!(error, IcebergError::Configuration { .. }));
        assert!(!error.is_retryable());
        assert!(!error.is_transient());
        assert!(error.is_permanent());
    }

    #[test]
    fn test_error_context() {
        let error = IcebergError::connection("test error");
        let context = error.context();
        assert_eq!(context.error_type, "connection");
        assert!(context.retryable);
        assert!(context.transient);
        assert!(!context.permanent);
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let iceberg_error: IcebergError = io_error.into();
        assert!(matches!(iceberg_error, IcebergError::Storage { .. }));
    }
}
