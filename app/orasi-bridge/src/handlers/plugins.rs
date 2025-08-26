//! Plugin handlers

use axum::{
    extract::{Query, State},
    response::Json,
};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;
use std::path::{Path, PathBuf};
use std::fs;
use serde_json;
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::{
    error::{ApiError, ApiResult},
    rest::AppState,
    types::*,
};

/// Plugin manager for discovering and managing plugins
#[derive(Debug)]
pub struct PluginManager {
    plugins_dir: PathBuf,
    plugins: Arc<RwLock<HashMap<String, PluginInfo>>>,
    capabilities: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            plugins: Arc::new(RwLock::new(HashMap::new())),
            capabilities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Discover plugins from the plugins directory
    pub async fn discover_plugins(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut plugins = self.plugins.write().await;
        let mut capabilities = self.capabilities.write().await;
        
        // Clear existing plugins
        plugins.clear();
        capabilities.clear();

        // Create plugins directory if it doesn't exist
        if !self.plugins_dir.exists() {
            fs::create_dir_all(&self.plugins_dir)?;
        }

        // Scan for plugin directories
        for entry in fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(plugin_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(plugin_info) = self.load_plugin_info(&path, plugin_name).await {
                        plugins.insert(plugin_name.to_string(), plugin_info.clone());
                        
                        // Add capabilities
                        for capability in &plugin_info.capabilities {
                            capabilities
                                .entry(capability.clone())
                                .or_insert_with(Vec::new)
                                .push(plugin_name.to_string());
                        }
                    }
                }
            }
        }

        // Add built-in plugins
        self.add_builtin_plugins(&mut plugins, &mut capabilities).await;

        Ok(())
    }

    /// Load plugin information from a plugin directory
    async fn load_plugin_info(&self, plugin_path: &Path, plugin_name: &str) -> Result<PluginInfo, Box<dyn std::error::Error + Send + Sync>> {
        let manifest_path = plugin_path.join("plugin.json");
        
        if manifest_path.exists() {
            // Load from manifest file
            let manifest_content = fs::read_to_string(manifest_path)?;
            let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;
            
            Ok(PluginInfo {
                name: manifest["name"].as_str().unwrap_or(plugin_name).to_string(),
                version: manifest["version"].as_str().unwrap_or("1.0.0").to_string(),
                description: manifest["description"].as_str().unwrap_or("").to_string(),
                capabilities: manifest["capabilities"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_else(|| vec!["basic".to_string()]),
                status: self.determine_plugin_status(plugin_path).await,
            })
        } else {
            // Default plugin info
            Ok(PluginInfo {
                name: plugin_name.to_string(),
                version: "1.0.0".to_string(),
                description: format!("Plugin: {}", plugin_name),
                capabilities: vec!["basic".to_string()],
                status: self.determine_plugin_status(plugin_path).await,
            })
        }
    }

    /// Determine plugin status based on available files and health
    async fn determine_plugin_status(&self, plugin_path: &Path) -> PluginStatus {
        // Check if plugin has required files
        let has_executable = plugin_path.join("plugin").exists() || 
                           plugin_path.join("plugin.exe").exists() ||
                           plugin_path.join("plugin.py").exists() ||
                           plugin_path.join("plugin.js").exists() ||
                           plugin_path.join("processor.js").exists();
        
        let has_config = plugin_path.join("config.json").exists() ||
                        plugin_path.join("plugin.json").exists();

        if has_executable && has_config {
            // Try to perform a basic health check
            if self.perform_health_check(plugin_path).await {
                PluginStatus::Active
            } else {
                PluginStatus::Error
            }
        } else if has_config {
            PluginStatus::Loading
        } else {
            PluginStatus::Inactive
        }
    }

    /// Perform a basic health check on the plugin
    async fn perform_health_check(&self, plugin_path: &Path) -> bool {
        // Simple health check - check if plugin files are accessible
        let health_file = plugin_path.join(".health");
        
        if health_file.exists() {
            // Check if health file is recent (less than 5 minutes old)
            if let Ok(metadata) = fs::metadata(&health_file) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = std::time::SystemTime::now().duration_since(modified) {
                        return duration.as_secs() < 300; // 5 minutes
                    }
                }
            }
        }
        
        // If no health file, assume healthy if executable exists
        plugin_path.join("plugin").exists() || 
        plugin_path.join("plugin.exe").exists() ||
        plugin_path.join("plugin.py").exists() ||
        plugin_path.join("plugin.js").exists() ||
        plugin_path.join("processor.js").exists()
    }

    /// Add built-in plugins
    async fn add_builtin_plugins(
        &self,
        plugins: &mut HashMap<String, PluginInfo>,
        capabilities: &mut HashMap<String, Vec<String>>,
    ) {
        // Add built-in analytics plugin
        let analytics_plugin = PluginInfo {
            name: "analytics".to_string(),
            version: "1.0.0".to_string(),
            description: "Built-in analytics plugin for data analysis".to_string(),
            capabilities: vec![
                "analytics".to_string(),
                "query".to_string(),
                "aggregation".to_string(),
                "visualization".to_string(),
            ],
            status: PluginStatus::Active,
        };
        
        plugins.insert("analytics".to_string(), analytics_plugin.clone());
        
        // Add built-in query plugin
        let query_plugin = PluginInfo {
            name: "query-engine".to_string(),
            version: "1.0.0".to_string(),
            description: "Built-in query engine for data queries".to_string(),
            capabilities: vec![
                "query".to_string(),
                "sql".to_string(),
                "filtering".to_string(),
                "sorting".to_string(),
            ],
            status: PluginStatus::Active,
        };
        
        plugins.insert("query-engine".to_string(), query_plugin.clone());

        // Add built-in streaming plugin
        let streaming_plugin = PluginInfo {
            name: "streaming-processor".to_string(),
            version: "1.0.0".to_string(),
            description: "Built-in streaming data processor".to_string(),
            capabilities: vec![
                "streaming".to_string(),
                "real-time".to_string(),
                "processing".to_string(),
                "transformation".to_string(),
            ],
            status: PluginStatus::Active,
        };
        
        plugins.insert("streaming-processor".to_string(), streaming_plugin.clone());

        // Update capabilities
        for capability in &analytics_plugin.capabilities {
            capabilities
                .entry(capability.clone())
                .or_insert_with(Vec::new)
                .push("analytics".to_string());
        }
        
        for capability in &query_plugin.capabilities {
            capabilities
                .entry(capability.clone())
                .or_insert_with(Vec::new)
                .push("query-engine".to_string());
        }
        
        for capability in &streaming_plugin.capabilities {
            capabilities
                .entry(capability.clone())
                .or_insert_with(Vec::new)
                .push("streaming-processor".to_string());
        }
    }

    /// Get all plugins
    pub async fn get_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.values().cloned().collect()
    }

    /// Get capabilities
    pub async fn get_capabilities(&self) -> HashMap<String, Vec<String>> {
        let capabilities = self.capabilities.read().await;
        capabilities.clone()
    }

    /// Get plugin by name
    pub async fn get_plugin(&self, name: &str) -> Option<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.get(name).cloned()
    }
}

