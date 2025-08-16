//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling for the Snowflake connector
//! 
//! This module provides structured error types with context and recovery strategies
//! for the Snowflake connector.

use std::fmt;
use std::error::Error as StdError;
use thiserror::Error;

/// Result type for Snowflake operations
pub type SnowflakeResult<T> = Result<T, SnowflakeError>;

/// Main error type for the Snowflake connector
#[derive(Error, Debug)]
pub enum SnowflakeError {
    /// Configuration errors
    #[error("Snowflake configuration error: {message}")]
    Configuration { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Connection errors
    #[error("Snowflake connection error: {message}")]
    Connection { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Authentication errors
    #[error("Snowflake authentication error: {message}")]
    Authentication { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// SQL execution errors
    #[error("Snowflake SQL execution error: {message}")]
    SqlExecution { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Warehouse errors
    #[error("Snowflake warehouse error: {message}")]
    Warehouse { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Write operation errors
    #[error("Snowflake write error: {message}")]
    Write { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Read operation errors
    #[error("Snowflake read error: {message}")]
    Read { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Storage errors
    #[error("Snowflake storage error: {message}")]
    Storage { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Serialization/deserialization errors
    #[error("Snowflake serialization error: {message}")]
    Serialization { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Validation errors
    #[error("Snowflake validation error: {message}")]
    Validation { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Timeout errors
    #[error("Snowflake timeout error: {message}")]
    Timeout { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Rate limiting errors
    #[error("Snowflake rate limiting error: {message}")]
    RateLimit { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Internal errors
    #[error("Snowflake internal error: {message}")]
    Internal { message: String, source: Option<Box<dyn StdError + Send + Sync>> },

    /// Unknown errors
    #[error("Snowflake unknown error: {message}")]
    Unknown { message: String, source: Option<Box<dyn StdError + Send + Sync>> },
}

impl SnowflakeError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        SnowflakeError::Configuration {
            message: message.into(),
            source: None,
        }
    }

    /// Create a configuration error with source
    pub fn configuration_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Configuration {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a connection error
    pub fn connection(message: impl Into<String>) -> Self {
        SnowflakeError::Connection {
            message: message.into(),
            source: None,
        }
    }

    /// Create a connection error with source
    pub fn connection_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Connection {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        SnowflakeError::Authentication {
            message: message.into(),
            source: None,
        }
    }

    /// Create an authentication error with source
    pub fn authentication_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Authentication {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a SQL execution error
    pub fn sql_execution(message: impl Into<String>) -> Self {
        SnowflakeError::SqlExecution {
            message: message.into(),
            source: None,
        }
    }

    /// Create a SQL execution error with source
    pub fn sql_execution_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::SqlExecution {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a warehouse error
    pub fn warehouse(message: impl Into<String>) -> Self {
        SnowflakeError::Warehouse {
            message: message.into(),
            source: None,
        }
    }

    /// Create a warehouse error with source
    pub fn warehouse_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Warehouse {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a write error
    pub fn write(message: impl Into<String>) -> Self {
        SnowflakeError::Write {
            message: message.into(),
            source: None,
        }
    }

    /// Create a write error with source
    pub fn write_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Write {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a read error
    pub fn read(message: impl Into<String>) -> Self {
        SnowflakeError::Read {
            message: message.into(),
            source: None,
        }
    }

    /// Create a read error with source
    pub fn read_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Read {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a storage error
    pub fn storage(message: impl Into<String>) -> Self {
        SnowflakeError::Storage {
            message: message.into(),
            source: None,
        }
    }

    /// Create a storage error with source
    pub fn storage_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Storage {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        SnowflakeError::Serialization {
            message: message.into(),
            source: None,
        }
    }

    /// Create a serialization error with source
    pub fn serialization_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Serialization {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        SnowflakeError::Validation {
            message: message.into(),
            source: None,
        }
    }

    /// Create a validation error with source
    pub fn validation_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Validation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        SnowflakeError::Timeout {
            message: message.into(),
            source: None,
        }
    }

    /// Create a timeout error with source
    pub fn timeout_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Timeout {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        SnowflakeError::RateLimit {
            message: message.into(),
            source: None,
        }
    }

    /// Create a rate limit error with source
    pub fn rate_limit_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::RateLimit {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        SnowflakeError::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create an internal error with source
    pub fn internal_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        SnowflakeError::Unknown {
            message: message.into(),
            source: None,
        }
    }

    /// Create an unknown error with source
    pub fn unknown_with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        SnowflakeError::Unknown {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SnowflakeError::Connection { .. }
                | SnowflakeError::Timeout { .. }
                | SnowflakeError::RateLimit { .. }
        )
    }

    /// Check if the error is transient
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            SnowflakeError::Connection { .. }
                | SnowflakeError::Timeout { .. }
                | SnowflakeError::RateLimit { .. }
                | SnowflakeError::Warehouse { .. }
        )
    }

    /// Check if the error is permanent
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            SnowflakeError::Configuration { .. }
                | SnowflakeError::Authentication { .. }
                | SnowflakeError::Validation { .. }
                | SnowflakeError::Serialization { .. }
        )
    }

    /// Get error context for logging
    pub fn context(&self) -> SnowflakeErrorContext {
        SnowflakeErrorContext {
            error_type: self.error_type(),
            retryable: self.is_retryable(),
            transient: self.is_transient(),
            permanent: self.is_permanent(),
        }
    }

    /// Get the error type as a string
    pub fn error_type(&self) -> &'static str {
        match self {
            SnowflakeError::Configuration { .. } => "configuration",
            SnowflakeError::Connection { .. } => "connection",
            SnowflakeError::Authentication { .. } => "authentication",
            SnowflakeError::SqlExecution { .. } => "sql_execution",
            SnowflakeError::Warehouse { .. } => "warehouse",
            SnowflakeError::Write { .. } => "write",
            SnowflakeError::Read { .. } => "read",
            SnowflakeError::Storage { .. } => "storage",
            SnowflakeError::Serialization { .. } => "serialization",
            SnowflakeError::Validation { .. } => "validation",
            SnowflakeError::Timeout { .. } => "timeout",
            SnowflakeError::RateLimit { .. } => "rate_limit",
            SnowflakeError::Internal { .. } => "internal",
            SnowflakeError::Unknown { .. } => "unknown",
        }
    }
}

/// Error context for logging and monitoring
#[derive(Debug, Clone)]
pub struct SnowflakeErrorContext {
    pub error_type: &'static str,
    pub retryable: bool,
    pub transient: bool,
    pub permanent: bool,
}

impl fmt::Display for SnowflakeErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SnowflakeErrorContext {{ type: {}, retryable: {}, transient: {}, permanent: {} }}",
            self.error_type, self.retryable, self.transient, self.permanent
        )
    }
}

/// Error conversion traits for common error types
impl From<std::io::Error> for SnowflakeError {
    fn from(err: std::io::Error) -> Self {
        SnowflakeError::storage_with_source("IO error", err)
    }
}

impl From<serde_json::Error> for SnowflakeError {
    fn from(err: serde_json::Error) -> Self {
        SnowflakeError::serialization_with_source("JSON serialization error", err)
    }
}

impl From<url::ParseError> for SnowflakeError {
    fn from(err: url::ParseError) -> Self {
        SnowflakeError::configuration_with_source("URL parse error", err)
    }
}

impl From<tokio::time::error::Elapsed> for SnowflakeError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        SnowflakeError::timeout_with_source("Operation timeout", err)
    }
}

impl From<config::ConfigError> for SnowflakeError {
    fn from(err: config::ConfigError) -> Self {
        SnowflakeError::configuration_with_source("Configuration error", err)
    }
}

impl From<validator::ValidationErrors> for SnowflakeError {
    fn from(err: validator::ValidationErrors) -> Self {
        SnowflakeError::validation_with_source("Configuration validation failed", err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = SnowflakeError::configuration("test error");
        assert!(matches!(error, SnowflakeError::Configuration { .. }));
        assert!(!error.is_retryable());
        assert!(!error.is_transient());
        assert!(error.is_permanent());
    }

    #[test]
    fn test_error_context() {
        let error = SnowflakeError::connection("test error");
        let context = error.context();
        assert_eq!(context.error_type, "connection");
        assert!(context.retryable);
        assert!(context.transient);
        assert!(!context.permanent);
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let snowflake_error: SnowflakeError = io_error.into();
        assert!(matches!(snowflake_error, SnowflakeError::Storage { .. }));
    }
}
