//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling for the Apache Hudi connector
//!
//! This module provides structured error types with context and recovery strategies
//! for the Apache Hudi connector.

use std::error::Error as StdError;
use std::fmt;
use thiserror::Error;

/// Result type for Hudi operations
pub type HudiResult<T> = Result<T, HudiError>;

/// Main error type for the Apache Hudi connector
#[derive(Error, Debug)]
pub enum HudiError {
    /// Configuration errors
    #[error("Hudi configuration error: {message}")]
    Configuration {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Connection errors
    #[error("Hudi connection error: {message}")]
    Connection {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Table operation errors
    #[error("Hudi table operation error: {message}")]
    TableOperation {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Schema errors
    #[error("Hudi schema error: {message}")]
    Schema {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Write operation errors
    #[error("Hudi write error: {message}")]
    Write {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Read operation errors
    #[error("Hudi read error: {message}")]
    Read {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Storage errors
    #[error("Hudi storage error: {message}")]
    Storage {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Serialization/deserialization errors
    #[error("Hudi serialization error: {message}")]
    Serialization {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Validation errors
    #[error("Hudi validation error: {message}")]
    Validation {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Timeout errors
    #[error("Hudi timeout error: {message}")]
    Timeout {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Internal errors
    #[error("Hudi internal error: {message}")]
    Internal {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Unknown errors
    #[error("Hudi unknown error: {message}")]
    Unknown {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
}

impl HudiError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        HudiError::Configuration {
            message: message.into(),
            source: None,
        }
    }

    /// Create a configuration error with source
    pub fn configuration_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Configuration {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a connection error
    pub fn connection(message: impl Into<String>) -> Self {
        HudiError::Connection {
            message: message.into(),
            source: None,
        }
    }

    /// Create a connection error with source
    pub fn connection_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Connection {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a table operation error
    pub fn table_operation(message: impl Into<String>) -> Self {
        HudiError::TableOperation {
            message: message.into(),
            source: None,
        }
    }

    /// Create a table operation error with source
    pub fn table_operation_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::TableOperation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a schema error
    pub fn schema(message: impl Into<String>) -> Self {
        HudiError::Schema {
            message: message.into(),
            source: None,
        }
    }

    /// Create a schema error with source
    pub fn schema_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Schema {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a write error
    pub fn write(message: impl Into<String>) -> Self {
        HudiError::Write {
            message: message.into(),
            source: None,
        }
    }

    /// Create a write error with source
    pub fn write_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Write {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a read error
    pub fn read(message: impl Into<String>) -> Self {
        HudiError::Read {
            message: message.into(),
            source: None,
        }
    }

    /// Create a read error with source
    pub fn read_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Read {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a storage error
    pub fn storage(message: impl Into<String>) -> Self {
        HudiError::Storage {
            message: message.into(),
            source: None,
        }
    }

    /// Create a storage error with source
    pub fn storage_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Storage {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        HudiError::Serialization {
            message: message.into(),
            source: None,
        }
    }

    /// Create a serialization error with source
    pub fn serialization_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Serialization {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        HudiError::Validation {
            message: message.into(),
            source: None,
        }
    }

    /// Create a validation error with source
    pub fn validation_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Validation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        HudiError::Timeout {
            message: message.into(),
            source: None,
        }
    }

    /// Create a timeout error with source
    pub fn timeout_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Timeout {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        HudiError::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create an internal error with source
    pub fn internal_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        HudiError::Unknown {
            message: message.into(),
            source: None,
        }
    }

    /// Create an unknown error with source
    pub fn unknown_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        HudiError::Unknown {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            HudiError::Connection { .. } | HudiError::Timeout { .. } | HudiError::Storage { .. }
        )
    }

    /// Check if the error is transient
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            HudiError::Connection { .. } | HudiError::Timeout { .. } | HudiError::Storage { .. }
        )
    }

    /// Check if the error is permanent
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            HudiError::Configuration { .. }
                | HudiError::Schema { .. }
                | HudiError::Validation { .. }
                | HudiError::Serialization { .. }
        )
    }

    /// Get error context for logging
    pub fn context(&self) -> HudiErrorContext {
        HudiErrorContext {
            error_type: self.error_type(),
            retryable: self.is_retryable(),
            transient: self.is_transient(),
            permanent: self.is_permanent(),
        }
    }

    /// Get the error type as a string
    pub fn error_type(&self) -> &'static str {
        match self {
            HudiError::Configuration { .. } => "configuration",
            HudiError::Connection { .. } => "connection",
            HudiError::TableOperation { .. } => "table_operation",
            HudiError::Schema { .. } => "schema",
            HudiError::Write { .. } => "write",
            HudiError::Read { .. } => "read",
            HudiError::Storage { .. } => "storage",
            HudiError::Serialization { .. } => "serialization",
            HudiError::Validation { .. } => "validation",
            HudiError::Timeout { .. } => "timeout",
            HudiError::Internal { .. } => "internal",
            HudiError::Unknown { .. } => "unknown",
        }
    }
}

/// Error context for logging and monitoring
#[derive(Debug, Clone)]
pub struct HudiErrorContext {
    pub error_type: &'static str,
    pub retryable: bool,
    pub transient: bool,
    pub permanent: bool,
}

impl fmt::Display for HudiErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HudiErrorContext {{ type: {}, retryable: {}, transient: {}, permanent: {} }}",
            self.error_type, self.retryable, self.transient, self.permanent
        )
    }
}

/// Error conversion traits for common error types
impl From<std::io::Error> for HudiError {
    fn from(err: std::io::Error) -> Self {
        HudiError::storage_with_source("IO error", err)
    }
}

impl From<serde_json::Error> for HudiError {
    fn from(err: serde_json::Error) -> Self {
        HudiError::serialization_with_source("JSON serialization error", err)
    }
}

impl From<url::ParseError> for HudiError {
    fn from(err: url::ParseError) -> Self {
        HudiError::configuration_with_source("URL parse error", err)
    }
}

impl From<tokio::time::error::Elapsed> for HudiError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        HudiError::timeout_with_source("Operation timeout", err)
    }
}

impl From<config::ConfigError> for HudiError {
    fn from(err: config::ConfigError) -> Self {
        HudiError::configuration_with_source("Configuration error", err)
    }
}

impl From<validator::ValidationErrors> for HudiError {
    fn from(err: validator::ValidationErrors) -> Self {
        HudiError::validation_with_source("Configuration validation failed", err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = HudiError::configuration("test error");
        assert!(matches!(error, HudiError::Configuration { .. }));
        assert!(!error.is_retryable());
        assert!(!error.is_transient());
        assert!(error.is_permanent());
    }

    #[test]
    fn test_error_context() {
        let error = HudiError::connection("test error");
        let context = error.context();
        assert_eq!(context.error_type, "connection");
        assert!(context.retryable);
        assert!(context.transient);
        assert!(!context.permanent);
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let hudi_error: HudiError = io_error.into();
        assert!(matches!(hudi_error, HudiError::Storage { .. }));
    }
}
