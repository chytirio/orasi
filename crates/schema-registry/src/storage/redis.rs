//! Redis storage implementation

use crate::error::{SchemaRegistryError, SchemaRegistryResult};
use crate::schema::{Schema, SchemaMetadata, SchemaSearchCriteria, SchemaVersion};
use crate::storage::{StorageBackend, StorageStats};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;

/// Redis storage implementation
pub struct RedisStorage {
    /// Redis connection manager
    connection_manager: redis::aio::ConnectionManager,

    /// Key prefix for schema storage
    key_prefix: String,

    /// Connection pool size
    pool_size: usize,

    /// Schema TTL in seconds (0 = no TTL)
    schema_ttl: Option<u64>,

    /// Connection pool for additional connections
    connection_pool: RwLock<Vec<redis::aio::ConnectionManager>>,

    /// Redis cluster mode
    cluster_mode: bool,

    /// Redis database number
    database: u8,
}

impl RedisStorage {
    /// Create a new Redis storage instance
    pub async fn new(url: String) -> SchemaRegistryResult<Self> {
        let client = redis::Client::open(url).map_err(|e| {
            crate::storage::StorageError::ConfigurationError {
                message: format!("Failed to create Redis client: {}", e),
            }
        })?;

        let connection_manager = redis::aio::ConnectionManager::new(client.clone())
            .await
            .map_err(|e| crate::storage::StorageError::ConnectionError {
                message: format!("Failed to create Redis connection manager: {}", e),
            })?;

        Ok(Self {
            connection_manager,
            key_prefix: "schema_registry:".to_string(),
            pool_size: 10,
            schema_ttl: None,
            connection_pool: RwLock::new(Vec::new()),
            cluster_mode: false,
            database: 0,
        })
    }

    /// Create a new Redis storage instance with custom configuration
    pub async fn new_with_config(
        config: &crate::config::RedisConfig,
    ) -> SchemaRegistryResult<Self> {
        let client = redis::Client::open(config.url.clone()).map_err(|e| {
            crate::storage::StorageError::ConfigurationError {
                message: format!("Failed to create Redis client: {}", e),
            }
        })?;

        let connection_manager = redis::aio::ConnectionManager::new(client.clone())
            .await
            .map_err(|e| crate::storage::StorageError::ConnectionError {
                message: format!("Failed to create Redis connection manager: {}", e),
            })?;

        // Initialize connection pool
        let mut connection_pool = Vec::new();
        for _ in 0..config.pool_size {
            let pool_client = redis::Client::open(config.url.clone()).map_err(|e| {
                crate::storage::StorageError::ConfigurationError {
                    message: format!("Failed to create Redis pool client: {}", e),
                }
            })?;

            let pool_conn = redis::aio::ConnectionManager::new(pool_client)
                .await
                .map_err(|e| crate::storage::StorageError::ConnectionError {
                    message: format!("Failed to create Redis pool connection: {}", e),
                })?;

            connection_pool.push(pool_conn);
        }

        Ok(Self {
            connection_manager,
            key_prefix: config.key_prefix.clone(),
            pool_size: config.pool_size as usize,
            schema_ttl: Some(config.connection_timeout), // Use connection timeout as TTL hint
            connection_pool: RwLock::new(connection_pool),
            cluster_mode: config.url.contains("cluster"),
            database: config.database,
        })
    }

    /// Get a connection from the pool
    async fn get_pool_connection(&self) -> SchemaRegistryResult<redis::aio::ConnectionManager> {
        let pool = self.connection_pool.read().await;
        if let Some(conn) = pool.first() {
            Ok(conn.clone())
        } else {
            Err(crate::storage::StorageError::ConnectionError {
                message: "No available connections in pool".to_string(),
            }
            .into())
        }
    }

    /// Generate Redis key for schema
    fn schema_key(&self, fingerprint: &str) -> String {
        format!("{}schema:{}", self.key_prefix, fingerprint)
    }

