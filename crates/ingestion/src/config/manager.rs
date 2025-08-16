//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration management for telemetry ingestion system
//!
//! This module provides comprehensive configuration management including
//! environment-based configuration, validation, hot-reloading, and
//! configuration persistence.

use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::{Mutex, RwLock};
use toml;
use tracing::{error, info};
use yaml_rust::{Yaml, YamlLoader};

use crate::monitoring::metrics::MetricsConfig;

/// Configuration manager
pub struct ConfigurationManager {
    config: Arc<RwLock<IngestionConfig>>,
    config_path: PathBuf,
    watcher: Option<notify::RecommendedWatcher>,
    validation_rules: Vec<ValidationRule>,
    hot_reload_enabled: bool,
    last_modified: Arc<Mutex<Instant>>,
}

/// Main ingestion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    /// System configuration
    pub system: SystemConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Receivers configuration
    pub receivers: HashMap<String, ReceiverConfig>,
    /// Processors configuration
    pub processors: HashMap<String, ProcessorConfig>,
    /// Exporters configuration
    pub exporters: HashMap<String, ExporterConfig>,
    /// Environment-specific overrides
    pub environments: HashMap<String, EnvironmentOverride>,
}

/// System configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Service name
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Environment (dev, staging, prod)
    pub environment: String,
    /// Log level
    pub log_level: String,
    /// Data directory
    pub data_dir: PathBuf,
    /// Temp directory
    pub temp_dir: PathBuf,
    /// Max concurrent operations
    pub max_concurrent_operations: usize,
    /// Graceful shutdown timeout
    pub graceful_shutdown_timeout: Duration,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Connection pool configuration
    pub connection_pool: ConnectionPoolConfig,
    /// Batch processing configuration
    pub batch_processing: BatchProcessorConfig,
    /// Memory limits
    pub memory_limits: MemoryLimits,
    /// CPU limits
    pub cpu_limits: CpuLimits,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum connections
    pub max_connections: usize,
    /// Maximum idle connections
    pub max_idle_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Idle timeout
    pub idle_timeout: Duration,
    /// Keep-alive timeout
    pub keep_alive_timeout: Duration,
}

/// Batch processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessorConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Maximum batch bytes
    pub max_batch_bytes: usize,
    /// Batch timeout
    pub batch_timeout: Duration,
    /// Maximum memory usage
    pub max_memory_bytes: usize,
    /// Backpressure threshold
    pub backpressure_threshold: f64,
}

/// Memory limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    /// Maximum heap size
    pub max_heap_size: usize,
    /// Maximum stack size
    pub max_stack_size: usize,
    /// Memory warning threshold
    pub warning_threshold: f64,
    /// Memory critical threshold
    pub critical_threshold: f64,
}

/// CPU limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuLimits {
    /// Maximum CPU usage percentage
    pub max_cpu_percent: f64,
    /// CPU warning threshold
    pub warning_threshold: f64,
    /// CPU critical threshold
    pub critical_threshold: f64,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Metrics configuration
    pub metrics: MetricsConfig,
    /// Health check configuration
    pub health_check: HealthCheckConfig,
    /// Alerting configuration
    pub alerting: AlertingConfig,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check interval
    pub check_interval: Duration,
    /// Health check timeout
    pub timeout: Duration,
    /// Health check endpoints
    pub endpoints: Vec<String>,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Alert rules
    pub rules: Vec<AlertRuleConfig>,
    /// Notification channels
    pub notification_channels: Vec<NotificationChannel>,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleConfig {
    /// Rule name
    pub name: String,
    /// Rule condition
    pub condition: String,
    /// Rule severity
    pub severity: String,
    /// Rule message
    pub message: String,
    /// Rule enabled
    pub enabled: bool,
}

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel name
    pub name: String,
    /// Channel type
    pub channel_type: String,
    /// Channel configuration
    pub config: HashMap<String, String>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Authentication configuration
    pub authentication: AuthenticationConfig,
    /// Authorization configuration
    pub authorization: AuthorizationConfig,
    /// TLS configuration
    pub tls: TlsConfig,
    /// Encryption configuration
    pub encryption: EncryptionConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Authentication enabled
    pub enabled: bool,
    /// Authentication method
    pub method: String,
    /// JWT secret
    pub jwt_secret: Option<String>,
    /// OAuth configuration
    pub oauth: Option<OAuthConfig>,
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Authorization URL
    pub auth_url: String,
    /// Token URL
    pub token_url: String,
    /// Redirect URL
    pub redirect_url: String,
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Authorization enabled
    pub enabled: bool,
    /// Role-based access control
    pub rbac: RbacConfig,
    /// Permission matrix
    pub permissions: HashMap<String, Vec<String>>,
}

