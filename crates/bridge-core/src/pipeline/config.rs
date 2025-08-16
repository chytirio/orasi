//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Pipeline configuration for the OpenTelemetry Data Lake Bridge
//!
//! This module provides configuration structures for the telemetry ingestion
//! pipeline.

use serde::{Deserialize, Serialize};

/// Telemetry ingestion pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Pipeline name
    pub name: String,

    /// Maximum batch size
    pub max_batch_size: usize,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,

    /// Buffer size for telemetry data
    pub buffer_size: usize,

    /// Enable backpressure handling
    pub enable_backpressure: bool,

    /// Backpressure threshold percentage
    pub backpressure_threshold: u8,

    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Enable health checks
    pub enable_health_checks: bool,

    /// Health check interval in milliseconds
    pub health_check_interval_ms: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            name: "default-pipeline".to_string(),
            max_batch_size: 1000,
            flush_interval_ms: 5000,
            buffer_size: 10000,
            enable_backpressure: true,
            backpressure_threshold: 80,
            enable_metrics: true,
            enable_health_checks: true,
            health_check_interval_ms: 30000,
        }
    }
}
