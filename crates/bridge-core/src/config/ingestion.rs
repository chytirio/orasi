//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Ingestion configuration for the OpenTelemetry Data Lake Bridge
//!
//! This module provides configuration structures for telemetry ingestion.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use validator::Validate;

use super::AuthenticationConfig;

/// Ingestion configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IngestionConfig {
    /// OTLP endpoint for receiving telemetry data
    #[validate(url)]
    pub otlp_endpoint: String,

    /// HTTP endpoint for receiving telemetry data
    #[validate(url)]
    pub http_endpoint: Option<String>,

    /// Batch size for processing telemetry data
    #[validate(range(min = 1, max = 10000))]
    pub batch_size: usize,

    /// Flush interval in milliseconds
    #[validate(range(min = 100, max = 60000))]
    pub flush_interval_ms: u64,

    /// Buffer size for telemetry data
    #[validate(range(min = 1000, max = 100000))]
    pub buffer_size: usize,

    /// Compression level for data storage
    #[validate(range(min = 0, max = 9))]
    pub compression_level: u32,

    /// Enable data persistence for reliability
    pub enable_persistence: bool,

    /// Persistence directory path
    pub persistence_path: Option<PathBuf>,

    /// Maximum persistence file size in bytes
    pub max_persistence_size: Option<u64>,

    /// Enable backpressure handling
    pub enable_backpressure: bool,

    /// Backpressure threshold percentage
    #[validate(range(min = 50, max = 95))]
    pub backpressure_threshold: u8,

    /// Authentication configuration for ingestion
    pub authentication: Option<AuthenticationConfig>,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: "0.0.0.0:4317".to_string(),
            http_endpoint: Some("0.0.0.0:4318".to_string()),
            batch_size: 1000,
            flush_interval_ms: 5000,
            buffer_size: 10000,
            compression_level: 6,
            enable_persistence: false,
            persistence_path: None,
            max_persistence_size: None,
            enable_backpressure: true,
            backpressure_threshold: 80,
            authentication: None,
        }
    }
}