/// RBAC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacConfig {
    /// Roles
    pub roles: HashMap<String, Role>,
    /// Default role
    pub default_role: String,
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,
    /// Role permissions
    pub permissions: Vec<String>,
    /// Role description
    pub description: String,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// TLS enabled
    pub enabled: bool,
    /// Certificate file
    pub cert_file: Option<PathBuf>,
    /// Key file
    pub key_file: Option<PathBuf>,
    /// CA file
    pub ca_file: Option<PathBuf>,
    /// TLS version
    pub version: String,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Encryption enabled
    pub enabled: bool,
    /// Encryption algorithm
    pub algorithm: String,
    /// Encryption key
    pub key: Option<String>,
    /// Key rotation interval
    pub key_rotation_interval: Duration,
}

/// Receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverConfig {
    /// Receiver name
    pub name: String,
    /// Receiver type
    pub receiver_type: String,
    /// Receiver enabled
    pub enabled: bool,
    /// Receiver configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Processor name
    pub name: String,
    /// Processor type
    pub processor_type: String,
    /// Processor enabled
    pub enabled: bool,
    /// Processor configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Exporter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExporterConfig {
    /// Exporter name
    pub name: String,
    /// Exporter type
    pub exporter_type: String,
    /// Exporter enabled
    pub enabled: bool,
    /// Exporter configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Environment override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentOverride {
    /// Override system config
    pub system: Option<SystemConfig>,
    /// Override performance config
    pub performance: Option<PerformanceConfig>,
    /// Override monitoring config
    pub monitoring: Option<MonitoringConfig>,
    /// Override security config
    pub security: Option<SecurityConfig>,
}

/// Validation rule
pub struct ValidationRule {
    /// Rule name
    pub name: String,
    /// Rule condition
    pub condition: Box<dyn Fn(&IngestionConfig) -> ValidationResult + Send + Sync>,
}

impl std::fmt::Debug for ValidationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationRule")
            .field("name", &self.name)
            .field("condition", &"<function>")
            .finish()
    }
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Is valid
    pub is_valid: bool,
    /// Error message
    pub error_message: Option<String>,
}

impl ConfigurationManager {
    /// Create new configuration manager
    pub async fn new(config_path: PathBuf) -> BridgeResult<Self> {
        let config = Self::load_config(&config_path).await?;
        let config = Arc::new(RwLock::new(config));
        let validation_rules = Self::create_default_validation_rules();
        let last_modified = Arc::new(Mutex::new(Instant::now()));

        Ok(Self {
            config,
            config_path,
            watcher: None,
            validation_rules,
            hot_reload_enabled: false,
            last_modified,
        })
    }

