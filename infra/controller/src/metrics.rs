//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0

//! Metrics collection for the Orasi controller

use std::time::Instant;

/// Metrics collection for the controller
#[derive(Clone)]
pub struct Metrics {
    /// Reconciliation duration histogram
    pub reconcile_duration: String,
    /// Total reconciliations counter
    pub reconciliations_total: String,
    /// Reconciliation errors counter
    pub reconciliation_errors_total: String,
}

impl Metrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            reconcile_duration: "doc_controller_reconcile_duration_seconds".to_string(),
            reconciliations_total: "doc_controller_reconciliations_total".to_string(),
            reconciliation_errors_total: "doc_controller_reconciliation_errors_total".to_string(),
        }
    }

    /// Record reconciliation duration
    pub fn record_reconcile_duration(&self, duration: std::time::Duration) {
        metrics::histogram!(
            "doc_controller_reconcile_duration_seconds",
            duration.as_secs_f64()
        );
    }

    /// Increment reconciliation counter
    pub fn increment_reconciliations(&self) {
        metrics::counter!("doc_controller_reconciliations_total", 1);
    }

    /// Increment reconciliation errors counter
    pub fn increment_reconciliation_errors(&self) {
        metrics::counter!("doc_controller_reconciliation_errors_total", 1);
    }

    /// Time a reconciliation operation
    pub fn time_reconciliation<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        self.record_reconcile_duration(duration);
        result
    }

    /// Time an async reconciliation operation
    pub async fn time_reconciliation_async<F, Fut, T>(&self, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        self.record_reconcile_duration(duration);
        result
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
