//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Tenant storage abstraction

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::{config::Config, primitives::ByteStream, Client as S3Client};
use redis::{aio::ConnectionManager, AsyncCommands, Client, Commands, Connection, RedisResult};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{MultiTenancyError, MultiTenancyResult};
use crate::models::Tenant;

/// Tenant storage trait
#[async_trait]
pub trait TenantStorage: Send + Sync {
    /// Store a tenant
    async fn store_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()>;

    /// Get a tenant by ID
    async fn get_tenant(&self, tenant_id: &str) -> MultiTenancyResult<Tenant>;

    /// Update a tenant
    async fn update_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()>;

    /// Delete a tenant
    async fn delete_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()>;

    /// List all tenants
    async fn list_tenants(&self) -> MultiTenancyResult<Vec<Tenant>>;

    /// Search tenants by criteria
    async fn search_tenants(
        &self,
        criteria: TenantSearchCriteria,
    ) -> MultiTenancyResult<Vec<Tenant>>;
}

/// Tenant search criteria
#[derive(Debug, Clone)]
pub struct TenantSearchCriteria {
    /// Tenant status filter
    pub status: Option<crate::models::TenantStatus>,
    /// Tenant type filter
    pub tenant_type: Option<crate::models::TenantType>,
    /// Owner user ID filter
    pub owner_user_id: Option<String>,
    /// Name pattern filter
    pub name_pattern: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl Default for TenantSearchCriteria {
    fn default() -> Self {
        Self {
            status: None,
            tenant_type: None,
            owner_user_id: None,
            name_pattern: None,
            limit: None,
            offset: None,
        }
    }
}

/// Mock tenant storage for testing
pub struct MockTenantStorage {
    tenants: Arc<RwLock<HashMap<String, Tenant>>>,
}

impl MockTenantStorage {
    /// Create a new mock storage
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new mock storage with initial tenants
    pub fn with_tenants(tenants: Vec<Tenant>) -> Self {
        let mut map = HashMap::new();
        for tenant in tenants {
            map.insert(tenant.id.clone(), tenant);
        }

        Self {
            tenants: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl TenantStorage for MockTenantStorage {
    async fn store_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        let mut tenants = self.tenants.write().await;
        tenants.insert(tenant.id.clone(), tenant.clone());
        Ok(())
    }

    async fn get_tenant(&self, tenant_id: &str) -> MultiTenancyResult<Tenant> {
        let tenants = self.tenants.read().await;
        tenants
            .get(tenant_id)
            .cloned()
            .ok_or_else(|| MultiTenancyError::tenant_not_found(tenant_id))
    }

    async fn update_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        let mut tenants = self.tenants.write().await;
        tenants.insert(tenant.id.clone(), tenant.clone());
        Ok(())
    }

    async fn delete_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        let mut tenants = self.tenants.write().await;
        tenants.remove(tenant_id);
        Ok(())
    }

    async fn list_tenants(&self) -> MultiTenancyResult<Vec<Tenant>> {
        let tenants = self.tenants.read().await;
        Ok(tenants.values().cloned().collect())
    }

    async fn search_tenants(
        &self,
        criteria: TenantSearchCriteria,
    ) -> MultiTenancyResult<Vec<Tenant>> {
        let tenants = self.tenants.read().await;
        let mut results: Vec<Tenant> = tenants.values().cloned().collect();

        // Apply filters
        if let Some(status) = criteria.status {
            results.retain(|t| t.status == status);
        }

        if let Some(tenant_type) = criteria.tenant_type {
            results.retain(|t| t.tenant_type == tenant_type);
        }

        if let Some(owner_user_id) = criteria.owner_user_id {
            results.retain(|t| {
                t.owner_user_id
                    .as_ref()
                    .map_or(false, |id| id == &owner_user_id)
            });
        }

        if let Some(name_pattern) = criteria.name_pattern {
            results.retain(|t| t.name.contains(&name_pattern));
        }

        // Apply pagination
        if let Some(offset) = criteria.offset {
            if offset < results.len() {
                results = results[offset..].to_vec();
            } else {
                results.clear();
            }
        }

        if let Some(limit) = criteria.limit {
            if limit < results.len() {
                results = results[..limit].to_vec();
            }
        }

        Ok(results)
    }
}

impl Default for MockTenantStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Database tenant storage implementation
pub struct DatabaseTenantStorage {
    // Database connection pool would go here
    // For now, we'll use a mock implementation
    mock_storage: MockTenantStorage,
}

impl DatabaseTenantStorage {
    /// Create a new database storage
    pub fn new(_database_url: &str) -> Self {
        Self {
            mock_storage: MockTenantStorage::new(),
        }
    }
}

#[async_trait]
impl TenantStorage for DatabaseTenantStorage {
    async fn store_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        self.mock_storage.store_tenant(tenant).await
    }

