//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query caching for the bridge
//!
//! This module provides intelligent query result caching with TTL,
//! cache invalidation, and performance optimization.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::executors::QueryResult;
use crate::parsers::ParsedQuery;

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache name
    pub name: String,

    /// Cache version
    pub version: String,

    /// Maximum cache size in bytes
    pub max_size_bytes: u64,

    /// Default TTL in seconds
    pub default_ttl_seconds: u64,

    /// Enable cache
    pub enabled: bool,

    /// Enable debug logging
    pub debug_logging: bool,
}

impl CacheConfig {
    /// Create new configuration with defaults
    pub fn new() -> Self {
        Self {
            name: "query_cache".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            max_size_bytes: 1024 * 1024 * 100, // 100MB
            default_ttl_seconds: 300,          // 5 minutes
            enabled: true,
            debug_logging: false,
        }
    }
}

/// Cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Entry ID
    pub id: Uuid,

    /// Query hash
    pub query_hash: String,

    /// Query result
    pub result: QueryResult,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,

    /// Access count
    pub access_count: u64,

    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,

    /// Entry size in bytes (estimated)
    pub size_bytes: u64,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Cache name
    pub cache: String,

    /// Total entries
    pub total_entries: u64,

    /// Cache hits
    pub hits: u64,

    /// Cache misses
    pub misses: u64,

    /// Cache hit rate
    pub hit_rate: f64,

    /// Total cache size in bytes
    pub total_size_bytes: u64,

    /// Evicted entries count
    pub evicted_entries: u64,

    /// Last cleanup timestamp
    pub last_cleanup: Option<DateTime<Utc>>,
}

/// Query cache implementation
#[derive(Debug)]
pub struct QueryCache {
    config: CacheConfig,
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
    is_initialized: bool,
}

