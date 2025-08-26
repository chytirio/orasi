//! Window trait and types

use async_trait::async_trait;
use bridge_core::{types::TelemetryRecord, BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Window trait for streaming data windows
#[async_trait]
pub trait Window: Send + Sync {
    /// Get window ID
    fn id(&self) -> &str;

    /// Get window start time
    fn start_time(&self) -> DateTime<Utc>;

    /// Get window end time
    fn end_time(&self) -> DateTime<Utc>;

    /// Check if window is active
    fn is_active(&self) -> bool;

    /// Add record to window
    async fn add_record(&mut self, record: TelemetryRecord) -> BridgeResult<()>;

    /// Get window data
    async fn get_data(&self) -> BridgeResult<TelemetryBatch>;

    /// Close window
    async fn close(&mut self) -> BridgeResult<()>;

    /// Get window statistics
    async fn get_stats(&self) -> BridgeResult<WindowStats>;
}

/// Window statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowStats {
    /// Window ID
    pub window_id: String,

    /// Window start time
    pub start_time: DateTime<Utc>,

    /// Window end time
    pub end_time: DateTime<Utc>,

    /// Number of records in window
    pub record_count: u64,

    /// Window size in bytes
    pub size_bytes: u64,

    /// Window status
    pub status: WindowStatus,
}

/// Window status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowStatus {
    Active,
    Closed,
    Expired,
}
