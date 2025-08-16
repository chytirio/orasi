//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Lakehouse connector traits for the OpenTelemetry Data Lake Bridge
//!
//! This module provides traits and types for lakehouse connectors that manage
//! connections to various data lakehouse systems.

use crate::error::BridgeResult;
use crate::types::{
    LogsBatch, LogsQuery, LogsResult, MetricsBatch, MetricsQuery, MetricsResult, TelemetryBatch,
    TracesBatch, TracesQuery, TracesResult, WriteResult,
};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Lakehouse connector trait for connecting to data lakehouses
#[async_trait]
pub trait LakehouseConnector: Send + Sync {
    /// Configuration type for the connector
    type Config: DeserializeOwned + Send + Sync;

    /// Write handle type
    type WriteHandle: LakehouseWriter;

    /// Read handle type
    type ReadHandle: LakehouseReader;

    /// Connect to the lakehouse
    async fn connect(config: Self::Config) -> BridgeResult<Self>
    where
        Self: Sized;

    /// Get a write handle
    async fn writer(&self) -> BridgeResult<Self::WriteHandle>;

    /// Get a read handle
    async fn reader(&self) -> BridgeResult<Self::ReadHandle>;

    /// Get connector name
    fn name(&self) -> &str;

    /// Get connector version
    fn version(&self) -> &str;

    /// Check if connector is healthy
    async fn health_check(&self) -> BridgeResult<bool>;

    /// Get connector statistics
    async fn get_stats(&self) -> BridgeResult<ConnectorStats>;

    /// Shutdown the connector
    async fn shutdown(&self) -> BridgeResult<()>;
}

/// Connector statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorStats {
    /// Total connections
    pub total_connections: u64,

    /// Active connections
    pub active_connections: u64,

    /// Total write operations
    pub total_writes: u64,

    /// Total read operations
    pub total_reads: u64,

    /// Average write time in milliseconds
    pub avg_write_time_ms: f64,

    /// Average read time in milliseconds
    pub avg_read_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last operation timestamp
    pub last_operation_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Lakehouse writer trait for writing data to lakehouses
#[async_trait]
pub trait LakehouseWriter: Send + Sync {
    /// Write metrics batch
    async fn write_metrics(&self, metrics: MetricsBatch) -> BridgeResult<WriteResult>;

    /// Write traces batch
    async fn write_traces(&self, traces: TracesBatch) -> BridgeResult<WriteResult>;

    /// Write logs batch
    async fn write_logs(&self, logs: LogsBatch) -> BridgeResult<WriteResult>;

    /// Write any telemetry batch
    async fn write_batch(&self, batch: TelemetryBatch) -> BridgeResult<WriteResult>;

    /// Flush pending writes
    async fn flush(&self) -> BridgeResult<()>;

    /// Get writer statistics
    async fn get_stats(&self) -> BridgeResult<WriterStats>;

    /// Close the writer
    async fn close(&self) -> BridgeResult<()>;
}

/// Writer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriterStats {
    /// Total write operations
    pub total_writes: u64,

    /// Total records written
    pub total_records: u64,

    /// Writes in last minute
    pub writes_per_minute: u64,

    /// Records written in last minute
    pub records_per_minute: u64,

    /// Average write time in milliseconds
    pub avg_write_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last write timestamp
    pub last_write_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Lakehouse reader trait for reading data from lakehouses
#[async_trait]
pub trait LakehouseReader: Send + Sync {
    /// Query metrics
    async fn query_metrics(&self, query: MetricsQuery) -> BridgeResult<MetricsResult>;

    /// Query traces
    async fn query_traces(&self, query: TracesQuery) -> BridgeResult<TracesResult>;

    /// Query logs
    async fn query_logs(&self, query: LogsQuery) -> BridgeResult<LogsResult>;

    /// Execute custom query
    async fn execute_query(&self, query: String) -> BridgeResult<serde_json::Value>;

    /// Get reader statistics
    async fn get_stats(&self) -> BridgeResult<ReaderStats>;

    /// Close the reader
    async fn close(&self) -> BridgeResult<()>;
}

/// Reader statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderStats {
    /// Total read operations
    pub total_reads: u64,

    /// Total records read
    pub total_records: u64,

    /// Reads in last minute
    pub reads_per_minute: u64,

    /// Records read in last minute
    pub records_per_minute: u64,

    /// Average read time in milliseconds
    pub avg_read_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last read timestamp
    pub last_read_time: Option<chrono::DateTime<chrono::Utc>>,
}