    /// Load configuration from file
    async fn load_config(config_path: &PathBuf) -> BridgeResult<IngestionConfig> {
        let config_content = fs::read_to_string(config_path).await.map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Failed to read config file: {}", e))
        })?;

        let config: IngestionConfig = match config_path.extension().and_then(|s| s.to_str()) {
            Some("json") => serde_json::from_str(&config_content).map_err(|e| {
                bridge_core::BridgeError::configuration(format!(
                    "Failed to parse JSON config: {}",
                    e
                ))
            })?,
            Some("toml") => toml::from_str(&config_content).map_err(|e| {
                bridge_core::BridgeError::configuration(format!(
                    "Failed to parse TOML config: {}",
                    e
                ))
            })?,
            Some("yaml") | Some("yml") => {
                let yaml_docs = YamlLoader::load_from_str(&config_content).map_err(|e| {
                    bridge_core::BridgeError::configuration(format!(
                        "Failed to parse YAML config: {}",
                        e
                    ))
                })?;
                if let Some(yaml_doc) = yaml_docs.first() {
                    Self::yaml_to_config(yaml_doc)?
                } else {
                    return Err(bridge_core::BridgeError::configuration(
                        "Empty YAML document".to_string(),
                    ));
                }
            }
            _ => {
                return Err(bridge_core::BridgeError::configuration(
                    "Unsupported config file format".to_string(),
                ))
            }
        };

        Ok(config)
    }

    /// Convert YAML to configuration
    fn yaml_to_config(_yaml: &Yaml) -> BridgeResult<IngestionConfig> {
        // This is a simplified implementation
        // In a real implementation, you would properly convert YAML to the config struct
        serde_json::from_value(serde_json::json!({
            "system": {
                "service_name": "orasi-ingestion",
                "service_version": "1.0.0",
                "environment": "development",
                "log_level": "info",
                "data_dir": "/tmp/orasi/data",
                "temp_dir": "/tmp/orasi/temp",
                "max_concurrent_operations": 100,
                "graceful_shutdown_timeout": 30
            },
            "performance": {
                "connection_pool": {
                    "max_connections": 100,
                    "max_idle_connections": 20,
                    "connection_timeout": 30,
                    "idle_timeout": 300,
                    "keep_alive_timeout": 60
                },
                "batch_processing": {
                    "max_batch_size": 1000,
                    "max_batch_bytes": 10485760,
                    "batch_timeout": 5,
                    "max_memory_bytes": 104857600,
                    "backpressure_threshold": 0.8
                },
                "memory_limits": {
                    "max_heap_size": 1073741824,
                    "max_stack_size": 8388608,
                    "warning_threshold": 0.8,
                    "critical_threshold": 0.95
                },
                "cpu_limits": {
                    "max_cpu_percent": 80.0,
                    "warning_threshold": 70.0,
                    "critical_threshold": 90.0
                }
            },
            "monitoring": {
                "metrics": {
                    "enable_prometheus": true,
                    "enable_custom_metrics": true,
                    "collection_interval": 15,
                    "retention_period": 3600,
                    "enable_detailed_metrics": true,
                    "metrics_port": 9090,
                    "metrics_path": "/metrics"
                },
                "health_check": {
                    "check_interval": 30,
                    "timeout": 5,
                    "endpoints": ["/health", "/ready"]
                },
                "alerting": {
                    "rules": [],
                    "notification_channels": []
                }
            },
            "security": {
                "authentication": {
                    "enabled": false,
                    "method": "jwt",
                    "jwt_secret": null,
                    "oauth": null
                },
                "authorization": {
                    "enabled": false,
                    "rbac": {
                        "roles": {},
                        "default_role": "user"
                    },
                    "permissions": {}
                },
                "tls": {
                    "enabled": false,
                    "cert_file": null,
                    "key_file": null,
                    "ca_file": null,
                    "version": "1.3"
                },
                "encryption": {
                    "enabled": false,
                    "algorithm": "AES-256-GCM",
                    "key": null,
                    "key_rotation_interval": 86400
                }
            },
            "receivers": {},
            "processors": {},
            "exporters": {},
            "environments": {}
        }))
        .map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Failed to parse config: {}", e))
        })
    }

    /// Create default validation rules
    fn create_default_validation_rules() -> Vec<ValidationRule> {
        vec![
            ValidationRule {
                name: "system_config".to_string(),
                condition: Box::new(|config| {
                    if config.system.service_name.is_empty() {
                        return ValidationResult {
                            is_valid: false,
                            error_message: Some("Service name cannot be empty".to_string()),
                        };
                    }
                    ValidationResult {
                        is_valid: true,
                        error_message: None,
                    }
                }),
            },
            ValidationRule {
                name: "performance_config".to_string(),
                condition: Box::new(|config| {
                    if config.performance.batch_processing.max_batch_size == 0 {
                        return ValidationResult {
                            is_valid: false,
                            error_message: Some(
                                "Max batch size must be greater than 0".to_string(),
                            ),
                        };
                    }
                    ValidationResult {
                        is_valid: true,
                        error_message: None,
                    }
                }),
            },
            ValidationRule {
                name: "security_config".to_string(),
                condition: Box::new(|config| {
                    if config.security.authentication.enabled
                        && config.security.authentication.jwt_secret.is_none()
                    {
                        return ValidationResult {
                            is_valid: false,
                            error_message: Some(
                                "JWT secret is required when authentication is enabled".to_string(),
                            ),
                        };
                    }
                    ValidationResult {
                        is_valid: true,
                        error_message: None,
                    }
                }),
            },
        ]
    }

    /// Enable hot reloading
    pub async fn enable_hot_reload(&mut self) -> BridgeResult<()> {
        if self.hot_reload_enabled {
            return Ok(());
        }

        let config_path = self.config_path.clone();
        let config = self.config.clone();
        let last_modified = self.last_modified.clone();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, _>| match res {
                Ok(event) => {
                    if event.kind.is_modify() {
                        let config_path = config_path.clone();
                        let config = config.clone();
                        let last_modified = last_modified.clone();

                        tokio::spawn(async move {
                            if let Ok(new_config) = Self::load_config(&config_path).await {
                                let mut config_write = config.write().await;
                                *config_write = new_config;
                                drop(config_write);

                                let mut last_modified_write = last_modified.lock().await;
                                *last_modified_write = Instant::now();
                                drop(last_modified_write);

                                info!("Configuration reloaded successfully");
                            } else {
                                error!("Failed to reload configuration");
                            }
                        });
                    }
                }
                Err(e) => error!("Configuration file watch error: {}", e),
            })
            .map_err(|e| {
                bridge_core::BridgeError::configuration(format!(
                    "Failed to create file watcher: {}",
                    e
                ))
            })?;

        watcher
            .watch(&self.config_path, RecursiveMode::NonRecursive)
            .map_err(|e| {
                bridge_core::BridgeError::configuration(format!(
                    "Failed to watch config file: {}",
                    e
                ))
            })?;

        self.watcher = Some(watcher);
        self.hot_reload_enabled = true;
        info!("Hot reloading enabled for configuration");

        Ok(())
    }

    /// Disable hot reloading
    pub async fn disable_hot_reload(&mut self) {
        self.watcher = None;
        self.hot_reload_enabled = false;
        info!("Hot reloading disabled for configuration");
    }

    /// Get current configuration
    pub async fn get_config(&self) -> IngestionConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, new_config: IngestionConfig) -> BridgeResult<()> {
        // Validate new configuration
        self.validate_config(&new_config)?;

        // Update configuration
        let mut config = self.config.write().await;
        *config = new_config;
        drop(config);

        // Update last modified timestamp
        let mut last_modified = self.last_modified.lock().await;
        *last_modified = Instant::now();

        info!("Configuration updated successfully");
        Ok(())
    }

    /// Validate configuration
    pub fn validate_config(&self, config: &IngestionConfig) -> BridgeResult<()> {
        for rule in &self.validation_rules {
            let result = (rule.condition)(config);
            if !result.is_valid {
                return Err(bridge_core::BridgeError::configuration(format!(
                    "Validation rule '{}' failed: {}",
                    rule.name,
                    result
                        .error_message
                        .unwrap_or_else(|| "Unknown error".to_string())
                )));
            }
        }
        Ok(())
    }

    /// Add validation rule
    pub fn add_validation_rule(&mut self, rule: ValidationRule) {
        self.validation_rules.push(rule);
    }

    /// Get environment-specific configuration
    pub async fn get_environment_config(&self, environment: &str) -> BridgeResult<IngestionConfig> {
        let config = self.get_config().await;

        if let Some(env_override) = config.environments.get(environment) {
            let mut env_config = config.clone();

            // Apply environment overrides
            if let Some(system) = &env_override.system {
                env_config.system = system.clone();
            }
            if let Some(performance) = &env_override.performance {
                env_config.performance = performance.clone();
            }
            if let Some(monitoring) = &env_override.monitoring {
                env_config.monitoring = monitoring.clone();
            }
            if let Some(security) = &env_override.security {
                env_config.security = security.clone();
            }

            Ok(env_config)
        } else {
            Ok(config)
        }
    }

    /// Save configuration to file
    pub async fn save_config(&self, config: &IngestionConfig) -> BridgeResult<()> {
        let config_content = match self.config_path.extension().and_then(|s| s.to_str()) {
            Some("json") => serde_json::to_string_pretty(config).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to serialize JSON config: {}",
                    e
                ))
            })?,
            Some("toml") => toml::to_string_pretty(config).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to serialize TOML config: {}",
                    e
                ))
            })?,
            _ => {
                return Err(bridge_core::BridgeError::configuration(
                    "Unsupported config file format".to_string(),
                ))
            }
        };

        fs::write(&self.config_path, config_content)
            .await
            .map_err(|e| {
                bridge_core::BridgeError::configuration(format!(
                    "Failed to write config file: {}",
                    e
                ))
            })?;

        info!("Configuration saved successfully");
        Ok(())
    }

    /// Get configuration from environment variables
    pub fn get_from_env(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }

    /// Set environment variable
    pub fn set_env(&self, key: &str, value: &str) {
        env::set_var(key, value);
    }

    /// Get last modification time
    pub async fn get_last_modified(&self) -> Instant {
        let last_modified = self.last_modified.lock().await;
        *last_modified
    }

    /// Check if hot reload is enabled
    pub fn is_hot_reload_enabled(&self) -> bool {
        self.hot_reload_enabled
    }
}

