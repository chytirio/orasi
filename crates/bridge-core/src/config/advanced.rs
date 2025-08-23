//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Advanced configuration management for the OpenTelemetry Data Lake Bridge
//!
//! This module provides advanced configuration features including:
//! - Dynamic configuration updates
//! - Hot reloading
//! - Configuration validation
//! - Environment variable support
//! - Configuration encryption
//! - Configuration versioning
//! - Configuration backup and restore

use crate::BridgeResult;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{watch, RwLock};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Advanced configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// Enable advanced features
    pub enabled: bool,

    /// Configuration file path
    pub config_file: Option<PathBuf>,

    /// Enable hot reloading
    pub hot_reload: bool,

    /// Enable configuration validation
    pub validation: bool,

    /// Enable configuration encryption
    pub encryption: bool,

    /// Enable configuration backup
    pub backup: bool,

    /// Enable configuration versioning
    pub versioning: bool,

    /// Environment variable substitution
    pub env_substitution: bool,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            config_file: None,
            hot_reload: false,
            validation: true,
            encryption: false,
            backup: true,
            versioning: true,
            env_substitution: true,
        }
    }
}

/// Configuration change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    /// Event ID
    pub id: Uuid,

    /// Timestamp of the change
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Configuration section that changed
    pub section: String,

    /// Change type
    pub change_type: ConfigChangeType,

    /// Old value (if available)
    pub old_value: Option<serde_json::Value>,

    /// New value
    pub new_value: serde_json::Value,

    /// Change description
    pub description: String,

    /// User who made the change
    pub user: Option<String>,
}

/// Configuration change type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigChangeType {
    Created,
    Updated,
    Deleted,
    Reloaded,
}

/// Configuration validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationResult {
    /// Is configuration valid
    pub is_valid: bool,

    /// Validation errors
    pub errors: Vec<ConfigValidationError>,

    /// Validation warnings
    pub warnings: Vec<ConfigValidationWarning>,

    /// Validation timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Configuration validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Configuration path
    pub path: String,

    /// Severity
    pub severity: ValidationSeverity,
}

/// Configuration validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationWarning {
    /// Warning code
    pub code: String,

    /// Warning message
    pub message: String,

    /// Configuration path
    pub path: String,

    /// Suggested fix
    pub suggestion: Option<String>,
}

/// Validation severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Configuration encryption settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEncryptionSettings {
    /// Enable configuration encryption
    pub enabled: bool,

    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,

    /// Key derivation function
    pub key_derivation: KeyDerivationFunction,

    /// Salt for key derivation
    pub salt: Option<String>,

    /// Environment variable for encryption key
    pub key_env_var: String,
}

/// Encryption algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    AES256GCM,
    ChaCha20Poly1305,
    XChaCha20Poly1305,
}

/// Key derivation function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyDerivationFunction {
    PBKDF2,
    Argon2,
    Scrypt,
}

/// Configuration backup settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigBackupSettings {
    /// Enable automatic backups
    pub enabled: bool,

    /// Backup directory
    pub backup_directory: PathBuf,

    /// Maximum number of backups to keep
    pub max_backups: usize,

    /// Backup interval in seconds
    pub backup_interval_seconds: u64,

    /// Compress backups
    pub compress_backups: bool,

    /// Encrypt backups
    pub encrypt_backups: bool,
}

/// Configuration versioning settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersioningSettings {
    /// Enable configuration versioning
    pub enabled: bool,

    /// Version history directory
    pub history_directory: PathBuf,

    /// Maximum number of versions to keep
    pub max_versions: usize,

    /// Auto-version on changes
    pub auto_version: bool,

    /// Version naming pattern
    pub version_pattern: String,
}

/// Advanced configuration manager
#[derive(Debug)]
pub struct AdvancedConfigManager {
    /// Configuration file path
    config_path: PathBuf,

    /// Current configuration
    current_config: Arc<RwLock<serde_json::Value>>,

    /// Configuration change watcher
    change_watcher: watch::Sender<ConfigChangeEvent>,

    /// Configuration change receiver
    change_receiver: watch::Receiver<ConfigChangeEvent>,

    /// Configuration validation rules
    validation_rules: Arc<RwLock<Vec<ConfigValidationRule>>>,

    /// Encryption settings
    encryption_settings: ConfigEncryptionSettings,

    /// Backup settings
    backup_settings: ConfigBackupSettings,

    /// Versioning settings
    versioning_settings: ConfigVersioningSettings,

    /// Environment variable mappings
    env_mappings: HashMap<String, String>,

