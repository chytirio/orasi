//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0

//! Main controller logic for the Orasi controller

use crate::{Document, ControllerResult, Metrics};
use std::sync::Arc;
use tracing::{info, warn};

/// Controller for managing Document resources
pub struct Controller {
    /// Metrics collection
    metrics: Arc<Metrics>,
}

impl Controller {
    /// Create a new controller instance
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Self { metrics }
    }

    /// Run the controller
    pub async fn run(self) -> ControllerResult<()> {
        info!("Starting Orasi controller");

        // Simulate controller running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            info!("Controller heartbeat");
        }
    }

    /// Reconcile a document
    pub async fn reconcile_document(&self, doc: &Document) -> ControllerResult<()> {
        let name = doc.metadata.title.as_deref().unwrap_or("unknown");
        info!("Reconciling Document: {}", name);

        // Time the reconciliation
        let result = self
            .metrics
            .time_reconciliation_async(|| async {
                self.process_document(doc).await
            })
            .await;

        match result {
            Ok(_) => {
                self.metrics.increment_reconciliations();
                info!("Successfully reconciled Document: {}", name);
                Ok(())
            }
            Err(e) => {
                self.metrics.increment_reconciliation_errors();
                warn!("Failed to reconcile Document {}: {:?}", name, e);
                Err(e)
            }
        }
    }

    /// Process a document
    async fn process_document(&self, doc: &Document) -> ControllerResult<()> {
        let spec = &doc.spec;

        // Update status based on hidden field
        if let Some(hidden) = spec.hidden {
            info!("Document hidden status: {}", hidden);
            
            // Create event based on hidden status
            let event_type = if hidden { "DocumentHidden" } else { "DocumentVisible" };
            info!("Event: {} - Document is {}", event_type, if hidden { "hidden" } else { "visible" });
        }

        // Simulate processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(())
    }
}