    async fn get_tenant(&self, tenant_id: &str) -> MultiTenancyResult<Tenant> {
        self.mock_storage.get_tenant(tenant_id).await
    }

    async fn update_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        self.mock_storage.update_tenant(tenant).await
    }

    async fn delete_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        self.mock_storage.delete_tenant(tenant_id).await
    }

    async fn list_tenants(&self) -> MultiTenancyResult<Vec<Tenant>> {
        self.mock_storage.list_tenants().await
    }

    async fn search_tenants(
        &self,
        criteria: TenantSearchCriteria,
    ) -> MultiTenancyResult<Vec<Tenant>> {
        self.mock_storage.search_tenants(criteria).await
    }
}

/// File-based tenant storage implementation
pub struct FileTenantStorage {
    file_path: String,
    mock_storage: MockTenantStorage,
}

impl FileTenantStorage {
    /// Create a new file storage
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            mock_storage: MockTenantStorage::new(),
        }
    }
}

#[async_trait]
impl TenantStorage for FileTenantStorage {
    async fn store_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        self.mock_storage.store_tenant(tenant).await
    }

    async fn get_tenant(&self, tenant_id: &str) -> MultiTenancyResult<Tenant> {
        self.mock_storage.get_tenant(tenant_id).await
    }

    async fn update_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        self.mock_storage.update_tenant(tenant).await
    }

    async fn delete_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        self.mock_storage.delete_tenant(tenant_id).await
    }

    async fn list_tenants(&self) -> MultiTenancyResult<Vec<Tenant>> {
        self.mock_storage.list_tenants().await
    }

    async fn search_tenants(
        &self,
        criteria: TenantSearchCriteria,
    ) -> MultiTenancyResult<Vec<Tenant>> {
        self.mock_storage.search_tenants(criteria).await
    }
}

/// Redis-based tenant storage implementation
pub struct RedisTenantStorage {
    connection_manager: ConnectionManager,
    key_prefix: String,
}

impl RedisTenantStorage {
    /// Create a new Redis storage
    pub async fn new(redis_url: &str) -> MultiTenancyResult<Self> {
        let client = Client::open(redis_url).map_err(|e| {
            MultiTenancyError::configuration_error(&format!("Failed to create Redis client: {}", e))
        })?;

        let connection_manager = ConnectionManager::new(client).await.map_err(|e| {
            MultiTenancyError::configuration_error(&format!("Failed to connect to Redis: {}", e))
        })?;

        Ok(Self {
            connection_manager,
            key_prefix: "tenant:".to_string(),
        })
    }

    /// Get the Redis key for a tenant
    fn tenant_key(&self, tenant_id: &str) -> String {
        format!("{}{}", self.key_prefix, tenant_id)
    }

    /// Get the Redis key for tenant list
    fn tenant_list_key(&self) -> String {
        format!("{}list", self.key_prefix)
    }
}

#[async_trait]
impl TenantStorage for RedisTenantStorage {
    async fn store_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        let tenant_json = serde_json::to_string(tenant).map_err(|e| {
            MultiTenancyError::storage_error(&format!("Failed to serialize tenant: {}", e))
        })?;

        let mut conn = self.connection_manager.clone();

        // Store tenant data
        let tenant_key = self.tenant_key(&tenant.id);
        conn.set::<_, _, ()>(&tenant_key, &tenant_json)
            .await
            .map_err(|e| {
                MultiTenancyError::storage_error(&format!("Failed to store tenant in Redis: {}", e))
            })?;

        // Add to tenant list for listing/searching
        conn.sadd::<_, _, ()>(&self.tenant_list_key(), &tenant.id)
            .await
            .map_err(|e| {
                MultiTenancyError::storage_error(&format!("Failed to add tenant to list: {}", e))
            })?;

