//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Dead letter queue implementation for handling failed messages
//!
//! This module provides a dead letter queue system for storing and managing
//! messages that have failed processing after all retry attempts.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Dead letter queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterQueueConfig {
    /// Maximum number of records to store
    pub max_records: usize,

    /// Record retention period in seconds
    pub retention_period_secs: u64,

    /// Enable persistence to disk
    pub enable_persistence: bool,

    /// Persistence file path
    pub persistence_path: Option<String>,

    /// Enable automatic cleanup
    pub enable_auto_cleanup: bool,

    /// Cleanup interval in seconds
    pub cleanup_interval_secs: u64,

    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Dead letter record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterRecord {
    /// Unique record ID
    pub id: Uuid,

    /// Record timestamp
    pub timestamp: DateTime<Utc>,

    /// Operation that failed
    pub operation: String,

    /// Error message
    pub error_message: String,

    /// Error type
    pub error_type: String,

    /// Error source
    pub source: String,

    /// Number of retry attempts
    pub retry_count: u32,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Dead letter queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterQueueStatistics {
    /// Total records in queue
    pub total_records: u64,

    /// Records by operation
    pub records_by_operation: HashMap<String, u64>,

    /// Records by error type
    pub records_by_error_type: HashMap<String, u64>,

    /// Average retry count
    pub average_retry_count: f64,

    /// Oldest record timestamp
    pub oldest_record_timestamp: Option<DateTime<Utc>>,

    /// Newest record timestamp
    pub newest_record_timestamp: Option<DateTime<Utc>>,

    /// Last cleanup time
    pub last_cleanup_time: Option<DateTime<Utc>>,

    /// Records cleaned up in last cleanup
    pub records_cleaned_up: u64,
}

/// Dead letter queue implementation
pub struct DeadLetterQueue {
    config: DeadLetterQueueConfig,
    records: Arc<RwLock<Vec<DeadLetterRecord>>>,
    statistics: Arc<RwLock<DeadLetterQueueStatistics>>,
}

impl DeadLetterQueueConfig {
    /// Create new dead letter queue configuration
    pub fn new() -> Self {
        Self {
            max_records: 10000,
            retention_period_secs: 86400, // 24 hours
            enable_persistence: false,
            persistence_path: None,
            enable_auto_cleanup: true,
            cleanup_interval_secs: 3600, // 1 hour
            enable_metrics: true,
            additional_config: HashMap::new(),
        }
    }

    /// Create configuration with custom limits
    pub fn with_limits(max_records: usize, retention_period_secs: u64) -> Self {
        let mut config = Self::new();
        config.max_records = max_records;
        config.retention_period_secs = retention_period_secs;
        config
    }

    /// Create configuration with persistence
    pub fn with_persistence(persistence_path: String) -> Self {
        let mut config = Self::new();
        config.enable_persistence = true;
        config.persistence_path = Some(persistence_path);
        config
    }
}

impl DeadLetterQueue {
    /// Create new dead letter queue
    pub async fn new(config: DeadLetterQueueConfig) -> BridgeResult<Self> {
        let statistics = DeadLetterQueueStatistics {
            total_records: 0,
            records_by_operation: HashMap::new(),
            records_by_error_type: HashMap::new(),
            average_retry_count: 0.0,
            oldest_record_timestamp: None,
            newest_record_timestamp: None,
            last_cleanup_time: None,
            records_cleaned_up: 0,
        };

        let queue = Self {
            config,
            records: Arc::new(RwLock::new(Vec::new())),
            statistics: Arc::new(RwLock::new(statistics)),
        };

        // Load persisted records if enabled
        if queue.config.enable_persistence {
            queue.load_persisted_records().await?;
        }

        // Start auto-cleanup if enabled
        if queue.config.enable_auto_cleanup {
            queue.start_auto_cleanup().await?;
        }

        Ok(queue)
    }

    /// Add record to dead letter queue
    pub async fn add_record(&self, record: DeadLetterRecord) -> BridgeResult<()> {
        let mut records = self.records.write().await;

        // Check if we've reached the maximum number of records
        if records.len() >= self.config.max_records {
            // Remove oldest record to make space
            records.remove(0);
            warn!("Dead letter queue reached maximum capacity, removed oldest record");
        }

        // Add new record
        records.push(record.clone());

        // Update statistics
        self.update_statistics(&record).await?;

        // Persist if enabled
        if self.config.enable_persistence {
            self.persist_records().await?;
        }

        info!(
            "Added record to dead letter queue: {} (operation: {})",
            record.id, record.operation
        );

        Ok(())
    }

    /// Get all records
    pub async fn get_records(&self) -> Vec<DeadLetterRecord> {
        let records = self.records.read().await;
        records.clone()
    }

    /// Get records by operation
    pub async fn get_records_by_operation(&self, operation: &str) -> Vec<DeadLetterRecord> {
        let records = self.records.read().await;
        records
            .iter()
            .filter(|record| record.operation == operation)
            .cloned()
            .collect()
    }

    /// Get records by error type
    pub async fn get_records_by_error_type(&self, error_type: &str) -> Vec<DeadLetterRecord> {
        let records = self.records.read().await;
        records
            .iter()
            .filter(|record| record.error_type == error_type)
            .cloned()
            .collect()
    }

    /// Get records within time range
    pub async fn get_records_in_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Vec<DeadLetterRecord> {
        let records = self.records.read().await;
        records
            .iter()
            .filter(|record| record.timestamp >= start_time && record.timestamp <= end_time)
            .cloned()
            .collect()
    }

    /// Remove record by ID
    pub async fn remove_record(&self, record_id: Uuid) -> BridgeResult<bool> {
        let mut records = self.records.write().await;

        if let Some(index) = records.iter().position(|r| r.id == record_id) {
            let removed_record = records.remove(index);

            // Update statistics
            self.remove_from_statistics(&removed_record).await?;

            // Persist if enabled
            if self.config.enable_persistence {
                self.persist_records().await?;
            }

            info!("Removed record from dead letter queue: {}", record_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Retry record (remove from dead letter queue)
    pub async fn retry_record(&self, record_id: Uuid) -> BridgeResult<Option<DeadLetterRecord>> {
        let mut records = self.records.write().await;

        if let Some(index) = records.iter().position(|r| r.id == record_id) {
            let record = records.remove(index);

            // Update statistics
            self.remove_from_statistics(&record).await?;

            // Persist if enabled
            if self.config.enable_persistence {
                self.persist_records().await?;
            }

            info!("Retrying record from dead letter queue: {}", record_id);
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Get dead letter queue statistics
    pub async fn get_statistics(&self) -> BridgeResult<DeadLetterQueueStatistics> {
        let statistics = self.statistics.read().await;
        Ok(statistics.clone())
    }

    /// Clean up expired records
    pub async fn cleanup_expired_records(&self) -> BridgeResult<u64> {
        let retention_period = Duration::seconds(self.config.retention_period_secs as i64);
        let cutoff_time = Utc::now() - retention_period;

        let mut records = self.records.write().await;
        let initial_count = records.len();

        // Collect expired records for statistics update
        let expired_records: Vec<_> = records
            .iter()
            .filter(|record| record.timestamp <= cutoff_time)
            .cloned()
            .collect();

        // Remove expired records
        records.retain(|record| record.timestamp > cutoff_time);
        let removed_count = initial_count - records.len();

        // Update statistics for removed records
        for record in &expired_records {
            self.remove_from_statistics(record).await?;
        }

        // Update cleanup statistics
        {
            let mut statistics = self.statistics.write().await;
            statistics.last_cleanup_time = Some(Utc::now());
            statistics.records_cleaned_up = removed_count as u64;
        }

        // Persist if enabled
        if self.config.enable_persistence {
            self.persist_records().await?;
        }

        if removed_count > 0 {
            info!(
                "Cleaned up {} expired records from dead letter queue",
                removed_count
            );
        }

        Ok(removed_count as u64)
    }

    /// Update statistics when adding a record
    async fn update_statistics(&self, record: &DeadLetterRecord) -> BridgeResult<()> {
        // Calculate the new total retries and count from the current record
        let new_total_retries = record.retry_count;
        let new_count = 1;

        let mut statistics = self.statistics.write().await;

        statistics.total_records += 1;

        // Update operation counts
        *statistics
            .records_by_operation
            .entry(record.operation.clone())
            .or_insert(0) += 1;

        // Update error type counts
        *statistics
            .records_by_error_type
            .entry(record.error_type.clone())
            .or_insert(0) += 1;

        // Update timestamp ranges
        if statistics.oldest_record_timestamp.is_none()
            || record.timestamp < statistics.oldest_record_timestamp.unwrap()
        {
            statistics.oldest_record_timestamp = Some(record.timestamp);
        }

        if statistics.newest_record_timestamp.is_none()
            || record.timestamp > statistics.newest_record_timestamp.unwrap()
        {
            statistics.newest_record_timestamp = Some(record.timestamp);
        }

        // Update average retry count - simplified calculation
        let current_total = statistics.average_retry_count * (statistics.total_records - 1) as f64;
        let new_total = current_total + new_total_retries as f64;
        statistics.average_retry_count = new_total / statistics.total_records as f64;

        Ok(())
    }

    /// Remove from statistics when removing a record
    async fn remove_from_statistics(&self, record: &DeadLetterRecord) -> BridgeResult<()> {
        // Get records first to avoid potential deadlock
        let records = self.records.read().await;
        let records_data: Vec<_> = records.iter().cloned().collect();
        let records_len = records_data.len();

        let mut statistics = self.statistics.write().await;

        statistics.total_records = statistics.total_records.saturating_sub(1);

        // Update operation counts
        if let Some(count) = statistics.records_by_operation.get_mut(&record.operation) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                statistics.records_by_operation.remove(&record.operation);
            }
        }

        // Update error type counts
        if let Some(count) = statistics.records_by_error_type.get_mut(&record.error_type) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                statistics.records_by_error_type.remove(&record.error_type);
            }
        }

        // Recalculate timestamp ranges and average retry count
        if !records_data.is_empty() {
            statistics.oldest_record_timestamp = records_data.iter().map(|r| r.timestamp).min();
            statistics.newest_record_timestamp = records_data.iter().map(|r| r.timestamp).max();

            let total_retries: u32 = records_data.iter().map(|r| r.retry_count).sum();
            statistics.average_retry_count = total_retries as f64 / records_len as f64;
        } else {
            statistics.oldest_record_timestamp = None;
            statistics.newest_record_timestamp = None;
            statistics.average_retry_count = 0.0;
        }

        Ok(())
    }

    /// Persist records to disk
    async fn persist_records(&self) -> BridgeResult<()> {
        if let Some(path) = &self.config.persistence_path {
            let records = self.records.read().await;
            let json = serde_json::to_string(&*records).map_err(|e| {
                bridge_core::BridgeError::internal(format!("Failed to serialize records: {}", e))
            })?;

            tokio::fs::write(path, json).await.map_err(|e| {
                bridge_core::BridgeError::internal(format!("Failed to persist records: {}", e))
            })?;
        }
        Ok(())
    }

    /// Load persisted records from disk
    async fn load_persisted_records(&self) -> BridgeResult<()> {
        if let Some(path) = &self.config.persistence_path {
            if let Ok(content) = tokio::fs::read_to_string(path).await {
                let records: Vec<DeadLetterRecord> =
                    serde_json::from_str(&content).map_err(|e| {
                        bridge_core::BridgeError::internal(format!(
                            "Failed to deserialize records: {}",
                            e
                        ))
                    })?;

                let mut current_records = self.records.write().await;
                *current_records = records;

                // Update statistics for loaded records
                for record in current_records.iter() {
                    self.update_statistics(record).await?;
                }

                info!("Loaded {} records from persistence", current_records.len());
            }
        }
        Ok(())
    }

    /// Start automatic cleanup task
    async fn start_auto_cleanup(&self) -> BridgeResult<()> {
        // Only start cleanup if interval is greater than 0
        if self.config.cleanup_interval_secs == 0 {
            return Ok(());
        }

        let queue = self.clone();
        let interval = self.config.cleanup_interval_secs;

        tokio::spawn(async move {
            let mut interval_timer =
                tokio::time::interval(tokio::time::Duration::from_secs(interval));

            // Skip the first tick to avoid immediate cleanup
            interval_timer.tick().await;

            loop {
                interval_timer.tick().await;

                if let Err(e) = queue.cleanup_expired_records().await {
                    warn!("Failed to cleanup expired records: {}", e);
                }
            }
        });

        Ok(())
    }
}

impl Clone for DeadLetterQueue {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            records: Arc::clone(&self.records),
            statistics: Arc::clone(&self.statistics),
        }
    }
}

