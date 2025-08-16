//! Configuration service for integrating gRPC config updates with the bridge configuration system

use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use serde_json::Value;
use tokio::sync::RwLock;
use tokio::fs;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{info, warn, error};

use bridge_core::{BridgeResult, BridgeError, BridgeConfig};
use crate::{
    config::{BridgeAPIConfig, ConfigManager},
    metrics::ApiMetrics,
    proto::*,
};

/// Configuration management service for handling config updates
pub struct ConfigService {
    config: BridgeAPIConfig,
    bridge_config: Arc<RwLock<BridgeConfig>>,
    config_manager: Arc<ConfigManager>,
    metrics: ApiMetrics,
    config_path: PathBuf,
    last_config_hash: Arc<RwLock<String>>,
    component_restart_handlers: Arc<RwLock<HashMap<String, Box<dyn ComponentRestartHandler + Send + Sync>>>>,
}

/// Trait for components that can be restarted when configuration changes
#[async_trait::async_trait]
pub trait ComponentRestartHandler: Send + Sync {
    /// Get component name
    fn name(&self) -> &str;
    
    /// Restart the component with new configuration
    async fn restart(&self, config: &BridgeConfig) -> BridgeResult<()>;
    
    /// Check if component is healthy
    async fn health_check(&self) -> BridgeResult<bool>;
    
    /// Get component status
    async fn get_status(&self) -> BridgeResult<ComponentStatus>;
}

/// Component status information
#[derive(Debug, Clone)]
pub struct ComponentStatus {
    pub name: String,
    pub status: ComponentState,
    pub last_restart: Option<SystemTime>,
    pub restart_count: u32,
    pub error_message: Option<String>,
}

/// Component state
#[derive(Debug, Clone)]
pub enum ComponentState {
    Running,
    Starting,
    Stopping,
    Stopped,
    Error,
}

impl ConfigService {
    /// Create a new configuration service
    pub fn new(
        config: BridgeAPIConfig, 
        bridge_config: BridgeConfig,
        config_path: PathBuf,
        metrics: ApiMetrics,
    ) -> Self {
        let config_manager = Arc::new(ConfigManager::new(config.clone(), config_path.to_string_lossy().to_string()));
        let bridge_config = Arc::new(RwLock::new(bridge_config));
        let last_config_hash = Arc::new(RwLock::new(String::new()));
        let component_restart_handlers = Arc::new(RwLock::new(HashMap::new()));

        Self {
            config,
            bridge_config,
            config_manager,
            metrics,
            config_path,
            last_config_hash,
            component_restart_handlers,
        }
    }

    /// Register a component restart handler
    pub async fn register_component_handler(&self, handler: Box<dyn ComponentRestartHandler + Send + Sync>) {
        let handler_name = handler.name().to_string();
        let mut handlers = self.component_restart_handlers.write().await;
        handlers.insert(handler_name.clone(), handler);
        info!("Registered restart handler for component: {}", handler_name);
    }

    /// Unregister a component restart handler
    pub async fn unregister_component_handler(&self, component_name: &str) {
        let mut handlers = self.component_restart_handlers.write().await;
        handlers.remove(component_name);
        info!("Unregistered restart handler for component: {}", component_name);
    }

