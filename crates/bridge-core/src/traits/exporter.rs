//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Lakehouse exporter traits for the OpenTelemetry Data Lake Bridge
//!
//! This module provides traits and types for lakehouse exporters that write
//! telemetry data to various data lakehouse systems.

use crate::error::BridgeResult;
use crate::types::{ExportResult, ProcessedBatch};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Lakehouse exporter trait for exporting data to lakehouses
#[async_trait]
pub trait LakehouseExporter: Send + Sync {
    /// Export processed batch to lakehouse
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult>;

    /// Get exporter name
    fn name(&self) -> &str;

    /// Get exporter version
    fn version(&self) -> &str;

    /// Check if exporter is healthy
    async fn health_check(&self) -> BridgeResult<bool>;

    /// Get exporter statistics
    async fn get_stats(&self) -> BridgeResult<ExporterStats>;

    /// Shutdown the exporter
    async fn shutdown(&self) -> BridgeResult<()>;
}

/// Exporter statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExporterStats {
    /// Total batches exported
    pub total_batches: u64,

    /// Total records exported
    pub total_records: u64,

    /// Batches exported in last minute
    pub batches_per_minute: u64,

    /// Records exported in last minute
    pub records_per_minute: u64,

    /// Average export time in milliseconds
    pub avg_export_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last export timestamp
    pub last_export_time: Option<chrono::DateTime<chrono::Utc>>,
}
