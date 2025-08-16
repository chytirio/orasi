//! Error types for Orasi Agent

use thiserror::Error;

/// Error type for agent operations
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Cluster coordination error: {0}")]
    Cluster(String),

    #[error("Service discovery error: {0}")]
    ServiceDiscovery(String),

    #[error("Task processing error: {0}")]
    TaskProcessing(String),

    #[error("Ingestion error: {0}")]
    Ingestion(String),

    #[error("Indexing error: {0}")]
    Indexing(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Health check error: {0}")]
    Health(String),

    #[error("Metrics error: {0}")]
    Metrics(String),

    #[error("State management error: {0}")]
    State(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Shutdown error: {0}")]
    Shutdown(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        AgentError::Serialization(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for AgentError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        AgentError::Timeout(err.to_string())
    }
}

impl From<sqlx::Error> for AgentError {
    fn from(err: sqlx::Error) -> Self {
        AgentError::Storage(err.to_string())
    }
}
