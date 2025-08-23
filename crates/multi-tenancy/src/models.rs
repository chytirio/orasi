//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Tenant model definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use crate::error::MultiTenancyError;

/// Tenant ID type
pub type TenantId = String;

/// Tenant status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TenantStatus {
    Active,
    Inactive,
    Suspended,
    Pending,
    Deleted,
}

/// Tenant type/category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TenantType {
    Small,
    Medium,
    Large,
    Enterprise,
    Custom(String),
}

/// Resource quota definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// CPU quota in cores
    pub cpu_cores: f64,
    /// Memory quota in bytes
    pub memory_bytes: u64,
    /// Storage quota in bytes
    pub storage_bytes: u64,
    /// Network bandwidth quota in bytes per second
    pub network_bandwidth_bps: u64,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Maximum requests per second
    pub max_requests_per_second: u32,
    /// Maximum data ingestion rate in bytes per second
    pub max_ingestion_rate_bps: u64,
    /// Maximum query execution time in seconds
    pub max_query_time_seconds: u32,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            cpu_cores: 1.0,
            memory_bytes: 1_073_741_824,        // 1GB
            storage_bytes: 10_737_418_240,      // 10GB
            network_bandwidth_bps: 100_000_000, // 100MB/s
            max_connections: 100,
            max_requests_per_second: 1000,
            max_ingestion_rate_bps: 10_000_000, // 10MB/s
            max_query_time_seconds: 300,        // 5 minutes
        }
    }
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Current CPU usage in cores
    pub cpu_cores_used: f64,
    /// Current memory usage in bytes
    pub memory_bytes_used: u64,
    /// Current storage usage in bytes
    pub storage_bytes_used: u64,
    /// Current network bandwidth usage in bytes per second
    pub network_bandwidth_bps_used: u64,
    /// Current number of connections
    pub connections_used: u32,
    /// Current requests per second
    pub requests_per_second: u32,
    /// Current data ingestion rate in bytes per second
    pub ingestion_rate_bps: u64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_cores_used: 0.0,
            memory_bytes_used: 0,
            storage_bytes_used: 0,
            network_bandwidth_bps_used: 0,
            connections_used: 0,
            requests_per_second: 0,
            ingestion_rate_bps: 0,
            last_updated: Utc::now(),
        }
    }
}

/// Billing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingInfo {
    /// Billing plan
    pub plan: String,
    /// Monthly cost in cents
    pub monthly_cost_cents: u64,
    /// Usage-based billing enabled
    pub usage_based_billing: bool,
    /// Overage charges enabled
    pub overage_charges: bool,
    /// Payment method
    pub payment_method: Option<String>,
    /// Billing cycle start date
    pub billing_cycle_start: DateTime<Utc>,
    /// Next billing date
    pub next_billing_date: DateTime<Utc>,
}

/// Compliance requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceInfo {
    /// GDPR compliance required
    pub gdpr_compliance: bool,
    /// CCPA compliance required
    pub ccpa_compliance: bool,
    /// HIPAA compliance required
    pub hipaa_compliance: bool,
    /// SOC2 compliance required
    pub soc2_compliance: bool,
    /// ISO27001 compliance required
    pub iso27001_compliance: bool,
    /// Data residency requirements
    pub data_residency: Vec<String>,
    /// Audit logging required
    pub audit_logging: bool,
}

impl Default for ComplianceInfo {
    fn default() -> Self {
        Self {
            gdpr_compliance: false,
            ccpa_compliance: false,
            hipaa_compliance: false,
            soc2_compliance: false,
            iso27001_compliance: false,
            data_residency: Vec::new(),
            audit_logging: true,
        }
    }
}

/// Tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    /// Tenant-specific settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Feature flags
    pub features: HashMap<String, bool>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            settings: HashMap::new(),
            features: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Main tenant structure
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Tenant {
    /// Unique tenant ID
    pub id: TenantId,

    /// Tenant name
    #[validate(length(min = 1, max = 200))]
    pub name: String,

    /// Tenant description
    pub description: Option<String>,

    /// Tenant status
    pub status: TenantStatus,

    /// Tenant type
    pub tenant_type: TenantType,

    /// Resource quotas
    pub resource_quota: ResourceQuota,

    /// Current resource usage
    pub resource_usage: ResourceUsage,

    /// Billing information
    pub billing_info: Option<BillingInfo>,

    /// Compliance requirements
    pub compliance_info: ComplianceInfo,

    /// Tenant configuration
    pub config: TenantConfig,

    /// Owner user ID
    pub owner_user_id: Option<String>,

    /// Admin user IDs
    pub admin_user_ids: Vec<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Deletion timestamp (if soft deleted)
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Tenant {
    /// Create a new tenant
    pub fn new(id: impl Into<TenantId>, name: impl Into<String>, tenant_type: TenantType) -> Self {
        let id = id.into();
        let name = name.into();
        let now = Utc::now();

        Self {
            id,
            name,
            description: None,
            status: TenantStatus::Pending,
            tenant_type,
            resource_quota: ResourceQuota::default(),
            resource_usage: ResourceUsage::default(),
            billing_info: None,
            compliance_info: ComplianceInfo::default(),
            config: TenantConfig::default(),
            owner_user_id: None,
            admin_user_ids: Vec::new(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Check if tenant is active
    pub fn is_active(&self) -> bool {
        self.status == TenantStatus::Active
    }

    /// Check if tenant is suspended
    pub fn is_suspended(&self) -> bool {
        self.status == TenantStatus::Suspended
    }

    /// Check if tenant is deleted
    pub fn is_deleted(&self) -> bool {
        self.status == TenantStatus::Deleted || self.deleted_at.is_some()
    }

    /// Check if user is admin for this tenant
    pub fn is_admin(&self, user_id: &str) -> bool {
        self.admin_user_ids.contains(&user_id.to_string())
            || self
                .owner_user_id
                .as_ref()
                .map_or(false, |id| id == user_id)
    }

    /// Check if user has access to this tenant
    pub fn has_user_access(&self, user_id: &str) -> bool {
        self.is_admin(user_id)
            || self
                .owner_user_id
                .as_ref()
                .map_or(false, |id| id == user_id)
    }

    /// Update resource usage
    pub fn update_resource_usage(&mut self, usage: ResourceUsage) {
        self.resource_usage = usage;
        self.updated_at = Utc::now();
    }

    /// Check if resource quota is exceeded
    pub fn is_quota_exceeded(&self) -> bool {
        self.resource_usage.cpu_cores_used > self.resource_quota.cpu_cores
            || self.resource_usage.memory_bytes_used > self.resource_quota.memory_bytes
            || self.resource_usage.storage_bytes_used > self.resource_quota.storage_bytes
            || self.resource_usage.connections_used > self.resource_quota.max_connections
            || self.resource_usage.requests_per_second > self.resource_quota.max_requests_per_second
            || self.resource_usage.ingestion_rate_bps > self.resource_quota.max_ingestion_rate_bps
    }

    /// Get quota utilization percentage for a resource
    pub fn get_quota_utilization(&self, resource: &str) -> f64 {
        match resource {
            "cpu" => (self.resource_usage.cpu_cores_used / self.resource_quota.cpu_cores) * 100.0,
            "memory" => {
                (self.resource_usage.memory_bytes_used as f64
                    / self.resource_quota.memory_bytes as f64)
                    * 100.0
            }
            "storage" => {
                (self.resource_usage.storage_bytes_used as f64
                    / self.resource_quota.storage_bytes as f64)
                    * 100.0
            }
            "connections" => {
                (self.resource_usage.connections_used as f64
                    / self.resource_quota.max_connections as f64)
                    * 100.0
            }
            "requests_per_second" => {
                (self.resource_usage.requests_per_second as f64
                    / self.resource_quota.max_requests_per_second as f64)
                    * 100.0
            }
            "ingestion_rate" => {
                (self.resource_usage.ingestion_rate_bps as f64
                    / self.resource_quota.max_ingestion_rate_bps as f64)
                    * 100.0
            }
            _ => 0.0,
        }
    }

    /// Validate tenant configuration
    pub fn validate(&self) -> Result<(), MultiTenancyError> {
        // Use validator crate's validate method
        <Self as validator::Validate>::validate(self)
            .map_err(|e| MultiTenancyError::validation_error(e.to_string()))?;

        if self.id.is_empty() {
            return Err(MultiTenancyError::validation_error(
                "Tenant ID cannot be empty",
            ));
        }

        if self.name.is_empty() {
            return Err(MultiTenancyError::validation_error(
                "Tenant name cannot be empty",
            ));
        }

        Ok(())
    }
}

impl Default for Tenant {
    fn default() -> Self {
        Self::new("default", "Default Tenant", TenantType::Small)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        assert_eq!(tenant.id, "test-tenant");
        assert_eq!(tenant.name, "Test Tenant");
        assert_eq!(tenant.tenant_type, TenantType::Medium);
        assert_eq!(tenant.status, TenantStatus::Pending);
    }

    #[test]
    fn test_tenant_validation() {
        let mut tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        assert!(tenant.validate().is_ok());

        tenant.id = String::new();
        assert!(tenant.validate().is_err());
    }

    #[test]
    fn test_quota_utilization() {
        let mut tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        tenant.resource_usage.cpu_cores_used = 0.5;
        tenant.resource_quota.cpu_cores = 1.0;

        let utilization = tenant.get_quota_utilization("cpu");
        assert_eq!(utilization, 50.0);
    }

    #[test]
    fn test_admin_access() {
        let mut tenant = Tenant::new("test-tenant", "Test Tenant", TenantType::Medium);
        tenant.owner_user_id = Some("owner-123".to_string());
        tenant.admin_user_ids = vec!["admin-456".to_string()];

        assert!(tenant.is_admin("owner-123"));
        assert!(tenant.is_admin("admin-456"));
        assert!(!tenant.is_admin("user-789"));
    }
}
