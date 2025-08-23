//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Windowing for streaming queries
//!
//! This module provides windowing capabilities for streaming queries,
//! including time-based, count-based, and session windows.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Window type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowType {
    /// Time-based window
    Time,

    /// Count-based window
    Count,

    /// Session window
    Session,

    /// Sliding window
    Sliding,

    /// Tumbling window
    Tumbling,
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window type
    pub window_type: WindowType,

    /// Window size in milliseconds (for time windows) or count (for count windows)
    pub window_size: u64,

    /// Window slide in milliseconds (for sliding windows)
    pub window_slide: Option<u64>,

    /// Session timeout in milliseconds (for session windows)
    pub session_timeout: Option<u64>,

    /// Enable window watermarking
    pub enable_watermarking: bool,

    /// Watermark delay in milliseconds
    pub watermark_delay: Option<u64>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    /// Window ID
    pub id: Uuid,

    /// Window start time
    pub start_time: DateTime<Utc>,

    /// Window end time
    pub end_time: DateTime<Utc>,

    /// Window type
    pub window_type: WindowType,

    /// Window size
    pub window_size: u64,

    /// Number of records in window
    pub record_count: u64,

    /// Window metadata
    pub metadata: HashMap<String, String>,
}

/// Time window implementation
pub struct TimeWindow {
    id: Uuid,
    config: WindowConfig,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    record_count: u64,
    metadata: HashMap<String, String>,
}

impl TimeWindow {
    /// Create a new time window
    pub fn new(config: WindowConfig) -> Self {
        let now = Utc::now();
        let end_time = now + chrono::Duration::milliseconds(config.window_size as i64);

        Self {
            id: Uuid::new_v4(),
            config,
            start_time: now,
            end_time,
            record_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// Check if a timestamp is within the window
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start_time && timestamp < self.end_time
    }

    /// Add a record to the window
    pub fn add_record(&mut self) {
        self.record_count += 1;
    }

    /// Check if the window is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.end_time
    }

    /// Get window info
    pub fn get_info(&self) -> WindowInfo {
        WindowInfo {
            id: self.id,
            start_time: self.start_time,
            end_time: self.end_time,
            window_type: self.config.window_type.clone(),
            window_size: self.config.window_size,
            record_count: self.record_count,
            metadata: self.metadata.clone(),
        }
    }

    /// Advance the window
    pub fn advance(&mut self) {
        let slide = self.config.window_slide.unwrap_or(self.config.window_size);
        self.start_time = self.start_time + chrono::Duration::milliseconds(slide as i64);
        self.end_time = self.end_time + chrono::Duration::milliseconds(slide as i64);
        self.record_count = 0;
        self.id = Uuid::new_v4();
    }
}

/// Window manager trait
#[async_trait]
pub trait WindowManager: Send + Sync {
    /// Create a new window
    async fn create_window(&mut self, config: WindowConfig) -> BridgeResult<Uuid>;

    /// Add a record to a window
    async fn add_record(&mut self, window_id: Uuid, timestamp: DateTime<Utc>) -> BridgeResult<()>;

    /// Get window information
    async fn get_window_info(&self, window_id: Uuid) -> BridgeResult<Option<WindowInfo>>;

    /// Get all active windows
    async fn get_active_windows(&self) -> BridgeResult<Vec<WindowInfo>>;

    /// Remove expired windows
    async fn remove_expired_windows(&mut self) -> BridgeResult<Vec<WindowInfo>>;

    /// Get manager statistics
    async fn get_stats(&self) -> BridgeResult<WindowManagerStats>;
}

/// Window manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowManagerStats {
    /// Total windows created
    pub total_windows: u64,

    /// Active windows
    pub active_windows: u64,

    /// Expired windows
    pub expired_windows: u64,

    /// Total records processed
    pub total_records: u64,

    /// Average records per window
    pub avg_records_per_window: f64,

    /// Last window creation time
    pub last_window_creation: Option<DateTime<Utc>>,

    /// Last record addition time
    pub last_record_addition: Option<DateTime<Utc>>,
}

/// Default window manager implementation
pub struct DefaultWindowManager {
    windows: Arc<RwLock<HashMap<Uuid, TimeWindow>>>,
    stats: Arc<RwLock<WindowManagerStats>>,
}

impl DefaultWindowManager {
    /// Create a new window manager
    pub fn new() -> Self {
        let stats = WindowManagerStats {
            total_windows: 0,
            active_windows: 0,
            expired_windows: 0,
            total_records: 0,
            avg_records_per_window: 0.0,
            last_window_creation: None,
            last_record_addition: None,
        };

        Self {
            windows: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(stats)),
        }
    }
}

#[async_trait]
impl WindowManager for DefaultWindowManager {
    async fn create_window(&mut self, config: WindowConfig) -> BridgeResult<Uuid> {
        let window = TimeWindow::new(config);
        let window_id = window.id;

        {
            let mut windows = self.windows.write().await;
            windows.insert(window_id, window);
        }

        {
            let mut stats = self.stats.write().await;
            stats.total_windows += 1;
            stats.active_windows += 1;
            stats.last_window_creation = Some(Utc::now());
        }

        info!("Created window: {}", window_id);
        Ok(window_id)
    }

    async fn add_record(&mut self, window_id: Uuid, timestamp: DateTime<Utc>) -> BridgeResult<()> {
        {
            let mut windows = self.windows.write().await;
            if let Some(window) = windows.get_mut(&window_id) {
                if window.contains(timestamp) {
                    window.add_record();

                    {
                        let mut stats = self.stats.write().await;
                        stats.total_records += 1;
                        stats.avg_records_per_window =
                            stats.total_records as f64 / stats.total_windows as f64;
                        stats.last_record_addition = Some(Utc::now());
                    }
                }
            }
        }

        Ok(())
    }

    async fn get_window_info(&self, window_id: Uuid) -> BridgeResult<Option<WindowInfo>> {
        let windows = self.windows.read().await;
        Ok(windows.get(&window_id).map(|w| w.get_info()))
    }

    async fn get_active_windows(&self) -> BridgeResult<Vec<WindowInfo>> {
        let windows = self.windows.read().await;
        Ok(windows.values().map(|w| w.get_info()).collect())
    }

    async fn remove_expired_windows(&mut self) -> BridgeResult<Vec<WindowInfo>> {
        let mut expired_windows = Vec::new();

        {
            let mut windows = self.windows.write().await;
            let expired_ids: Vec<Uuid> = windows
                .iter()
                .filter(|(_, window)| window.is_expired())
                .map(|(id, _)| *id)
                .collect();

            for id in expired_ids {
                if let Some(window) = windows.remove(&id) {
                    expired_windows.push(window.get_info());
                }
            }
        }

        {
            let mut stats = self.stats.write().await;
            stats.expired_windows += expired_windows.len() as u64;
            stats.active_windows = stats
                .active_windows
                .saturating_sub(expired_windows.len() as u64);
        }

        info!("Removed {} expired windows", expired_windows.len());
        Ok(expired_windows)
    }

    async fn get_stats(&self) -> BridgeResult<WindowManagerStats> {
        Ok(self.stats.read().await.clone())
    }
}
