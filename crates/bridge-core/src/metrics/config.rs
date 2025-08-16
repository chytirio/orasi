//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Metrics configuration for the OpenTelemetry Data Lake Bridge
//!
//! This module provides configuration structures for metrics collection.

use std::time::Duration;

/// Metrics configuration
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,

    /// Metrics endpoint
    pub endpoint: String,

    /// Metrics collection interval
    pub collection_interval: Duration,

    /// Export metrics to Prometheus
    pub export_prometheus: bool,

    /// Prometheus endpoint
    pub prometheus_endpoint: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Enabled for Phase 2
            endpoint: "0.0.0.0:9091".to_string(),
            collection_interval: Duration::from_secs(15),
            export_prometheus: true, // Enabled for Phase 2
            prometheus_endpoint: "0.0.0.0:9091".to_string(),
        }
    }
}
