//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry receiver traits for the OpenTelemetry Data Lake Bridge
//!
//! This module provides traits and types for telemetry receivers that ingest
//! telemetry data from various sources.

use crate::error::BridgeResult;
use crate::types::TelemetryBatch;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Telemetry receiver trait for ingesting telemetry data
#[async_trait]
pub trait TelemetryReceiver: Send + Sync {
    /// Receive telemetry data
    async fn receive(&self) -> BridgeResult<TelemetryBatch>;

    /// Check if receiver is healthy
    async fn health_check(&self) -> BridgeResult<bool>;

    /// Get receiver statistics
    async fn get_stats(&self) -> BridgeResult<ReceiverStats>;

    /// Shutdown the receiver
    async fn shutdown(&self) -> BridgeResult<()>;
}

/// Receiver statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverStats {
    /// Total records received
    pub total_records: u64,

    /// Records received in last minute
    pub records_per_minute: u64,

    /// Total bytes received
    pub total_bytes: u64,

    /// Bytes received in last minute
    pub bytes_per_minute: u64,

    /// Error count
    pub error_count: u64,

    /// Last receive timestamp
    pub last_receive_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Protocol-specific statistics
    pub protocol_stats: Option<HashMap<String, String>>,
}