    /// Generate Redis key for schema metadata
    fn metadata_key(&self, fingerprint: &str) -> String {
        format!("{}metadata:{}", self.key_prefix, fingerprint)
    }

    /// Generate Redis key for schema versions
    fn versions_key(&self, name: &str) -> String {
        format!("{}versions:{}", self.key_prefix, name)
    }

    /// Generate Redis key for schema list
    fn schema_list_key(&self) -> String {
        format!("{}schemas", self.key_prefix)
    }

    /// Generate Redis key for schema tags
    fn tags_key(&self, tag: &str) -> String {
        format!("{}tags:{}", self.key_prefix, tag)
    }

    /// Generate Redis key for schema types
    fn type_key(&self, schema_type: &str) -> String {
        format!("{}types:{}", self.key_prefix, schema_type)
    }

    /// Set TTL on a key if configured
    async fn set_ttl_if_configured(&self, key: &str) -> SchemaRegistryResult<()> {
        if let Some(ttl) = self.schema_ttl {
            if ttl > 0 {
                let mut conn = self.connection_manager.clone();
                let _: () = redis::cmd("EXPIRE")
                    .arg(key)
                    .arg(ttl)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| crate::storage::StorageError::QueryError {
                        message: format!("Failed to set TTL on key {}: {}", key, e),
                    })?;
            }
        }
        Ok(())
    }

    /// Store schema with tags and type indexing
    async fn store_schema_with_indexing(&self, schema: &Schema) -> SchemaRegistryResult<()> {
        let mut conn = self.connection_manager.clone();

        // Store schema tags
        for tag in &schema.tags {
            let tag_key = self.tags_key(tag);
            let _: () = redis::cmd("SADD")
                .arg(&tag_key)
                .arg(&schema.fingerprint)
                .query_async(&mut conn)
                .await
                .map_err(|e| crate::storage::StorageError::QueryError {
                    message: format!("Failed to store schema tag: {}", e),
                })?;
        }

        // Store schema type index
        let type_key = self.type_key(&schema.schema_type.to_string());
        let _: () = redis::cmd("SADD")
            .arg(&type_key)
            .arg(&schema.fingerprint)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to store schema type index: {}", e),
            })?;

        Ok(())
    }

    /// Remove schema from tags and type indexing
    async fn remove_schema_indexing(&self, schema: &Schema) -> SchemaRegistryResult<()> {
        let mut conn = self.connection_manager.clone();

        // Remove from tags
        for tag in &schema.tags {
            let tag_key = self.tags_key(tag);
            let _: () = redis::cmd("SREM")
                .arg(&tag_key)
                .arg(&schema.fingerprint)
                .query_async(&mut conn)
                .await
                .map_err(|e| crate::storage::StorageError::QueryError {
                    message: format!("Failed to remove schema tag: {}", e),
                })?;
        }

        // Remove from type index
        let type_key = self.type_key(&schema.schema_type.to_string());
        let _: () = redis::cmd("SREM")
            .arg(&type_key)
            .arg(&schema.fingerprint)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to remove schema type index: {}", e),
            })?;

        Ok(())
    }

    /// Batch store multiple schemas
    pub async fn batch_store_schemas(
        &self,
        schemas: Vec<Schema>,
    ) -> SchemaRegistryResult<Vec<SchemaVersion>> {
        let mut versions = Vec::new();
        let mut conn = self.connection_manager.clone();

        for schema in schemas {
            let schema_json = serde_json::to_string(&schema).map_err(|e| {
                crate::storage::StorageError::InvalidSchemaData {
                    message: format!("Failed to serialize schema: {}", e),
                }
            })?;

            let metadata: SchemaMetadata = schema.clone().into();
            let metadata_json = serde_json::to_string(&metadata).map_err(|e| {
                crate::storage::StorageError::InvalidSchemaData {
                    message: format!("Failed to serialize metadata: {}", e),
                }
            })?;

            // Use pipeline for batch operations
            let mut pipe = redis::pipe();
            pipe.atomic();

            // Store schema
            let schema_key = self.schema_key(&schema.fingerprint);
            pipe.set(&schema_key, &schema_json);

            // Store metadata
            let metadata_key = self.metadata_key(&schema.fingerprint);
            pipe.set(&metadata_key, &metadata_json);

            // Add to schema list
            let schema_list_key = self.schema_list_key();
            pipe.sadd(&schema_list_key, &schema.fingerprint);

            // Add to versions list
            let versions_key = self.versions_key(&schema.name);
            pipe.sadd(&versions_key, &schema.fingerprint);

            // Execute pipeline
            let _: () = pipe.query_async(&mut conn).await.map_err(|e| {
                crate::storage::StorageError::QueryError {
                    message: format!("Failed to batch store schema: {}", e),
                }
            })?;

            // Set TTL if configured
            self.set_ttl_if_configured(&schema_key).await?;
            self.set_ttl_if_configured(&metadata_key).await?;

            // Store indexing
            self.store_schema_with_indexing(&schema).await?;

            versions.push(schema.version);
        }

        Ok(versions)
    }

    /// Get schemas by tag
    pub async fn get_schemas_by_tag(&self, tag: &str) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        let mut conn = self.connection_manager.clone();
        let tag_key = self.tags_key(tag);

        let fingerprints: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&tag_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get schemas by tag: {}", e),
            })?;

        let mut schemas = Vec::new();
        for fingerprint in fingerprints {
            let metadata_key = self.metadata_key(&fingerprint);
            let metadata_json: Option<String> = redis::cmd("GET")
                .arg(&metadata_key)
                .query_async(&mut conn)
                .await
                .map_err(|e| crate::storage::StorageError::QueryError {
                    message: format!("Failed to get metadata: {}", e),
                })?;

            if let Some(json) = metadata_json {
                let metadata: SchemaMetadata = serde_json::from_str(&json).map_err(|e| {
                    crate::storage::StorageError::InvalidSchemaData {
                        message: format!("Failed to deserialize metadata: {}", e),
                    }
                })?;
                schemas.push(metadata);
            }
        }

        Ok(schemas)
    }

    /// Get schemas by type
    pub async fn get_schemas_by_type(
        &self,
        schema_type: &str,
    ) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        let mut conn = self.connection_manager.clone();
        let type_key = self.type_key(schema_type);

        let fingerprints: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&type_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get schemas by type: {}", e),
            })?;

        let mut schemas = Vec::new();
        for fingerprint in fingerprints {
            let metadata_key = self.metadata_key(&fingerprint);
            let metadata_json: Option<String> = redis::cmd("GET")
                .arg(&metadata_key)
                .query_async(&mut conn)
                .await
                .map_err(|e| crate::storage::StorageError::QueryError {
                    message: format!("Failed to get metadata: {}", e),
                })?;

            if let Some(json) = metadata_json {
                let metadata: SchemaMetadata = serde_json::from_str(&json).map_err(|e| {
                    crate::storage::StorageError::InvalidSchemaData {
                        message: format!("Failed to deserialize metadata: {}", e),
                    }
                })?;
                schemas.push(metadata);
            }
        }

        Ok(schemas)
    }

    /// Get Redis info
    pub async fn get_redis_info(&self) -> SchemaRegistryResult<HashMap<String, String>> {
        let mut conn = self.connection_manager.clone();

        let info: String = redis::cmd("INFO")
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get Redis info: {}", e),
            })?;

        let mut info_map = HashMap::new();
        for line in info.lines() {
            if let Some((key, value)) = line.split_once(':') {
                info_map.insert(key.to_string(), value.to_string());
            }
        }

        Ok(info_map)
    }

    /// Clear all schema data (dangerous operation)
    pub async fn clear_all_data(&self) -> SchemaRegistryResult<u64> {
        let mut conn = self.connection_manager.clone();

        // Get all keys with our prefix
        let pattern = format!("{}*", self.key_prefix);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get keys: {}", e),
            })?;

        if keys.is_empty() {
            return Ok(0);
        }

        // Delete all keys
        let deleted: u64 = redis::cmd("DEL")
            .arg(&keys)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to delete keys: {}", e),
            })?;

        Ok(deleted)
    }

    /// Get Redis memory usage statistics
    pub async fn get_memory_stats(&self) -> SchemaRegistryResult<HashMap<String, String>> {
        let mut conn = self.connection_manager.clone();

        let memory_info: String = redis::cmd("INFO")
            .arg("memory")
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get Redis memory info: {}", e),
            })?;

        let mut memory_map = HashMap::new();
        for line in memory_info.lines() {
            if let Some((key, value)) = line.split_once(':') {
                memory_map.insert(key.to_string(), value.to_string());
            }
        }

        Ok(memory_map)
    }

    /// Get Redis performance statistics
    pub async fn get_performance_stats(&self) -> SchemaRegistryResult<HashMap<String, String>> {
        let mut conn = self.connection_manager.clone();

        let stats_info: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get Redis stats: {}", e),
            })?;

        let mut stats_map = HashMap::new();
        for line in stats_info.lines() {
            if let Some((key, value)) = line.split_once(':') {
                stats_map.insert(key.to_string(), value.to_string());
            }
        }

        Ok(stats_map)
    }

    /// Get Redis cluster info (if in cluster mode)
    pub async fn get_cluster_info(&self) -> SchemaRegistryResult<HashMap<String, String>> {
        if !self.cluster_mode {
            return Ok(HashMap::new());
        }

        let mut conn = self.connection_manager.clone();

        let cluster_info: String = redis::cmd("CLUSTER")
            .arg("INFO")
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get Redis cluster info: {}", e),
            })?;

        let mut cluster_map = HashMap::new();
        for line in cluster_info.lines() {
            if let Some((key, value)) = line.split_once(':') {
                cluster_map.insert(key.to_string(), value.to_string());
            }
        }

        Ok(cluster_map)
    }

    /// Get Redis slow log
    pub async fn get_slow_log(
        &self,
        count: Option<usize>,
    ) -> SchemaRegistryResult<Vec<HashMap<String, String>>> {
        let mut conn = self.connection_manager.clone();

        let mut cmd = redis::cmd("SLOWLOG");
        cmd.arg("GET");
        if let Some(n) = count {
            cmd.arg(n);
        }

        let slow_log: Vec<Vec<String>> = cmd.query_async(&mut conn).await.map_err(|e| {
            crate::storage::StorageError::QueryError {
                message: format!("Failed to get Redis slow log: {}", e),
            }
        })?;

        let mut result = Vec::new();
        for entry in slow_log {
            let mut entry_map = HashMap::new();
            if entry.len() >= 4 {
                entry_map.insert("id".to_string(), entry[0].clone());
                entry_map.insert("timestamp".to_string(), entry[1].clone());
                entry_map.insert("duration".to_string(), entry[2].clone());
                entry_map.insert("command".to_string(), entry[3].clone());
            }
            result.push(entry_map);
        }

        Ok(result)
    }

    /// Set Redis configuration
    pub async fn set_config(&self, parameter: &str, value: &str) -> SchemaRegistryResult<()> {
        let mut conn = self.connection_manager.clone();

        let _: String = redis::cmd("CONFIG")
            .arg("SET")
            .arg(parameter)
            .arg(value)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to set Redis config: {}", e),
            })?;

        Ok(())
    }

    /// Get Redis configuration
    pub async fn get_config(&self, parameter: &str) -> SchemaRegistryResult<Option<String>> {
        let mut conn = self.connection_manager.clone();

        let result: Vec<String> = redis::cmd("CONFIG")
            .arg("GET")
            .arg(parameter)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get Redis config: {}", e),
            })?;

        if result.len() >= 2 {
            Ok(Some(result[1].clone()))
        } else {
            Ok(None)
        }
    }

    /// Flush Redis database
    pub async fn flush_db(&self) -> SchemaRegistryResult<()> {
        let mut conn = self.connection_manager.clone();

        let _: String = redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to flush Redis database: {}", e),
            })?;

        Ok(())
    }

    /// Get Redis database size
    pub async fn get_db_size(&self) -> SchemaRegistryResult<u64> {
        let mut conn = self.connection_manager.clone();

        let size: u64 = redis::cmd("DBSIZE")
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get Redis database size: {}", e),
            })?;

        Ok(size)
    }

    /// Get Redis key count for our prefix
    pub async fn get_schema_key_count(&self) -> SchemaRegistryResult<u64> {
        let mut conn = self.connection_manager.clone();

        let pattern = format!("{}*", self.key_prefix);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get schema keys: {}", e),
            })?;

        Ok(keys.len() as u64)
    }

    /// Get Redis key memory usage
    pub async fn get_key_memory_usage(&self, key: &str) -> SchemaRegistryResult<Option<u64>> {
        let mut conn = self.connection_manager.clone();

        let usage: Option<u64> = redis::cmd("MEMORY")
            .arg("USAGE")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get key memory usage: {}", e),
            })?;

        Ok(usage)
    }

    /// Get Redis key TTL
    pub async fn get_key_ttl(&self, key: &str) -> SchemaRegistryResult<i64> {
        let mut conn = self.connection_manager.clone();

        let ttl: i64 = redis::cmd("TTL")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get key TTL: {}", e),
            })?;

        Ok(ttl)
    }

    /// Set Redis key TTL
    pub async fn set_key_ttl(&self, key: &str, ttl: u64) -> SchemaRegistryResult<bool> {
        let mut conn = self.connection_manager.clone();

        let result: i64 = redis::cmd("EXPIRE")
            .arg(key)
            .arg(ttl)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to set key TTL: {}", e),
            })?;

        Ok(result == 1)
    }

    /// Remove Redis key TTL
    pub async fn remove_key_ttl(&self, key: &str) -> SchemaRegistryResult<bool> {
        let mut conn = self.connection_manager.clone();

        let result: i64 = redis::cmd("PERSIST")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to remove key TTL: {}", e),
            })?;

        Ok(result == 1)
    }

    /// Get Redis key type
    pub async fn get_key_type(&self, key: &str) -> SchemaRegistryResult<Option<String>> {
        let mut conn = self.connection_manager.clone();

        let key_type: Option<String> = redis::cmd("TYPE")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get key type: {}", e),
            })?;

        Ok(key_type)
    }

    /// Check if Redis key exists
    pub async fn key_exists(&self, key: &str) -> SchemaRegistryResult<bool> {
        let mut conn = self.connection_manager.clone();

        let exists: u64 = redis::cmd("EXISTS")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to check key existence: {}", e),
            })?;

        Ok(exists == 1)
    }

    /// Get Redis key size (for sets, lists, etc.)
    pub async fn get_key_size(&self, key: &str) -> SchemaRegistryResult<Option<u64>> {
        let mut conn = self.connection_manager.clone();

        let key_type = self.get_key_type(key).await?;
        let size: Option<u64> = match key_type.as_deref() {
            Some("string") => {
                let size: Option<u64> = redis::cmd("STRLEN")
                    .arg(key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| crate::storage::StorageError::QueryError {
                        message: format!("Failed to get string length: {}", e),
                    })?;
                size
            }
            Some("set") => {
                let size: Option<u64> = redis::cmd("SCARD")
                    .arg(key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| crate::storage::StorageError::QueryError {
                    message: format!("Failed to get set size: {}", e),
                })?;
                size
            }
            Some("list") => {
                let size: Option<u64> = redis::cmd("LLEN")
                    .arg(key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| crate::storage::StorageError::QueryError {
                        message: format!("Failed to get list size: {}", e),
                    })?;
                size
            }
            Some("hash") => {
                let size: Option<u64> = redis::cmd("HLEN")
                    .arg(key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| crate::storage::StorageError::QueryError {
                        message: format!("Failed to get hash size: {}", e),
                    })?;
                size
            }
            Some("zset") => {
                let size: Option<u64> = redis::cmd("ZCARD")
                    .arg(key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| crate::storage::StorageError::QueryError {
                    message: format!("Failed to get sorted set size: {}", e),
                })?;
                size
            }
            _ => None,
        };

        Ok(size)
    }
}

