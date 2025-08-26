//! Time window implementation

use async_trait::async_trait;
use bridge_core::{types::TelemetryRecord, BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use super::super::types::{Window, WindowStats, WindowStatus};

/// Time window implementation
pub struct TimeWindow {
    id: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    records: Vec<TelemetryRecord>,
    is_active: bool,
}

impl TimeWindow {
    /// Create new time window
    pub fn new(id: String, start_time: DateTime<Utc>, duration: chrono::Duration) -> Self {
        let end_time = start_time + duration;

        Self {
            id,
            start_time,
            end_time,
            records: Vec::new(),
            is_active: true,
        }
    }
}

#[async_trait]
impl Window for TimeWindow {
    fn id(&self) -> &str {
        &self.id
    }

    fn start_time(&self) -> DateTime<Utc> {
        self.start_time
    }

    fn end_time(&self) -> DateTime<Utc> {
        self.end_time
    }

    fn is_active(&self) -> bool {
        self.is_active && Utc::now() < self.end_time
    }

    async fn add_record(&mut self, record: TelemetryRecord) -> BridgeResult<()> {
        if !self.is_active() {
            return Err(bridge_core::BridgeError::stream(
                "Cannot add record to inactive window".to_string(),
            ));
        }

        self.records.push(record);
        Ok(())
    }

    async fn get_data(&self) -> BridgeResult<TelemetryBatch> {
        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "time_window".to_string(),
            size: self.records.len(),
            records: self.records.clone(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("window_id".to_string(), self.id.clone());
                metadata.insert("window_start".to_string(), self.start_time.to_rfc3339());
                metadata.insert("window_end".to_string(), self.end_time.to_rfc3339());
                metadata
            },
        })
    }

    async fn close(&mut self) -> BridgeResult<()> {
        self.is_active = false;
        Ok(())
    }

    async fn get_stats(&self) -> BridgeResult<WindowStats> {
        let status = if !self.is_active {
            WindowStatus::Closed
        } else if Utc::now() >= self.end_time {
            WindowStatus::Expired
        } else {
            WindowStatus::Active
        };

        Ok(WindowStats {
            window_id: self.id.clone(),
            start_time: self.start_time,
            end_time: self.end_time,
            record_count: self.records.len() as u64,
            size_bytes: self.records.len() as u64 * 100, // Approximate
            status,
        })
    }
}