    /// Update bridge configuration
    pub async fn update_config(
        &self,
        config_request: &UpdateConfigRequest,
    ) -> BridgeResult<UpdateConfigResponse> {
        tracing::info!("Processing config update request: validate_only={}", config_request.validate_only);
        
        // Validate the configuration JSON
        let config_json = if config_request.validate_only {
            // Just validate the JSON structure
            match serde_json::from_str::<Value>(&config_request.config_json) {
                Ok(_) => {
                    tracing::info!("Configuration validation successful");
                    return Ok(UpdateConfigResponse {
                        success: true,
                        error_message: String::new(),
                        restarted_components: Vec::new(),
                        validation_errors: Vec::new(),
                    });
                }
                Err(e) => {
                    tracing::error!("Configuration validation failed: {}", e);
                    return Ok(UpdateConfigResponse {
                        success: false,
                        error_message: format!("Invalid JSON: {}", e),
                        restarted_components: Vec::new(),
                        validation_errors: vec![format!("Invalid JSON: {}", e)],
                    });
                }
            }
        } else {
            // Parse the configuration
            match serde_json::from_str::<Value>(&config_request.config_json) {
                Ok(config_value) => config_value,
                Err(e) => {
                    return Ok(UpdateConfigResponse {
                        success: false,
                        error_message: format!("Invalid JSON: {}", e),
                        restarted_components: Vec::new(),
                        validation_errors: vec![format!("Invalid JSON: {}", e)],
                    });
                }
            }
        };

        // Integrate with actual configuration management system
        let mut restarted_components = Vec::new();
        let mut validation_errors = Vec::new();

        // Validate configuration sections
        if let Some(config_obj) = config_json.as_object() {
            // Validate API configuration
            if let Some(api_config) = config_obj.get("api") {
                if let Err(e) = self.validate_api_config(api_config) {
                    validation_errors.push(format!("API config error: {}", e));
                }
            }

            // Validate gRPC configuration
            if let Some(grpc_config) = config_obj.get("grpc") {
                if let Err(e) = self.validate_grpc_config(grpc_config) {
                    validation_errors.push(format!("gRPC config error: {}", e));
                }
            }

            // Validate metrics configuration
            if let Some(metrics_config) = config_obj.get("metrics") {
                if let Err(e) = self.validate_metrics_config(metrics_config) {
                    validation_errors.push(format!("Metrics config error: {}", e));
                }
            }

            // Validate bridge-core configuration sections
            if let Some(bridge_config) = config_obj.get("bridge") {
                if let Err(e) = self.validate_bridge_config(bridge_config).await {
                    validation_errors.push(format!("Bridge config error: {}", e));
                }
            }
        }

        // Check if validation passed
        if !validation_errors.is_empty() {
            return Ok(UpdateConfigResponse {
                success: false,
                error_message: format!("Configuration validation failed: {}", validation_errors.join("; ")),
                restarted_components,
                validation_errors,
            });
        }

        // Apply configuration if validation passed
        if config_request.restart_components {
            match self.apply_configuration_and_restart(&config_json).await {
                Ok(restarted) => {
                    restarted_components = restarted;
                    tracing::info!("Configuration updated and components restarted: {:?}", restarted_components);
                }
                Err(e) => {
                    return Ok(UpdateConfigResponse {
                        success: false,
                        error_message: format!("Failed to apply configuration: {}", e),
                        restarted_components,
                        validation_errors: vec![format!("Application error: {}", e)],
                    });
                }
            }
        } else {
            // Just update configuration without restarting components
            if let Err(e) = self.update_bridge_configuration(&config_json).await {
                return Ok(UpdateConfigResponse {
                    success: false,
                    error_message: format!("Failed to update configuration: {}", e),
                    restarted_components,
                    validation_errors: vec![format!("Update error: {}", e)],
                });
            }
            tracing::info!("Configuration updated successfully");
        }

        Ok(UpdateConfigResponse {
            success: true,
            error_message: String::new(),
            restarted_components,
            validation_errors,
        })
    }

    /// Apply configuration and restart components
    async fn apply_configuration_and_restart(&self, config_json: &Value) -> BridgeResult<Vec<String>> {
        let mut restarted_components = Vec::new();

        // Update bridge configuration
        self.update_bridge_configuration(config_json).await?;

        // Get current bridge configuration
        let bridge_config = self.bridge_config.read().await;

        // Restart registered components
        let handlers = self.component_restart_handlers.read().await;
        for (component_name, handler) in handlers.iter() {
            info!("Restarting component: {}", component_name);
            
            match handler.restart(&bridge_config).await {
                Ok(_) => {
                    restarted_components.push(component_name.clone());
                    info!("Successfully restarted component: {}", component_name);
                }
                Err(e) => {
                    error!("Failed to restart component {}: {}", component_name, e);
                    // Continue with other components even if one fails
                }
            }
        }

        // Update configuration hash
        let config_hash = self.compute_config_hash(&bridge_config).await;
        {
            let mut hash = self.last_config_hash.write().await;
            *hash = config_hash;
        }

        // Persist configuration to file
        self.persist_configuration(config_json).await?;

        Ok(restarted_components)
    }

