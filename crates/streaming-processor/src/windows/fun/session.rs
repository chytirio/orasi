//! Session window implementation

use async_trait::async_trait;
use bridge_core::{types::TelemetryRecord, BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use super::super::types::{Window, WindowStats, WindowStatus};

/// Session window implementation
pub struct SessionWindow {
    id: String,
    start_time: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    session_timeout: chrono::Duration,
    records: Vec<TelemetryRecord>,
    is_active: bool,
}

impl SessionWindow {
    /// Create new session window
    pub fn new(id: String, session_timeout: chrono::Duration) -> Self {
        let now = Utc::now();

        Self {
            id,
            start_time: now,
            last_activity: now,
            session_timeout,
            records: Vec::new(),
            is_active: true,
        }
    }
}

#[async_trait]
impl Window for SessionWindow {
    fn id(&self) -> &str {
        &self.id
    }

    fn start_time(&self) -> DateTime<Utc> {
        self.start_time
    }

    fn end_time(&self) -> DateTime<Utc> {
        self.last_activity + self.session_timeout
    }

    fn is_active(&self) -> bool {
        self.is_active && Utc::now() < self.end_time()
    }

    async fn add_record(&mut self, record: TelemetryRecord) -> BridgeResult<()> {
        if !self.is_active() {
            return Err(bridge_core::BridgeError::stream(
                "Cannot add record to inactive window".to_string(),
            ));
        }

        self.last_activity = Utc::now();
        self.records.push(record);
        Ok(())
    }

    async fn get_data(&self) -> BridgeResult<TelemetryBatch> {
        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "session_window".to_string(),
            size: self.records.len(),
            records: self.records.clone(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("window_id".to_string(), self.id.clone());
                metadata.insert("session_start".to_string(), self.start_time.to_rfc3339());
                metadata.insert("last_activity".to_string(), self.last_activity.to_rfc3339());
                metadata.insert(
                    "session_timeout_secs".to_string(),
                    self.session_timeout.num_seconds().to_string(),
                );
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
        } else if Utc::now() >= self.end_time() {
            WindowStatus::Expired
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
