//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error conversions for the OpenTelemetry Data Lake Bridge
//!
//! This module provides error conversion implementations for common error types.

use super::types::BridgeError;

/// Error conversion traits for common error types
impl From<std::io::Error> for BridgeError {
    fn from(err: std::io::Error) -> Self {
        BridgeError::network_with_source("IO error", err)
    }
}

impl From<serde_json::Error> for BridgeError {
    fn from(err: serde_json::Error) -> Self {
        BridgeError::serialization_with_source("JSON serialization error", err)
    }
}

impl From<tonic::transport::Error> for BridgeError {
    fn from(err: tonic::transport::Error) -> Self {
        BridgeError::network_with_source("gRPC transport error", err)
    }
}

impl From<tonic::Status> for BridgeError {
    fn from(err: tonic::Status) -> Self {
        BridgeError::network_with_source("gRPC status error", err)
    }
}

impl From<hyper::Error> for BridgeError {
    fn from(err: hyper::Error) -> Self {
        BridgeError::network_with_source("HTTP error", err)
    }
}

impl From<tokio::time::error::Elapsed> for BridgeError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        BridgeError::timeout_with_source("Operation timeout", err)
    }
}

impl From<config::ConfigError> for BridgeError {
    fn from(err: config::ConfigError) -> Self {
        BridgeError::configuration_with_source("Configuration error", err)
    }
}

impl From<validator::ValidationErrors> for BridgeError {
    fn from(err: validator::ValidationErrors) -> Self {
        BridgeError::validation_with_source("Validation error", err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let bridge_err: BridgeError = io_err.into();
        assert!(matches!(bridge_err, BridgeError::Network { .. }));
    }
}
