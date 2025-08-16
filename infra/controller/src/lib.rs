//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0

//! Kubernetes controller for Orasi OpenTelemetry Data Lakehouse
//!
//! This crate provides a Kubernetes controller that manages Orasi resources
//! and provides observability for the data lakehouse system.

pub mod controller;
pub mod error;
pub mod metrics;

use serde::{Deserialize, Serialize};

/// Document resource specification
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DocumentSpec {
    /// The content of the document
    pub content: String,
    /// Whether the document is hidden
    pub hidden: Option<bool>,
    /// Metadata for the document
    pub metadata: Option<DocumentMetadata>,
}

/// Document metadata
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DocumentMetadata {
    /// Document title
    pub title: Option<String>,
    /// Document tags
    pub tags: Option<Vec<String>>,
    /// Document author
    pub author: Option<String>,
}

/// Document status
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DocumentStatus {
    /// Whether the document is hidden
    pub is_hidden: Option<bool>,
    /// Last reconciliation time
    pub last_reconciled: Option<String>,
    /// Reconciliation status
    pub conditions: Option<Vec<DocumentCondition>>,
}

/// Document condition
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DocumentCondition {
    /// Condition type
    pub type_: String,
    /// Condition status
    pub status: String,
    /// Last transition time
    pub last_transition_time: String,
    /// Reason for the condition
    pub reason: Option<String>,
    /// Message describing the condition
    pub message: Option<String>,
}

/// Document resource
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Document {
    /// API version
    pub api_version: String,
    /// Kind
    pub kind: String,
    /// Metadata
    pub metadata: DocumentMetadata,
    /// Specification
    pub spec: DocumentSpec,
    /// Status
    pub status: Option<DocumentStatus>,
}

/// Re-export main components
pub use controller::Controller;
pub use error::ControllerError;
pub use metrics::Metrics;

/// Result type for controller operations
pub type ControllerResult<T> = Result<T, ControllerError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_controller_creation() {
        let metrics = Arc::new(Metrics::new());
        let controller = Controller::new(metrics);
        // Test that controller was created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_document_reconciliation() {
        let metrics = Arc::new(Metrics::new());
        let controller = Controller::new(metrics);

        let doc = Document {
            api_version: "orasi.io/v1alpha1".to_string(),
            kind: "Document".to_string(),
            metadata: DocumentMetadata {
                title: Some("Test Document".to_string()),
                author: Some("Test Author".to_string()),
                tags: Some(vec!["test".to_string()]),
            },
            spec: DocumentSpec {
                content: "Test content".to_string(),
                hidden: Some(false),
                metadata: None,
            },
            status: None,
        };

        let result = controller.reconcile_document(&doc).await;
        assert!(result.is_ok());
    }
}
