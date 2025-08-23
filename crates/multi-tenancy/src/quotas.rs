//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Resource quota management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{MultiTenancyError, MultiTenancyResult};
use crate::models::{ResourceQuota, ResourceUsage, Tenant, TenantId};

/// Resource type for quota tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Cpu,
    Memory,
    Storage,
    NetworkBandwidth,
    Connections,
    RequestsPerSecond,
    IngestionRate,
    QueryTime,
}

/// Quota enforcement mode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuotaEnforcementMode {
    /// Strict enforcement - reject requests when quota exceeded
    Strict,
    /// Soft enforcement - allow requests but log warnings
    Soft,
    /// Best effort - no enforcement, just monitoring
    BestEffort,
}

/// Quota manager configuration
#[derive(Debug, Clone)]
pub struct QuotaManagerConfig {
    /// Enforcement mode
    pub enforcement_mode: QuotaEnforcementMode,
    /// Update interval for usage tracking
    pub update_interval: Duration,
    /// Warning threshold percentage
    pub warning_threshold: f64,
    /// Critical threshold percentage
    pub critical_threshold: f64,
    /// Whether to enable automatic scaling
    pub enable_auto_scaling: bool,
    /// Auto-scaling cooldown period
    pub auto_scaling_cooldown: Duration,
}

impl Default for QuotaManagerConfig {
    fn default() -> Self {
        Self {
            enforcement_mode: QuotaEnforcementMode::Strict,
            update_interval: Duration::from_secs(60),
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            enable_auto_scaling: false,
            auto_scaling_cooldown: Duration::from_secs(300),
        }
    }
}

/// Quota manager for tracking and enforcing resource limits
pub struct QuotaManager {
    /// Configuration
    config: QuotaManagerConfig,
    /// Tenant quotas
    tenant_quotas: Arc<RwLock<HashMap<TenantId, ResourceQuota>>>,
    /// Tenant usage
    tenant_usage: Arc<RwLock<HashMap<TenantId, ResourceUsage>>>,
    /// Usage history for trending
    usage_history: Arc<RwLock<HashMap<TenantId, Vec<ResourceUsage>>>>,
    /// Last auto-scaling time per tenant
    last_auto_scaling: Arc<RwLock<HashMap<TenantId, Instant>>>,
}

