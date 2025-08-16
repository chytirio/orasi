//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry processor traits for the OpenTelemetry Data Lake Bridge
//!
//! This module provides traits and types for telemetry processors that transform
//! and process telemetry data.

use crate::error::BridgeResult;
use crate::types::{ProcessedBatch, TelemetryBatch};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Telemetry processor trait for processing telemetry data
#[async_trait]
pub trait TelemetryProcessor: Send + Sync {
    /// Process telemetry batch
    async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch>;

    /// Get processor name
    fn name(&self) -> &str;

    /// Get processor version
    fn version(&self) -> &str;

    /// Check if processor is healthy
    async fn health_check(&self) -> BridgeResult<bool>;

    /// Get processor statistics
    async fn get_stats(&self) -> BridgeResult<ProcessorStats>;

    /// Shutdown the processor
    async fn shutdown(&self) -> BridgeResult<()>;
}

/// Processor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorStats {
    /// Total batches processed
    pub total_batches: u64,

    /// Total records processed
    pub total_records: u64,

    /// Batches processed in last minute
    pub batches_per_minute: u64,

    /// Records processed in last minute
    pub records_per_minute: u64,

    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last processing timestamp
    pub last_process_time: Option<chrono::DateTime<chrono::Utc>>,
}