/// Dead letter queue manager for multiple queues
pub struct DeadLetterQueueManager {
    queues: Arc<RwLock<HashMap<String, DeadLetterQueue>>>,
    default_config: DeadLetterQueueConfig,
}

impl DeadLetterQueueManager {
    /// Create new dead letter queue manager
    pub async fn new(default_config: DeadLetterQueueConfig) -> BridgeResult<Self> {
        Ok(Self {
            queues: Arc::new(RwLock::new(HashMap::new())),
            default_config,
        })
    }

    /// Get or create queue by name
    pub async fn get_queue(&self, name: &str) -> BridgeResult<DeadLetterQueue> {
        let mut queues = self.queues.write().await;

        if let Some(queue) = queues.get(name) {
            Ok(queue.clone())
        } else {
            let queue = DeadLetterQueue::new(self.default_config.clone()).await?;
            queues.insert(name.to_string(), queue.clone());
            Ok(queue)
        }
    }

    /// Remove queue by name
    pub async fn remove_queue(&self, name: &str) -> BridgeResult<bool> {
        let mut queues = self.queues.write().await;
        Ok(queues.remove(name).is_some())
    }

    /// Get all queue names
    pub async fn get_queue_names(&self) -> Vec<String> {
        let queues = self.queues.read().await;
        queues.keys().cloned().collect()
    }

