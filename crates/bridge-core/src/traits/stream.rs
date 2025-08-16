//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Stream processing traits for the OpenTelemetry Data Lake Bridge
//!
//! This module provides traits and types for stream processing components.

use crate::error::BridgeResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stream source trait for streaming data sources
#[async_trait]
pub trait StreamSource: Send + Sync {
    /// Get stream identifier
    fn stream_id(&self) -> &str;

    /// Get stream metadata
    fn metadata(&self) -> HashMap<String, String>;

    /// Check if source is healthy
    async fn health_check(&self) -> BridgeResult<bool>;

    /// Get source statistics
    async fn get_stats(&self) -> BridgeResult<StreamSourceStats>;

    /// Shutdown the source
    async fn shutdown(&self) -> BridgeResult<()>;
}

/// Stream source statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSourceStats {
    /// Total records produced
    pub total_records: u64,

    /// Records produced in last minute
    pub records_per_minute: u64,

    /// Total bytes produced
    pub total_bytes: u64,

    /// Bytes produced in last minute
    pub bytes_per_minute: u64,

    /// Error count
    pub error_count: u64,

    /// Last produce timestamp
    pub last_produce_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Stream processor trait for processing streaming data
#[async_trait]
pub trait StreamProcessor: Send + Sync {
    /// Process input stream
    async fn process_stream(&self, input: DataStream) -> BridgeResult<DataStream>;

    /// Get processor name
    fn name(&self) -> &str;

    /// Get processor version
    fn version(&self) -> &str;

    /// Check if processor is healthy
    async fn health_check(&self) -> BridgeResult<bool>;

    /// Get processor statistics
    async fn get_stats(&self) -> BridgeResult<StreamProcessorStats>;

    /// Shutdown the processor
    async fn shutdown(&self) -> BridgeResult<()>;
}

/// Stream processor statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamProcessorStats {
    /// Total records processed
    pub total_records: u64,

    /// Records processed in last minute
    pub records_per_minute: u64,

    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last processing timestamp
    pub last_process_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Stream sink trait for streaming data sinks
#[async_trait]
pub trait StreamSink: Send + Sync {
    /// Get sink identifier
    fn sink_id(&self) -> &str;

    /// Get sink metadata
    fn metadata(&self) -> HashMap<String, String>;

    /// Check if sink is healthy
    async fn health_check(&self) -> BridgeResult<bool>;

    /// Get sink statistics
    async fn get_stats(&self) -> BridgeResult<StreamSinkStats>;

    /// Shutdown the sink
    async fn shutdown(&self) -> BridgeResult<()>;
}

/// Stream sink statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSinkStats {
    /// Total records consumed
    pub total_records: u64,

    /// Records consumed in last minute
    pub records_per_minute: u64,

    /// Total bytes consumed
    pub total_bytes: u64,

    /// Bytes consumed in last minute
    pub bytes_per_minute: u64,

    /// Error count
    pub error_count: u64,

    /// Last consume timestamp
    pub last_consume_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Data stream for streaming operations
#[derive(Debug, Clone)]
pub struct DataStream {
    /// Stream identifier
    pub stream_id: String,

    /// Stream data
    pub data: Vec<u8>,

    /// Stream metadata
    pub metadata: HashMap<String, String>,

    /// Stream timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
