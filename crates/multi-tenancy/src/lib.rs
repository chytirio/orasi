//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Multi-tenancy support for Orasi observability platform
//!
//! This crate provides comprehensive multi-tenancy capabilities including:
//! - Tenant isolation and management
//! - Resource quotas and limits
//! - Tenant-specific configurations
//! - Cross-tenant analytics (optional)
//! - Tenant billing and usage tracking

pub mod config;
pub mod error;
pub mod isolation;
pub mod manager;
pub mod models;
pub mod quotas;
pub mod storage;

// Re-export commonly used types
pub use config::MultiTenancyConfig;
pub use error::{MultiTenancyError, MultiTenancyResult};
pub use isolation::{IsolationLevel, TenantIsolationEngine};
pub use manager::TenantManager;
pub use models::{Tenant, TenantId, TenantStatus, TenantType};
pub use quotas::QuotaManager;
pub use storage::TenantStorage;

/// Multi-tenancy result type
pub type Result<T> = MultiTenancyResult<T>;

/// Multi-tenancy error type
pub type Error = MultiTenancyError;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_tenant_creation() {
        let config = MultiTenancyConfig::default();
        let storage = Arc::new(storage::MockTenantStorage::new());
        let manager = TenantManager::new(config, storage).await.unwrap();

        let tenant = manager
            .create_tenant(
                "test-tenant",
                "Test Tenant",
                TenantType::Small,
                Some("user-1".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(tenant.id, "test-tenant");
        assert_eq!(tenant.name, "Test Tenant");
        assert_eq!(tenant.tenant_type, TenantType::Small);
        assert_eq!(tenant.status, TenantStatus::Pending);
    }

    #[tokio::test]
    async fn test_tenant_activation() {
        let config = MultiTenancyConfig::default();
        let storage = Arc::new(storage::MockTenantStorage::new());
        let manager = TenantManager::new(config, storage).await.unwrap();

        let _tenant = manager
            .create_tenant(
                "test-tenant",
                "Test Tenant",
                TenantType::Small,
                Some("user-1".to_string()),
            )
            .await
            .unwrap();

        manager.activate_tenant("test-tenant").await.unwrap();

        let tenant = manager.get_tenant("test-tenant").await.unwrap();
        assert_eq!(tenant.status, TenantStatus::Active);
    }

    #[tokio::test]
    async fn test_resource_quota_check() {
        let config = MultiTenancyConfig::default();
        let storage = Arc::new(storage::MockTenantStorage::new());
        let manager = TenantManager::new(config, storage).await.unwrap();

        let _tenant = manager
            .create_tenant(
                "test-tenant",
                "Test Tenant",
                TenantType::Small,
                Some("user-1".to_string()),
            )
            .await
            .unwrap();

        manager.activate_tenant("test-tenant").await.unwrap();

        // Check valid resource request
        let result = manager
            .check_resource_request("test-tenant", &quotas::ResourceType::Cpu, 0.5)
            .await;
        assert!(result.is_ok());

        // Check invalid resource request (exceeds quota)
        let result = manager
            .check_resource_request("test-tenant", &quotas::ResourceType::Cpu, 2.0)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tenant_listing() {
        let config = MultiTenancyConfig::default();
        let storage = Arc::new(storage::MockTenantStorage::new());
        let manager = TenantManager::new(config, storage).await.unwrap();

        let _tenant1 = manager
            .create_tenant(
                "tenant-1",
                "Tenant 1",
                TenantType::Small,
                Some("user-1".to_string()),
            )
            .await
            .unwrap();

        let _tenant2 = manager
            .create_tenant(
                "tenant-2",
                "Tenant 2",
                TenantType::Medium,
                Some("user-2".to_string()),
            )
            .await
            .unwrap();

        let tenants = manager.list_tenants().await.unwrap();
        assert_eq!(tenants.len(), 2);

        let active_tenants = manager.list_active_tenants().await.unwrap();
        assert_eq!(active_tenants.len(), 0); // No tenants activated yet
    }
}