        Ok(())
    }

    async fn get_tenant(&self, tenant_id: &str) -> MultiTenancyResult<Tenant> {
        let mut conn = self.connection_manager.clone();
        let tenant_key = self.tenant_key(tenant_id);

        let tenant_json: Option<String> = conn.get(&tenant_key).await.map_err(|e| {
            MultiTenancyError::storage_error(&format!("Failed to get tenant from Redis: {}", e))
        })?;

        match tenant_json {
            Some(json) => serde_json::from_str(&json).map_err(|e| {
                MultiTenancyError::storage_error(&format!("Failed to deserialize tenant: {}", e))
            }),
            None => Err(MultiTenancyError::tenant_not_found(tenant_id)),
        }
    }

    async fn update_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        // For Redis, update is the same as store
        self.store_tenant(tenant).await
    }

    async fn delete_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        let mut conn = self.connection_manager.clone();
        let tenant_key = self.tenant_key(tenant_id);

        // Remove tenant data
        conn.del::<_, ()>(&tenant_key).await.map_err(|e| {
            MultiTenancyError::storage_error(&format!("Failed to delete tenant from Redis: {}", e))
        })?;

        // Remove from tenant list
        conn.srem::<_, _, ()>(&self.tenant_list_key(), tenant_id)
            .await
            .map_err(|e| {
                MultiTenancyError::storage_error(&format!(
                    "Failed to remove tenant from list: {}",
                    e
                ))
            })?;

        Ok(())
    }

    async fn list_tenants(&self) -> MultiTenancyResult<Vec<Tenant>> {
        let mut conn = self.connection_manager.clone();

        // Get all tenant IDs from the set
        let tenant_ids: Vec<String> =
            conn.smembers(&self.tenant_list_key()).await.map_err(|e| {
                MultiTenancyError::storage_error(&format!(
                    "Failed to get tenant list from Redis: {}",
                    e
                ))
            })?;

        let mut tenants = Vec::new();
        for tenant_id in tenant_ids {
            match self.get_tenant(&tenant_id).await {
                Ok(tenant) => tenants.push(tenant),
                Err(e) => {
                    tracing::warn!("Failed to get tenant {}: {}", tenant_id, e);
                    // Continue with other tenants
                }
            }
        }

        Ok(tenants)
    }

    async fn search_tenants(
        &self,
        criteria: TenantSearchCriteria,
    ) -> MultiTenancyResult<Vec<Tenant>> {
        // Get all tenants first, then filter in memory
        // For production, you might want to use Redis search capabilities
        let all_tenants = self.list_tenants().await?;
        let mut results = all_tenants;

        // Apply filters
        if let Some(status) = criteria.status {
            results.retain(|t| t.status == status);
        }

        if let Some(tenant_type) = criteria.tenant_type {
            results.retain(|t| t.tenant_type == tenant_type);
        }

        if let Some(owner_user_id) = criteria.owner_user_id {
            results.retain(|t| {
                t.owner_user_id
                    .as_ref()
                    .map_or(false, |id| id == &owner_user_id)
            });
        }

        if let Some(name_pattern) = criteria.name_pattern {
            results.retain(|t| t.name.contains(&name_pattern));
        }

        // Apply pagination
        if let Some(offset) = criteria.offset {
            if offset < results.len() {
                results = results[offset..].to_vec();
            } else {
                results.clear();
            }
        }

        if let Some(limit) = criteria.limit {
            if limit < results.len() {
                results = results[..limit].to_vec();
            }
        }

        Ok(results)
    }
}

/// S3-based tenant storage implementation
pub struct S3TenantStorage {
    s3_client: S3Client,
    bucket: String,
    key_prefix: String,
}

impl S3TenantStorage {
    /// Create a new S3 storage
    pub async fn new(s3_config: &crate::config::S3Config) -> MultiTenancyResult<Self> {
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(s3_config.region.clone()))
            .load()
            .await;