    /// Configuration watcher
    file_watcher: Option<tokio::task::JoinHandle<()>>,

    /// Last modification time
    last_modified: Arc<RwLock<Option<Instant>>>,
}

/// Configuration validation rule
pub struct ConfigValidationRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// Validation function
    pub validator: Box<dyn Fn(&serde_json::Value) -> BridgeResult<()> + Send + Sync>,
}

impl std::fmt::Debug for ConfigValidationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigValidationRule")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("validator", &"<function>")
            .finish()
    }
}

impl AdvancedConfigManager {
    /// Create a new advanced configuration manager
    pub fn new(config_path: PathBuf) -> BridgeResult<Self> {
        let (change_watcher, change_receiver) = watch::channel(ConfigChangeEvent {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            section: "init".to_string(),
            change_type: ConfigChangeType::Created,
            old_value: None,
            new_value: serde_json::Value::Null,
            description: "Configuration manager initialized".to_string(),
            user: None,
        });

        let encryption_settings = ConfigEncryptionSettings {
            enabled: false,
            algorithm: EncryptionAlgorithm::AES256GCM,
            key_derivation: KeyDerivationFunction::PBKDF2,
            salt: None,
            key_env_var: "BRIDGE_CONFIG_KEY".to_string(),
        };

        let backup_settings = ConfigBackupSettings {
            enabled: true,
            backup_directory: PathBuf::from("./config/backups"),
            max_backups: 10,
            backup_interval_seconds: 3600,
            compress_backups: true,
            encrypt_backups: false,
        };

        let versioning_settings = ConfigVersioningSettings {
            enabled: true,
            history_directory: PathBuf::from("./config/versions"),
            max_versions: 50,
            auto_version: true,
            version_pattern: "v{timestamp}_{hash}".to_string(),
        };

        Ok(Self {
            config_path,
            current_config: Arc::new(RwLock::new(serde_json::Value::Null)),
            change_watcher,
            change_receiver,
            validation_rules: Arc::new(RwLock::new(Vec::new())),
            encryption_settings,
            backup_settings,
            versioning_settings,
            env_mappings: HashMap::new(),
            file_watcher: None,
            last_modified: Arc::new(RwLock::new(None)),
        })
    }

    /// Initialize the configuration manager
    pub async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing advanced configuration manager");

        // Create backup and version directories
        if self.backup_settings.enabled {
            fs::create_dir_all(&self.backup_settings.backup_directory)?;
        }

        if self.versioning_settings.enabled {
            fs::create_dir_all(&self.versioning_settings.history_directory)?;
        }

        // Load initial configuration
        self.load_configuration().await?;

        // Set up file watcher for hot reloading
        self.setup_file_watcher().await?;

        // Set up validation rules
        self.setup_default_validation_rules().await?;