// Global plugin manager instance
lazy_static::lazy_static! {
    static ref PLUGIN_MANAGER: Arc<PluginManager> = {
        let plugins_dir = std::env::var("ORASI_PLUGINS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./plugins"));
        Arc::new(PluginManager::new(plugins_dir))
    };
}

/// Get plugin capabilities
async fn get_plugin_capabilities() -> (Vec<PluginInfo>, HashMap<String, Vec<String>>) {
    // Discover plugins if not already done
    if let Err(e) = PLUGIN_MANAGER.discover_plugins().await {
        eprintln!("Failed to discover plugins: {}", e);
    }

    let plugins = PLUGIN_MANAGER.get_plugins().await;
    let capabilities = PLUGIN_MANAGER.get_capabilities().await;

    (plugins, capabilities)
}

/// Execute plugin query
async fn execute_plugin_query(
    request: &PluginQueryRequest,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Get plugin info
    let plugin_info = PLUGIN_MANAGER.get_plugin(&request.plugin_name).await;
    
    if let Some(plugin) = plugin_info {
        match plugin.name.as_str() {
            "analytics" => execute_analytics_query(request).await,
            "query-engine" => execute_query_engine_query(request).await,
            "streaming-processor" => execute_streaming_query(request).await,
            _ => execute_generic_plugin_query(request, &plugin).await,
        }
    } else {
        Err(format!("Plugin '{}' not found", request.plugin_name).into())
    }
}