impl QuotaManager {
    /// Create a new quota manager
    pub fn new(config: QuotaManagerConfig) -> Self {
        Self {
            config,
            tenant_quotas: Arc::new(RwLock::new(HashMap::new())),
            tenant_usage: Arc::new(RwLock::new(HashMap::new())),
            usage_history: Arc::new(RwLock::new(HashMap::new())),
            last_auto_scaling: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tenant with quotas
    pub async fn register_tenant(&self, tenant: &Tenant) -> MultiTenancyResult<()> {
        let mut quotas = self.tenant_quotas.write().await;
        quotas.insert(tenant.id.clone(), tenant.resource_quota.clone());

        let mut usage = self.tenant_usage.write().await;
        usage.insert(tenant.id.clone(), tenant.resource_usage.clone());

        info!("Registered tenant {} with quotas", tenant.id);
        Ok(())
    }

    /// Unregister a tenant
    pub async fn unregister_tenant(&self, tenant_id: &str) -> MultiTenancyResult<()> {
        let mut quotas = self.tenant_quotas.write().await;
        quotas.remove(tenant_id);

        let mut usage = self.tenant_usage.write().await;
        usage.remove(tenant_id);

        let mut history = self.usage_history.write().await;
        history.remove(tenant_id);

        let mut last_scaling = self.last_auto_scaling.write().await;
        last_scaling.remove(tenant_id);

        info!("Unregistered tenant {} from quota management", tenant_id);
        Ok(())
    }

    /// Update resource usage for a tenant
    pub async fn update_usage(
        &self,
        tenant_id: &str,
        usage: ResourceUsage,
    ) -> MultiTenancyResult<()> {
        let mut tenant_usage = self.tenant_usage.write().await;
        tenant_usage.insert(tenant_id.to_string(), usage.clone());

        // Store in history
        let mut history = self.usage_history.write().await;
        let tenant_history = history
            .entry(tenant_id.to_string())
            .or_insert_with(Vec::new);
        tenant_history.push(usage);

        // Keep only last 100 entries
        if tenant_history.len() > 100 {
            tenant_history.remove(0);
        }

        debug!("Updated usage for tenant {}", tenant_id);
        Ok(())
    }

    /// Check if a resource request is allowed
    pub async fn check_resource_request(
        &self,
        tenant_id: &str,
        resource_type: &ResourceType,
        amount: f64,
    ) -> MultiTenancyResult<bool> {
        let quotas = self.tenant_quotas.read().await;
        let usage = self.tenant_usage.read().await;

        let quota = quotas
            .get(tenant_id)
            .ok_or_else(|| MultiTenancyError::tenant_not_found(tenant_id))?;

        let current_usage = usage
            .get(tenant_id)
            .ok_or_else(|| MultiTenancyError::tenant_not_found(tenant_id))?;

        let (current, limit) = match resource_type {
            ResourceType::Cpu => (current_usage.cpu_cores_used, quota.cpu_cores),
            ResourceType::Memory => (
                current_usage.memory_bytes_used as f64,
                quota.memory_bytes as f64,
            ),
            ResourceType::Storage => (
                current_usage.storage_bytes_used as f64,
                quota.storage_bytes as f64,
            ),
            ResourceType::NetworkBandwidth => (
                current_usage.network_bandwidth_bps_used as f64,
                quota.network_bandwidth_bps as f64,
            ),
            ResourceType::Connections => (
                current_usage.connections_used as f64,
                quota.max_connections as f64,
            ),
            ResourceType::RequestsPerSecond => (
                current_usage.requests_per_second as f64,
                quota.max_requests_per_second as f64,
            ),
            ResourceType::IngestionRate => (
                current_usage.ingestion_rate_bps as f64,
                quota.max_ingestion_rate_bps as f64,
            ),
            ResourceType::QueryTime => (0.0, quota.max_query_time_seconds as f64),
        };

        let new_usage = current + amount;
        let utilization = (new_usage / limit) * 100.0;

        // Check thresholds
        if utilization >= self.config.critical_threshold {
            warn!(
                "Critical quota threshold exceeded for tenant {}: {}% utilization of {:?}",
                tenant_id, utilization, resource_type
            );
        } else if utilization >= self.config.warning_threshold {
            warn!(
                "Warning quota threshold exceeded for tenant {}: {}% utilization of {:?}",
                tenant_id, utilization, resource_type
            );
        }

        // Check enforcement mode
        match self.config.enforcement_mode {
            QuotaEnforcementMode::Strict => {
                if new_usage > limit {
                    return Err(MultiTenancyError::quota_exceeded(
                        tenant_id,
                        format!("{:?}", resource_type),
                    ));
                }
            }
            QuotaEnforcementMode::Soft => {
                if new_usage > limit {
                    warn!(
                        "Quota exceeded for tenant {}: {:?} ({} > {})",
                        tenant_id, resource_type, new_usage, limit
                    );
                }
            }
            QuotaEnforcementMode::BestEffort => {
                // No enforcement, just monitoring
            }
        }

        Ok(true)
    }

    /// Get current usage for a tenant
    pub async fn get_usage(&self, tenant_id: &str) -> MultiTenancyResult<ResourceUsage> {
        let usage = self.tenant_usage.read().await;
        usage
            .get(tenant_id)
            .cloned()
            .ok_or_else(|| MultiTenancyError::tenant_not_found(tenant_id))
    }

    /// Get quota for a tenant
    pub async fn get_quota(&self, tenant_id: &str) -> MultiTenancyResult<ResourceQuota> {
        let quotas = self.tenant_quotas.read().await;
        quotas
            .get(tenant_id)
            .cloned()
            .ok_or_else(|| MultiTenancyError::tenant_not_found(tenant_id))
    }

    /// Get usage history for a tenant
    pub async fn get_usage_history(
        &self,
        tenant_id: &str,
    ) -> MultiTenancyResult<Vec<ResourceUsage>> {
        let history = self.usage_history.read().await;
        Ok(history.get(tenant_id).cloned().unwrap_or_default())
    }

    /// Get utilization percentage for a resource
    pub async fn get_utilization(
        &self,
        tenant_id: &str,
        resource_type: &ResourceType,
    ) -> MultiTenancyResult<f64> {
        let quota = self.get_quota(tenant_id).await?;
        let usage = self.get_usage(tenant_id).await?;

        let (current, limit) = match resource_type {
            ResourceType::Cpu => (usage.cpu_cores_used, quota.cpu_cores),
            ResourceType::Memory => (usage.memory_bytes_used as f64, quota.memory_bytes as f64),
            ResourceType::Storage => (usage.storage_bytes_used as f64, quota.storage_bytes as f64),
            ResourceType::NetworkBandwidth => (
                usage.network_bandwidth_bps_used as f64,
                quota.network_bandwidth_bps as f64,
            ),
            ResourceType::Connections => {
                (usage.connections_used as f64, quota.max_connections as f64)
            }
            ResourceType::RequestsPerSecond => (
                usage.requests_per_second as f64,
                quota.max_requests_per_second as f64,
            ),
            ResourceType::IngestionRate => (
                usage.ingestion_rate_bps as f64,
                quota.max_ingestion_rate_bps as f64,
            ),
            ResourceType::QueryTime => (0.0, quota.max_query_time_seconds as f64),
        };

        Ok((current / limit) * 100.0)
    }

    /// Check if auto-scaling is needed
    pub async fn check_auto_scaling(&self, tenant_id: &str) -> MultiTenancyResult<bool> {
        if !self.config.enable_auto_scaling {
            return Ok(false);
        }

        let last_scaling = self.last_auto_scaling.read().await;
        if let Some(last_time) = last_scaling.get(tenant_id) {
            if last_time.elapsed() < self.config.auto_scaling_cooldown {
                return Ok(false);
            }
        }

        // Check if utilization is consistently high
        let history = self.get_usage_history(tenant_id).await?;
        if history.len() < 5 {
            return Ok(false);
        }

        // Check last 5 measurements for high utilization
        let recent_usage = &history[history.len() - 5..];
        let avg_cpu_utilization =
            recent_usage.iter().map(|u| u.cpu_cores_used).sum::<f64>() / recent_usage.len() as f64;

        let quota = self.get_quota(tenant_id).await?;
        let cpu_utilization = (avg_cpu_utilization / quota.cpu_cores) * 100.0;

        Ok(cpu_utilization > self.config.critical_threshold)
    }

    /// Update quota for a tenant
    pub async fn update_quota(
        &self,
        tenant_id: &str,
        quota: ResourceQuota,
    ) -> MultiTenancyResult<()> {
        let mut quotas = self.tenant_quotas.write().await;
        quotas.insert(tenant_id.to_string(), quota);

        info!("Updated quota for tenant {}", tenant_id);
        Ok(())
    }

    /// Get all tenants with their usage
    pub async fn get_all_tenants_usage(&self) -> HashMap<TenantId, (ResourceQuota, ResourceUsage)> {
        let quotas = self.tenant_quotas.read().await;
        let usage = self.tenant_usage.read().await;

        let mut result = HashMap::new();
        for (tenant_id, quota) in quotas.iter() {
            if let Some(tenant_usage) = usage.get(tenant_id) {
                result.insert(tenant_id.clone(), (quota.clone(), tenant_usage.clone()));
            }
        }

        result
    }

    /// Get configuration
    pub fn get_config(&self) -> &QuotaManagerConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: QuotaManagerConfig) {
        self.config = config;
        info!("Updated quota manager configuration");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Tenant, TenantType};

    #[tokio::test]
    async fn test_tenant_registration() {
        let config = QuotaManagerConfig::default();
        let manager = QuotaManager::new(config);

        let tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        assert!(manager.register_tenant(&tenant).await.is_ok());

        let quota = manager.get_quota("test-tenant").await.unwrap();
        assert_eq!(quota.cpu_cores, 1.0);
    }

    #[tokio::test]
    async fn test_resource_request_check() {
        let config = QuotaManagerConfig {
            enforcement_mode: QuotaEnforcementMode::Strict,
            ..Default::default()
        };
        let manager = QuotaManager::new(config);

        let tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        manager.register_tenant(&tenant).await.unwrap();

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

    #[tokio::test]
    async fn test_usage_tracking() {
        let config = QuotaManagerConfig::default();
        let manager = QuotaManager::new(config);

        let tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        manager.register_tenant(&tenant).await.unwrap();

        let mut usage = ResourceUsage::default();
        usage.cpu_cores_used = 0.5;
        usage.memory_bytes_used = 512 * 1024 * 1024; // 512MB

        assert!(manager
            .update_usage("test-tenant", usage.clone())
            .await
            .is_ok());

        let retrieved_usage = manager.get_usage("test-tenant").await.unwrap();
        assert_eq!(retrieved_usage.cpu_cores_used, 0.5);
        assert_eq!(retrieved_usage.memory_bytes_used, 512 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_utilization_calculation() {
        let config = QuotaManagerConfig::default();
        let manager = QuotaManager::new(config);

        let tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        manager.register_tenant(&tenant).await.unwrap();

        let mut usage = ResourceUsage::default();
        usage.cpu_cores_used = 0.5;
        manager.update_usage("test-tenant", usage).await.unwrap();

        let utilization = manager
            .get_utilization("test-tenant", &ResourceType::Cpu)
            .await
            .unwrap();
        assert_eq!(utilization, 50.0);
    }
}