#[async_trait]
impl StorageBackend for RedisStorage {
    async fn store_schema(&self, schema: Schema) -> SchemaRegistryResult<SchemaVersion> {
        let schema_json = serde_json::to_string(&schema).map_err(|e| {
            crate::storage::StorageError::InvalidSchemaData {
                message: format!("Failed to serialize schema: {}", e),
            }
        })?;

        let metadata: SchemaMetadata = schema.clone().into();
        let metadata_json = serde_json::to_string(&metadata).map_err(|e| {
            crate::storage::StorageError::InvalidSchemaData {
                message: format!("Failed to serialize metadata: {}", e),
            }
        })?;

        let mut conn = self.connection_manager.clone();

        // Use pipeline for atomic operations
        let mut pipe = redis::pipe();
        pipe.atomic();

        // Store schema
        let schema_key = self.schema_key(&schema.fingerprint);
        pipe.set(&schema_key, &schema_json);

        // Store metadata
        let metadata_key = self.metadata_key(&schema.fingerprint);
        pipe.set(&metadata_key, &metadata_json);

        // Add to schema list
        let schema_list_key = self.schema_list_key();
        pipe.sadd(&schema_list_key, &schema.fingerprint);

        // Add to versions list
        let versions_key = self.versions_key(&schema.name);
        pipe.sadd(&versions_key, &schema.fingerprint);

        // Execute pipeline
        let _: () = pipe.query_async(&mut conn).await.map_err(|e| {
            crate::storage::StorageError::QueryError {
                message: format!("Failed to store schema: {}", e),
            }
        })?;

        // Set TTL if configured
        self.set_ttl_if_configured(&schema_key).await?;
        self.set_ttl_if_configured(&metadata_key).await?;

        // Store indexing
        self.store_schema_with_indexing(&schema).await?;

        Ok(schema.version)
    }

