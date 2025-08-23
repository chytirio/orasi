//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Tenant isolation engine

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{MultiTenancyError, MultiTenancyResult};
use crate::models::{Tenant, TenantId};

/// Isolation levels for tenant data and resources
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Strict isolation - complete separation of data and resources
    Strict,
    /// Moderate isolation - shared infrastructure with data separation
    Moderate,
    /// Relaxed isolation - minimal separation, shared resources
    Relaxed,
}

/// Data isolation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataIsolationStrategy {
    /// Separate databases per tenant
    SeparateDatabases,
    /// Shared database with tenant-specific schemas
    SharedDatabaseSeparateSchemas,
    /// Shared database with tenant column filtering
    SharedDatabaseTenantColumn,
    /// Separate storage buckets/containers per tenant
    SeparateStorage,
    /// Shared storage with tenant prefixes
    SharedStorageTenantPrefix,
}

/// Network isolation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkIsolationStrategy {
    /// Separate network segments per tenant
    SeparateNetworks,
    /// Shared network with tenant-specific subnets
    SharedNetworkTenantSubnets,
    /// Shared network with tenant routing rules
    SharedNetworkTenantRouting,
    /// No network isolation
    NoIsolation,
}

/// Resource isolation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceIsolationStrategy {
    /// Dedicated resources per tenant
    DedicatedResources,
    /// Shared resources with quotas
    SharedResourcesWithQuotas,
    /// Shared resources with best-effort isolation
    SharedResourcesBestEffort,
}

/// Isolation configuration
#[derive(Debug, Clone)]
pub struct IsolationConfig {
    /// Data isolation strategy
    pub data_isolation: DataIsolationStrategy,
    /// Network isolation strategy
    pub network_isolation: NetworkIsolationStrategy,
    /// Resource isolation strategy
    pub resource_isolation: ResourceIsolationStrategy,
    /// Whether cross-tenant analytics are allowed
    pub allow_cross_tenant_analytics: bool,
    /// Whether tenant data can be shared for compliance
    pub allow_compliance_sharing: bool,
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            data_isolation: DataIsolationStrategy::SharedDatabaseTenantColumn,
            network_isolation: NetworkIsolationStrategy::SharedNetworkTenantRouting,
            resource_isolation: ResourceIsolationStrategy::SharedResourcesWithQuotas,
            allow_cross_tenant_analytics: false,
            allow_compliance_sharing: false,
        }
    }
}

/// Tenant isolation engine
pub struct TenantIsolationEngine {
    /// Isolation configuration
    config: IsolationConfig,
    /// Tenant isolation mappings
    tenant_mappings: Arc<RwLock<HashMap<TenantId, TenantIsolationInfo>>>,
    /// Cross-tenant access permissions
    cross_tenant_permissions: Arc<RwLock<HashMap<String, Vec<TenantId>>>>,
}

/// Tenant isolation information
#[derive(Debug, Clone)]
pub struct TenantIsolationInfo {
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Database name/schema for this tenant
    pub database_name: String,
    /// Storage bucket/container for this tenant
    pub storage_bucket: String,
    /// Network subnet/segment for this tenant
    pub network_subnet: Option<String>,
    /// Resource pool for this tenant
    pub resource_pool: String,
    /// Isolation level for this tenant
    pub isolation_level: IsolationLevel,
}

impl TenantIsolationInfo {
    /// Create new tenant isolation info
    pub fn new(tenant_id: TenantId, isolation_level: IsolationLevel) -> Self {
        let database_name = format!("tenant_{}", tenant_id);
        let storage_bucket = format!("tenant-{}", tenant_id);
        let resource_pool = format!("pool_{}", tenant_id);

        Self {
            tenant_id,
            database_name,
            storage_bucket,
            network_subnet: None,
            resource_pool,
            isolation_level,
        }
    }
}