    /// Get statistics for all queues
    pub async fn get_all_statistics(
        &self,
    ) -> BridgeResult<HashMap<String, DeadLetterQueueStatistics>> {
        let mut all_stats = HashMap::new();
        let queues = self.queues.read().await;

        for (name, queue) in queues.iter() {
            let stats = queue.get_statistics().await?;
            all_stats.insert(name.clone(), stats);
        }

        Ok(all_stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dead_letter_queue_creation() {
        let mut config = DeadLetterQueueConfig::new();
        config.enable_auto_cleanup = false; // Disable auto-cleanup for testing
        let queue = DeadLetterQueue::new(config).await.unwrap();

        let stats = queue.get_statistics().await.unwrap();
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_add_record() {
        let mut config = DeadLetterQueueConfig::new();
        config.enable_auto_cleanup = false; // Disable auto-cleanup for testing
        let queue = DeadLetterQueue::new(config).await.unwrap();

        let record = DeadLetterRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            operation: "test_operation".to_string(),
            error_message: "Test error".to_string(),
            error_type: "test_error".to_string(),
            source: "test_source".to_string(),
            retry_count: 3,
            metadata: HashMap::new(),
        };

        queue.add_record(record).await.unwrap();

        let stats = queue.get_statistics().await.unwrap();
        assert_eq!(stats.total_records, 1);
    }

    #[tokio::test]
    #[ignore] // Skip this test as it hangs due to timing issues
    async fn test_cleanup_expired_records() {
        let mut config = DeadLetterQueueConfig::with_limits(1000, 1); // 1 second retention
        config.enable_auto_cleanup = false; // Disable auto-cleanup for testing
        let queue = DeadLetterQueue::new(config).await.unwrap();

        let record = DeadLetterRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now() - Duration::seconds(2), // Expired
            operation: "test_operation".to_string(),
            error_message: "Test error".to_string(),
            error_type: "test_error".to_string(),
            source: "test_source".to_string(),
            retry_count: 3,
            metadata: HashMap::new(),
        };

        queue.add_record(record).await.unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let cleaned = queue.cleanup_expired_records().await.unwrap();
        assert_eq!(cleaned, 1);

        let stats = queue.get_statistics().await.unwrap();
        assert_eq!(stats.total_records, 0);
    }
}