        let s3_config_builder = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Some(aws_config.region().unwrap().clone()));

        let s3_config_builder = if let (Some(access_key), Some(secret_key)) =
            (&s3_config.access_key_id, &s3_config.secret_access_key)
        {
            s3_config_builder.credentials_provider(Credentials::new(
                access_key, secret_key, None, None, "static",
            ))
        } else {
            s3_config_builder
        };

        let s3_config_builder = if let Some(endpoint_url) = &s3_config.endpoint_url {
            s3_config_builder.endpoint_url(endpoint_url)
        } else {
            s3_config_builder
        };

        let s3_client = S3Client::from_conf(s3_config_builder.build());

        Ok(Self {
            s3_client,
            bucket: s3_config.bucket.clone(),
            key_prefix: "tenants/".to_string(),
        })
    }

    /// Get the S3 key for a tenant
    fn tenant_key(&self, tenant_id: &str) -> String {
        format!("{}{}.json", self.key_prefix, tenant_id)
    }

    /// Get the S3 key for tenant list
    fn tenant_list_key(&self) -> String {
        format!("{}list.json", self.key_prefix)
    }
}

#[async_trait]
impl TenantStorage for S3TenantStorage {
    async fn store_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        let tenant_json = serde_json::to_string(tenant).map_err(|e| {
            MultiTenancyError::storage_error(&format!("Failed to serialize tenant: {}", e))
        })?;

        let tenant_key = self.tenant_key(&tenant.id);

        // Store tenant data
        self.s3_client
            .put_object()
            .bucket(&self.bucket)
            .key(&tenant_key)
            .body(ByteStream::from(tenant_json.into_bytes()))
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| {
                MultiTenancyError::storage_error(&format!("Failed to store tenant in S3: {}", e))
            })?;

        // Update tenant list
        self.update_tenant_list(tenant).await?;

        Ok(())
    }

    async fn get_tenant(&self, tenant_id: &str) -> MultiTenancyResult<Tenant> {
        let tenant_key = self.tenant_key(tenant_id);

        let response = self
            .s3_client
            .get_object()
            .bucket(&self.bucket)
            .key(&tenant_key)
            .send()
            .await
            .map_err(|e| {
                MultiTenancyError::storage_error(&format!("Failed to get tenant from S3: {}", e))
            })?;

        let tenant_data = response.body.collect().await.map_err(|e| {
            MultiTenancyError::storage_error(&format!("Failed to read tenant data from S3: {}", e))
        })?;

        let tenant_json = String::from_utf8(tenant_data.to_vec()).map_err(|e| {
            MultiTenancyError::storage_error(&format!(
                "Failed to convert tenant data to string: {}",
                e
            ))
        })?;

        serde_json::from_str(&tenant_json).map_err(|e| {
            MultiTenancyError::storage_error(&format!("Failed to deserialize tenant: {}", e))
        })
    }

    async fn update_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        // For S3, update is the same as store
        self.store_tenant(tenant).await
    }

    async fn delete_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        let tenant_key = self.tenant_key(tenant_id);

        // Delete tenant data
        self.s3_client
            .delete_object()
            .bucket(&self.bucket)
            .key(&tenant_key)
            .send()
            .await
            .map_err(|e| {
                MultiTenancyError::storage_error(&format!("Failed to delete tenant from S3: {}", e))
            })?;

        // Update tenant list
        self.remove_from_tenant_list(tenant_id).await?;

        Ok(())
    }

    async fn list_tenants(&self) -> MultiTenancyResult<Vec<Tenant>> {
        // Try to get from tenant list first
        match self.get_tenant_list().await {
            Ok(tenant_ids) => {
                let mut tenants = Vec::new();
                for tenant_id in tenant_ids {
                    match self.get_tenant(&tenant_id).await {
                        Ok(tenant) => tenants.push(tenant),
                        Err(e) => {
                            tracing::warn!("Failed to get tenant {}: {}", tenant_id, e);
                            // Continue with other tenants
                        }
                    }
                }
                Ok(tenants)
            }
            Err(_) => {
                // Fallback: list all objects with tenant prefix
                self.list_tenants_from_s3().await
            }
        }
    }

    async fn search_tenants(
        &self,
        criteria: TenantSearchCriteria,
    ) -> MultiTenancyResult<Vec<Tenant>> {
        // Get all tenants first, then filter in memory
        // For production, you might want to use S3 Select or Athena for better performance
        let all_tenants = self.list_tenants().await?;
        let mut results = all_tenants;

        // Apply filters
        if let Some(status) = criteria.status {
            results.retain(|t| t.status == status);
        }

        if let Some(tenant_type) = criteria.tenant_type {
            results.retain(|t| t.tenant_type == tenant_type);
        }

        if let Some(owner_user_id) = criteria.owner_user_id {
            results.retain(|t| {
                t.owner_user_id
                    .as_ref()
                    .map_or(false, |id| id == &owner_user_id)
            });
        }

        if let Some(name_pattern) = criteria.name_pattern {
            results.retain(|t| t.name.contains(&name_pattern));
        }

        // Apply pagination
        if let Some(offset) = criteria.offset {
            if offset < results.len() {
                results = results[offset..].to_vec();
            } else {
                results.clear();
            }
        }

        if let Some(limit) = criteria.limit {
            if limit < results.len() {
                results = results[..limit].to_vec();
            }
        }

        Ok(results)
    }
}

