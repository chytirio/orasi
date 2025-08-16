//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data state management for the bridge
//! 
//! This module provides state management functionality for streaming data
//! including state stores and managers.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};

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

/// State manager for managing multiple state stores
pub struct StateManager {
    stores: HashMap<String, Box<dyn StateStore>>,
}

impl StateManager {
    /// Create new state manager
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }
    
    /// Add state store
    pub fn add_store(&mut self, name: String, store: Box<dyn StateStore>) {
        self.stores.insert(name, store);
    }
    
    /// Remove state store
    pub fn remove_store(&mut self, name: &str) -> Option<Box<dyn StateStore>> {
        self.stores.remove(name)
    }
    
    /// Get state store
    pub fn get_store(&self, name: &str) -> Option<&dyn StateStore> {
        self.stores.get(name).map(|s| s.as_ref())
    }
    
    /// Get all store names
    pub fn get_store_names(&self) -> Vec<String> {
        self.stores.keys().cloned().collect()
    }
    
    /// Get state value from store
    pub async fn get(&self, store_name: &str, key: &str) -> BridgeResult<Option<String>> {
        if let Some(store) = self.get_store(store_name) {
            store.get(key).await
        } else {
            Err(bridge_core::BridgeError::configuration(
                format!("State store not found: {}", store_name)
            ))
        }
    }
    
    /// Set state value in store
    pub async fn set(&self, store_name: &str, key: &str, value: String) -> BridgeResult<()> {
        if let Some(store) = self.get_store(store_name) {
            store.set(key, value).await
        } else {
            Err(bridge_core::BridgeError::configuration(
                format!("State store not found: {}", store_name)
            ))
        }
    }
    
    /// Delete state value from store
    pub async fn delete(&self, store_name: &str, key: &str) -> BridgeResult<()> {
        if let Some(store) = self.get_store(store_name) {
            store.delete(key).await
        } else {
            Err(bridge_core::BridgeError::configuration(
                format!("State store not found: {}", store_name)
            ))
        }
    }
}

/// State factory for creating state stores
pub struct StateFactory;

impl StateFactory {
    /// Create in-memory state store
    pub fn create_in_memory_store(name: String) -> Box<dyn StateStore> {
        Box::new(InMemoryStateStore::new(name))
    }
    
    /// Create state store by type
    pub fn create_store(store_type: &str, name: String) -> Option<Box<dyn StateStore>> {
        match store_type {
            "memory" => Some(Self::create_in_memory_store(name)),
            _ => None,
        }
    }
    
    /// Get available store types
    pub fn get_available_store_types() -> Vec<String> {
        vec!["memory".to_string()]
    }
}