impl QueryCache {
    /// Create new query cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats {
                cache: "query_cache".to_string(),
                total_entries: 0,
                hits: 0,
                misses: 0,
                hit_rate: 0.0,
                total_size_bytes: 0,
                evicted_entries: 0,
                last_cleanup: None,
            })),
            is_initialized: false,
        }
    }

    /// Generate query hash
    fn generate_query_hash(&self, query: &ParsedQuery) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.query_text.hash(&mut hasher);
        query.ast.node_count.hash(&mut hasher);
        query.ast.depth.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Estimate result size in bytes
    fn estimate_result_size(&self, result: &QueryResult) -> u64 {
        let mut size = 0u64;

        // Estimate size based on result data
        for row in &result.data {
            for (_, value) in &row.data {
                size += match value {
                    crate::executors::QueryValue::String(s) => s.len() as u64,
                    crate::executors::QueryValue::Integer(_) => 8,
                    crate::executors::QueryValue::Float(_) => 8,
                    crate::executors::QueryValue::Boolean(_) => 1,
                    crate::executors::QueryValue::Null => 0,
                    crate::executors::QueryValue::Array(arr) => arr.len() as u64 * 8,
                    crate::executors::QueryValue::Object(obj) => obj.len() as u64 * 16,
                };
            }
        }

        // Add overhead for metadata
        size += result.metadata.len() as u64 * 32;
        size += 64; // UUID and timestamp overhead

        size
    }

    /// Check if cache entry is expired
    fn is_expired(&self, entry: &CacheEntry) -> bool {
        Utc::now() > entry.expires_at
    }

    /// Clean up expired entries
    async fn cleanup_expired_entries(&self) -> BridgeResult<()> {
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;

        let initial_count = entries.len();
        let mut removed_size = 0u64;

        entries.retain(|_, entry| {
            if self.is_expired(entry) {
                removed_size += entry.size_bytes;
                false
            } else {
                true
            }
        });

        let removed_count = initial_count - entries.len();
        stats.total_entries = entries.len() as u64;
        stats.total_size_bytes = stats.total_size_bytes.saturating_sub(removed_size);
        stats.evicted_entries += removed_count as u64;
        stats.last_cleanup = Some(Utc::now());

        if self.config.debug_logging && removed_count > 0 {
            info!(
                "Cleaned up {} expired cache entries, freed {} bytes",
                removed_count, removed_size
            );
        }

        Ok(())
    }

    /// Evict entries if cache is full
    async fn evict_if_needed(&self) -> BridgeResult<()> {
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;

        if stats.total_size_bytes <= self.config.max_size_bytes {
            return Ok(());
        }

        // Simple LRU eviction: remove oldest entries first
        let mut entries_vec: Vec<_> = entries.drain().collect();
        entries_vec.sort_by(|a, b| a.1.last_accessed.cmp(&b.1.last_accessed));

        let mut removed_size = 0u64;
        let mut removed_count = 0u64;

        for (key, entry) in entries_vec {
            if stats.total_size_bytes - removed_size <= self.config.max_size_bytes {
                entries.insert(key, entry);
            } else {
                removed_size += entry.size_bytes;
                removed_count += 1;
            }
        }

        stats.total_entries = entries.len() as u64;
        stats.total_size_bytes = stats.total_size_bytes.saturating_sub(removed_size);
        stats.evicted_entries += removed_count;

        if self.config.debug_logging && removed_count > 0 {
            info!(
                "Evicted {} cache entries, freed {} bytes",
                removed_count, removed_size
            );
        }

        Ok(())
    }

    /// Get cache entry
    pub async fn get(&self, query: &ParsedQuery) -> BridgeResult<Option<QueryResult>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let query_hash = self.generate_query_hash(query);
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = entries.get_mut(&query_hash) {
            if self.is_expired(entry) {
                // Remove expired entry
                let size_bytes = entry.size_bytes;
                entries.remove(&query_hash);
                stats.total_entries = entries.len() as u64;
                stats.total_size_bytes = stats.total_size_bytes.saturating_sub(size_bytes);
                stats.misses += 1;

                if self.config.debug_logging {
                    info!("Cache miss: expired entry for query hash {}", query_hash);
                }

                return Ok(None);
            }

            // Update access statistics
            entry.access_count += 1;
            entry.last_accessed = Utc::now();
            stats.hits += 1;

            if self.config.debug_logging {
                info!("Cache hit for query hash {}", query_hash);
            }

            Ok(Some(entry.result.clone()))
        } else {
            stats.misses += 1;

            if self.config.debug_logging {
                info!("Cache miss for query hash {}", query_hash);
            }

            Ok(None)
        }
    }

    /// Put cache entry
    pub async fn put(&self, query: &ParsedQuery, result: QueryResult) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let query_hash = self.generate_query_hash(query);
        let size_bytes = self.estimate_result_size(&result);
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.config.default_ttl_seconds as i64);

        let entry = CacheEntry {
            id: Uuid::new_v4(),
            query_hash: query_hash.clone(),
            result: result.clone(),
            created_at: now,
            expires_at,
            access_count: 0,
            last_accessed: now,
            size_bytes,
        };

        // Check if we need to evict entries
        self.evict_if_needed().await?;

        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;

        // Remove existing entry if it exists
        if let Some(existing_entry) = entries.remove(&query_hash) {
            stats.total_size_bytes = stats
                .total_size_bytes
                .saturating_sub(existing_entry.size_bytes);
        }

        // Add new entry
        entries.insert(query_hash.clone(), entry);
        stats.total_entries = entries.len() as u64;
        stats.total_size_bytes += size_bytes;

        // Update hit rate
        let total_requests = stats.hits + stats.misses;
        if total_requests > 0 {
            stats.hit_rate = stats.hits as f64 / total_requests as f64;
        }

        if self.config.debug_logging {
            info!(
                "Cached query result: hash={}, size={} bytes, expires_at={}",
                query_hash, size_bytes, expires_at
            );
        }

        Ok(())
    }

    /// Invalidate cache entry
    pub async fn invalidate(&self, query: &ParsedQuery) -> BridgeResult<()> {
        let query_hash = self.generate_query_hash(query);
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = entries.remove(&query_hash) {
            stats.total_entries = entries.len() as u64;
            stats.total_size_bytes = stats.total_size_bytes.saturating_sub(entry.size_bytes);

            if self.config.debug_logging {
                info!("Invalidated cache entry for query hash {}", query_hash);
            }
        }

        Ok(())
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> BridgeResult<()> {
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;

        let cleared_count = entries.len();
        entries.clear();

        stats.total_entries = 0;
        stats.total_size_bytes = 0;
        stats.last_cleanup = Some(Utc::now());

        if self.config.debug_logging {
            info!("Cleared {} cache entries", cleared_count);
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> BridgeResult<CacheStats> {
        // Clean up expired entries before returning stats
        self.cleanup_expired_entries().await?;

        Ok(self.stats.read().await.clone())
    }
}

/// Cache trait for different cache implementations
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get cache entry
    async fn get(&self, query: &ParsedQuery) -> BridgeResult<Option<QueryResult>>;

    /// Put cache entry
    async fn put(&self, query: &ParsedQuery, result: QueryResult) -> BridgeResult<()>;

    /// Invalidate cache entry
    async fn invalidate(&self, query: &ParsedQuery) -> BridgeResult<()>;

    /// Clear all cache entries
    async fn clear(&self) -> BridgeResult<()>;

    /// Get cache statistics
    async fn get_stats(&self) -> BridgeResult<CacheStats>;
}

#[async_trait]
impl Cache for QueryCache {
    async fn get(&self, query: &ParsedQuery) -> BridgeResult<Option<QueryResult>> {
        self.get(query).await
    }

    async fn put(&self, query: &ParsedQuery, result: QueryResult) -> BridgeResult<()> {
        self.put(query, result).await
    }

    async fn invalidate(&self, query: &ParsedQuery) -> BridgeResult<()> {
        self.invalidate(query).await
    }

    async fn clear(&self) -> BridgeResult<()> {
        self.clear().await
    }

    async fn get_stats(&self) -> BridgeResult<CacheStats> {
        self.get_stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::{AstNode, NodeType, QueryAst};

    fn create_test_query() -> ParsedQuery {
        ParsedQuery {
            id: Uuid::new_v4(),
            query_text: "SELECT * FROM test_table".to_string(),
            ast: QueryAst {
                root: AstNode {
                    node_type: NodeType::Select,
                    value: Some("SELECT * FROM test_table".to_string()),
                    children: Vec::new(),
                    metadata: HashMap::new(),
                },
                node_count: 1,
                depth: 1,
            },
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    fn create_test_result() -> QueryResult {
        QueryResult {
            id: Uuid::new_v4(),
            query_id: Uuid::new_v4(),
            status: crate::executors::ExecutionStatus::Success,
            data: vec![],
            metadata: HashMap::new(),
            execution_time_ms: 100,
            execution_timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_cache_creation() {
        let config = CacheConfig::new();
        let cache = QueryCache::new(config);

        let stats = cache.get_stats().await.unwrap();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let config = CacheConfig::new();
        let cache = QueryCache::new(config);

        let query = create_test_query();
        let result = create_test_result();

        // Put result in cache
        cache.put(&query, result.clone()).await.unwrap();

        // Get result from cache
        let cached_result = cache.get(&query).await.unwrap();
        assert!(cached_result.is_some());

        let stats = cache.get_stats().await.unwrap();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let config = CacheConfig::new();
        let cache = QueryCache::new(config);

        let query = create_test_query();

        // Try to get non-existent result
        let cached_result = cache.get(&query).await.unwrap();
        assert!(cached_result.is_none());

        let stats = cache.get_stats().await.unwrap();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let config = CacheConfig::new();
        let cache = QueryCache::new(config);

        let query = create_test_query();
        let result = create_test_result();

        // Put result in cache
        cache.put(&query, result).await.unwrap();

        // Verify it's cached
        let cached_result = cache.get(&query).await.unwrap();
        assert!(cached_result.is_some());

        // Invalidate cache
        cache.invalidate(&query).await.unwrap();

        // Verify it's no longer cached
        let cached_result = cache.get(&query).await.unwrap();
        assert!(cached_result.is_none());
    }
}
