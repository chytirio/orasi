//! Storage error types

use crate::error::SchemaRegistryError;

/// Storage error types
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Schema not found
    #[error("Schema not found: {fingerprint}")]
    SchemaNotFound { fingerprint: String },

    /// Schema already exists
    #[error("Schema already exists: {fingerprint}")]
    SchemaAlreadyExists { fingerprint: String },

    /// Invalid schema data
    #[error("Invalid schema data: {message}")]
    InvalidSchemaData { message: String },

    /// Storage connection error
    #[error("Storage connection error: {message}")]
    ConnectionError { message: String },

    /// Storage query error
    #[error("Storage query error: {message}")]
    QueryError { message: String },

    /// Storage transaction error
    #[error("Storage transaction error: {message}")]
    TransactionError { message: String },

    /// Storage timeout error
    #[error("Storage timeout error: {message}")]
    TimeoutError { message: String },

    /// Storage configuration error
    #[error("Storage configuration error: {message}")]
    ConfigurationError { message: String },
}

impl From<StorageError> for SchemaRegistryError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::SchemaNotFound { fingerprint } => {
                SchemaRegistryError::SchemaNotFound(fingerprint)
            }
            StorageError::SchemaAlreadyExists { fingerprint } => {
                SchemaRegistryError::SchemaAlreadyExists(fingerprint)
            }
            StorageError::InvalidSchemaData { message } => {
                SchemaRegistryError::Validation { message }
            }
            StorageError::ConnectionError { message } => SchemaRegistryError::Storage { message },
            StorageError::QueryError { message } => SchemaRegistryError::Storage { message },
            StorageError::TransactionError { message } => SchemaRegistryError::Storage { message },
            StorageError::TimeoutError { message } => SchemaRegistryError::Timeout { message },
            StorageError::ConfigurationError { message } => SchemaRegistryError::Config { message },
        }
    }
}