        info!("Advanced configuration manager initialized successfully");
        Ok(())
    }

    /// Load configuration from file
    pub async fn load_configuration(&self) -> BridgeResult<()> {
        if !self.config_path.exists() {
            warn!("Configuration file does not exist: {:?}", self.config_path);
            return Ok(());
        }

        let config_content = fs::read_to_string(&self.config_path)?;

        // Handle encrypted configuration
        let config_json = if self.encryption_settings.enabled {
            self.decrypt_configuration(&config_content).await?
        } else {
            config_content
        };

        // Parse configuration
        let config: serde_json::Value = serde_json::from_str(&config_json)?;

        // Apply environment variable substitutions
        let config = self.apply_environment_substitutions(config).await?;

        // Validate configuration
        let validation_result = self.validate_configuration(&config).await;
        if !validation_result.is_valid {
            error!(
                "Configuration validation failed: {:?}",
                validation_result.errors
            );
            return Err(crate::BridgeError::configuration(
                "Configuration validation failed",
            ));
        }

        // Update current configuration
        {
            let mut current = self.current_config.write().await;
            *current = config;
        }

        // Create backup if enabled
        if self.backup_settings.enabled {
            self.create_backup().await?;
        }

        // Create version if enabled
        if self.versioning_settings.enabled && self.versioning_settings.auto_version {
            self.create_version().await?;
        }

        // Update last modified time
        {
            let mut last_modified = self.last_modified.write().await;
            *last_modified = Some(Instant::now());
        }

        info!(
            "Configuration loaded successfully from {:?}",
            self.config_path
        );
        Ok(())
    }

    /// Save configuration to file
    pub async fn save_configuration(&self, config: serde_json::Value) -> BridgeResult<()> {
        // Validate configuration before saving
        let validation_result = self.validate_configuration(&config).await;
        if !validation_result.is_valid {
            error!(
                "Configuration validation failed: {:?}",
                validation_result.errors
            );
            return Err(crate::BridgeError::configuration(
                "Configuration validation failed",
            ));
        }

        // Create backup before saving
        if self.backup_settings.enabled {
            self.create_backup().await?;
        }

        // Serialize configuration
        let config_json = serde_json::to_string_pretty(&config)?;

        // Encrypt configuration if enabled
        let final_content = if self.encryption_settings.enabled {
            self.encrypt_configuration(&config_json).await?
        } else {
            config_json
        };

        // Write to file
        fs::write(&self.config_path, final_content)?;

        // Update current configuration
        {
            let mut current = self.current_config.write().await;
            *current = config.clone();
        }

        // Emit change event
        self.emit_change_event(
            "all".to_string(),
            ConfigChangeType::Updated,
            None,
            config,
            "Configuration saved".to_string(),
            None,
        )
        .await;

        info!("Configuration saved successfully to {:?}", self.config_path);
        Ok(())
    }

    /// Get current configuration
    pub async fn get_configuration(&self) -> serde_json::Value {
        self.current_config.read().await.clone()
    }

    /// Update configuration section
    pub async fn update_configuration_section(
        &self,
        section: &str,
        value: serde_json::Value,
        user: Option<String>,
    ) -> BridgeResult<()> {
        let mut config = self.get_configuration().await;

        // Get old value for change event
        let old_value = config.get(section).cloned();

        // Update section
        if let Some(config_obj) = config.as_object_mut() {
            config_obj.insert(section.to_string(), value.clone());
        }

        // Save updated configuration
        self.save_configuration(config).await?;

        // Emit change event
        self.emit_change_event(
            section.to_string(),
            ConfigChangeType::Updated,
            old_value,
            value,
            format!("Configuration section '{}' updated", section),
            user,
        )
        .await;

        Ok(())
    }

    /// Watch for configuration changes
    pub fn watch_changes(&self) -> watch::Receiver<ConfigChangeEvent> {
        self.change_receiver.clone()
    }

    /// Add validation rule
    pub async fn add_validation_rule(&self, rule: ConfigValidationRule) {
        let mut rules = self.validation_rules.write().await;
        rules.push(rule);
    }

    /// Set environment variable mapping for substitution
    pub fn set_env_mapping(&mut self, config_var: String, env_var: String) {
        self.env_mappings.insert(config_var, env_var);
    }

    /// Clear all environment variable mappings
    pub fn clear_env_mappings(&mut self) {
        self.env_mappings.clear();
    }

    /// Validate configuration
    pub async fn validate_configuration(
        &self,
        config: &serde_json::Value,
    ) -> ConfigValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Run custom validation rules
        let rules = self.validation_rules.read().await;
        for rule in rules.iter() {
            if let Err(e) = (rule.validator)(config) {
                errors.push(ConfigValidationError {
                    code: "CUSTOM_VALIDATION".to_string(),
                    message: e.to_string(),
                    path: rule.name.clone(),
                    severity: ValidationSeverity::High,
                });
            }
        }

        // Run built-in validations
        self.run_built_in_validations(config, &mut errors, &mut warnings)
            .await;

        ConfigValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create configuration backup
    pub async fn create_backup(&self) -> BridgeResult<()> {
        if !self.backup_settings.enabled {
            return Ok(());
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("config_backup_{}.json", timestamp);
        let backup_path = self.backup_settings.backup_directory.join(&backup_filename);

        let config = self.get_configuration().await;
        let config_json = serde_json::to_string_pretty(&config)?;

        let backup_content = if self.backup_settings.compress_backups {
            self.compress_data(&config_json).await?
        } else {
            config_json
        };

        fs::write(&backup_path, backup_content)?;

        // Clean up old backups
        self.cleanup_old_backups().await?;

        info!("Configuration backup created: {:?}", backup_path);
        Ok(())
    }

    /// Create configuration version
    pub async fn create_version(&self) -> BridgeResult<()> {
        if !self.versioning_settings.enabled {
            return Ok(());
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let config_hash = self.calculate_config_hash().await?;
        let version_filename = format!("config_v{}_{}.json", timestamp, config_hash);
        let version_path = self
            .versioning_settings
            .history_directory
            .join(&version_filename);

        let config = self.get_configuration().await;
        let config_json = serde_json::to_string_pretty(&config)?;
        fs::write(&version_path, config_json)?;

        // Clean up old versions
        self.cleanup_old_versions().await?;

        info!("Configuration version created: {:?}", version_path);
        Ok(())
    }

    /// Restore configuration from backup
    pub async fn restore_from_backup(&self, backup_filename: &str) -> BridgeResult<()> {
        let backup_path = self.backup_settings.backup_directory.join(backup_filename);

        if !backup_path.exists() {
            return Err(crate::BridgeError::configuration(format!(
                "Backup file not found: {:?}",
                backup_path
            )));
        }

        let backup_content = fs::read_to_string(&backup_path)?;

        // Handle compressed backups
        let config_json = if self.backup_settings.compress_backups {
            self.decompress_data(&backup_content).await?
        } else {
            backup_content
        };

        let config: serde_json::Value = serde_json::from_str(&config_json)?;

        // Validate restored configuration
        let validation_result = self.validate_configuration(&config).await;
        if !validation_result.is_valid {
            return Err(crate::BridgeError::configuration(
                "Restored configuration validation failed",
            ));
        }

        // Save restored configuration
        self.save_configuration(config).await?;

        info!("Configuration restored from backup: {:?}", backup_path);
        Ok(())
    }

    /// List available backups
    pub async fn list_backups(&self) -> BridgeResult<Vec<String>> {
        if !self.backup_settings.backup_directory.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();
        for entry in fs::read_dir(&self.backup_settings.backup_directory)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = entry.file_name().to_str() {
                    backups.push(filename.to_string());
                }
            }
        }

        backups.sort();
        Ok(backups)
    }

    /// List available versions
    pub async fn list_versions(&self) -> BridgeResult<Vec<String>> {
        if !self.versioning_settings.history_directory.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in fs::read_dir(&self.versioning_settings.history_directory)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = entry.file_name().to_str() {
                    versions.push(filename.to_string());
                }
            }
        }

        versions.sort();
        Ok(versions)
    }

    /// Set up file watcher for hot reloading
    async fn setup_file_watcher(&mut self) -> BridgeResult<()> {
        let config_path = self.config_path.clone();
        let last_modified = self.last_modified.clone();
        let change_watcher = self.change_watcher.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                if let Ok(metadata) = fs::metadata(&config_path) {
                    if let Ok(modified) = metadata.modified() {
                        let modified_instant = Instant::now(); // Simplified for now

                        let should_reload = {
                            let last_modified_guard = last_modified.read().await;
                            if let Some(last) = *last_modified_guard {
                                modified_instant > last
                            } else {
                                true
                            }
                        };

                        if should_reload {
                            info!("Configuration file modified, triggering reload");

                            // Emit reload event
                            let _ = change_watcher.send(ConfigChangeEvent {
                                id: Uuid::new_v4(),
                                timestamp: chrono::Utc::now(),
                                section: "file_watcher".to_string(),
                                change_type: ConfigChangeType::Reloaded,
                                old_value: None,
                                new_value: serde_json::Value::Null,
                                description: "Configuration file modified, reload triggered"
                                    .to_string(),
                                user: None,
                            });
                        }
                    }
                }
            }
        });

        self.file_watcher = Some(handle);
        Ok(())
    }

    /// Set up default validation rules
    async fn setup_default_validation_rules(&self) -> BridgeResult<()> {
        // Add basic validation rules
        self.add_validation_rule(ConfigValidationRule {
            name: "required_fields".to_string(),
            description: "Check for required configuration fields".to_string(),
            validator: Box::new(|config| {
                if let Some(obj) = config.as_object() {
                    if !obj.contains_key("version") {
                        return Err(crate::BridgeError::configuration(
                            "Configuration must contain 'version' field",
                        ));
                    }
                }
                Ok(())
            }),
        })
        .await;

        Ok(())
    }

    /// Run built-in validations
    async fn run_built_in_validations(
        &self,
        config: &serde_json::Value,
        errors: &mut Vec<ConfigValidationError>,
        warnings: &mut Vec<ConfigValidationWarning>,
    ) {
        // Validate JSON structure
        if !config.is_object() && !config.is_array() {
            errors.push(ConfigValidationError {
                code: "INVALID_STRUCTURE".to_string(),
                message: "Configuration must be an object or array".to_string(),
                path: "root".to_string(),
                severity: ValidationSeverity::Critical,
            });
            return;
        }

        // Check for circular references (basic check)
        if let Err(e) = serde_json::to_string(config) {
            errors.push(ConfigValidationError {
                code: "CIRCULAR_REFERENCE".to_string(),
                message: format!("Circular reference detected: {}", e),
                path: "root".to_string(),
                severity: ValidationSeverity::Critical,
            });
        }

        // Validate required top-level fields
        if let Some(obj) = config.as_object() {
            // Check for version field
            if !obj.contains_key("version") {
                warnings.push(ConfigValidationWarning {
                    code: "MISSING_VERSION".to_string(),
                    message: "Configuration should include a 'version' field".to_string(),
                    path: "root".to_string(),
                    suggestion: Some(
                        "Add a 'version' field to track configuration schema version".to_string(),
                    ),
                });
            }

            // Check for deprecated fields
            let deprecated_fields = ["legacy_setting", "old_config"];
            for field in deprecated_fields {
                if obj.contains_key(field) {
                    warnings.push(ConfigValidationWarning {
                        code: "DEPRECATED_FIELD".to_string(),
                        message: format!("Field '{}' is deprecated", field),
                        path: format!("root.{}", field),
                        suggestion: Some(format!("Remove or replace the '{}' field", field)),
                    });
                }
            }

            // Validate nested objects recursively
            self.validate_nested_objects(obj, "root", errors, warnings)
                .await;
        }
    }

    /// Apply environment variable substitutions
    async fn apply_environment_substitutions(
        &self,
        config: serde_json::Value,
    ) -> BridgeResult<serde_json::Value> {
        if !self.env_mappings.is_empty() {
            return self.substitute_environment_variables(config).await;
        }
        Ok(config)
    }

    /// Substitute environment variables in configuration
    async fn substitute_environment_variables(
        &self,
        config: serde_json::Value,
    ) -> BridgeResult<serde_json::Value> {
        self.substitute_environment_variables_inner(config).await
    }

    /// Inner implementation to avoid recursive async issues
    async fn substitute_environment_variables_inner(
        &self,
        config: serde_json::Value,
    ) -> BridgeResult<serde_json::Value> {
        match config {
            serde_json::Value::String(s) => {
                // Check if string contains environment variable pattern ${VAR_NAME}
                if s.contains("${") && s.contains("}") {
                    let mut result = s.clone();
                    for (var_name, env_var) in &self.env_mappings {
                        let pattern = format!("${{{}}}", var_name);
                        if let Ok(env_value) = std::env::var(env_var) {
                            result = result.replace(&pattern, &env_value);
                        } else {
                            warn!(
                                "Environment variable {} not found for mapping {}",
                                env_var, var_name
                            );
                        }
                    }
                    Ok(serde_json::Value::String(result))
                } else {
                    Ok(serde_json::Value::String(s))
                }
            }
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (key, value) in map {
                    let substituted_value =
                        Box::pin(self.substitute_environment_variables_inner(value)).await?;
                    new_map.insert(key, substituted_value);
                }
                Ok(serde_json::Value::Object(new_map))
            }
            serde_json::Value::Array(arr) => {
                let mut new_arr = Vec::new();
                for value in arr {
                    let substituted_value =
                        Box::pin(self.substitute_environment_variables_inner(value)).await?;
                    new_arr.push(substituted_value);
                }
                Ok(serde_json::Value::Array(new_arr))
            }
            _ => Ok(config),
        }
    }

    /// Encrypt configuration
    async fn encrypt_configuration(&self, config: &str) -> BridgeResult<String> {
        if !self.encryption_settings.enabled {
            return Ok(config.to_string());
        }

        // Get encryption key from environment
        let key = std::env::var(&self.encryption_settings.key_env_var).map_err(|_| {
            crate::BridgeError::configuration(format!(
                "Encryption key not found in environment variable: {}",
                self.encryption_settings.key_env_var
            ))
        })?;

        // Derive encryption key
        let derived_key = self.derive_key(&key).await?;

        // Encrypt the configuration
        let encrypted = match self.encryption_settings.algorithm {
            EncryptionAlgorithm::AES256GCM => {
                self.encrypt_aes256gcm(config.as_bytes(), &derived_key)
                    .await?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.encrypt_chacha20poly1305(config.as_bytes(), &derived_key)
                    .await?
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                self.encrypt_xchacha20poly1305(config.as_bytes(), &derived_key)
                    .await?
            }
        };

        // Return base64 encoded encrypted data
        Ok(base64::engine::general_purpose::STANDARD.encode(encrypted))
    }

    /// Decrypt configuration
    async fn decrypt_configuration(&self, config: &str) -> BridgeResult<String> {
        if !self.encryption_settings.enabled {
            return Ok(config.to_string());
        }

        // Get encryption key from environment
        let key = std::env::var(&self.encryption_settings.key_env_var).map_err(|_| {
            crate::BridgeError::configuration(format!(
                "Encryption key not found in environment variable: {}",
                self.encryption_settings.key_env_var
            ))
        })?;

        // Derive encryption key
        let derived_key = self.derive_key(&key).await?;

        // Decode base64 encrypted data
        let encrypted_data = base64::engine::general_purpose::STANDARD
            .decode(config)
            .map_err(|e| {
                crate::BridgeError::configuration(format!(
                    "Failed to decode base64 encrypted data: {}",
                    e
                ))
            })?;

        // Decrypt the configuration
        let decrypted = match self.encryption_settings.algorithm {
            EncryptionAlgorithm::AES256GCM => {
                self.decrypt_aes256gcm(&encrypted_data, &derived_key)
                    .await?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.decrypt_chacha20poly1305(&encrypted_data, &derived_key)
                    .await?
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                self.decrypt_xchacha20poly1305(&encrypted_data, &derived_key)
                    .await?
            }
        };

        // Convert bytes back to string
        String::from_utf8(decrypted).map_err(|e| {
            crate::BridgeError::configuration(format!(
                "Failed to convert decrypted data to string: {}",
                e
            ))
        })
    }

    /// Compress data using gzip
    async fn compress_data(&self, data: &str) -> BridgeResult<String> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data.as_bytes()).map_err(|e| {
            crate::BridgeError::configuration(format!("Failed to compress data: {}", e))
        })?;

        let compressed = encoder.finish().map_err(|e| {
            crate::BridgeError::configuration(format!("Failed to finish compression: {}", e))
        })?;

        Ok(base64::engine::general_purpose::STANDARD.encode(compressed))
    }

    /// Decompress data using gzip
    async fn decompress_data(&self, compressed_data: &str) -> BridgeResult<String> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let compressed_bytes = base64::engine::general_purpose::STANDARD
            .decode(compressed_data)
            .map_err(|e| {
                crate::BridgeError::configuration(format!(
                    "Failed to decode base64 compressed data: {}",
                    e
                ))
            })?;

        let mut decoder = GzDecoder::new(&compressed_bytes[..]);
        let mut decompressed = String::new();
        decoder.read_to_string(&mut decompressed).map_err(|e| {
            crate::BridgeError::configuration(format!("Failed to decompress data: {}", e))
        })?;

        Ok(decompressed)
    }

    /// Derive encryption key using the configured KDF
    async fn derive_key(&self, password: &str) -> BridgeResult<Vec<u8>> {
        let salt = self
            .encryption_settings
            .salt
            .as_deref()
            .unwrap_or("default_salt");

        match self.encryption_settings.key_derivation {
            KeyDerivationFunction::PBKDF2 => {
                use pbkdf2::{pbkdf2, pbkdf2_hmac};
                use sha2::Sha256;

                let mut key = [0u8; 32];
                pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), 10000, &mut key);
                Ok(key.to_vec())
            }
            KeyDerivationFunction::Argon2 => {
                use argon2::password_hash::{rand_core::OsRng, SaltString};
                use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

                let salt = SaltString::generate(&mut OsRng);
                let argon2 = Argon2::default();
                let password_hash =
                    argon2
                        .hash_password(password.as_bytes(), &salt)
                        .map_err(|e| {
                            crate::BridgeError::configuration(format!(
                                "Failed to hash password with Argon2: {}",
                                e
                            ))
                        })?;

                Ok(password_hash.hash.unwrap().as_bytes().to_vec())
            }
            KeyDerivationFunction::Scrypt => {
                use scrypt::{scrypt, Params};

                let params = Params::new(14, 8, 1, 32).map_err(|e| {
                    crate::BridgeError::configuration(format!(
                        "Failed to create scrypt params: {}",
                        e
                    ))
                })?;

                let mut key = [0u8; 32];
                scrypt(password.as_bytes(), salt.as_bytes(), &params, &mut key).map_err(|e| {
                    crate::BridgeError::configuration(format!(
                        "Failed to derive key with scrypt: {}",
                        e
                    ))
                })?;

                Ok(key.to_vec())
            }
        }
    }

    /// Encrypt data using AES-256-GCM
    async fn encrypt_aes256gcm(&self, data: &[u8], key: &[u8]) -> BridgeResult<Vec<u8>> {
        use aes_gcm::aead::{Aead, KeyInit};
        use aes_gcm::{Aes256Gcm, Key, Nonce};

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Nonce::from_slice(b"unique nonce"); // In production, use a random nonce

        cipher.encrypt(nonce, data).map_err(|e| {
            crate::BridgeError::configuration(format!("Failed to encrypt with AES-256-GCM: {}", e))
        })
    }

    /// Decrypt data using AES-256-GCM
    async fn decrypt_aes256gcm(&self, encrypted_data: &[u8], key: &[u8]) -> BridgeResult<Vec<u8>> {
        use aes_gcm::aead::{Aead, KeyInit};
        use aes_gcm::{Aes256Gcm, Key, Nonce};

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Nonce::from_slice(b"unique nonce"); // In production, use a random nonce

        cipher.decrypt(nonce, encrypted_data).map_err(|e| {
            crate::BridgeError::configuration(format!("Failed to decrypt with AES-256-GCM: {}", e))
        })
    }

    /// Encrypt data using ChaCha20-Poly1305
    async fn encrypt_chacha20poly1305(&self, data: &[u8], key: &[u8]) -> BridgeResult<Vec<u8>> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Nonce::from_slice(b"unique nonce"); // In production, use a random nonce

        cipher.encrypt(nonce, data).map_err(|e| {
            crate::BridgeError::configuration(format!(
                "Failed to encrypt with ChaCha20-Poly1305: {}",
                e
            ))
        })
    }

    /// Decrypt data using ChaCha20-Poly1305
    async fn decrypt_chacha20poly1305(
        &self,
        encrypted_data: &[u8],
        key: &[u8],
    ) -> BridgeResult<Vec<u8>> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Nonce::from_slice(b"unique nonce"); // In production, use a random nonce

        cipher.decrypt(nonce, encrypted_data).map_err(|e| {
            crate::BridgeError::configuration(format!(
                "Failed to decrypt with ChaCha20-Poly1305: {}",
                e
            ))
        })
    }

    /// Encrypt data using XChaCha20-Poly1305
    async fn encrypt_xchacha20poly1305(&self, data: &[u8], key: &[u8]) -> BridgeResult<Vec<u8>> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};

        let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = XNonce::from_slice(b"unique nonce 24 bytes"); // In production, use a random nonce

        cipher.encrypt(nonce, data).map_err(|e| {
            crate::BridgeError::configuration(format!(
                "Failed to encrypt with XChaCha20-Poly1305: {}",
                e
            ))
        })
    }

    /// Decrypt data using XChaCha20-Poly1305
    async fn decrypt_xchacha20poly1305(
        &self,
        encrypted_data: &[u8],
        key: &[u8],
    ) -> BridgeResult<Vec<u8>> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};

        let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = XNonce::from_slice(b"unique nonce 24 bytes"); // In production, use a random nonce

        cipher.decrypt(nonce, encrypted_data).map_err(|e| {
            crate::BridgeError::configuration(format!(
                "Failed to decrypt with XChaCha20-Poly1305: {}",
                e
            ))
        })
    }

    /// Validate nested objects recursively
    async fn validate_nested_objects(
        &self,
        obj: &serde_json::Map<String, serde_json::Value>,
        path: &str,
        errors: &mut Vec<ConfigValidationError>,
        warnings: &mut Vec<ConfigValidationWarning>,
    ) {
        self.validate_nested_objects_inner(obj, path, errors, warnings)
            .await;
    }

    /// Inner implementation to avoid recursive async issues
    async fn validate_nested_objects_inner(
        &self,
        obj: &serde_json::Map<String, serde_json::Value>,
        path: &str,
        errors: &mut Vec<ConfigValidationError>,
        warnings: &mut Vec<ConfigValidationWarning>,
    ) {
        for (key, value) in obj {
            let current_path = format!("{}.{}", path, key);

            // Check for null values
            if value.is_null() {
                warnings.push(ConfigValidationWarning {
                    code: "NULL_VALUE".to_string(),
                    message: format!("Field '{}' has a null value", current_path),
                    path: current_path.clone(),
                    suggestion: Some(
                        "Consider removing null values or providing defaults".to_string(),
                    ),
                });
            }

            // Recursively validate nested objects
            if let Some(nested_obj) = value.as_object() {
                Box::pin(self.validate_nested_objects_inner(
                    nested_obj,
                    &current_path,
                    errors,
                    warnings,
                ))
                .await;
            }

            // Check for empty strings
            if let Some(s) = value.as_str() {
                if s.trim().is_empty() {
                    warnings.push(ConfigValidationWarning {
                        code: "EMPTY_STRING".to_string(),
                        message: format!("Field '{}' has an empty string value", current_path),
                        path: current_path.clone(),
                        suggestion: Some(
                            "Consider providing a meaningful value or removing the field"
                                .to_string(),
                        ),
                    });
                }
            }
        }
    }

    /// Calculate configuration hash
    async fn calculate_config_hash(&self) -> BridgeResult<String> {
        let config = self.get_configuration().await;
        let config_json = serde_json::to_string(&config)?;

        // Simple hash for now - in production, use a proper hash function
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        config_json.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }

    /// Clean up old backups
    async fn cleanup_old_backups(&self) -> BridgeResult<()> {
        let backups = self.list_backups().await?;

        if backups.len() > self.backup_settings.max_backups {
            let to_remove = backups.len() - self.backup_settings.max_backups;
            for backup in backups.iter().take(to_remove) {
                let backup_path = self.backup_settings.backup_directory.join(backup);
                if let Err(e) = fs::remove_file(&backup_path) {
                    warn!("Failed to remove old backup {:?}: {}", backup_path, e);
                }
            }
        }

        Ok(())
    }

    /// Clean up old versions
    async fn cleanup_old_versions(&self) -> BridgeResult<()> {
        let versions = self.list_versions().await?;

        if versions.len() > self.versioning_settings.max_versions {
            let to_remove = versions.len() - self.versioning_settings.max_versions;
            for version in versions.iter().take(to_remove) {
                let version_path = self.versioning_settings.history_directory.join(version);
                if let Err(e) = fs::remove_file(&version_path) {
                    warn!("Failed to remove old version {:?}: {}", version_path, e);
                }
            }
        }

        Ok(())
    }

    /// Emit configuration change event
    async fn emit_change_event(
        &self,
        section: String,
        change_type: ConfigChangeType,
        old_value: Option<serde_json::Value>,
        new_value: serde_json::Value,
        description: String,
        user: Option<String>,
    ) {
        let event = ConfigChangeEvent {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            section,
            change_type,
            old_value,
            new_value,
            description,
            user,
        };

        if let Err(e) = self.change_watcher.send(event) {
            warn!("Failed to emit configuration change event: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = AdvancedConfigManager::new(config_path);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_config_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = AdvancedConfigManager::new(config_path.clone()).unwrap();
        manager.init().await.unwrap();

        let valid_config = serde_json::json!({
            "version": "1.0.0",
            "name": "test"
        });

        let validation_result = manager.validate_configuration(&valid_config).await;
        assert!(validation_result.is_valid);
    }

    #[tokio::test]
    async fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = AdvancedConfigManager::new(config_path.clone()).unwrap();
        manager.init().await.unwrap();

        let config = serde_json::json!({
            "version": "1.0.0",
            "name": "test",
            "settings": {
                "enabled": true
            }
        });

        manager.save_configuration(config.clone()).await.unwrap();
        manager.load_configuration().await.unwrap();

        let loaded_config = manager.get_configuration().await;
        assert_eq!(config, loaded_config);
    }

    #[tokio::test]
    async fn test_compression() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = AdvancedConfigManager::new(config_path.clone()).unwrap();
        manager.init().await.unwrap();

        let test_data = "This is a test string that should be compressed and then decompressed";
        let compressed = manager.compress_data(test_data).await.unwrap();
        let decompressed = manager.decompress_data(&compressed).await.unwrap();

        assert_eq!(test_data, decompressed);
    }

    #[tokio::test]
    async fn test_environment_substitution() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = AdvancedConfigManager::new(config_path.clone()).unwrap();
        manager.init().await.unwrap();

        // Set up environment variable
        std::env::set_var("TEST_DB_HOST", "localhost");
        manager.set_env_mapping("db_host".to_string(), "TEST_DB_HOST".to_string());

        let config_with_env = serde_json::json!({
            "version": "1.0.0",
            "database": {
                "host": "${db_host}",
                "port": 5432
            }
        });

        let substituted = manager
            .substitute_environment_variables(config_with_env)
            .await
            .unwrap();

        if let Some(database) = substituted.get("database") {
            if let Some(host) = database.get("host") {
                assert_eq!(host.as_str().unwrap(), "localhost");
            }
        }
    }

    #[tokio::test]
    async fn test_backup_compression() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = AdvancedConfigManager::new(config_path.clone()).unwrap();
        manager.backup_settings.compress_backups = true;
        manager.init().await.unwrap();

        let config = serde_json::json!({
            "version": "1.0.0",
            "name": "test_backup"
        });

        manager.save_configuration(config.clone()).await.unwrap();

        // Verify backup was created
        let backups = manager.list_backups().await.unwrap();
        assert!(!backups.is_empty());
    }
}
