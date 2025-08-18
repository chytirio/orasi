//! In-memory storage implementation

use crate::error::{SchemaRegistryError, SchemaRegistryResult};
use crate::schema::{Schema, SchemaMetadata, SchemaSearchCriteria, SchemaVersion};
use crate::storage::{StorageBackend, StorageStats};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory storage implementation
pub struct MemoryStorage {
    /// Schema storage
    schemas: Arc<RwLock<HashMap<String, Schema>>>,

    /// Schema metadata index
    metadata_index: Arc<RwLock<HashMap<String, SchemaMetadata>>>,

    /// Name to fingerprint mapping
    name_to_fingerprint: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl MemoryStorage {
    /// Create a new memory storage instance
    pub fn new() -> SchemaRegistryResult<Self> {
        Ok(Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            metadata_index: Arc::new(RwLock::new(HashMap::new())),
            name_to_fingerprint: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Update metadata index
    async fn update_metadata_index(&self, schema: &Schema) -> SchemaRegistryResult<()> {
        let metadata: SchemaMetadata = schema.clone().into();
        let fingerprint = schema.fingerprint.clone();
        let name = schema.name.clone();

        // Update metadata index
        {
            let mut index = self.metadata_index.write().await;
            index.insert(fingerprint.clone(), metadata);
        }

        // Update name to fingerprint mapping
        {
            let mut name_map = self.name_to_fingerprint.write().await;
            name_map
                .entry(name)
                .or_insert_with(Vec::new)
                .push(fingerprint);
        }

        Ok(())
    }

    /// Remove from metadata index
    async fn remove_from_metadata_index(&self, fingerprint: &str) -> SchemaRegistryResult<()> {
        // Remove from metadata index
        {
            let mut index = self.metadata_index.write().await;
            index.remove(fingerprint);
        }

        // Remove from name to fingerprint mapping
        {
            let mut name_map = self.name_to_fingerprint.write().await;
            for fingerprints in name_map.values_mut() {
                fingerprints.retain(|f| f != fingerprint);
            }
            // Clean up empty entries
            name_map.retain(|_, fingerprints| !fingerprints.is_empty());
        }

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn store_schema(&self, schema: Schema) -> SchemaRegistryResult<SchemaVersion> {
        let fingerprint = schema.fingerprint.clone();
        let version = schema.version.clone();

        // Store schema
        {
            let mut schemas = self.schemas.write().await;
            schemas.insert(fingerprint.clone(), schema.clone());
        }

        // Update metadata index
        self.update_metadata_index(&schema).await?;

        Ok(version)
    }

    async fn get_schema(&self, fingerprint: &str) -> SchemaRegistryResult<Option<Schema>> {
        let schemas = self.schemas.read().await;
        Ok(schemas.get(fingerprint).cloned())
    }

    async fn list_schemas(&self) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        let index = self.metadata_index.read().await;
        Ok(index.values().cloned().collect())
    }

    async fn search_schemas(
        &self,
        criteria: &SchemaSearchCriteria,
    ) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        let index = self.metadata_index.read().await;
        let mut results = Vec::new();

        for metadata in index.values() {
            // Apply name filter
            if let Some(name_filter) = &criteria.name {
                if !metadata
                    .name
                    .to_lowercase()
                    .contains(&name_filter.to_lowercase())
                {
                    continue;
                }
            }

            // Apply type filter
            if let Some(schema_type) = &criteria.schema_type {
                if metadata.schema_type != *schema_type {
                    continue;
                }
            }

            // Apply format filter
            if let Some(format) = &criteria.format {
                if metadata.format != *format {
                    continue;
                }
            }

            // Apply tags filter
            if !criteria.tags.is_empty() {
                let has_all_tags = criteria.tags.iter().all(|tag| metadata.tags.contains(tag));
                if !has_all_tags {
                    continue;
                }
            }

            // Apply owner filter
            if let Some(owner) = &criteria.owner {
                if metadata.owner.as_ref() != Some(owner) {
                    continue;
                }
            }

            // Apply visibility filter
            if let Some(visibility) = &criteria.visibility {
                if metadata.visibility != *visibility {
                    continue;
                }
            }

            // Apply date filters
            if let Some(created_after) = criteria.created_after {
                if metadata.created_at < created_after {
                    continue;
                }
            }

            if let Some(created_before) = criteria.created_before {
                if metadata.created_at > created_before {
                    continue;
                }
            }

            if let Some(updated_after) = criteria.updated_after {
                if metadata.updated_at < updated_after {
                    continue;
                }
            }

            if let Some(updated_before) = criteria.updated_before {
                if metadata.updated_at > updated_before {
                    continue;
                }
            }

            results.push(metadata.clone());
        }

        // Apply pagination
        let offset = criteria.offset.unwrap_or(0);
        let limit = criteria.limit.unwrap_or(100);

        results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        results = results.into_iter().skip(offset).take(limit).collect();

        Ok(results)
    }

    async fn delete_schema(&self, fingerprint: &str) -> SchemaRegistryResult<bool> {
        // Remove from schemas
        let removed = {
            let mut schemas = self.schemas.write().await;
            schemas.remove(fingerprint).is_some()
        };

        if removed {
            // Remove from metadata index
            self.remove_from_metadata_index(fingerprint).await?;
        }

        Ok(removed)
    }

    async fn get_schema_versions(&self, name: &str) -> SchemaRegistryResult<Vec<SchemaVersion>> {
        let name_map = self.name_to_fingerprint.read().await;
        let mut versions = Vec::new();

        if let Some(fingerprints) = name_map.get(name) {
            let index = self.metadata_index.read().await;
            for fingerprint in fingerprints {
                if let Some(metadata) = index.get(fingerprint) {
                    versions.push(metadata.version.clone());
                }
            }
        }

        versions.sort();
        Ok(versions)
    }

    async fn get_latest_schema(&self, name: &str) -> SchemaRegistryResult<Option<Schema>> {
        let name_map = self.name_to_fingerprint.read().await;

        if let Some(fingerprints) = name_map.get(name) {
            let index = self.metadata_index.read().await;
            let mut latest_metadata: Option<&SchemaMetadata> = None;

            for fingerprint in fingerprints {
                if let Some(metadata) = index.get(fingerprint) {
                    match latest_metadata {
                        Some(latest) => {
                            if metadata.version > latest.version {
                                latest_metadata = Some(metadata);
                            }
                        }
                        None => {
                            latest_metadata = Some(metadata);
                        }
                    }
                }
            }

            if let Some(metadata) = latest_metadata {
                let schemas = self.schemas.read().await;
                return Ok(schemas.get(&metadata.fingerprint).cloned());
            }
        }

        Ok(None)
    }

    async fn health_check(&self) -> SchemaRegistryResult<bool> {
        // Basic health check - ensure we can read and write
        let test_schema = crate::schema::Schema::new(
            "health-check".to_string(),
            crate::schema::SchemaVersion::new(1, 0, 0),
            crate::schema::SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            crate::schema::SchemaFormat::Json,
        );

        let fingerprint = test_schema.fingerprint.clone();

        // Try to store and retrieve
        self.store_schema(test_schema).await?;
        let retrieved = self.get_schema(&fingerprint).await?;

        // Clean up
        self.delete_schema(&fingerprint).await?;

        Ok(retrieved.is_some())
    }

    async fn get_stats(&self) -> SchemaRegistryResult<StorageStats> {
        let schemas = self.schemas.read().await;
        let index = self.metadata_index.read().await;

        let total_schemas = schemas.len() as u64;
        let total_versions = index.len() as u64;

        let total_size: usize = schemas.values().map(|s| s.size()).sum();
        let avg_size = if total_schemas > 0 {
            total_size / total_schemas as usize
        } else {
            0
        };

        let last_activity = index
            .values()
            .map(|m| m.updated_at)
            .max()
            .unwrap_or_else(chrono::Utc::now);

        Ok(StorageStats {
            total_schemas,
            total_versions,
            total_size_bytes: total_size as u64,
            avg_schema_size_bytes: avg_size as u64,
            last_activity,
        })
    }

    async fn shutdown(&self) -> SchemaRegistryResult<()> {
        // For in-memory storage, we just clear the data structures
        // This is optional but good for cleanup
        {
            let mut schemas = self.schemas.write().await;
            schemas.clear();
        }
        {
            let mut index = self.metadata_index.write().await;
            index.clear();
        }
        {
            let mut name_map = self.name_to_fingerprint.write().await;
            name_map.clear();
        }

        tracing::debug!("Memory storage shutdown completed");
        Ok(())
    }
}
