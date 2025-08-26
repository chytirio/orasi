//! State store trait and implementations

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State store trait for streaming data state management
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Get state value
    async fn get(&self, key: &str) -> BridgeResult<Option<String>>;

    /// Set state value
    async fn set(&self, key: &str, value: String) -> BridgeResult<()>;

    /// Delete state value
    async fn delete(&self, key: &str) -> BridgeResult<()>;

    /// Check if key exists
    async fn exists(&self, key: &str) -> BridgeResult<bool>;

    /// Get all keys
    async fn keys(&self) -> BridgeResult<Vec<String>>;

    /// Clear all state
    async fn clear(&self) -> BridgeResult<()>;

    /// Get store statistics
    async fn get_stats(&self) -> BridgeResult<StateStats>;
}

/// State statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStats {
    /// Store name
    pub store_name: String,

    /// Number of keys
    pub key_count: u64,

    /// Total size in bytes
    pub total_size_bytes: u64,

    /// Last access time
    pub last_access_time: Option<DateTime<Utc>>,

    /// Store status
    pub status: StateStatus,
}

/// State status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateStatus {
    Active,
    Closed,
    Error,
}

/// In-memory state store implementation
pub struct InMemoryStateStore {
    name: String,
    data: Arc<RwLock<HashMap<String, String>>>,
    stats: Arc<RwLock<StateStats>>,
}

impl InMemoryStateStore {
    /// Create new in-memory state store
    pub fn new(name: String) -> Self {
        let stats = StateStats {
            store_name: name.clone(),
            key_count: 0,
            total_size_bytes: 0,
            last_access_time: None,
            status: StateStatus::Active,
        };

        Self {
            name,
            data: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(stats)),
        }
    }
}

#[async_trait]
impl StateStore for InMemoryStateStore {
    async fn get(&self, key: &str) -> BridgeResult<Option<String>> {
        let data = self.data.read().await;
        let value = data.get(key).cloned();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.last_access_time = Some(Utc::now());
        }

        Ok(value)
    }

    async fn set(&self, key: &str, value: String) -> BridgeResult<()> {
        let mut data = self.data.write().await;
        let old_size = data.get(key).map(|v| v.len()).unwrap_or(0);
        data.insert(key.to_string(), value.clone());

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.key_count = data.len() as u64;
            stats.total_size_bytes = stats.total_size_bytes - old_size as u64 + value.len() as u64;
            stats.last_access_time = Some(Utc::now());
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> BridgeResult<()> {
        let mut data = self.data.write().await;
        let old_size = data.get(key).map(|v| v.len()).unwrap_or(0);
        data.remove(key);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.key_count = data.len() as u64;
            stats.total_size_bytes = stats.total_size_bytes.saturating_sub(old_size as u64);
            stats.last_access_time = Some(Utc::now());
        }

        Ok(())
    }

    async fn exists(&self, key: &str) -> BridgeResult<bool> {
        let data = self.data.read().await;
        Ok(data.contains_key(key))
    }

    async fn keys(&self) -> BridgeResult<Vec<String>> {
        let data = self.data.read().await;
        Ok(data.keys().cloned().collect())
    }

    async fn clear(&self) -> BridgeResult<()> {
        let mut data = self.data.write().await;
        data.clear();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.key_count = 0;
            stats.total_size_bytes = 0;
            stats.last_access_time = Some(Utc::now());
        }

        Ok(())
    }

    async fn get_stats(&self) -> BridgeResult<StateStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}
