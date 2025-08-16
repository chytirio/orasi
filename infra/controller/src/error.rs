//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0

//! Error types for the Orasi controller

use thiserror::Error;

/// Controller error types
#[derive(Error, Debug)]
pub enum ControllerError {
    /// General error
    #[error("General error: {0}")]
    GeneralError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// HTTP server error
    #[error("HTTP server error: {0}")]
    HttpError(#[from] hyper::Error),

    /// Metrics error
    #[error("Metrics error: {0}")]
    MetricsError(String),

    /// Telemetry error
    #[error("Telemetry error: {0}")]
    TelemetryError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Resource already exists
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// Invalid resource state
    #[error("Invalid resource state: {0}")]
    InvalidState(String),

    /// Reconciliation error
    #[error("Reconciliation error: {0}")]
    ReconciliationError(String),
}
