//! Redis storage implementation

use crate::error::{SchemaRegistryError, SchemaRegistryResult};
use crate::schema::{Schema, SchemaMetadata, SchemaSearchCriteria, SchemaVersion};
use crate::storage::{StorageBackend, StorageStats};
use async_trait::async_trait;

/// Redis storage implementation
pub struct RedisStorage {
    /// Redis client
    client: redis::Client,

    /// Redis connection manager
    connection_manager: redis::aio::ConnectionManager,

    /// Key prefix for schema storage
    key_prefix: String,
}

impl RedisStorage {
    /// Create a new Redis storage instance
    pub async fn new(url: String) -> SchemaRegistryResult<Self> {
        let client = redis::Client::open(url).map_err(|e| crate::storage::StorageError::ConfigurationError {
            message: format!("Failed to create Redis client: {}", e),
        })?;

        let connection_manager = redis::aio::ConnectionManager::new(client.clone())
            .await
            .map_err(|e| crate::storage::StorageError::ConnectionError {
                message: format!("Failed to create Redis connection manager: {}", e),
            })?;

        Ok(Self {
            client,
            connection_manager,
            key_prefix: "schema_registry:".to_string(),
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

        Ok(Self {
            client,
            connection_manager,
            key_prefix: config.key_prefix.clone(),
        })
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
}

#[async_trait]
impl StorageBackend for RedisStorage {
    async fn store_schema(&self, schema: Schema) -> SchemaRegistryResult<SchemaVersion> {
        let schema_json =
            serde_json::to_string(&schema).map_err(|e| crate::storage::StorageError::InvalidSchemaData {
                message: format!("Failed to serialize schema: {}", e),
            })?;

        let metadata: SchemaMetadata = schema.clone().into();
        let metadata_json =
            serde_json::to_string(&metadata).map_err(|e| crate::storage::StorageError::InvalidSchemaData {
                message: format!("Failed to serialize metadata: {}", e),
            })?;

        let mut conn = self.connection_manager.clone();

        // Store schema
        let schema_key = self.schema_key(&schema.fingerprint);
        let _: () = redis::cmd("SET")
            .arg(&schema_key)
            .arg(&schema_json)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to store schema: {}", e),
            })?;

        // Store metadata
        let metadata_key = self.metadata_key(&schema.fingerprint);
        let _: () = redis::cmd("SET")
            .arg(&metadata_key)
            .arg(&metadata_json)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to store metadata: {}", e),
            })?;

        // Add to schema list
        let schema_list_key = self.schema_list_key();
        let _: () = redis::cmd("SADD")
            .arg(&schema_list_key)
            .arg(&schema.fingerprint)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to add to schema list: {}", e),
            })?;

        // Add to versions list
        let versions_key = self.versions_key(&schema.name);
        let _: () = redis::cmd("SADD")
            .arg(&versions_key)
            .arg(&schema.fingerprint)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to add to versions list: {}", e),
            })?;

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
                let metadata: SchemaMetadata =
                    serde_json::from_str(&json).map_err(|e| crate::storage::StorageError::InvalidSchemaData {
                        message: format!("Failed to deserialize metadata: {}", e),
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
        let all_schemas = self.list_schemas().await?;
        let mut filtered_schemas = Vec::new();

        for schema in all_schemas {
            let mut matches = true;

            if let Some(ref name) = criteria.name {
                if !schema.name.contains(name) {
                    matches = false;
                }
            }

            if let Some(ref schema_type) = criteria.schema_type {
                if schema.schema_type != *schema_type {
                    matches = false;
                }
            }

            if let Some(ref format) = criteria.format {
                if schema.format != *format {
                    matches = false;
                }
            }

            if !criteria.tags.is_empty() {
                let mut has_matching_tag = false;
                for tag in &criteria.tags {
                    if schema.tags.contains(tag) {
                        has_matching_tag = true;
                        break;
                    }
                }
                if !has_matching_tag {
                    matches = false;
                }
            }

            if matches {
                filtered_schemas.push(schema);
            }
        }

        Ok(filtered_schemas)
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

        // Get schema to find name for version cleanup
        let schema = self.get_schema(fingerprint).await?;
        let schema_name = if let Some(ref s) = schema {
            s.name.clone()
        } else {
            return Ok(false);
        };

        // Delete schema and metadata
        let _: () = redis::cmd("DEL")
            .arg(&schema_key)
            .arg(&metadata_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to delete schema: {}", e),
            })?;

        // Remove from schema list
        let schema_list_key = self.schema_list_key();
        let _: () = redis::cmd("SREM")
            .arg(&schema_list_key)
            .arg(fingerprint)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to remove from schema list: {}", e),
            })?;

        // Remove from versions list
        let versions_key = self.versions_key(&schema_name);
        let _: () = redis::cmd("SREM")
            .arg(&versions_key)
            .arg(fingerprint)
            .query_async(&mut conn)
            .await
            .map_err(|e| crate::storage::StorageError::QueryError {
                message: format!("Failed to remove from versions list: {}", e),
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
        // For Redis, the connection manager will be dropped automatically
        // when the struct is dropped, but we can log the shutdown
        tracing::debug!("Redis storage shutdown completed");
        Ok(())
    }
}