impl S3TenantStorage {
    /// Update the tenant list
    async fn update_tenant_list(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        let mut tenant_ids = self.get_tenant_list().await.unwrap_or_default();
        if !tenant_ids.contains(&tenant.id) {
            tenant_ids.push(tenant.id.clone());
            self.store_tenant_list(&tenant_ids).await?;
        }
        Ok(())
    }

    /// Remove tenant from the tenant list
    async fn remove_from_tenant_list(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        let mut tenant_ids = self.get_tenant_list().await.unwrap_or_default();
        tenant_ids.retain(|id| id != tenant_id);
        self.store_tenant_list(&tenant_ids).await?;
        Ok(())
    }

    /// Get the tenant list
    async fn get_tenant_list(&self) -> MultiTenancyResult<Vec<String>> {
        let list_key = self.tenant_list_key();

        match self
            .s3_client
            .get_object()
            .bucket(&self.bucket)
            .key(&list_key)
            .send()
            .await
        {
            Ok(response) => {
                let data = response.body.collect().await.map_err(|e| {
                    MultiTenancyError::storage_error(&format!(
                        "Failed to read tenant list from S3: {}",
                        e
                    ))
                })?;

                let json = String::from_utf8(data.to_vec()).map_err(|e| {
                    MultiTenancyError::storage_error(&format!(
                        "Failed to convert tenant list to string: {}",
                        e
                    ))
                })?;

                serde_json::from_str(&json).map_err(|e| {
                    MultiTenancyError::storage_error(&format!(
                        "Failed to deserialize tenant list: {}",
                        e
                    ))
                })
            }
            Err(_) => Ok(Vec::new()), // Return empty list if file doesn't exist
        }
    }

    /// Store the tenant list
    async fn store_tenant_list(&self, tenant_ids: &[String]) -> MultiTenancyResult<()> {
        let list_key = self.tenant_list_key();
        let json = serde_json::to_string(tenant_ids).map_err(|e| {
            MultiTenancyError::storage_error(&format!("Failed to serialize tenant list: {}", e))
        })?;

        self.s3_client
            .put_object()
            .bucket(&self.bucket)
            .key(&list_key)
            .body(ByteStream::from(json.into_bytes()))
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| {
                MultiTenancyError::storage_error(&format!(
                    "Failed to store tenant list in S3: {}",
                    e
                ))
            })?;