impl TenantIsolationEngine {
    /// Create a new tenant isolation engine
    pub fn new(config: IsolationConfig) -> Self {
        Self {
            config,
            tenant_mappings: Arc::new(RwLock::new(HashMap::new())),
            cross_tenant_permissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tenant for isolation
    pub async fn register_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        let isolation_info = TenantIsolationInfo::new(
            tenant.id.clone(),
            match tenant.tenant_type {
                crate::models::TenantType::Enterprise => IsolationLevel::Strict,
                crate::models::TenantType::Large => IsolationLevel::Strict,
                crate::models::TenantType::Medium => IsolationLevel::Moderate,
                crate::models::TenantType::Small => IsolationLevel::Relaxed,
                crate::models::TenantType::Custom(_) => IsolationLevel::Moderate,
            },
        );

        let mut mappings = self.tenant_mappings.write().await;
        mappings.insert(tenant.id.clone(), isolation_info);

        info!("Registered tenant {} for isolation", tenant.id);
        Ok(())
    }

    /// Unregister a tenant from isolation
    pub async fn unregister_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        let mut mappings = self.tenant_mappings.write().await;
        mappings.remove(tenant_id);

        info!("Unregistered tenant {} from isolation", tenant_id);
        Ok(())
    }

    /// Get isolation info for a tenant
    pub async fn get_isolation_info(
        &self,
        tenant_id: &str,
    ) -> MultiTenancyResult<TenantIsolationInfo> {
        let mappings = self.tenant_mappings.read().await;
        mappings
            .get(tenant_id)
            .cloned()
            .ok_or_else(|| MultiTenancyError::tenant_not_found(tenant_id))
    }

    /// Check if tenant can access data from another tenant
    pub async fn can_access_tenant_data(
        &self,
        source_tenant_id: &str,
        target_tenant_id: &str,
        operation: &str,
    ) -> MultiTenancyResult<bool> {
        // Same tenant always has access
        if source_tenant_id == target_tenant_id {
            return Ok(true);
        }

        // Check if cross-tenant analytics are allowed
        if !self.config.allow_cross_tenant_analytics {
            return Ok(false);
        }

        // Check explicit permissions
        let permissions = self.cross_tenant_permissions.read().await;
        if let Some(allowed_tenants) =
            permissions.get(&format!("{}:{}", source_tenant_id, operation))
        {
            return Ok(allowed_tenants.contains(&target_tenant_id.to_string()));
        }

        // Check compliance sharing
        if self.config.allow_compliance_sharing && operation == "compliance" {
            return Ok(true);
        }

        Ok(false)
    }

    /// Grant cross-tenant access permission
    pub async fn grant_cross_tenant_access(
        &self,
        source_tenant_id: &str,
        target_tenant_id: &str,
        operation: &str,
    ) -> MultiTenancyResult<()> {
        let mut permissions = self.cross_tenant_permissions.write().await;
        let key = format!("{}:{}", source_tenant_id, operation);

        let allowed_tenants = permissions.entry(key).or_insert_with(Vec::new);
        if !allowed_tenants.contains(&target_tenant_id.to_string()) {
            allowed_tenants.push(target_tenant_id.to_string());
        }

        info!(
            "Granted {} access from tenant {} to tenant {}",
            operation, source_tenant_id, target_tenant_id
        );
        Ok(())
    }

    /// Revoke cross-tenant access permission
    pub async fn revoke_cross_tenant_access(
        &self,
        source_tenant_id: &str,
        target_tenant_id: &str,
        operation: &str,
    ) -> MultiTenancyResult<()> {
        let mut permissions = self.cross_tenant_permissions.write().await;
        let key = format!("{}:{}", source_tenant_id, operation);

        if let Some(allowed_tenants) = permissions.get_mut(&key) {
            allowed_tenants.retain(|id| id != target_tenant_id);
        }

        info!(
            "Revoked {} access from tenant {} to tenant {}",
            operation, source_tenant_id, target_tenant_id
        );
        Ok(())
    }

