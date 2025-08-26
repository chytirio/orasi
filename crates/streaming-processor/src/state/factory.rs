//! State factory for creating state stores

use super::store::{InMemoryStateStore, StateStore};

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
