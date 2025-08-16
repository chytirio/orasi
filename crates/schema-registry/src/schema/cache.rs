//! Schema cache for performance optimization
//!
//! This module contains the schema cache implementation for caching resolved schemas.

use chrono;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::resolution::ResolvedSchema;

/// Schema cache for performance optimization
pub struct SchemaCache {
    /// Cache of resolved schemas
    resolved_schemas: HashMap<String, (ResolvedSchema, chrono::DateTime<chrono::Utc>)>,

    /// Cache TTL in seconds
    ttl_seconds: u64,

    /// Maximum cache size
    max_size: usize,
}

impl SchemaCache {
    /// Create a new schema cache
    pub fn new(ttl_seconds: u64, max_size: usize) -> Self {
        Self {
            resolved_schemas: HashMap::new(),
            ttl_seconds,
            max_size,
        }
    }

    /// Get a resolved schema from cache
    pub fn get(&self, fingerprint: &str) -> Option<ResolvedSchema> {
        if let Some((resolved_schema, cached_at)) = self.resolved_schemas.get(fingerprint) {
            let now = chrono::Utc::now();
            let age = now.signed_duration_since(*cached_at).num_seconds() as u64;

            if age < self.ttl_seconds {
                return Some(resolved_schema.clone());
            }
        }
        None
    }

    /// Store a resolved schema in cache
    pub fn put(&mut self, fingerprint: String, resolved_schema: ResolvedSchema) {
        // Check if cache is full
        if self.resolved_schemas.len() >= self.max_size {
            // Remove oldest entry
            let oldest_key = self
                .resolved_schemas
                .iter()
                .min_by_key(|(_, (_, cached_at))| cached_at)
                .map(|(key, _)| key.clone());

            if let Some(key) = oldest_key {
                self.resolved_schemas.remove(&key);
            }
        }

        let now = chrono::Utc::now();
        self.resolved_schemas
            .insert(fingerprint, (resolved_schema, now));
    }

    /// Remove a schema from cache
    pub fn remove(&mut self, fingerprint: &str) {
        self.resolved_schemas.remove(fingerprint);
    }

    /// Clear all cached schemas
    pub fn clear(&mut self) {
        self.resolved_schemas.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let now = chrono::Utc::now();
        let mut expired_count = 0;
        let mut total_age = 0;

        for (_, (_, cached_at)) in &self.resolved_schemas {
            let age = now.signed_duration_since(*cached_at).num_seconds() as u64;
            if age >= self.ttl_seconds {
                expired_count += 1;
            }
            total_age += age;
        }

        let avg_age = if self.resolved_schemas.is_empty() {
            0
        } else {
            total_age / self.resolved_schemas.len() as u64
        };

        CacheStats {
            total_entries: self.resolved_schemas.len(),
            expired_entries: expired_count,
            max_size: self.max_size,
            ttl_seconds: self.ttl_seconds,
            average_age_seconds: avg_age,
        }
    }

    /// Clean up expired entries
    pub fn cleanup(&mut self) {
        let now = chrono::Utc::now();
        self.resolved_schemas.retain(|_, (_, cached_at)| {
            let age = now.signed_duration_since(*cached_at).num_seconds() as u64;
            age < self.ttl_seconds
        });
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cached entries
    pub total_entries: usize,

    /// Number of expired entries
    pub expired_entries: usize,

    /// Maximum cache size
    pub max_size: usize,

    /// Cache TTL in seconds
    pub ttl_seconds: u64,

    /// Average age of cached entries in seconds
    pub average_age_seconds: u64,
}
