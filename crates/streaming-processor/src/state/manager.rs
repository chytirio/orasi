//! State manager for managing multiple state stores

use bridge_core::BridgeResult;
use std::collections::HashMap;

use super::store::StateStore;

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
            Err(bridge_core::BridgeError::configuration(format!(
                "State store not found: {}",
                store_name
            )))
        }
    }

    /// Set state value in store
    pub async fn set(&self, store_name: &str, key: &str, value: String) -> BridgeResult<()> {
        if let Some(store) = self.get_store(store_name) {
            store.set(key, value).await
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "State store not found: {}",
                store_name
            )))
        }
    }

    /// Delete state value from store
    pub async fn delete(&self, store_name: &str, key: &str) -> BridgeResult<()> {
        if let Some(store) = self.get_store(store_name) {
            store.delete(key).await
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "State store not found: {}",
                store_name
            )))
        }
    }
}
