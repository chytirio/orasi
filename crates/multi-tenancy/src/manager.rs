//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Tenant manager for orchestrating multi-tenancy operations

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::config::MultiTenancyConfig;
use crate::error::{MultiTenancyError, MultiTenancyResult};
use crate::isolation::{IsolationConfig, TenantIsolationEngine};
use crate::models::{Tenant, TenantId, TenantStatus, TenantType};
use crate::quotas::{QuotaManager, QuotaManagerConfig, ResourceType};

/// Tenant manager for orchestrating multi-tenancy operations
pub struct TenantManager {
    /// Configuration
    config: MultiTenancyConfig,
    /// Tenant isolation engine
    isolation_engine: Arc<TenantIsolationEngine>,
    /// Quota manager
    quota_manager: Arc<QuotaManager>,
    /// Tenant storage
    tenant_storage: Arc<dyn crate::storage::TenantStorage>,
    /// Active tenants cache
    tenants_cache: Arc<RwLock<HashMap<TenantId, Tenant>>>,
}

impl TenantManager {
    /// Create a new tenant manager
    pub async fn new(
        config: MultiTenancyConfig,
        storage: Arc<dyn crate::storage::TenantStorage>,
    ) -> MultiTenancyResult<Self> {
        let isolation_config = IsolationConfig {
            data_isolation: config.data_isolation.clone(),
            network_isolation: config.network_isolation.clone(),
            resource_isolation: config.resource_isolation.clone(),
            allow_cross_tenant_analytics: config.allow_cross_tenant_analytics,
            allow_compliance_sharing: config.allow_compliance_sharing,
        };

        let quota_config = QuotaManagerConfig {
            enforcement_mode: config.quota_enforcement_mode.clone(),
            update_interval: config.usage_update_interval,
            warning_threshold: config.warning_threshold,
            critical_threshold: config.critical_threshold,
            enable_auto_scaling: config.enable_auto_scaling,
            auto_scaling_cooldown: config.auto_scaling_cooldown,
        };

        let isolation_engine = Arc::new(TenantIsolationEngine::new(isolation_config));
        let quota_manager = Arc::new(QuotaManager::new(quota_config));

        let manager = Self {
            config,
            isolation_engine,
            quota_manager,
            tenant_storage: storage,
            tenants_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load existing tenants from storage
        manager.load_tenants().await?;

        Ok(manager)
    }

    /// Create a new tenant
    pub async fn create_tenant(
        &self,
        id: impl Into<TenantId>,
        name: impl Into<String>,
        tenant_type: TenantType,
        owner_user_id: Option<String>,
    ) -> MultiTenancyResult<Tenant> {
        let id = id.into();
        let name = name.into();

        // Check if tenant already exists
        if self.tenant_exists(&id).await? {
            return Err(MultiTenancyError::tenant_already_exists(&id));
        }

        // Create new tenant
        let mut tenant = Tenant::new(id.clone(), name, tenant_type);
        tenant.owner_user_id = owner_user_id;
        tenant.status = TenantStatus::Pending;

        // Validate tenant
        tenant.validate()?;

        // Register with isolation engine
        self.isolation_engine.register_tenant(&tenant).await?;

        // Register with quota manager
        self.quota_manager.register_tenant(&tenant).await?;

        // Store tenant
        self.tenant_storage.store_tenant(&tenant).await?;

        // Add to cache
        let mut cache = self.tenants_cache.write().await;
        cache.insert(id.clone(), tenant.clone());

        info!("Created tenant: {}", id);
        Ok(tenant)
    }

    /// Get a tenant by ID
    pub async fn get_tenant(&self, tenant_id: &str) -> MultiTenancyResult<Tenant> {
        // Check cache first
        let cache = self.tenants_cache.read().await;
        if let Some(tenant) = cache.get(tenant_id) {
            return Ok(tenant.clone());
        }

        // Load from storage
        let tenant = self.tenant_storage.get_tenant(tenant_id).await?;

        // Add to cache
        let mut cache = self.tenants_cache.write().await;
        cache.insert(tenant_id.to_string(), tenant.clone());

        Ok(tenant)
    }

    /// Update a tenant
    pub async fn update_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        // Validate tenant
        tenant.validate()?;

        // Update in storage
        self.tenant_storage.update_tenant(tenant).await?;

        // Update cache
        let mut cache = self.tenants_cache.write().await;
        cache.insert(tenant.id.clone(), tenant.clone());

        // Update isolation engine if needed
        self.isolation_engine.register_tenant(tenant).await?;

        // Update quota manager
        self.quota_manager
            .update_quota(&tenant.id, tenant.resource_quota.clone())
            .await?;

        info!("Updated tenant: {}", tenant.id);
        Ok(())
    }