/// Execute analytics plugin query
async fn execute_analytics_query(
    request: &PluginQueryRequest,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Simulate analytics processing
    let query_data = &request.query_data;
    
    Ok(serde_json::json!({
        "plugin": request.plugin_name,
        "type": "analytics",
        "data": query_data,
        "timestamp": chrono::Utc::now(),
        "status": "success",
        "result_count": 1,
        "analytics": {
            "summary": "Data analysis completed",
            "metrics": {
                "total_records": 1000,
                "processed_records": 1000,
                "analysis_time_ms": 150
            }
        }
    }))
}

/// Execute query engine plugin query
async fn execute_query_engine_query(
    request: &PluginQueryRequest,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Simulate query engine processing
    let query_data = &request.query_data;
    
    Ok(serde_json::json!({
        "plugin": request.plugin_name,
        "type": "query",
        "data": query_data,
        "timestamp": chrono::Utc::now(),
        "status": "success",
        "result_count": 1,
        "query": {
            "execution_time_ms": 45,
            "rows_returned": 250,
            "query_plan": "SELECT * FROM telemetry_data WHERE timestamp > '2024-01-01'"
        }
    }))
}

/// Execute streaming processor plugin query
async fn execute_streaming_query(
    request: &PluginQueryRequest,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Simulate streaming processing
    let query_data = &request.query_data;
    
    Ok(serde_json::json!({
        "plugin": request.plugin_name,
        "type": "streaming",
        "data": query_data,
        "timestamp": chrono::Utc::now(),
        "status": "success",
        "result_count": 1,
        "streaming": {
            "stream_id": uuid::Uuid::new_v4(),
            "messages_processed": 1500,
            "throughput_mps": 25.5,
            "latency_ms": 12
        }
    }))
}

/// Execute generic plugin query
async fn execute_generic_plugin_query(
    request: &PluginQueryRequest,
    plugin: &PluginInfo,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Generic plugin execution
    Ok(serde_json::json!({
        "plugin": request.plugin_name,
        "version": plugin.version,
        "data": request.query_data,
        "timestamp": chrono::Utc::now(),
        "status": "success",
        "result_count": 1,
        "capabilities": plugin.capabilities,
    }))
}

/// Plugin capabilities handler
pub async fn plugin_capabilities_handler(
    Query(_params): Query<PluginCapabilitiesRequest>,
) -> ApiResult<Json<ApiResponse<PluginCapabilitiesResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Get actual plugin capabilities
    let (plugins, capabilities) = get_plugin_capabilities().await;

    let response = PluginCapabilitiesResponse {
        plugins,
        capabilities,
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Plugin query handler
pub async fn plugin_query_handler(
    State(_state): State<AppState>,
    Json(request): Json<PluginQueryRequest>,
) -> ApiResult<Json<ApiResponse<PluginQueryResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Execute plugin query
    let results = match execute_plugin_query(&request).await {
        Ok(query_results) => query_results,
        Err(e) => serde_json::json!({
            "error": e.to_string(),
            "plugin": request.plugin_name,
            "timestamp": chrono::Utc::now(),
        }),
    };

    let metadata = PluginQueryMetadata {
        query_id: Uuid::new_v4(),
        execution_time_ms: start_time.elapsed().as_millis() as u64,
        cache_hit: false,
        plugin_version: "1.0.0".to_string(),
    };

    let response = PluginQueryResponse {
        plugin_name: request.plugin_name,
        results,
        metadata,
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}
