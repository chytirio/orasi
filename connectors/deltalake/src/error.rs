//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling for Delta Lake connector
//! 
//! This module provides error types and handling for Delta Lake operations.
//! 
//! NOTE: This is a simplified implementation for Phase 2 - advanced error
//! handling will be added in Phase 3.

use thiserror::Error;
use std::fmt;

/// Delta Lake result type
pub type DeltaLakeResult<T> = Result<T, DeltaLakeError>;

/// Delta Lake error types
#[derive(Error, Debug)]
pub enum DeltaLakeError {
    /// Configuration error
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Connection error
    #[error("Connection error: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Write error
    #[error("Write error: {message}")]
    Write {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Read error
    #[error("Read error: {message}")]
    Read {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Schema error
    #[error("Schema error: {message}")]
    Schema {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Transaction error
    #[error("Transaction error: {message}")]
    Transaction {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Validation error
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Internal error
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl DeltaLakeError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
            source: None,
        }
    }

    /// Create a configuration error with source
    pub fn configuration_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Configuration {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a connection error
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            source: None,
        }
    }

    /// Create a connection error with source
    pub fn connection_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Connection {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a write error
    pub fn write(message: impl Into<String>) -> Self {
        Self::Write {
            message: message.into(),
            source: None,
        }
    }

    /// Create a write error with source
    pub fn write_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Write {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a read error
    pub fn read(message: impl Into<String>) -> Self {
        Self::Read {
            message: message.into(),
            source: None,
        }
    }

    /// Create a read error with source
    pub fn read_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Read {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a schema error
    pub fn schema(message: impl Into<String>) -> Self {
        Self::Schema {
            message: message.into(),
            source: None,
        }
    }

    /// Create a schema error with source
    pub fn schema_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Schema {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a transaction error
    pub fn transaction(message: impl Into<String>) -> Self {
        Self::Transaction {
            message: message.into(),
            source: None,
        }
    }

    /// Create a transaction error with source
    pub fn transaction_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Transaction {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            source: None,
        }
    }

    /// Create a validation error with source
    pub fn validation_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Validation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create an internal error with source
    pub fn internal_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

impl fmt::Display for DeltaLakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeltaLakeError::Configuration { message, source } => {
                write!(f, "Configuration error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (caused by: {})", source)?;
                }
                Ok(())
            }
            DeltaLakeError::Connection { message, source } => {
                write!(f, "Connection error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (caused by: {})", source)?;
                }
                Ok(())
            }
            DeltaLakeError::Write { message, source } => {
                write!(f, "Write error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (caused by: {})", source)?;
                }
                Ok(())
            }
            DeltaLakeError::Read { message, source } => {
                write!(f, "Read error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (caused by: {})", source)?;
                }
                Ok(())
            }
            DeltaLakeError::Schema { message, source } => {
                write!(f, "Schema error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (caused by: {})", source)?;
                }
                Ok(())
            }
            DeltaLakeError::Transaction { message, source } => {
                write!(f, "Transaction error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (caused by: {})", source)?;
                }
                Ok(())
            }
            DeltaLakeError::Validation { message, source } => {
                write!(f, "Validation error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (caused by: {})", source)?;
                }
                Ok(())
            }
            DeltaLakeError::Internal { message, source } => {
                write!(f, "Internal error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (caused by: {})", source)?;
                }
                Ok(())
            }
        }
    }
}

// Convert from std::io::Error
impl From<std::io::Error> for DeltaLakeError {
    fn from(err: std::io::Error) -> Self {
        Self::internal_with_source("IO error", err)
    }
}

// Convert from serde_json::Error
impl From<serde_json::Error> for DeltaLakeError {
    fn from(err: serde_json::Error) -> Self {
        Self::validation_with_source("JSON serialization error", err)
    }
}

// Convert from toml::de::Error
impl From<toml::de::Error> for DeltaLakeError {
    fn from(err: toml::de::Error) -> Self {
        Self::configuration_with_source("TOML parsing error", err)
    }
}

// Convert from String
impl From<String> for DeltaLakeError {
    fn from(message: String) -> Self {
        Self::internal(message)
    }
}

// Convert from &str
impl From<&str> for DeltaLakeError {
    fn from(message: &str) -> Self {
        Self::internal(message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = DeltaLakeError::configuration("Test config error");
        assert!(matches!(error, DeltaLakeError::Configuration { .. }));
        
        let error = DeltaLakeError::connection("Test connection error");
        assert!(matches!(error, DeltaLakeError::Connection { .. }));
        
        let error = DeltaLakeError::write("Test write error");
        assert!(matches!(error, DeltaLakeError::Write { .. }));
        
        let error = DeltaLakeError::read("Test read error");
        assert!(matches!(error, DeltaLakeError::Read { .. }));
    }

    #[test]
    fn test_error_with_source() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error = DeltaLakeError::configuration_with_source("Config error", io_error);
        assert!(matches!(error, DeltaLakeError::Configuration { .. }));
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error: DeltaLakeError = io_error.into();
        assert!(matches!(error, DeltaLakeError::Internal { .. }));
        
        let error: DeltaLakeError = "Test error".into();
        assert!(matches!(error, DeltaLakeError::Internal { .. }));
    }

    #[test]
    fn test_error_display() {
        let error = DeltaLakeError::configuration("Test error");
        let display = format!("{}", error);
        assert!(display.contains("Configuration error: Test error"));
    }
}
