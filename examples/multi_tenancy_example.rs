//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Multi-tenancy example demonstrating tenant management, isolation, and quotas

use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use multi_tenancy::{
    config::MultiTenancyConfig,
    manager::TenantManager,
    models::{Tenant, TenantType},
    quotas::ResourceType,
    storage::MockTenantStorage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Multi-Tenancy Example");

    // Create configuration
    let config = MultiTenancyConfig::default();
    info!("Using default multi-tenancy configuration");

    // Create storage
    let storage = Arc::new(MockTenantStorage::new());

    // Create tenant manager
    let tenant_manager = TenantManager::new(config, storage).await?;
    info!("Tenant manager initialized");

    // Example 1: Create tenants of different types
    info!("=== Creating Tenants ===");

    let small_tenant = tenant_manager
        .create_tenant(
            "acme-small",
            "ACME Small",
            TenantType::Small,
            Some("user-1".to_string()),
        )
        .await?;
    info!("Created small tenant: {}", small_tenant.id);

    let medium_tenant = tenant_manager
        .create_tenant(
            "acme-medium",
            "ACME Medium",
            TenantType::Medium,
            Some("user-2".to_string()),
        )
        .await?;
    info!("Created medium tenant: {}", medium_tenant.id);

    let large_tenant = tenant_manager
        .create_tenant(
            "acme-large",
            "ACME Large",
            TenantType::Large,
            Some("user-3".to_string()),
        )
        .await?;
    info!("Created large tenant: {}", large_tenant.id);

    let enterprise_tenant = tenant_manager
        .create_tenant(
            "acme-enterprise",
            "ACME Enterprise",
            TenantType::Enterprise,
            Some("user-4".to_string()),
        )
        .await?;
    info!("Created enterprise tenant: {}", enterprise_tenant.id);

    // Example 2: Activate tenants
    info!("=== Activating Tenants ===");

    tenant_manager.activate_tenant("acme-small").await?;
    tenant_manager.activate_tenant("acme-medium").await?;
    tenant_manager.activate_tenant("acme-large").await?;
    tenant_manager.activate_tenant("acme-enterprise").await?;
    info!("All tenants activated");

    // Example 3: Check resource quotas
    info!("=== Resource Quotas ===");

    let small_quota = tenant_manager
        .get_quota_manager()
        .get_quota("acme-small")
        .await?;
    info!("Small tenant CPU quota: {} cores", small_quota.cpu_cores);
    info!(
        "Small tenant memory quota: {} MB",
        small_quota.memory_bytes / 1024 / 1024
    );

    let enterprise_quota = tenant_manager
        .get_quota_manager()
        .get_quota("acme-enterprise")
        .await?;
    info!(
        "Enterprise tenant CPU quota: {} cores",
        enterprise_quota.cpu_cores
    );
    info!(
        "Enterprise tenant memory quota: {} MB",
        enterprise_quota.memory_bytes / 1024 / 1024
    );

    // Example 4: Resource request validation
    info!("=== Resource Request Validation ===");

    // Valid request for small tenant
    let valid_request = tenant_manager
        .check_resource_request("acme-small", &ResourceType::Cpu, 0.3)
        .await;
    match valid_request {
        Ok(_) => info!("Valid CPU request for small tenant"),
        Err(e) => warn!("Invalid CPU request for small tenant: {}", e),
    }

    // Invalid request (exceeds quota)
    let invalid_request = tenant_manager
        .check_resource_request("acme-small", &ResourceType::Cpu, 1.0)
        .await;
    match invalid_request {
        Ok(_) => info!("Unexpected: CPU request should have been rejected"),
        Err(e) => info!("Correctly rejected CPU request: {}", e),
    }

    // Example 5: Update resource usage
    info!("=== Resource Usage Tracking ===");

    use multi_tenancy::models::ResourceUsage;
    let mut usage = ResourceUsage::default();
    usage.cpu_cores_used = 0.2;
    usage.memory_bytes_used = 256 * 1024 * 1024; // 256MB
    usage.connections_used = 25;

    tenant_manager
        .update_resource_usage("acme-small", usage)
        .await?;
    info!("Updated resource usage for small tenant");

    // Get usage statistics
    let usage_stats = tenant_manager.get_tenant_usage("acme-small").await?;
    for (resource, utilization) in usage_stats {
        info!("Small tenant {} utilization: {:.1}%", resource, utilization);
    }

    // Example 6: Cross-tenant access control
    info!("=== Cross-Tenant Access Control ===");

    // Grant cross-tenant access
    tenant_manager
        .grant_cross_tenant_access("acme-medium", "acme-small", "analytics")
        .await?;
    info!("Granted analytics access from medium to small tenant");

    // Check access
    let has_access = tenant_manager
        .can_access_tenant_data("acme-medium", "acme-small", "analytics")
        .await?;
    info!(
        "Medium tenant can access small tenant analytics: {}",
        has_access
    );

    // Revoke access
    tenant_manager
        .revoke_cross_tenant_access("acme-medium", "acme-small", "analytics")
        .await?;
    info!("Revoked analytics access from medium to small tenant");

    let no_access = tenant_manager
        .can_access_tenant_data("acme-medium", "acme-small", "analytics")
        .await?;
    info!(
        "Medium tenant can access small tenant analytics after revocation: {}",
        no_access
    );

    // Example 7: Tenant isolation information
    info!("=== Tenant Isolation Information ===");

    let isolation_info = tenant_manager
        .get_tenant_isolation_info("acme-enterprise")
        .await?;
    info!(
        "Enterprise tenant database: {}",
        isolation_info.database_name
    );
    info!(
        "Enterprise tenant storage bucket: {}",
        isolation_info.storage_bucket
    );
    info!(
        "Enterprise tenant resource pool: {}",
        isolation_info.resource_pool
    );

    // Example 8: List and search tenants
    info!("=== Tenant Listing and Search ===");

    let all_tenants = tenant_manager.list_tenants().await?;
    info!("Total tenants: {}", all_tenants.len());

    let active_tenants = tenant_manager.list_active_tenants().await?;
    info!("Active tenants: {}", active_tenants.len());

    // Example 9: Tenant lifecycle management
    info!("=== Tenant Lifecycle Management ===");

    // Suspend a tenant
    tenant_manager.suspend_tenant("acme-small").await?;
    info!("Suspended small tenant");

    // Try to use suspended tenant
    let suspended_request = tenant_manager
        .check_resource_request("acme-small", &ResourceType::Cpu, 0.1)
        .await;
    match suspended_request {
        Ok(_) => {
            warn!("Unexpected: Resource request should have been rejected for suspended tenant")
        }
        Err(e) => info!(
            "Correctly rejected resource request for suspended tenant: {}",
            e
        ),
    }

    // Reactivate tenant
    tenant_manager.activate_tenant("acme-small").await?;
    info!("Reactivated small tenant");

    // Example 10: Get all tenants with usage
    info!("=== All Tenants Usage Summary ===");

    let all_usage = tenant_manager.get_all_tenants_usage().await?;
    for (tenant_id, (quota, usage)) in all_usage {
        let cpu_utilization = (usage.cpu_cores_used / quota.cpu_cores) * 100.0;
        let memory_utilization =
            (usage.memory_bytes_used as f64 / quota.memory_bytes as f64) * 100.0;

        info!(
            "Tenant {}: CPU {:.1}% ({:.1}/{:.1} cores), Memory {:.1}% ({:.0}MB/{:.0}MB)",
            tenant_id,
            cpu_utilization,
            usage.cpu_cores_used,
            quota.cpu_cores,
            memory_utilization,
            usage.memory_bytes_used / 1024 / 1024,
            quota.memory_bytes / 1024 / 1024
        );
    }

    info!("=== Multi-Tenancy Example Completed Successfully ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_tenancy_example() {
        // This test ensures the example compiles and runs
        let config = MultiTenancyConfig::default();
        let storage = Arc::new(MockTenantStorage::new());
        let tenant_manager = TenantManager::new(config, storage).await.unwrap();

        // Create a test tenant
        let tenant = tenant_manager
            .create_tenant("test-tenant", "Test Tenant", TenantType::Medium, None)
            .await
            .unwrap();

        assert_eq!(tenant.id, "test-tenant");
        assert_eq!(tenant.name, "Test Tenant");
        assert_eq!(tenant.tenant_type, TenantType::Medium);
    }
}