    /// Update bridge configuration without restarting components
    async fn update_bridge_configuration(&self, config_json: &Value) -> BridgeResult<()> {
        // Parse the configuration into BridgeConfig
        let new_bridge_config = match self.parse_bridge_config(config_json) {
            Ok(config) => config,
            Err(e) => return Err(BridgeError::configuration(format!("Failed to parse bridge configuration: {}", e))),
        };

        // Validate the new configuration
        new_bridge_config.validate_config()?;

        // Update the bridge configuration
        {
            let mut config = self.bridge_config.write().await;
            *config = new_bridge_config.clone();
        }

        // Update configuration hash
        let config_hash = self.compute_config_hash(&new_bridge_config).await;
        {
            let mut hash = self.last_config_hash.write().await;
            *hash = config_hash;
        }

        // Persist configuration to file
        self.persist_configuration(config_json).await?;

        info!("Bridge configuration updated successfully");
        Ok(())
    }

    /// Parse bridge configuration from JSON
    fn parse_bridge_config(&self, config_json: &Value) -> BridgeResult<BridgeConfig> {
        // Extract bridge configuration section
        let bridge_config_json = if let Some(bridge_section) = config_json.get("bridge") {
            bridge_section
        } else {
            // If no bridge section, treat the entire config as bridge config
            config_json
        };

        // Convert to string and parse
        let config_str = serde_json::to_string(bridge_config_json)
            .map_err(|e| BridgeError::serialization_with_source("Failed to serialize bridge config", e))?;

        BridgeConfig::from_str(&config_str)
    }

    /// Validate bridge configuration
    async fn validate_bridge_config(&self, bridge_config: &Value) -> BridgeResult<()> {
        // Parse and validate bridge configuration
        let config = self.parse_bridge_config(&serde_json::json!({
            "bridge": bridge_config
        }))?;

        config.validate_config()?;
        Ok(())
    }

    /// Compute configuration hash for change detection
    async fn compute_config_hash(&self, config: &BridgeConfig) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        format!("{:?}", config).hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Persist configuration to file
    async fn persist_configuration(&self, config_json: &Value) -> BridgeResult<()> {
        let config_str = serde_json::to_string_pretty(config_json)
            .map_err(|e| BridgeError::serialization_with_source("Failed to serialize configuration", e))?;

        fs::write(&self.config_path, config_str).await
            .map_err(|e| BridgeError::configuration_with_source("Failed to persist configuration", e))?;

        info!("Configuration persisted to: {:?}", self.config_path);
        Ok(())
    }

    /// Get current configuration
    pub async fn get_current_config(&self) -> BridgeResult<BridgeConfig> {
        let config = self.bridge_config.read().await;
        Ok(config.clone())
    }

    /// Get configuration hash
    pub async fn get_config_hash(&self) -> String {
        let hash = self.last_config_hash.read().await;
        hash.clone()
    }

    /// Check if configuration has changed
    pub async fn has_config_changed(&self) -> BridgeResult<bool> {
        let current_config = self.bridge_config.read().await;
        let current_hash = self.compute_config_hash(&current_config).await;
        let last_hash = self.last_config_hash.read().await;
        
        Ok(current_hash != *last_hash)
    }

