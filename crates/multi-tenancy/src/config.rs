//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Multi-tenancy configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;
use validator::Validate;

use crate::isolation::{
    DataIsolationStrategy, NetworkIsolationStrategy, ResourceIsolationStrategy,
};
use crate::quotas::QuotaEnforcementMode;

/// Multi-tenancy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MultiTenancyConfig {
    /// Whether multi-tenancy is enabled
    pub enabled: bool,

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

    /// Quota enforcement mode
    pub quota_enforcement_mode: QuotaEnforcementMode,

    /// Usage update interval
    pub usage_update_interval: Duration,

    /// Warning threshold percentage
    pub warning_threshold: f64,

    /// Critical threshold percentage
    pub critical_threshold: f64,

    /// Whether to enable automatic scaling
    pub enable_auto_scaling: bool,

    /// Auto-scaling cooldown period
    pub auto_scaling_cooldown: Duration,

    /// Default resource quotas for new tenants
    pub default_quotas: DefaultQuotas,

    /// Tenant type configurations
    pub tenant_types: TenantTypeConfigs,

    /// Storage configuration
    pub storage: StorageConfig,

    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

impl Default for MultiTenancyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            data_isolation: DataIsolationStrategy::SharedDatabaseTenantColumn,
            network_isolation: NetworkIsolationStrategy::SharedNetworkTenantRouting,
            resource_isolation: ResourceIsolationStrategy::SharedResourcesWithQuotas,
            allow_cross_tenant_analytics: false,
            allow_compliance_sharing: false,
            quota_enforcement_mode: QuotaEnforcementMode::Strict,
            usage_update_interval: Duration::from_secs(60),
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            enable_auto_scaling: false,
            auto_scaling_cooldown: Duration::from_secs(300),
            default_quotas: DefaultQuotas::default(),
            tenant_types: TenantTypeConfigs::default(),
            storage: StorageConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

/// Default resource quotas for new tenants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultQuotas {
    /// Small tenant quotas
    pub small: TenantQuotas,
    /// Medium tenant quotas
    pub medium: TenantQuotas,
    /// Large tenant quotas
    pub large: TenantQuotas,
    /// Enterprise tenant quotas
    pub enterprise: TenantQuotas,
}

impl Default for DefaultQuotas {
    fn default() -> Self {
        Self {
            small: TenantQuotas {
                cpu_cores: 0.5,
                memory_bytes: 536_870_912,         // 512MB
                storage_bytes: 5_368_709_120,      // 5GB
                network_bandwidth_bps: 50_000_000, // 50MB/s
                max_connections: 50,
                max_requests_per_second: 500,
                max_ingestion_rate_bps: 5_000_000, // 5MB/s
                max_query_time_seconds: 60,
            },
            medium: TenantQuotas {
                cpu_cores: 1.0,
                memory_bytes: 1_073_741_824,        // 1GB
                storage_bytes: 10_737_418_240,      // 10GB
                network_bandwidth_bps: 100_000_000, // 100MB/s
                max_connections: 100,
                max_requests_per_second: 1000,
                max_ingestion_rate_bps: 10_000_000, // 10MB/s
                max_query_time_seconds: 300,
            },
            large: TenantQuotas {
                cpu_cores: 4.0,
                memory_bytes: 4_294_967_296,        // 4GB
                storage_bytes: 42_949_672_960,      // 40GB
                network_bandwidth_bps: 500_000_000, // 500MB/s
                max_connections: 500,
                max_requests_per_second: 5000,
                max_ingestion_rate_bps: 50_000_000, // 50MB/s
                max_query_time_seconds: 600,
            },
            enterprise: TenantQuotas {
                cpu_cores: 16.0,
                memory_bytes: 17_179_869_184,         // 16GB
                storage_bytes: 171_798_691_840,       // 160GB
                network_bandwidth_bps: 2_000_000_000, // 2GB/s
                max_connections: 2000,
                max_requests_per_second: 20000,
                max_ingestion_rate_bps: 200_000_000, // 200MB/s
                max_query_time_seconds: 1800,
            },
        }
    }
}

/// Tenant quotas configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuotas {
    /// CPU cores
    pub cpu_cores: f64,
    /// Memory in bytes
    pub memory_bytes: u64,
    /// Storage in bytes
    pub storage_bytes: u64,
    /// Network bandwidth in bytes per second
    pub network_bandwidth_bps: u64,
    /// Maximum connections
    pub max_connections: u32,
    /// Maximum requests per second
    pub max_requests_per_second: u32,
    /// Maximum ingestion rate in bytes per second
    pub max_ingestion_rate_bps: u64,
    /// Maximum query time in seconds
    pub max_query_time_seconds: u32,
}

/// Tenant type configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantTypeConfigs {
    /// Small tenant configuration
    pub small: TenantTypeConfig,
    /// Medium tenant configuration
    pub medium: TenantTypeConfig,
    /// Large tenant configuration
    pub large: TenantTypeConfig,
    /// Enterprise tenant configuration
    pub enterprise: TenantTypeConfig,
}

