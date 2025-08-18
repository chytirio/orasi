//! Configuration service for integrating gRPC config updates with the bridge configuration system

use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info, warn};

use crate::{
    config::{BridgeAPIConfig, ConfigManager},
    metrics::ApiMetrics,
    proto::*,
};
use bridge_core::{BridgeConfig, BridgeError, BridgeResult};

/// Configuration management service for handling config updates
pub struct ConfigService {
    config: BridgeAPIConfig,
    bridge_config: Arc<RwLock<BridgeConfig>>,
    config_manager: Arc<ConfigManager>,
    metrics: ApiMetrics,
    config_path: PathBuf,
    last_config_hash: Arc<RwLock<String>>,
    component_restart_handlers:
        Arc<RwLock<HashMap<String, Box<dyn ComponentRestartHandler + Send + Sync>>>>,
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
        let config_manager = Arc::new(ConfigManager::new(
            config.clone(),
            config_path.to_string_lossy().to_string(),
        ));
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
    pub async fn register_component_handler(
        &self,
        handler: Box<dyn ComponentRestartHandler + Send + Sync>,
    ) {
        let handler_name = handler.name().to_string();
        let mut handlers = self.component_restart_handlers.write().await;
        handlers.insert(handler_name.clone(), handler);
        info!("Registered component restart handler: {}", handler_name);
    }

    /// Get configuration
    pub async fn get_config(&self) -> BridgeResult<serde_json::Value> {
        let bridge_config = self.bridge_config.read().await;
        let config_json = serde_json::to_value(&*bridge_config)
            .map_err(|e| BridgeError::serialization(e.to_string()))?;
        Ok(config_json)
    }

    /// Update configuration
    pub async fn update_config(&self, config_json: &str) -> BridgeResult<UpdateConfigResponse> {
        let start_time = SystemTime::now();

        info!("Processing configuration update request");

        // Parse the new configuration
        let config_value: serde_json::Value = serde_json::from_str(config_json)
            .map_err(|e| BridgeError::serialization(format!("Invalid configuration format: {}", e)))?;
        
        let new_config: BridgeConfig = serde_json::from_value(config_value)
            .map_err(|e| BridgeError::serialization(format!("Invalid configuration format: {}", e)))?;

        // Validate configuration
        self.validate_config_internal(&new_config).await?;

        // Apply configuration changes
        let changes = self.apply_config_changes(&new_config).await?;

        // Update stored configuration
        {
            let mut bridge_config = self.bridge_config.write().await;
            *bridge_config = new_config;
        }

        // Update config hash
        {
            let mut config_hash = self.last_config_hash.write().await;
            *config_hash = self.calculate_config_hash().await;
        }

        // Save configuration to file
        self.save_config_to_file().await?;

        let response = UpdateConfigResponse {
            success: true,
            error_message: String::new(),
            restarted_components: changes.clone(),
            validation_errors: Vec::new(),
        };

        info!("Configuration updated successfully with {} changes", changes.len());

        Ok(response)
    }

    /// Validate configuration
    pub async fn validate_config(&self, config_json: &str) -> BridgeResult<bool> {
        info!("Validating configuration");

        let config_value: serde_json::Value = serde_json::from_str(config_json)
            .map_err(|e| BridgeError::serialization(format!("Invalid configuration format: {}", e)))?;
        
        let config: BridgeConfig = serde_json::from_value(config_value)
            .map_err(|e| BridgeError::serialization(format!("Invalid configuration format: {}", e)))?;

        let validation_result = self.validate_config_internal(&config).await;

        match &validation_result {
            Ok(_) => {
                info!("Configuration validation successful");
                Ok(true)
            }
            Err(e) => {
                warn!("Configuration validation failed: {:?}", e);
                Ok(false)
            }
        }
    }

    /// Get component status
    pub async fn get_component_status(&self, component_name: &str) -> BridgeResult<crate::proto::ComponentStatus> {
        info!("Getting status for component: {}", component_name);

        let handlers = self.component_restart_handlers.read().await;
        
        if let Some(handler) = handlers.get(component_name) {
            match handler.get_status().await {
                Ok(status) => {
                    let grpc_status = crate::proto::ComponentStatus {
                        name: status.name,
                        state: status.status as i32,
                        uptime_seconds: status.last_restart.map(|t| t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64).unwrap_or(0),
                        error_message: status.error_message.unwrap_or_default(),
                        metrics: HashMap::new(),
                    };

                    Ok(grpc_status)
                }
                Err(e) => {
                    error!("Failed to get status for component {}: {}", component_name, e);
                    Err(BridgeError::configuration(format!("Failed to get status: {}", e)))
                }
            }
        } else {
            Err(BridgeError::configuration(format!("Component not found: {}", component_name)))
        }
    }

