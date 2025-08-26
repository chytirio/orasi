//! Count window implementation

use async_trait::async_trait;
use bridge_core::{types::TelemetryRecord, BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use super::super::types::{Window, WindowStats, WindowStatus};

/// Count window implementation
pub struct CountWindow {
    id: String,
    start_time: DateTime<Utc>,
    max_count: usize,
    records: Vec<TelemetryRecord>,
    is_active: bool,
}

impl CountWindow {
    /// Create new count window
    pub fn new(id: String, max_count: usize) -> Self {
        Self {
            id,
            start_time: Utc::now(),
            max_count,
            records: Vec::new(),
            is_active: true,
        }
    }
}

#[async_trait]
impl Window for CountWindow {
    fn id(&self) -> &str {
        &self.id
    }

    fn start_time(&self) -> DateTime<Utc> {
        self.start_time
    }

    fn end_time(&self) -> DateTime<Utc> {
        // Count windows don't have a fixed end time
        self.start_time + chrono::Duration::hours(1) // Placeholder
    }

    fn is_active(&self) -> bool {
        self.is_active && self.records.len() < self.max_count
    }

    async fn add_record(&mut self, record: TelemetryRecord) -> BridgeResult<()> {
        if !self.is_active() {
            return Err(bridge_core::BridgeError::stream(
                "Cannot add record to inactive window".to_string(),
            ));
        }

        self.records.push(record);

        // Auto-close when max count is reached
        if self.records.len() >= self.max_count {
            self.is_active = false;
        }

        Ok(())
    }

    async fn get_data(&self) -> BridgeResult<TelemetryBatch> {
        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "count_window".to_string(),
            size: self.records.len(),
            records: self.records.clone(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("window_id".to_string(), self.id.clone());
                metadata.insert("max_count".to_string(), self.max_count.to_string());
                metadata.insert("current_count".to_string(), self.records.len().to_string());
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
        } else {
            WindowStatus::Active
        };

        Ok(WindowStats {
            window_id: self.id.clone(),
            start_time: self.start_time,
            end_time: self.end_time(),
            record_count: self.records.len() as u64,
            size_bytes: self.records.len() as u64 * 100, // Approximate
            status,
        })
    }
}
