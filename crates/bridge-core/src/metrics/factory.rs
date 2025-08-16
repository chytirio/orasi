//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Metrics factory for the OpenTelemetry Data Lake Bridge
//!
//! This module provides factory functionality for creating metrics instances.

use super::{BridgeMetrics, MetricsConfig};

/// Metrics factory for creating metrics instances
pub struct MetricsFactory;

impl MetricsFactory {
    /// Create metrics collector with default configuration
    pub fn create_default() -> BridgeMetrics {
        BridgeMetrics::new(MetricsConfig::default())
    }

    /// Create metrics collector with custom configuration
    pub fn create_with_config(config: MetricsConfig) -> BridgeMetrics {
        BridgeMetrics::new(config)
    }

    /// Create disabled metrics collector
    pub fn create_disabled() -> BridgeMetrics {
        let mut config = MetricsConfig::default();
        config.enabled = false;
        BridgeMetrics::new(config)
    }
}
