//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Monitoring configuration for the OpenTelemetry Data Lake Bridge
//!
//! This module provides configuration structures for monitoring and observability.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use validator::Validate;

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Metrics endpoint
    #[validate(url)]
    pub metrics_endpoint: String,

    /// Enable health checks
    pub enable_health_checks: bool,

    /// Health check endpoint
    #[validate(url)]
    pub health_endpoint: String,

    /// Enable structured logging
    pub enable_structured_logging: bool,

    /// Log level
    pub log_level: String,

    /// Log format (json, text)
    pub log_format: String,

    /// Log file path
    pub log_file_path: Option<PathBuf>,

    /// Enable distributed tracing
    pub enable_distributed_tracing: bool,

    /// Tracing endpoint
    pub tracing_endpoint: Option<String>,

    /// Enable performance profiling
    pub enable_performance_profiling: bool,

    /// Profiling interval in seconds
    pub profiling_interval_secs: Option<u64>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            metrics_endpoint: "0.0.0.0:9090".to_string(),
            enable_health_checks: true,
            health_endpoint: "0.0.0.0:8080".to_string(),
            enable_structured_logging: true,
            log_level: "info".to_string(),
            log_format: "json".to_string(),
            log_file_path: None,
            enable_distributed_tracing: false,
            tracing_endpoint: None,
            enable_performance_profiling: false,
            profiling_interval_secs: None,
        }
    }
}