/// Configuration builder for easy configuration creation
pub struct ConfigurationBuilder {
    config: IngestionConfig,
}

impl ConfigurationBuilder {
    /// Create new configuration builder
    pub fn new() -> Self {
        Self {
            config: IngestionConfig {
                system: SystemConfig {
                    service_name: "orasi-ingestion".to_string(),
                    service_version: "1.0.0".to_string(),
                    environment: "development".to_string(),
                    log_level: "info".to_string(),
                    data_dir: PathBuf::from("/tmp/orasi/data"),
                    temp_dir: PathBuf::from("/tmp/orasi/temp"),
                    max_concurrent_operations: 100,
                    graceful_shutdown_timeout: Duration::from_secs(30),
                },
                performance: PerformanceConfig {
                    connection_pool: ConnectionPoolConfig {
                        max_connections: 100,
                        max_idle_connections: 20,
                        connection_timeout: Duration::from_secs(30),
                        idle_timeout: Duration::from_secs(300),
                        keep_alive_timeout: Duration::from_secs(60),
                    },
                    batch_processing: BatchProcessorConfig {
                        max_batch_size: 1000,
                        max_batch_bytes: 10 * 1024 * 1024,
                        batch_timeout: Duration::from_secs(5),
                        max_memory_bytes: 100 * 1024 * 1024,
                        backpressure_threshold: 0.8,
                    },
                    memory_limits: MemoryLimits {
                        max_heap_size: 1024 * 1024 * 1024,
                        max_stack_size: 8 * 1024 * 1024,
                        warning_threshold: 0.8,
                        critical_threshold: 0.95,
                    },
                    cpu_limits: CpuLimits {
                        max_cpu_percent: 80.0,
                        warning_threshold: 70.0,
                        critical_threshold: 90.0,
                    },
                },
                monitoring: MonitoringConfig {
                    metrics: MetricsConfig {
                        enable_metrics: true,
                        collection_interval: Duration::from_secs(15),
                        enable_detailed_metrics: true,
                    },
                    health_check: HealthCheckConfig {
                        check_interval: Duration::from_secs(30),
                        timeout: Duration::from_secs(5),
                        endpoints: vec!["/health".to_string(), "/ready".to_string()],
                    },
                    alerting: AlertingConfig {
                        rules: Vec::new(),
                        notification_channels: Vec::new(),
                    },
                },
                security: SecurityConfig {
                    authentication: AuthenticationConfig {
                        enabled: false,
                        method: "jwt".to_string(),
                        jwt_secret: None,
                        oauth: None,
                    },
                    authorization: AuthorizationConfig {
                        enabled: false,
                        rbac: RbacConfig {
                            roles: HashMap::new(),
                            default_role: "user".to_string(),
                        },
                        permissions: HashMap::new(),
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
                        key_rotation_interval: Duration::from_secs(86400),
                    },
                },
                receivers: HashMap::new(),
                processors: HashMap::new(),
                exporters: HashMap::new(),
                environments: HashMap::new(),
            },
        }
    }