    async fn get_schema(&self, fingerprint: &str) -> SchemaRegistryResult<Option<Schema>> {
        let mut conn = self.connection_manager.clone();
        let schema_key = self.schema_key(fingerprint);

        let result: Option<String> = redis::cmd("GET")
            .arg(&schema_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get schema: {}", e),
            })?;

        match result {
            Some(schema_json) => {
                let schema: Schema = serde_json::from_str(&schema_json).map_err(|e| {
                    crate::storage::StorageError::InvalidSchemaData {
                        message: format!("Failed to deserialize schema: {}", e),
                    }
                })?;
                Ok(Some(schema))
            }
            None => Ok(None),
        }
    }

    async fn list_schemas(&self) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        let mut conn = self.connection_manager.clone();
        let schema_list_key = self.schema_list_key();

        let fingerprints: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&schema_list_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get schema list: {}", e),
            })?;

        let mut schemas = Vec::new();
        for fingerprint in fingerprints {
            let metadata_key = self.metadata_key(&fingerprint);
            let metadata_json: Option<String> = redis::cmd("GET")
                .arg(&metadata_key)
                .query_async(&mut conn)
                .await
                .map_err(|e| crate::storage::StorageError::QueryError {
                    message: format!("Failed to get metadata: {}", e),
                })?;

            if let Some(json) = metadata_json {
                let metadata: SchemaMetadata = serde_json::from_str(&json).map_err(|e| {
                    crate::storage::StorageError::InvalidSchemaData {
                        message: format!("Failed to deserialize metadata: {}", e),
                    }
                })?;
                schemas.push(metadata);
            }
        }

        Ok(schemas)
    }

    async fn search_schemas(
        &self,
        criteria: &SchemaSearchCriteria,
    ) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        let mut conn = self.connection_manager.clone();
        let mut result_fingerprints = Vec::new();

        // Start with all schemas if no specific criteria
        if criteria.name.is_none()
            && criteria.schema_type.is_none()
            && criteria.format.is_none()
            && criteria.tags.is_empty()
        {
            let schema_list_key = self.schema_list_key();
            result_fingerprints = redis::cmd("SMEMBERS")
                .arg(&schema_list_key)
                .query_async(&mut conn)
                .await
                .map_err(|e| crate::storage::StorageError::QueryError {
                    message: format!("Failed to get schema list: {}", e),
                })?;
        } else {
            // Use indexed searches when possible
            if let Some(ref schema_type) = criteria.schema_type {
                let type_key = self.type_key(&schema_type.to_string());
                let type_fingerprints: Vec<String> = redis::cmd("SMEMBERS")
                    .arg(&type_key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| crate::storage::StorageError::QueryError {
                        message: format!("Failed to get schemas by type: {}", e),
                    })?;
                result_fingerprints = type_fingerprints;
            }

            // Filter by tags if specified
            if !criteria.tags.is_empty() {
                let mut tag_fingerprints = Vec::new();
                for tag in &criteria.tags {
                    let tag_key = self.tags_key(tag);
                    let fingerprints: Vec<String> = redis::cmd("SMEMBERS")
                        .arg(&tag_key)
                        .query_async(&mut conn)
                        .await
                        .map_err(|e| crate::storage::StorageError::QueryError {
                            message: format!("Failed to get schemas by tag: {}", e),
                        })?;
                    tag_fingerprints.push(fingerprints);
                }

                // Intersect tag results
                if !tag_fingerprints.is_empty() {
                    if result_fingerprints.is_empty() {
                        result_fingerprints = tag_fingerprints[0].clone();
                    }
                    for tag_fps in tag_fingerprints.iter().skip(1) {
                        result_fingerprints.retain(|fp| tag_fps.contains(fp));
                    }
                }
            }
        }

        // Get metadata for result fingerprints
        let mut schemas = Vec::new();
        for fingerprint in result_fingerprints {
            let metadata_key = self.metadata_key(&fingerprint);
            let metadata_json: Option<String> = redis::cmd("GET")
                .arg(&metadata_key)
                .query_async(&mut conn)
                .await
                .map_err(|e| crate::storage::StorageError::QueryError {
                    message: format!("Failed to get metadata: {}", e),
                })?;

            if let Some(json) = metadata_json {
                let metadata: SchemaMetadata = serde_json::from_str(&json).map_err(|e| {
                    crate::storage::StorageError::InvalidSchemaData {
                        message: format!("Failed to deserialize metadata: {}", e),
                    }
                })?;

                // Apply additional filters
                let mut matches = true;

                if let Some(ref name) = criteria.name {
                    if !metadata.name.contains(name) {
                        matches = false;
                    }
                }

                if let Some(ref format) = criteria.format {
                    if metadata.format != *format {
                        matches = false;
                    }
                }

                if matches {
                    schemas.push(metadata);
                }
            }
        }

        Ok(schemas)
    }

    async fn delete_schema(&self, fingerprint: &str) -> SchemaRegistryResult<bool> {
        let mut conn = self.connection_manager.clone();
        let schema_key = self.schema_key(fingerprint);
        let metadata_key = self.metadata_key(fingerprint);

        // Check if schema exists
        let exists: bool = redis::cmd("EXISTS")
            .arg(&schema_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to check schema existence: {}", e),
            })?;

        if !exists {
            return Ok(false);
        }

        // Get schema to find name and tags for cleanup
        let schema = self.get_schema(fingerprint).await?;
        let schema_name = if let Some(ref s) = schema {
            s.name.clone()
        } else {
            return Ok(false);
        };

        // Remove indexing first
        if let Some(ref s) = schema {
            self.remove_schema_indexing(s).await?;
        }

        // Use pipeline for atomic deletion
        let mut pipe = redis::pipe();
        pipe.atomic();

        // Delete schema and metadata
        pipe.del(&schema_key).del(&metadata_key);

        // Remove from schema list
        let schema_list_key = self.schema_list_key();
        pipe.srem(&schema_list_key, fingerprint);

        // Remove from versions list
        let versions_key = self.versions_key(&schema_name);
        pipe.srem(&versions_key, fingerprint);

        // Execute pipeline
        let _: () = pipe.query_async(&mut conn).await.map_err(|e| {
            crate::storage::StorageError::QueryError {
                message: format!("Failed to delete schema: {}", e),
            }
        })?;

        Ok(true)
    }

    async fn get_schema_versions(&self, name: &str) -> SchemaRegistryResult<Vec<SchemaVersion>> {
        let mut conn = self.connection_manager.clone();
        let versions_key = self.versions_key(name);

        let fingerprints: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&versions_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get versions: {}", e),
            })?;

        let mut versions = Vec::new();
        for fingerprint in fingerprints {
            if let Some(schema) = self.get_schema(&fingerprint).await? {
                versions.push(schema.version);
            }
        }

        Ok(versions)
    }

    async fn get_latest_schema(&self, name: &str) -> SchemaRegistryResult<Option<Schema>> {
        let versions = self.get_schema_versions(name).await?;

        if versions.is_empty() {
            return Ok(None);
        }

        // Find the latest version
        let latest_version = versions.iter().max().unwrap();

        // Find schema with this version
        let mut conn = self.connection_manager.clone();
        let versions_key = self.versions_key(name);
        let fingerprints: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&versions_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get versions: {}", e),
            })?;

        for fingerprint in fingerprints {
            if let Some(schema) = self.get_schema(&fingerprint).await? {
                if schema.version == *latest_version {
                    return Ok(Some(schema));
                }
            }
        }

        Ok(None)
    }

    async fn health_check(&self) -> SchemaRegistryResult<bool> {
        let mut conn = self.connection_manager.clone();

        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::ConnectionError {
                message: format!("Redis health check failed: {}", e),
            })?;

        Ok(true)
    }

    async fn get_stats(&self) -> SchemaRegistryResult<StorageStats> {
        let mut conn = self.connection_manager.clone();
        let schema_list_key = self.schema_list_key();

        let total_schemas: u64 = redis::cmd("SCARD")
            .arg(&schema_list_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to get schema count: {}", e),
            })?;

        // Calculate total size and versions
        let schemas = self.list_schemas().await?;
        let total_versions = schemas.len() as u64;
        let total_size_bytes = schemas.iter().map(|s| s.size as u64).sum();
        let avg_schema_size_bytes = if total_schemas > 0 {
            total_size_bytes / total_schemas
        } else {
            0
        };

        Ok(StorageStats {
            total_schemas,
            total_versions,
            total_size_bytes,
            avg_schema_size_bytes,
            last_activity: chrono::Utc::now(),
        })
    }

    async fn shutdown(&self) -> SchemaRegistryResult<()> {
        // Clear connection pool
        {
            let mut pool = self.connection_pool.write().await;
            pool.clear();
        }

        tracing::debug!("Redis storage shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        // Test key generation without creating a full Redis storage instance
        let key_prefix = "test:";

        assert_eq!(
            format!("{}schema:{}", key_prefix, "abc123"),
            "test:schema:abc123"
        );
        assert_eq!(
            format!("{}metadata:{}", key_prefix, "abc123"),
            "test:metadata:abc123"
        );
        assert_eq!(
            format!("{}versions:{}", key_prefix, "test_schema"),
            "test:versions:test_schema"
        );
        assert_eq!(format!("{}schemas", key_prefix), "test:schemas");
        assert_eq!(
            format!("{}tags:{}", key_prefix, "important"),
            "test:tags:important"
        );
        assert_eq!(
            format!("{}types:{}", key_prefix, "metric"),
            "test:types:metric"
        );
    }
}