    /// Reload configuration from file
    pub async fn reload_from_file(&self) -> BridgeResult<()> {
        info!("Reloading configuration from file: {:?}", self.config_path);
        
        // Load configuration from file
        let config = BridgeConfig::from_file(&self.config_path)?;
        
        // Update bridge configuration
        {
            let mut bridge_config = self.bridge_config.write().await;
            *bridge_config = config.clone();
        }

        // Update configuration hash
        let config_hash = self.compute_config_hash(&config).await;
        {
            let mut hash = self.last_config_hash.write().await;
            *hash = config_hash;
        }

        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Start configuration file watching
    pub async fn start_config_watching(&self) -> BridgeResult<()> {
        info!("Starting configuration file watching");
        
        let config_path = self.config_path.clone();
        let config_service = self.clone();

        tokio::spawn(async move {
            let mut interval = interval(TokioDuration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                if let Err(e) = config_service.check_file_changes().await {
                    error!("Error checking configuration file changes: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Check for configuration file changes
    async fn check_file_changes(&self) -> BridgeResult<()> {
        // Read file metadata to check for changes
        let metadata = fs::metadata(&self.config_path).await
            .map_err(|e| BridgeError::configuration_with_source("Failed to read config file metadata", e))?;
        
        let current_modified = metadata.modified()
            .map_err(|e| BridgeError::configuration_with_source("Failed to get file modification time", e))?;

        // Store last modified time (simplified - in production you'd want to persist this)
        static mut LAST_MODIFIED: Option<SystemTime> = None;
        
        unsafe {
            if let Some(last_modified) = LAST_MODIFIED {
                if current_modified > last_modified {
                    info!("Configuration file changed, reloading...");
                    self.reload_from_file().await?;
                }
            }
            LAST_MODIFIED = Some(current_modified);
        }

        Ok(())
    }

    /// Get component status
    pub async fn get_component_status(&self) -> BridgeResult<Vec<ComponentStatus>> {
        let mut statuses = Vec::new();
        let handlers = self.component_restart_handlers.read().await;
        
        for (component_name, handler) in handlers.iter() {
            match handler.get_status().await {
                Ok(status) => statuses.push(status),
                Err(e) => {
                    warn!("Failed to get status for component {}: {}", component_name, e);
                    statuses.push(ComponentStatus {
                        name: component_name.clone(),
                        status: ComponentState::Error,
                        last_restart: None,
                        restart_count: 0,
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(statuses)
    }

    /// Validate API configuration
    fn validate_api_config(&self, api_config: &Value) -> BridgeResult<()> {
        if let Some(api_obj) = api_config.as_object() {
            // Validate required fields
            if !api_obj.contains_key("host") {
                return Err(BridgeError::configuration("API host is required".to_string()));
            }
            
            if !api_obj.contains_key("port") {
                return Err(BridgeError::configuration("API port is required".to_string()));
            }

            // Validate port is a number
            if let Some(port_value) = api_obj.get("port") {
                if !port_value.is_number() {
                    return Err(BridgeError::configuration("API port must be a number".to_string()));
                }
                
                if let Some(port) = port_value.as_u64() {
                    if port == 0 || port > 65535 {
                        return Err(BridgeError::configuration("API port must be between 1 and 65535".to_string()));
                    }
                }
            }
        } else {
            return Err(BridgeError::configuration("API configuration must be an object".to_string()));
        }

        Ok(())
    }

    /// Validate gRPC configuration
    fn validate_grpc_config(&self, grpc_config: &Value) -> BridgeResult<()> {
        if let Some(grpc_obj) = grpc_config.as_object() {
            // Validate required fields
            if !grpc_obj.contains_key("host") {
                return Err(BridgeError::configuration("gRPC host is required".to_string()));
            }
            
            if !grpc_obj.contains_key("port") {
                return Err(BridgeError::configuration("gRPC port is required".to_string()));
            }

            // Validate port is a number
            if let Some(port_value) = grpc_obj.get("port") {
                if !port_value.is_number() {
                    return Err(BridgeError::configuration("gRPC port must be a number".to_string()));
                }
                
                if let Some(port) = port_value.as_u64() {
                    if port == 0 || port > 65535 {
                        return Err(BridgeError::configuration("gRPC port must be between 1 and 65535".to_string()));
                    }
                }
            }
        } else {
            return Err(BridgeError::configuration("gRPC configuration must be an object".to_string()));
        }

        Ok(())
    }

    /// Validate metrics configuration
    fn validate_metrics_config(&self, metrics_config: &Value) -> BridgeResult<()> {
        if let Some(metrics_obj) = metrics_config.as_object() {
            // Validate required fields
            if !metrics_obj.contains_key("enabled") {
                return Err(BridgeError::configuration("Metrics enabled flag is required".to_string()));
            }

            // Validate enabled is a boolean
            if let Some(enabled_value) = metrics_obj.get("enabled") {
                if !enabled_value.is_boolean() {
                    return Err(BridgeError::configuration("Metrics enabled must be a boolean".to_string()));
                }
            }

            // Validate port if provided
            if let Some(port_value) = metrics_obj.get("port") {
                if !port_value.is_number() {
                    return Err(BridgeError::configuration("Metrics port must be a number".to_string()));
                }
                
                if let Some(port) = port_value.as_u64() {
                    if port == 0 || port > 65535 {
                        return Err(BridgeError::configuration("Metrics port must be between 1 and 65535".to_string()));
                    }
                }
            }
        } else {
            return Err(BridgeError::configuration("Metrics configuration must be an object".to_string()));
        }

        Ok(())
    }
}

impl Clone for ConfigService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            bridge_config: self.bridge_config.clone(),
            config_manager: self.config_manager.clone(),
            metrics: self.metrics.clone(),
            config_path: self.config_path.clone(),
            last_config_hash: self.last_config_hash.clone(),
            component_restart_handlers: self.component_restart_handlers.clone(),
        }
    }
}