    /// Delete a tenant
    pub async fn delete_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        let mut tenant = self.get_tenant(tenant_id).await?;
        tenant.status = TenantStatus::Deleted;
        tenant.deleted_at = Some(chrono::Utc::now());

        // Update in storage
        self.tenant_storage.update_tenant(&tenant).await?;

        // Remove from isolation engine
        self.isolation_engine.unregister_tenant(tenant_id).await?;

        // Remove from quota manager
        self.quota_manager.unregister_tenant(tenant_id).await?;

        // Remove from cache
        let mut cache = self.tenants_cache.write().await;
        cache.remove(tenant_id);

        info!("Deleted tenant: {}", tenant_id);
        Ok(())
    }

    /// Activate a tenant
    pub async fn activate_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        let mut tenant = self.get_tenant(tenant_id).await?;
        tenant.status = TenantStatus::Active;
        self.update_tenant(&tenant).await
    }

    /// Suspend a tenant
    pub async fn suspend_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        let mut tenant = self.get_tenant(tenant_id).await?;
        tenant.status = TenantStatus::Suspended;
        self.update_tenant(&tenant).await
    }

    /// List all tenants
    pub async fn list_tenants(&self) -> MultiTenancyResult<Vec<Tenant>> {
        let tenants = self.tenant_storage.list_tenants().await?;

        // Update cache
        let mut cache = self.tenants_cache.write().await;
        for tenant in &tenants {
            cache.insert(tenant.id.clone(), tenant.clone());
        }

        Ok(tenants)
    }

    /// List active tenants
    pub async fn list_active_tenants(&self) -> MultiTenancyResult<Vec<Tenant>> {
        let tenants = self.list_tenants().await?;
        Ok(tenants.into_iter().filter(|t| t.is_active()).collect())
    }

    /// Check if tenant exists
    pub async fn tenant_exists(&self, tenant_id: &str) -> MultiTenancyResult<bool> {
        // Check cache first
        let cache = self.tenants_cache.read().await;
        if cache.contains_key(tenant_id) {
            return Ok(true);
        }

        // Check storage
        match self.tenant_storage.get_tenant(tenant_id).await {
            Ok(_) => Ok(true),
            Err(MultiTenancyError::TenantNotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get tenant usage statistics
    pub async fn get_tenant_usage(
        &self,
        tenant_id: &str,
    ) -> MultiTenancyResult<HashMap<String, f64>> {
        let mut usage = HashMap::new();

        // Get resource utilization
        for resource_type in &[
            ResourceType::Cpu,
            ResourceType::Memory,
            ResourceType::Storage,
            ResourceType::Connections,
            ResourceType::RequestsPerSecond,
            ResourceType::IngestionRate,
        ] {
            let utilization = self
                .quota_manager
                .get_utilization(tenant_id, resource_type)
                .await?;
            usage.insert(format!("{:?}", resource_type), utilization);
        }

        Ok(usage)
    }

    /// Check resource request
    pub async fn check_resource_request(
        &self,
        tenant_id: &str,
        resource_type: &ResourceType,
        amount: f64,
    ) -> MultiTenancyResult<bool> {
        // Check if tenant is active
        let tenant = self.get_tenant(tenant_id).await?;
        if !tenant.is_active() {
            return Err(MultiTenancyError::tenant_inactive(tenant_id));
        }

        // Check quota
        self.quota_manager
            .check_resource_request(tenant_id, resource_type, amount)
            .await
    }

    /// Update resource usage
    pub async fn update_resource_usage(
        &self,
        tenant_id: &str,
        usage: crate::models::ResourceUsage,
    ) -> MultiTenancyResult<()> {
        // Update quota manager
        self.quota_manager
            .update_usage(tenant_id, usage.clone())
            .await?;

        // Update tenant
        let mut tenant = self.get_tenant(tenant_id).await?;
        tenant.update_resource_usage(usage);
        self.update_tenant(&tenant).await?;

        Ok(())
    }

    /// Grant cross-tenant access
    pub async fn grant_cross_tenant_access(
        &self,
        source_tenant_id: &str,
        target_tenant_id: &str,
        operation: &str,
    ) -> MultiTenancyResult<()> {
        self.isolation_engine
            .grant_cross_tenant_access(source_tenant_id, target_tenant_id, operation)
            .await
    }

    /// Revoke cross-tenant access
    pub async fn revoke_cross_tenant_access(
        &self,
        source_tenant_id: &str,
        target_tenant_id: &str,
        operation: &str,
    ) -> MultiTenancyResult<()> {
        self.isolation_engine
            .revoke_cross_tenant_access(source_tenant_id, target_tenant_id, operation)
            .await
    }

    /// Check cross-tenant access
    pub async fn can_access_tenant_data(
        &self,
        source_tenant_id: &str,
        target_tenant_id: &str,
        operation: &str,
    ) -> MultiTenancyResult<bool> {
        self.isolation_engine
            .can_access_tenant_data(source_tenant_id, target_tenant_id, operation)
            .await
    }

    /// Get tenant isolation info
    pub async fn get_tenant_isolation_info(
        &self,
        tenant_id: &str,
    ) -> MultiTenancyResult<crate::isolation::TenantIsolationInfo> {
        self.isolation_engine.get_isolation_info(tenant_id).await
    }

    /// Get all tenants with usage
    pub async fn get_all_tenants_usage(
        &self,
    ) -> MultiTenancyResult<
        HashMap<TenantId, (crate::models::ResourceQuota, crate::models::ResourceUsage)>,
    > {
        Ok(self.quota_manager.get_all_tenants_usage().await)
    }

    /// Load tenants from storage
    async fn load_tenants(&self) -> MultiTenancyResult<()> {
        let tenants = self.tenant_storage.list_tenants().await?;
        let tenant_count = tenants.len();

        for tenant in &tenants {
            // Register with isolation engine
            self.isolation_engine.register_tenant(tenant).await?;

            // Register with quota manager
            self.quota_manager.register_tenant(tenant).await?;

            // Add to cache
            let mut cache = self.tenants_cache.write().await;
            cache.insert(tenant.id.clone(), tenant.clone());
        }

        info!("Loaded {} tenants from storage", tenant_count);
        Ok(())
    }

    /// Get configuration
    pub fn get_config(&self) -> &MultiTenancyConfig {
        &self.config
    }

    /// Get isolation engine
    pub fn get_isolation_engine(&self) -> &Arc<TenantIsolationEngine> {
        &self.isolation_engine
    }

    /// Get quota manager
    pub fn get_quota_manager(&self) -> &Arc<QuotaManager> {
        &self.quota_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MockTenantStorage;

    #[tokio::test]
    async fn test_tenant_creation() {
        let config = MultiTenancyConfig::default();
        let storage = Arc::new(MockTenantStorage::new());
        let manager = TenantManager::new(config, storage).await.unwrap();

        let tenant = manager
            .create_tenant("test-tenant", "Test Tenant", TenantType::Medium, None)
            .await
            .unwrap();

        assert_eq!(tenant.id, "test-tenant");
        assert_eq!(tenant.name, "Test Tenant");
        assert_eq!(tenant.tenant_type, TenantType::Medium);
    }

    #[tokio::test]
    async fn test_tenant_activation() {
        let config = MultiTenancyConfig::default();
        let storage = Arc::new(MockTenantStorage::new());
        let manager = TenantManager::new(config, storage).await.unwrap();

        let tenant = manager
            .create_tenant("test-tenant", "Test Tenant", TenantType::Medium, None)
            .await
            .unwrap();

        assert_eq!(tenant.status, TenantStatus::Pending);

        manager.activate_tenant("test-tenant").await.unwrap();

        let activated_tenant = manager.get_tenant("test-tenant").await.unwrap();
        assert_eq!(activated_tenant.status, TenantStatus::Active);
    }

    #[tokio::test]
    async fn test_resource_request_check() {
        let config = MultiTenancyConfig::default();
        let storage = Arc::new(MockTenantStorage::new());
        let manager = TenantManager::new(config, storage).await.unwrap();

        let _tenant = manager
            .create_tenant("test-tenant", "Test Tenant", TenantType::Medium, None)
            .await
            .unwrap();

        manager.activate_tenant("test-tenant").await.unwrap();

        // Valid request
        assert!(manager
            .check_resource_request("test-tenant", &ResourceType::Cpu, 0.5)
            .await
            .unwrap());

        // Invalid request (exceeds quota)
        assert!(manager
            .check_resource_request("test-tenant", &ResourceType::Cpu, 2.0)
            .await
            .is_err());
    }
}