    /// Validate data access request
    pub async fn validate_data_access(
        &self,
        tenant_id: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> MultiTenancyResult<bool> {
        let isolation_info = self.get_isolation_info(tenant_id).await?;

        // Check if resource belongs to tenant
        match self.config.data_isolation {
            DataIsolationStrategy::SeparateDatabases => {
                // Each tenant has their own database, so all resources belong to them
                Ok(true)
            }
            DataIsolationStrategy::SharedDatabaseSeparateSchemas => {
                // Check if resource is in tenant's schema
                Ok(resource_id.starts_with(&isolation_info.database_name))
            }
            DataIsolationStrategy::SharedDatabaseTenantColumn => {
                // Check if resource has tenant column matching tenant ID
                Ok(resource_id.contains(tenant_id))
            }
            DataIsolationStrategy::SeparateStorage => {
                // Each tenant has their own storage, so all resources belong to them
                Ok(true)
            }
            DataIsolationStrategy::SharedStorageTenantPrefix => {
                // Check if resource has tenant prefix
                Ok(resource_id.starts_with(&format!("tenant-{}", tenant_id)))
            }
        }
    }

    /// Get tenant-specific database name
    pub async fn get_tenant_database(&self, tenant_id: &str) -> MultiTenancyResult<String> {
        let isolation_info = self.get_isolation_info(tenant_id).await?;
        Ok(isolation_info.database_name)
    }

    /// Get tenant-specific storage bucket
    pub async fn get_tenant_storage_bucket(&self, tenant_id: &str) -> MultiTenancyResult<String> {
        let isolation_info = self.get_isolation_info(tenant_id).await?;
        Ok(isolation_info.storage_bucket)
    }

    /// Get tenant-specific resource pool
    pub async fn get_tenant_resource_pool(&self, tenant_id: &str) -> MultiTenancyResult<String> {
        let isolation_info = self.get_isolation_info(tenant_id).await?;
        Ok(isolation_info.resource_pool)
    }

    /// Check if tenant isolation is enforced
    pub fn is_isolation_enforced(&self) -> bool {
        matches!(
            self.config.data_isolation,
            DataIsolationStrategy::SeparateDatabases | DataIsolationStrategy::SeparateStorage
        )
    }

    /// Get isolation configuration
    pub fn get_config(&self) -> &IsolationConfig {
        &self.config
    }

    /// Update isolation configuration
    pub fn update_config(&mut self, config: IsolationConfig) {
        self.config = config;
        info!("Updated tenant isolation configuration");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Tenant, TenantType};

    #[tokio::test]
    async fn test_tenant_registration() {
        let config = IsolationConfig::default();
        let engine = TenantIsolationEngine::new(config);

        let tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        assert!(engine.register_tenant(&tenant).await.is_ok());

        let isolation_info = engine.get_isolation_info("test-tenant").await.unwrap();
        assert_eq!(isolation_info.tenant_id, "test-tenant");
        assert_eq!(isolation_info.database_name, "tenant_test-tenant");
    }

    #[tokio::test]
    async fn test_cross_tenant_access() {
        let config = IsolationConfig {
            allow_cross_tenant_analytics: true,
            ..Default::default()
        };
        let engine = TenantIsolationEngine::new(config);

        // Grant access
        assert!(engine
            .grant_cross_tenant_access("tenant1", "tenant2", "analytics")
            .await
            .is_ok());

        // Check access
        assert!(engine
            .can_access_tenant_data("tenant1", "tenant2", "analytics")
            .await
            .unwrap());

        // Revoke access
        assert!(engine
            .revoke_cross_tenant_access("tenant1", "tenant2", "analytics")
            .await
            .is_ok());

        // Check access is revoked
        assert!(!engine
            .can_access_tenant_data("tenant1", "tenant2", "analytics")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_data_access_validation() {
        let config = IsolationConfig {
            data_isolation: DataIsolationStrategy::SharedStorageTenantPrefix,
            ..Default::default()
        };
        let engine = TenantIsolationEngine::new(config);

        let tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        engine.register_tenant(&tenant).await.unwrap();

        // Valid resource
        assert!(engine
            .validate_data_access("test-tenant", "file", "tenant-test-tenant-data.json")
            .await
            .unwrap());

        // Invalid resource
        assert!(!engine
            .validate_data_access("test-tenant", "file", "other-tenant-data.json")
            .await
            .unwrap());
    }
}
