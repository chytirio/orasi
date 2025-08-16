//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration management modules for telemetry ingestion
//!
//! This module provides comprehensive configuration management including
//! environment-based configuration, validation, hot-reloading, and
//! configuration persistence.

pub mod manager;

pub use manager::{
    AlertRuleConfig, AlertingConfig, AuthenticationConfig, AuthorizationConfig,
    BatchProcessorConfig, ConfigurationBuilder, ConfigurationManager, ConnectionPoolConfig,
    CpuLimits, EncryptionConfig, EnvironmentOverride, ExporterConfig, HealthCheckConfig,
    IngestionConfig, MemoryLimits, MonitoringConfig, NotificationChannel, OAuthConfig,
    PerformanceConfig, ProcessorConfig, RbacConfig, ReceiverConfig, Role, SecurityConfig,
    SystemConfig, TlsConfig, ValidationResult, ValidationRule,
};

/// Configuration utilities
pub mod utils {
    use super::*;
    use std::path::PathBuf;

    /// Load configuration from multiple sources
    pub async fn load_configuration(
        config_path: Option<PathBuf>,
        env_prefix: &str,
    ) -> BridgeResult<IngestionConfig> {
        let mut config = if let Some(path) = config_path {
            ConfigurationManager::new(path).await?.get_config().await
        } else {
            ConfigurationBuilder::new().build()
        };

        // Apply environment variable overrides
        apply_environment_overrides(&mut config, env_prefix);

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_environment_overrides(config: &mut IngestionConfig, prefix: &str) {
        // System configuration overrides
        if let Ok(service_name) = std::env::var(format!("{}_SERVICE_NAME", prefix)) {
            config.system.service_name = service_name;
        }
        if let Ok(environment) = std::env::var(format!("{}_ENVIRONMENT", prefix)) {
            config.system.environment = environment;
        }
        if let Ok(log_level) = std::env::var(format!("{}_LOG_LEVEL", prefix)) {
            config.system.log_level = log_level;
        }

        // Performance configuration overrides
        if let Ok(max_connections) = std::env::var(format!("{}_MAX_CONNECTIONS", prefix)) {
            if let Ok(value) = max_connections.parse::<usize>() {
                config.performance.connection_pool.max_connections = value;
            }
        }
        if let Ok(max_batch_size) = std::env::var(format!("{}_MAX_BATCH_SIZE", prefix)) {
            if let Ok(value) = max_batch_size.parse::<usize>() {
                config.performance.batch_processing.max_batch_size = value;
            }
        }

        // Monitoring configuration overrides
        // Metrics port configuration removed - not supported in current MetricsConfig

        // Security configuration overrides
        if let Ok(auth_enabled) = std::env::var(format!("{}_AUTH_ENABLED", prefix)) {
            if let Ok(value) = auth_enabled.parse::<bool>() {
                config.security.authentication.enabled = value;
            }
        }
        if let Ok(jwt_secret) = std::env::var(format!("{}_JWT_SECRET", prefix)) {
            config.security.authentication.jwt_secret = Some(jwt_secret);
        }
    }

    /// Validate configuration
    pub fn validate_configuration(config: &IngestionConfig) -> BridgeResult<()> {
        let mut errors = Vec::new();

        // Validate system configuration
        if config.system.service_name.is_empty() {
            errors.push("Service name cannot be empty".to_string());
        }
        if config.system.environment.is_empty() {
            errors.push("Environment cannot be empty".to_string());
        }

        // Validate performance configuration
        if config.performance.connection_pool.max_connections == 0 {
            errors.push("Max connections must be greater than 0".to_string());
        }
        if config.performance.batch_processing.max_batch_size == 0 {
            errors.push("Max batch size must be greater than 0".to_string());
        }

        // Validate monitoring configuration
        // Metrics port validation removed - not supported in current MetricsConfig

        // Validate security configuration
        if config.security.authentication.enabled
            && config.security.authentication.jwt_secret.is_none()
        {
            errors.push("JWT secret is required when authentication is enabled".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "Configuration validation failed: {}",
                errors.join(", ")
            )))
        }
    }

    /// Create default configuration
    pub fn create_default_config() -> IngestionConfig {
        ConfigurationBuilder::new().build()
    }

    /// Create development configuration
    pub fn create_development_config() -> IngestionConfig {
        ConfigurationBuilder::new()
            .system(SystemConfig {
                service_name: "orasi-ingestion-dev".to_string(),
                service_version: "1.0.0".to_string(),
                environment: "development".to_string(),
                log_level: "debug".to_string(),
                data_dir: PathBuf::from("/tmp/orasi/data"),
                temp_dir: PathBuf::from("/tmp/orasi/temp"),
                max_concurrent_operations: 50,
                graceful_shutdown_timeout: std::time::Duration::from_secs(30),
            })
            .performance(PerformanceConfig {
                connection_pool: ConnectionPoolConfig {
                    max_connections: 50,
                    max_idle_connections: 10,
                    connection_timeout: std::time::Duration::from_secs(30),
                    idle_timeout: std::time::Duration::from_secs(300),
                    keep_alive_timeout: std::time::Duration::from_secs(60),
                },
                batch_processing: BatchProcessorConfig {
                    max_batch_size: 500,
                    max_batch_bytes: 5 * 1024 * 1024,
                    batch_timeout: std::time::Duration::from_secs(5),
                    max_memory_bytes: 50 * 1024 * 1024,
                    backpressure_threshold: 0.8,
                },
                memory_limits: MemoryLimits {
                    max_heap_size: 512 * 1024 * 1024,
                    max_stack_size: 4 * 1024 * 1024,
                    warning_threshold: 0.8,
                    critical_threshold: 0.95,
                },
                cpu_limits: CpuLimits {
                    max_cpu_percent: 60.0,
                    warning_threshold: 50.0,
                    critical_threshold: 80.0,
                },
            })
            .monitoring(MonitoringConfig {
                metrics: crate::monitoring::metrics::MetricsConfig {
                    enable_metrics: true,
                    collection_interval: std::time::Duration::from_secs(15),
                    enable_detailed_metrics: true,
                },
                health_check: HealthCheckConfig {
                    check_interval: std::time::Duration::from_secs(30),
                    timeout: std::time::Duration::from_secs(5),
                    endpoints: vec!["/health".to_string(), "/ready".to_string()],
                },
                alerting: AlertingConfig {
                    rules: Vec::new(),
                    notification_channels: Vec::new(),
                },
            })
            .security(SecurityConfig {
                authentication: AuthenticationConfig {
                    enabled: false,
                    method: "jwt".to_string(),
                    jwt_secret: None,
                    oauth: None,
                },
                authorization: AuthorizationConfig {
                    enabled: false,
                    rbac: RbacConfig {
                        roles: std::collections::HashMap::new(),
                        default_role: "user".to_string(),
                    },
                    permissions: std::collections::HashMap::new(),
                },
                tls: TlsConfig {
                    enabled: false,
                    cert_file: None,
                    key_file: None,
                    ca_file: None,
                    version: "1.3".to_string(),
                },
                encryption: EncryptionConfig {
                    enabled: false,
                    algorithm: "AES-256-GCM".to_string(),
                    key: None,
                    key_rotation_interval: std::time::Duration::from_secs(86400),
                },
            })
            .build()
    }

    /// Create production configuration
    pub fn create_production_config() -> IngestionConfig {
        ConfigurationBuilder::new()
            .system(SystemConfig {
                service_name: "orasi-ingestion-prod".to_string(),
                service_version: "1.0.0".to_string(),
                environment: "production".to_string(),
                log_level: "info".to_string(),
                data_dir: PathBuf::from("/var/lib/orasi/data"),
                temp_dir: PathBuf::from("/var/lib/orasi/temp"),
                max_concurrent_operations: 1000,
                graceful_shutdown_timeout: std::time::Duration::from_secs(60),
            })
            .performance(PerformanceConfig {
                connection_pool: ConnectionPoolConfig {
                    max_connections: 500,
                    max_idle_connections: 100,
                    connection_timeout: std::time::Duration::from_secs(30),
                    idle_timeout: std::time::Duration::from_secs(300),
                    keep_alive_timeout: std::time::Duration::from_secs(60),
                },
                batch_processing: BatchProcessorConfig {
                    max_batch_size: 2000,
                    max_batch_bytes: 20 * 1024 * 1024,
                    batch_timeout: std::time::Duration::from_secs(5),
                    max_memory_bytes: 200 * 1024 * 1024,
                    backpressure_threshold: 0.8,
                },
                memory_limits: MemoryLimits {
                    max_heap_size: 2 * 1024 * 1024 * 1024,
                    max_stack_size: 16 * 1024 * 1024,
                    warning_threshold: 0.8,
                    critical_threshold: 0.95,
                },
                cpu_limits: CpuLimits {
                    max_cpu_percent: 80.0,
                    warning_threshold: 70.0,
                    critical_threshold: 90.0,
                },
            })
            .monitoring(MonitoringConfig {
                metrics: crate::monitoring::metrics::MetricsConfig {
                    enable_metrics: true,
                    collection_interval: std::time::Duration::from_secs(15),
                    enable_detailed_metrics: true,
                },
                health_check: HealthCheckConfig {
                    check_interval: std::time::Duration::from_secs(30),
                    timeout: std::time::Duration::from_secs(5),
                    endpoints: vec!["/health".to_string(), "/ready".to_string()],
                },
                alerting: AlertingConfig {
                    rules: vec![
                        AlertRuleConfig {
                            name: "high_error_rate".to_string(),
                            condition: "error_rate > 0.05".to_string(),
                            severity: "critical".to_string(),
                            message: "Error rate is too high".to_string(),
                            enabled: true,
                        },
                        AlertRuleConfig {
                            name: "high_latency".to_string(),
                            condition: "avg_latency > 1000".to_string(),
                            severity: "warning".to_string(),
                            message: "Average latency is too high".to_string(),
                            enabled: true,
                        },
                    ],
                    notification_channels: vec![NotificationChannel {
                        name: "slack".to_string(),
                        channel_type: "slack".to_string(),
                        config: std::collections::HashMap::new(),
                    }],
                },
            })
            .security(SecurityConfig {
                authentication: AuthenticationConfig {
                    enabled: true,
                    method: "jwt".to_string(),
                    jwt_secret: Some("production-jwt-secret".to_string()),
                    oauth: None,
                },
                authorization: AuthorizationConfig {
                    enabled: true,
                    rbac: RbacConfig {
                        roles: std::collections::HashMap::new(),
                        default_role: "user".to_string(),
                    },
                    permissions: std::collections::HashMap::new(),
                },
                tls: TlsConfig {
                    enabled: true,
                    cert_file: Some(PathBuf::from("/etc/orasi/certs/cert.pem")),
                    key_file: Some(PathBuf::from("/etc/orasi/certs/key.pem")),
                    ca_file: Some(PathBuf::from("/etc/orasi/certs/ca.pem")),
                    version: "1.3".to_string(),
                },
                encryption: EncryptionConfig {
                    enabled: true,
                    algorithm: "AES-256-GCM".to_string(),
                    key: Some("production-encryption-key".to_string()),
                    key_rotation_interval: std::time::Duration::from_secs(86400),
                },
            })
            .build()
    }
}

use bridge_core::BridgeResult;