    /// Set system configuration
    pub fn system(mut self, system: SystemConfig) -> Self {
        self.config.system = system;
        self
    }

    /// Set performance configuration
    pub fn performance(mut self, performance: PerformanceConfig) -> Self {
        self.config.performance = performance;
        self
    }

    /// Set monitoring configuration
    pub fn monitoring(mut self, monitoring: MonitoringConfig) -> Self {
        self.config.monitoring = monitoring;
        self
    }

    /// Set security configuration
    pub fn security(mut self, security: SecurityConfig) -> Self {
        self.config.security = security;
        self
    }

    /// Add receiver configuration
    pub fn receiver(mut self, name: String, config: ReceiverConfig) -> Self {
        self.config.receivers.insert(name, config);
        self
    }

    /// Add processor configuration
    pub fn processor(mut self, name: String, config: ProcessorConfig) -> Self {
        self.config.processors.insert(name, config);
        self
    }

    /// Add exporter configuration
    pub fn exporter(mut self, name: String, config: ExporterConfig) -> Self {
        self.config.exporters.insert(name, config);
        self
    }

    /// Add environment override
    pub fn environment(mut self, name: String, override_config: EnvironmentOverride) -> Self {
        self.config.environments.insert(name, override_config);
        self
    }

    /// Build configuration
    pub fn build(self) -> IngestionConfig {
        self.config
    }
}

use bridge_core::BridgeResult;