        Ok(())
    }

    /// List tenants by scanning S3 objects (fallback method)
    async fn list_tenants_from_s3(&self) -> MultiTenancyResult<Vec<Tenant>> {
        let response = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&self.key_prefix)
            .send()
            .await
            .map_err(|e| {
                MultiTenancyError::storage_error(&format!("Failed to list objects in S3: {}", e))
            })?;

        let mut tenants = Vec::new();

        if let Some(contents) = response.contents {
            for object in contents {
                if let Some(key) = object.key {
                    if key.ends_with(".json") && key != self.tenant_list_key() {
                        // Extract tenant ID from key
                        if let Some(tenant_id) = key
                            .strip_prefix(&self.key_prefix)
                            .and_then(|k| k.strip_suffix(".json"))
                        {
                            match self.get_tenant(tenant_id).await {
                                Ok(tenant) => tenants.push(tenant),
                                Err(e) => {
                                    tracing::warn!("Failed to get tenant from key {}: {}", key, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(tenants)
    }
}

/// Storage factory for creating storage instances
pub struct StorageFactory;

impl StorageFactory {
    /// Create storage based on configuration
    pub async fn create_storage(
        backend: &crate::config::StorageBackend,
        config: &crate::config::StorageConfig,
    ) -> MultiTenancyResult<Arc<dyn TenantStorage>> {
        match backend {
            crate::config::StorageBackend::Database => {
                let database_url = config.database_url.as_ref().ok_or_else(|| {
                    MultiTenancyError::configuration_error("Database URL not configured")
                })?;
                Ok(Arc::new(DatabaseTenantStorage::new(database_url)))
            }
            crate::config::StorageBackend::File => {
                let file_path = config.file_path.as_ref().ok_or_else(|| {
                    MultiTenancyError::configuration_error("File path not configured")
                })?;
                Ok(Arc::new(FileTenantStorage::new(file_path)))
            }
            crate::config::StorageBackend::Redis => {
                let redis_url = config.redis_url.as_ref().ok_or_else(|| {
                    MultiTenancyError::configuration_error("Redis URL not configured")
                })?;
                let storage = RedisTenantStorage::new(redis_url).await?;
                Ok(Arc::new(storage))
            }
            crate::config::StorageBackend::S3 => {
                let s3_config = config.s3.as_ref().ok_or_else(|| {
                    MultiTenancyError::configuration_error("S3 configuration not provided")
                })?;
                let storage = S3TenantStorage::new(s3_config).await?;
                Ok(Arc::new(storage))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Tenant, TenantType};

    #[tokio::test]
    async fn test_mock_storage() {
        let storage = MockTenantStorage::new();
        let tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);

        // Store tenant
        assert!(storage.store_tenant(&tenant).await.is_ok());

        // Get tenant
        let retrieved = storage.get_tenant("test-tenant").await.unwrap();
        assert_eq!(retrieved.id, "test-tenant");

        // List tenants
        let tenants = storage.list_tenants().await.unwrap();
        assert_eq!(tenants.len(), 1);
        assert_eq!(tenants[0].id, "test-tenant");

        // Search tenants
        let criteria = TenantSearchCriteria {
            status: Some(crate::models::TenantStatus::Pending),
            ..Default::default()
        };
        let results = storage.search_tenants(criteria).await.unwrap();
        assert_eq!(results.len(), 1);

        // Delete tenant
        assert!(storage.delete_tenant("test-tenant").await.is_ok());
        assert!(storage.get_tenant("test-tenant").await.is_err());
    }

    #[tokio::test]
    async fn test_storage_factory() {
        let config = crate::config::StorageConfig::default();
        let storage = StorageFactory::create_storage(&crate::config::StorageBackend::File, &config)
            .await
            .unwrap();

        let tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        assert!(storage.store_tenant(&tenant).await.is_ok());
    }

    #[tokio::test]
    async fn test_redis_storage() {
        // Skip test if Redis is not available
        if std::env::var("REDIS_URL").is_err() {
            return;
        }

        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let storage = RedisTenantStorage::new(&redis_url).await;

        if let Ok(storage) = storage {
            let tenant = Tenant::new("test-redis-tenant", "Test Redis Tenant", TenantType::Small);

            // Store tenant
            assert!(storage.store_tenant(&tenant).await.is_ok());

            // Get tenant
            let retrieved = storage.get_tenant("test-redis-tenant").await.unwrap();
            assert_eq!(retrieved.id, "test-redis-tenant");

            // List tenants
            let tenants = storage.list_tenants().await.unwrap();
            assert!(tenants.iter().any(|t| t.id == "test-redis-tenant"));

            // Delete tenant
            assert!(storage.delete_tenant("test-redis-tenant").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_s3_storage() {
        // Skip test if S3 is not configured
        if std::env::var("AWS_ACCESS_KEY_ID").is_err()
            || std::env::var("AWS_SECRET_ACCESS_KEY").is_err()
        {
            return;
        }

        let s3_config = crate::config::S3Config {
            bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "test-tenant-bucket".to_string()),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
            endpoint_url: None,
        };

        let storage = S3TenantStorage::new(&s3_config).await;

        if let Ok(storage) = storage {
            let tenant = Tenant::new("test-s3-tenant", "Test S3 Tenant", TenantType::Large);

            // Store tenant
            assert!(storage.store_tenant(&tenant).await.is_ok());

            // Get tenant
            let retrieved = storage.get_tenant("test-s3-tenant").await.unwrap();
            assert_eq!(retrieved.id, "test-s3-tenant");

            // List tenants
            let tenants = storage.list_tenants().await.unwrap();
            assert!(tenants.iter().any(|t| t.id == "test-s3-tenant"));

            // Delete tenant
            assert!(storage.delete_tenant("test-s3-tenant").await.is_ok());
        }
    }
}
