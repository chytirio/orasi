//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data windowing for the bridge
//!
//! This module provides windowing functionality for streaming data
//! including time windows, count windows, and session windows.

use async_trait::async_trait;
use bridge_core::{types::TelemetryRecord, BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

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

/// Window manager for managing multiple windows
pub struct WindowManager {
    windows: HashMap<String, Box<dyn Window>>,
}

impl WindowManager {
    /// Create new window manager
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    /// Add window
    pub fn add_window(&mut self, window: Box<dyn Window>) {
        self.windows.insert(window.id().to_string(), window);
    }

    /// Remove window
    pub fn remove_window(&mut self, window_id: &str) -> Option<Box<dyn Window>> {
        self.windows.remove(window_id)
    }

    /// Get window
    pub fn get_window(&self, window_id: &str) -> Option<&dyn Window> {
        self.windows.get(window_id).map(|w| w.as_ref())
    }

    /// Get all window IDs
    pub fn get_window_ids(&self) -> Vec<String> {
        self.windows.keys().cloned().collect()
    }

    /// Get active windows
    pub fn get_active_windows(&self) -> Vec<&dyn Window> {
        self.windows
            .values()
            .filter(|w| w.is_active())
            .map(|w| w.as_ref())
            .collect()
    }

    /// Clean up expired windows
    pub async fn cleanup_expired_windows(&mut self) -> BridgeResult<()> {
        let expired_ids: Vec<String> = self
            .windows
            .iter()
            .filter(|(_, window)| !window.is_active())
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired_ids {
            if let Some(mut window) = self.windows.remove(&id) {
                window.close().await?;
                info!("Closed expired window: {}", id);
            }
        }

        Ok(())
    }
}