impl Default for TenantTypeConfigs {
    fn default() -> Self {
        Self {
            small: TenantTypeConfig {
                isolation_level: "Relaxed".to_string(),
                allow_custom_config: false,
                max_users: 10,
                max_projects: 5,
                features: vec!["basic_analytics".to_string(), "standard_export".to_string()],
            },
            medium: TenantTypeConfig {
                isolation_level: "Moderate".to_string(),
                allow_custom_config: true,
                max_users: 50,
                max_projects: 20,
                features: vec![
                    "basic_analytics".to_string(),
                    "advanced_analytics".to_string(),
                    "standard_export".to_string(),
                    "custom_dashboards".to_string(),
                ],
            },
            large: TenantTypeConfig {
                isolation_level: "Moderate".to_string(),
                allow_custom_config: true,
                max_users: 200,
                max_projects: 100,
                features: vec![
                    "basic_analytics".to_string(),
                    "advanced_analytics".to_string(),
                    "standard_export".to_string(),
                    "custom_dashboards".to_string(),
                    "api_access".to_string(),
                    "priority_support".to_string(),
                ],
            },
            enterprise: TenantTypeConfig {
                isolation_level: "Strict".to_string(),
                allow_custom_config: true,
                max_users: 1000,
                max_projects: 500,
                features: vec![
                    "basic_analytics".to_string(),
                    "advanced_analytics".to_string(),
                    "standard_export".to_string(),
                    "custom_dashboards".to_string(),
                    "api_access".to_string(),
                    "priority_support".to_string(),
                    "dedicated_infrastructure".to_string(),
                    "custom_integrations".to_string(),
                    "compliance_reporting".to_string(),
                ],
            },
        }
    }
}

/// Tenant type configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantTypeConfig {
    /// Isolation level
    pub isolation_level: String,
    /// Whether custom configuration is allowed
    pub allow_custom_config: bool,
    /// Maximum number of users
    pub max_users: u32,
    /// Maximum number of projects
    pub max_projects: u32,
    /// Available features
    pub features: Vec<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: StorageBackend,
    /// Database connection string
    pub database_url: Option<String>,
    /// Redis connection string
    pub redis_url: Option<String>,
    /// File storage path
    pub file_path: Option<String>,
    /// S3 configuration
    pub s3: Option<S3Config>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Database,
            database_url: Some("sqlite:tenants.db".to_string()),
            redis_url: None,
            file_path: Some("./data/tenants".to_string()),
            s3: None,
        }
    }
}

/// Storage backend type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    Database,
    Redis,
    File,
    S3,
}

/// S3 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,
    /// AWS region
    pub region: String,
    /// AWS access key ID
    pub access_key_id: Option<String>,
    /// AWS secret access key
    pub secret_access_key: Option<String>,
    /// S3 endpoint URL (for custom endpoints)
    pub endpoint_url: Option<String>,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Whether to enable metrics collection
    pub enable_metrics: bool,
    /// Whether to enable usage tracking
    pub enable_usage_tracking: bool,
    /// Whether to enable audit logging
    pub enable_audit_logging: bool,
    /// Metrics collection interval
    pub metrics_interval: Duration,
    /// Usage tracking interval
    pub usage_tracking_interval: Duration,
    /// Audit log retention days
    pub audit_log_retention_days: u32,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            enable_usage_tracking: true,
            enable_audit_logging: true,
            metrics_interval: Duration::from_secs(60),
            usage_tracking_interval: Duration::from_secs(300),
            audit_log_retention_days: 90,
        }
    }
}

impl MultiTenancyConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: MultiTenancyConfig = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default quotas for tenant type
    pub fn get_default_quotas(&self, tenant_type: &str) -> Option<&TenantQuotas> {
        match tenant_type {
            "small" => Some(&self.default_quotas.small),
            "medium" => Some(&self.default_quotas.medium),
            "large" => Some(&self.default_quotas.large),
            "enterprise" => Some(&self.default_quotas.enterprise),
            _ => None,
        }
    }

    /// Get tenant type configuration
    pub fn get_tenant_type_config(&self, tenant_type: &str) -> Option<&TenantTypeConfig> {
        match tenant_type {
            "small" => Some(&self.tenant_types.small),
            "medium" => Some(&self.tenant_types.medium),
            "large" => Some(&self.tenant_types.large),
            "enterprise" => Some(&self.tenant_types.enterprise),
            _ => None,
        }
    }

    /// Check if feature is available for tenant type
    pub fn has_feature(&self, tenant_type: &str, feature: &str) -> bool {
        if let Some(config) = self.get_tenant_type_config(tenant_type) {
            config.features.contains(&feature.to_string())
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MultiTenancyConfig::default();
        assert!(config.enabled);
        assert!(!config.allow_cross_tenant_analytics);
        assert_eq!(config.warning_threshold, 80.0);
        assert_eq!(config.critical_threshold, 95.0);
    }

    #[test]
    fn test_default_quotas() {
        let config = MultiTenancyConfig::default();

        let small_quotas = config.get_default_quotas("small").unwrap();
        assert_eq!(small_quotas.cpu_cores, 0.5);
        assert_eq!(small_quotas.memory_bytes, 536_870_912);

        let enterprise_quotas = config.get_default_quotas("enterprise").unwrap();
        assert_eq!(enterprise_quotas.cpu_cores, 16.0);
        assert_eq!(enterprise_quotas.memory_bytes, 17_179_869_184);
    }

    #[test]
    fn test_tenant_type_config() {
        let config = MultiTenancyConfig::default();

        let small_config = config.get_tenant_type_config("small").unwrap();
        assert_eq!(small_config.max_users, 10);
        assert!(!small_config.allow_custom_config);

        let enterprise_config = config.get_tenant_type_config("enterprise").unwrap();
        assert_eq!(enterprise_config.max_users, 1000);
        assert!(enterprise_config.allow_custom_config);
    }

    #[test]
    fn test_feature_check() {
        let config = MultiTenancyConfig::default();

        assert!(config.has_feature("enterprise", "api_access"));
        assert!(!config.has_feature("small", "api_access"));
        assert!(config.has_feature("medium", "custom_dashboards"));
    }
}
