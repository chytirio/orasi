//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling for the S3/Parquet connector
//!
//! This module provides structured error types with context and recovery strategies
//! for the S3/Parquet connector.

use std::error::Error as StdError;
use std::fmt;
use thiserror::Error;

/// Result type for S3/Parquet operations
pub type S3ParquetResult<T> = Result<T, S3ParquetError>;

/// Main error type for the S3/Parquet connector
#[derive(Error, Debug)]
pub enum S3ParquetError {
    /// Configuration errors
    #[error("S3/Parquet configuration error: {message}")]
    Configuration {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// S3 connection errors
    #[error("S3 connection error: {message}")]
    S3Connection {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// S3 operation errors
    #[error("S3 operation error: {message}")]
    S3Operation {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Parquet errors
    #[error("Parquet error: {message}")]
    Parquet {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Write operation errors
    #[error("S3/Parquet write error: {message}")]
    Write {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Read operation errors
    #[error("S3/Parquet read error: {message}")]
    Read {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Storage errors
    #[error("S3/Parquet storage error: {message}")]
    Storage {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Serialization/deserialization errors
    #[error("S3/Parquet serialization error: {message}")]
    Serialization {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Validation errors
    #[error("S3/Parquet validation error: {message}")]
    Validation {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Timeout errors
    #[error("S3/Parquet timeout error: {message}")]
    Timeout {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Rate limiting errors
    #[error("S3/Parquet rate limiting error: {message}")]
    RateLimit {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Internal errors
    #[error("S3/Parquet internal error: {message}")]
    Internal {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Unknown errors
    #[error("S3/Parquet unknown error: {message}")]
    Unknown {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
}

impl S3ParquetError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        S3ParquetError::Configuration {
            message: message.into(),
            source: None,
        }
    }

    /// Create a configuration error with source
    pub fn configuration_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Configuration {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an S3 connection error
    pub fn s3_connection(message: impl Into<String>) -> Self {
        S3ParquetError::S3Connection {
            message: message.into(),
            source: None,
        }
    }

    /// Create an S3 connection error with source
    pub fn s3_connection_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::S3Connection {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an S3 operation error
    pub fn s3_operation(message: impl Into<String>) -> Self {
        S3ParquetError::S3Operation {
            message: message.into(),
            source: None,
        }
    }

    /// Create an S3 operation error with source
    pub fn s3_operation_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::S3Operation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a Parquet error
    pub fn parquet(message: impl Into<String>) -> Self {
        S3ParquetError::Parquet {
            message: message.into(),
            source: None,
        }
    }

    /// Create a Parquet error with source
    pub fn parquet_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Parquet {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a write error
    pub fn write(message: impl Into<String>) -> Self {
        S3ParquetError::Write {
            message: message.into(),
            source: None,
        }
    }

    /// Create a write error with source
    pub fn write_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Write {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a read error
    pub fn read(message: impl Into<String>) -> Self {
        S3ParquetError::Read {
            message: message.into(),
            source: None,
        }
    }

    /// Create a read error with source
    pub fn read_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Read {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a storage error
    pub fn storage(message: impl Into<String>) -> Self {
        S3ParquetError::Storage {
            message: message.into(),
            source: None,
        }
    }

    /// Create a storage error with source
    pub fn storage_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Storage {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        S3ParquetError::Serialization {
            message: message.into(),
            source: None,
        }
    }

    /// Create a serialization error with source
    pub fn serialization_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Serialization {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        S3ParquetError::Validation {
            message: message.into(),
            source: None,
        }
    }

    /// Create a validation error with source
    pub fn validation_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Validation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        S3ParquetError::Timeout {
            message: message.into(),
            source: None,
        }
    }

    /// Create a timeout error with source
    pub fn timeout_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Timeout {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        S3ParquetError::RateLimit {
            message: message.into(),
            source: None,
        }
    }

    /// Create a rate limit error with source
    pub fn rate_limit_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::RateLimit {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        S3ParquetError::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create an internal error with source
    pub fn internal_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        S3ParquetError::Unknown {
            message: message.into(),
            source: None,
        }
    }

    /// Create an unknown error with source
    pub fn unknown_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        S3ParquetError::Unknown {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            S3ParquetError::S3Connection { .. }
                | S3ParquetError::Timeout { .. }
                | S3ParquetError::RateLimit { .. }
        )
    }

    /// Check if the error is transient
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            S3ParquetError::S3Connection { .. }
                | S3ParquetError::Timeout { .. }
                | S3ParquetError::RateLimit { .. }
                | S3ParquetError::S3Operation { .. }
        )
    }

    /// Check if the error is permanent
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            S3ParquetError::Configuration { .. }
                | S3ParquetError::Validation { .. }
                | S3ParquetError::Serialization { .. }
                | S3ParquetError::Parquet { .. }
        )
    }

    /// Get error context for logging
    pub fn context(&self) -> S3ParquetErrorContext {
        S3ParquetErrorContext {
            error_type: self.error_type(),
            retryable: self.is_retryable(),
            transient: self.is_transient(),
            permanent: self.is_permanent(),
        }
    }

    /// Get the error type as a string
    pub fn error_type(&self) -> &'static str {
        match self {
            S3ParquetError::Configuration { .. } => "configuration",
            S3ParquetError::S3Connection { .. } => "s3_connection",
            S3ParquetError::S3Operation { .. } => "s3_operation",
            S3ParquetError::Parquet { .. } => "parquet",
            S3ParquetError::Write { .. } => "write",
            S3ParquetError::Read { .. } => "read",
            S3ParquetError::Storage { .. } => "storage",
            S3ParquetError::Serialization { .. } => "serialization",
            S3ParquetError::Validation { .. } => "validation",
            S3ParquetError::Timeout { .. } => "timeout",
            S3ParquetError::RateLimit { .. } => "rate_limit",
            S3ParquetError::Internal { .. } => "internal",
            S3ParquetError::Unknown { .. } => "unknown",
        }
    }
}

/// Error context for logging and monitoring
#[derive(Debug, Clone)]
pub struct S3ParquetErrorContext {
    pub error_type: &'static str,
    pub retryable: bool,
    pub transient: bool,
    pub permanent: bool,
}

impl fmt::Display for S3ParquetErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "S3ParquetErrorContext {{ type: {}, retryable: {}, transient: {}, permanent: {} }}",
            self.error_type, self.retryable, self.transient, self.permanent
        )
    }
}

/// Error conversion traits for common error types
impl From<std::io::Error> for S3ParquetError {
    fn from(err: std::io::Error) -> Self {
        S3ParquetError::storage_with_source("IO error", err)
    }
}

impl From<serde_json::Error> for S3ParquetError {
    fn from(err: serde_json::Error) -> Self {
        S3ParquetError::serialization_with_source("JSON serialization error", err)
    }
}

impl From<url::ParseError> for S3ParquetError {
    fn from(err: url::ParseError) -> Self {
        S3ParquetError::configuration_with_source("URL parse error", err)
    }
}

impl From<tokio::time::error::Elapsed> for S3ParquetError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        S3ParquetError::timeout_with_source("Operation timeout", err)
    }
}

impl From<config::ConfigError> for S3ParquetError {
    fn from(err: config::ConfigError) -> Self {
        S3ParquetError::configuration_with_source("Configuration error", err)
    }
}

impl From<validator::ValidationErrors> for S3ParquetError {
    fn from(err: validator::ValidationErrors) -> Self {
        S3ParquetError::validation_with_source("Configuration validation failed", err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = S3ParquetError::configuration("test error");
        assert!(matches!(error, S3ParquetError::Configuration { .. }));
        assert!(!error.is_retryable());
        assert!(!error.is_transient());
        assert!(error.is_permanent());
    }

    #[test]
    fn test_error_context() {
        let error = S3ParquetError::s3_connection("test error");
        let context = error.context();
        assert_eq!(context.error_type, "s3_connection");
        assert!(context.retryable);
        assert!(context.transient);
        assert!(!context.permanent);
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let s3_parquet_error: S3ParquetError = io_error.into();
        assert!(matches!(s3_parquet_error, S3ParquetError::Storage { .. }));
    }
}