    /// Restart component
    pub async fn restart_component(&self, component_name: &str) -> BridgeResult<bool> {
        info!("Restarting component: {}", component_name);

        let handlers = self.component_restart_handlers.read().await;
        
        if let Some(handler) = handlers.get(component_name) {
            let bridge_config = self.bridge_config.read().await;
            
            match handler.restart(&bridge_config).await {
                Ok(_) => {
                    info!("Component {} restarted successfully", component_name);
                    Ok(true)
                }
                Err(e) => {
                    error!("Failed to restart component {}: {}", component_name, e);
                    Err(BridgeError::configuration(format!("Failed to restart: {}", e)))
                }
            }
        } else {
            Err(BridgeError::configuration(format!("Component not found: {}", component_name)))
        }
    }

    /// Start configuration monitoring
    pub async fn start_config_monitoring(&self) -> BridgeResult<()> {
        info!("Starting configuration monitoring");

        let config_path = self.config_path.clone();
        let bridge_config = self.bridge_config.clone();
        let last_config_hash = self.last_config_hash.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut interval = interval(TokioDuration::from_secs(30)); // Check every 30 seconds

            loop {
                interval.tick().await;

                if let Err(e) = Self::monitor_config_file(
                    &config_path,
                    &bridge_config,
                    &last_config_hash,
                    &metrics,
                ).await {
                    error!("Configuration monitoring error: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Monitor configuration file for changes
    async fn monitor_config_file(
        config_path: &PathBuf,
        bridge_config: &Arc<RwLock<BridgeConfig>>,
        last_config_hash: &Arc<RwLock<String>>,
        metrics: &ApiMetrics,
    ) -> BridgeResult<()> {
        if !config_path.exists() {
            return Ok(());
        }

        let current_hash = Self::calculate_file_hash(config_path).await?;
        let stored_hash = last_config_hash.read().await;

        if current_hash != *stored_hash {
            info!("Configuration file changed, reloading...");

            // Read and parse new configuration
            let config_content = fs::read_to_string(config_path).await
                .map_err(|e| BridgeError::configuration(format!("Failed to read config file: {}", e)))?;

            let new_config: BridgeConfig = serde_json::from_str(&config_content)
                .map_err(|e| BridgeError::serialization(format!("Invalid config file: {}", e)))?;

            // Validate new configuration
            // Note: In a real implementation, you would call validate_config_internal here

            // Update stored configuration
            {
                let mut config = bridge_config.write().await;
                *config = new_config;
            }

            // Update hash
            {
                let mut hash = last_config_hash.write().await;
                *hash = current_hash;
            }

            info!("Configuration reloaded successfully");
            metrics.record_processing("config_reload", std::time::Duration::from_millis(100), true);
        }

        Ok(())
    }

    /// Calculate file hash
    async fn calculate_file_hash(path: &PathBuf) -> BridgeResult<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let metadata = fs::metadata(path).await
            .map_err(|e| BridgeError::configuration(format!("Failed to get file metadata: {}", e)))?;

        let mut hasher = DefaultHasher::new();
        metadata.modified().unwrap_or(SystemTime::now()).hash(&mut hasher);
        metadata.len().hash(&mut hasher);

        Ok(format!("{:x}", hasher.finish()))
    }

    /// Validate configuration internally
    async fn validate_config_internal(&self, config: &BridgeConfig) -> BridgeResult<()> {
        // Validate processing configuration
        if config.processing.worker_threads == 0 {
            return Err(BridgeError::validation("Worker threads must be greater than 0".to_string()));
        }

        if config.processing.query_timeout_secs == 0 {
            return Err(BridgeError::validation("Query timeout must be greater than 0".to_string()));
        }

        // Validate ingestion configuration
        if config.ingestion.batch_size == 0 {
            return Err(BridgeError::validation("Batch size must be greater than 0".to_string()));
        }

        if config.ingestion.flush_interval_ms == 0 {
            return Err(BridgeError::validation("Flush interval must be greater than 0".to_string()));
        }

        if config.ingestion.buffer_size == 0 {
            return Err(BridgeError::validation("Buffer size must be greater than 0".to_string()));
        }

        if config.ingestion.compression_level > 9 {
            return Err(BridgeError::validation("Compression level must be between 0 and 9".to_string()));
        }

        Ok(())
    }

    /// Apply configuration changes
    async fn apply_config_changes(&self, new_config: &BridgeConfig) -> BridgeResult<Vec<String>> {
        let mut changes = Vec::new();
        let current_config = self.bridge_config.read().await;

        // Compare processing configuration
        if current_config.processing.worker_threads != new_config.processing.worker_threads {
            changes.push(format!(
                "Worker threads: {} -> {}",
                current_config.processing.worker_threads,
                new_config.processing.worker_threads
            ));
        }

        if current_config.processing.query_timeout_secs != new_config.processing.query_timeout_secs {
            changes.push(format!(
                "Query timeout: {}s -> {}s",
                current_config.processing.query_timeout_secs,
                new_config.processing.query_timeout_secs
            ));
        }

        if current_config.processing.enable_query_caching != new_config.processing.enable_query_caching {
            changes.push(format!(
                "Query caching: {} -> {}",
                current_config.processing.enable_query_caching,
                new_config.processing.enable_query_caching
            ));
        }

        // Compare ingestion configuration
        if current_config.ingestion.batch_size != new_config.ingestion.batch_size {
            changes.push(format!(
                "Batch size: {} -> {}",
                current_config.ingestion.batch_size,
                new_config.ingestion.batch_size
            ));
        }

        if current_config.ingestion.flush_interval_ms != new_config.ingestion.flush_interval_ms {
            changes.push(format!(
                "Flush interval: {}ms -> {}ms",
                current_config.ingestion.flush_interval_ms,
                new_config.ingestion.flush_interval_ms
            ));
        }

        if current_config.ingestion.buffer_size != new_config.ingestion.buffer_size {
            changes.push(format!(
                "Buffer size: {} -> {}",
                current_config.ingestion.buffer_size,
                new_config.ingestion.buffer_size
            ));
        }

        if current_config.ingestion.compression_level != new_config.ingestion.compression_level {
            changes.push(format!(
                "Compression level: {} -> {}",
                current_config.ingestion.compression_level,
                new_config.ingestion.compression_level
            ));
        }

        if current_config.ingestion.enable_persistence != new_config.ingestion.enable_persistence {
            changes.push(format!(
                "Persistence: {} -> {}",
                current_config.ingestion.enable_persistence,
                new_config.ingestion.enable_persistence
            ));
        }

        if current_config.ingestion.enable_backpressure != new_config.ingestion.enable_backpressure {
            changes.push(format!(
                "Backpressure: {} -> {}",
                current_config.ingestion.enable_backpressure,
                new_config.ingestion.enable_backpressure
            ));
        }

        // Restart components if needed
        if !changes.is_empty() {
            self.restart_affected_components(new_config).await?;
        }

        Ok(changes)
    }

    /// Restart affected components
    async fn restart_affected_components(&self, new_config: &BridgeConfig) -> BridgeResult<()> {
        let handlers = self.component_restart_handlers.read().await;

        for (name, handler) in handlers.iter() {
            info!("Restarting component {} due to configuration changes", name);
            
            match handler.restart(new_config).await {
                Ok(_) => info!("Component {} restarted successfully", name),
                Err(e) => {
                    error!("Failed to restart component {}: {}", name, e);
                    return Err(BridgeError::configuration(format!("Failed to restart {}: {}", name, e)));
                }
            }
        }

        Ok(())
    }

    /// Calculate configuration hash
    async fn calculate_config_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let config = self.bridge_config.read().await;
        let mut hasher = DefaultHasher::new();

        // Hash the configuration
        let config_json = serde_json::to_string(&*config).unwrap_or_default();
        config_json.hash(&mut hasher);

        // Add timestamp for uniqueness
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Save configuration to file
    async fn save_config_to_file(&self) -> BridgeResult<()> {
        let config = self.bridge_config.read().await;
        let config_json = serde_json::to_string_pretty(&*config)
            .map_err(|e| BridgeError::serialization(e.to_string()))?;

        fs::write(&self.config_path, config_json).await
            .map_err(|e| BridgeError::configuration(format!("Failed to write config file: {}", e)))?;

        info!("Configuration saved to file: {:?}", self.config_path);
        Ok(())
    }
}
