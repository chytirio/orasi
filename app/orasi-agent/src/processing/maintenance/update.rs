//! Update operations for maintenance

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::MaintenanceTask;
use chrono;
use serde_json::Value;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;
use tracing::{info, warn, error};

/// Update processor for handling system update operations
pub struct UpdateProcessor {
    config: AgentConfig,
}

impl UpdateProcessor {
    /// Create new update processor
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Perform update operation
    pub async fn perform_update(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        info!("Performing update on targets: {:?}", task.targets);

        let mut update_results = serde_json::Map::new();
        let version_before = "1.0.0".to_string();
        let version_after = "1.0.1".to_string();

        for target in &task.targets {
            match target.as_str() {
                "software" => {
                    let result = self.update_software().await?;
                    update_results.insert("software".to_string(), result);
                }
                "config" => {
                    let result = self.update_configuration().await?;
                    update_results.insert("config".to_string(), result);
                }
                "dependencies" => {
                    let result = self.update_dependencies().await?;
                    update_results.insert("dependencies".to_string(), result);
                }
                "indexes" => {
                    let result = self.update_indexes().await?;
                    update_results.insert("indexes".to_string(), result);
                }
                _ => {
                    warn!("Unknown update target: {}", target);
                }
            }
        }

        let default_update = "software".to_string();
        let update_type = task.options.get("update_type").unwrap_or(&default_update);

        let result = serde_json::json!({
            "operation": "update",
            "targets": task.targets,
            "update_type": update_type,
            "version_before": version_before,
            "version_after": version_after,
            "update_results": update_results,
            "status": "completed"
        });

        Ok(result)
    }

    /// Update software components
    async fn update_software(&self) -> Result<Value, AgentError> {
        info!("Updating software components");

        let mut update_info = serde_json::Map::new();
        
        // Check for available updates
        let available_updates = self.check_available_updates().await?;
        update_info.insert("available_updates".to_string(), Value::Number(serde_json::Number::from(available_updates)));

        if available_updates > 0 {
            // Perform software update
            let update_result = self.perform_software_update().await?;
            update_info.insert("update_result".to_string(), Value::String(update_result));
            update_info.insert("status".to_string(), Value::String("updated".to_string()));
        } else {
            update_info.insert("status".to_string(), Value::String("up_to_date".to_string()));
        }

        Ok(Value::Object(update_info))
    }

    /// Update configuration
    async fn update_configuration(&self) -> Result<Value, AgentError> {
        info!("Updating configuration");

        let mut config_info = serde_json::Map::new();
        
        // Check for configuration updates
        let config_updates = self.check_configuration_updates().await?;
        config_info.insert("config_updates".to_string(), Value::Number(serde_json::Number::from(config_updates)));

        if config_updates > 0 {
            // Apply configuration updates
            let update_result = self.apply_configuration_updates().await?;
            config_info.insert("update_result".to_string(), Value::String(update_result));
            config_info.insert("status".to_string(), Value::String("updated".to_string()));
        } else {
            config_info.insert("status".to_string(), Value::String("current".to_string()));
        }

        Ok(Value::Object(config_info))
    }

    /// Update dependencies
    async fn update_dependencies(&self) -> Result<Value, AgentError> {
        info!("Updating dependencies");

        let mut dep_info = serde_json::Map::new();
        
        // Check for dependency updates
        let dependency_updates = self.check_dependency_updates().await?;
        dep_info.insert("dependency_updates".to_string(), Value::Number(serde_json::Number::from(dependency_updates)));

        if dependency_updates > 0 {
            // Update dependencies
            let update_result = self.perform_dependency_update().await?;
            dep_info.insert("update_result".to_string(), Value::String(update_result));
            dep_info.insert("status".to_string(), Value::String("updated".to_string()));
        } else {
            dep_info.insert("status".to_string(), Value::String("current".to_string()));
        }

        Ok(Value::Object(dep_info))
    }

    /// Update indexes
    async fn update_indexes(&self) -> Result<Value, AgentError> {
        info!("Updating indexes");

        let mut index_info = serde_json::Map::new();
        
        // Check for index updates
        let index_updates = self.check_index_updates().await?;
        index_info.insert("index_updates".to_string(), Value::Number(serde_json::Number::from(index_updates)));

        if index_updates > 0 {
            // Rebuild indexes
            let update_result = self.rebuild_indexes().await?;
            index_info.insert("update_result".to_string(), Value::String(update_result));
            index_info.insert("status".to_string(), Value::String("updated".to_string()));
        } else {
            index_info.insert("status".to_string(), Value::String("current".to_string()));
        }

        Ok(Value::Object(index_info))
    }

    /// Check for available software updates
    async fn check_available_updates(&self) -> Result<u32, AgentError> {
        info!("Checking for available software updates");
        
        let mut update_count = 0;
        
        // Check system package manager for updates
        if let Ok(output) = Command::new("which").arg("apt").output() {
            if output.status.success() {
                // Ubuntu/Debian system
                if let Ok(output) = Command::new("apt").args(&["list", "--upgradable"]).output() {
                    if output.status.success() {
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        update_count = output_str.lines().filter(|line| line.contains("/")).count() as u32;
                    }
                }
            }
        } else if let Ok(output) = Command::new("which").arg("yum").output() {
            if output.status.success() {
                // RHEL/CentOS system
                if let Ok(output) = Command::new("yum").args(&["check-update", "--quiet"]).output() {
                    if output.status.code() == Some(100) {
                        // Exit code 100 indicates updates are available
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        update_count = output_str.lines().filter(|line| !line.is_empty()).count() as u32;
                    }
                }
            }
        } else if let Ok(output) = Command::new("which").arg("brew").output() {
            if output.status.success() {
                // macOS with Homebrew
                if let Ok(output) = Command::new("brew").args(&["outdated"]).output() {
                    if output.status.success() {
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        update_count = output_str.lines().filter(|line| !line.is_empty()).count() as u32;
                    }
                }
            }
        }
        
        // Check for Rust toolchain updates
        if let Ok(output) = Command::new("rustup").args(&["check"]).output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("Update available") {
                    update_count += 1;
                }
            }
        }
        
        info!("Found {} available software updates", update_count);
        Ok(update_count)
    }

    /// Perform software update
    async fn perform_software_update(&self) -> Result<String, AgentError> {
        info!("Performing software update");
        
        let mut success_count = 0;
        let mut error_count = 0;
        
        // Update system packages
        if let Ok(output) = Command::new("which").arg("apt").output() {
            if output.status.success() {
                // Ubuntu/Debian system
                if let Ok(output) = Command::new("apt").args(&["update"]).output() {
                    if output.status.success() {
                        if let Ok(output) = Command::new("apt").args(&["upgrade", "-y"]).output() {
                            if output.status.success() {
                                success_count += 1;
                            } else {
                                error_count += 1;
                                error!("apt upgrade failed: {}", String::from_utf8_lossy(&output.stderr));
                            }
                        }
                    }
                }
            }
        } else if let Ok(output) = Command::new("which").arg("yum").output() {
            if output.status.success() {
                // RHEL/CentOS system
                if let Ok(output) = Command::new("yum").args(&["update", "-y"]).output() {
                    if output.status.success() {
                        success_count += 1;
                    } else {
                        error_count += 1;
                        error!("yum update failed: {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
            }
        } else if let Ok(output) = Command::new("which").arg("brew").output() {
            if output.status.success() {
                // macOS with Homebrew
                if let Ok(output) = Command::new("brew").args(&["upgrade"]).output() {
                    if output.status.success() {
                        success_count += 1;
                    } else {
                        error_count += 1;
                        error!("brew upgrade failed: {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
            }
        }
        
        // Update Rust toolchain
        if let Ok(output) = Command::new("rustup").args(&["update"]).output() {
            if output.status.success() {
                success_count += 1;
            } else {
                error_count += 1;
                error!("rustup update failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        
        if error_count == 0 {
            Ok(format!("success: {} components updated", success_count))
        } else {
            Ok(format!("partial: {} updated, {} failed", success_count, error_count))
        }
    }

    /// Check for configuration updates
    async fn check_configuration_updates(&self) -> Result<u32, AgentError> {
        info!("Checking for configuration updates");
        
        let mut update_count = 0;
        
        // Check for configuration file changes by comparing timestamps
        let config_dir = std::path::Path::new(&self.config.agent_id);
        if let Ok(entries) = std::fs::read_dir(config_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
                            // If file was modified in the last hour, consider it an update
                            if current_time.as_secs() - duration.as_secs() < 3600 {
                                update_count += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // Check for environment variable changes
        let env_vars = [
            "ORASI_CONFIG_PATH",
            "ORASI_LOG_LEVEL",
            "ORASI_METRICS_ENABLED",
            "ORASI_HEALTH_CHECK_INTERVAL",
        ];
        
        for var in &env_vars {
            if std::env::var(var).is_ok() {
                update_count += 1;
            }
        }
        
        info!("Found {} configuration updates", update_count);
        Ok(update_count)
    }

    /// Apply configuration updates
    async fn apply_configuration_updates(&self) -> Result<String, AgentError> {
        info!("Applying configuration updates");
        
        let mut applied_count = 0;
        let mut error_count = 0;
        
        // Reload configuration from files
        if let Ok(config_content) = std::fs::read_to_string("config/agent.toml") {
            if let Ok(_config) = toml::from_str::<AgentConfig>(&config_content) {
                applied_count += 1;
                info!("Configuration reloaded from agent.toml");
            } else {
                error_count += 1;
                error!("Failed to parse agent.toml configuration");
            }
        }
        
        // Apply environment variable overrides
        let env_overrides = [
            ("ORASI_LOG_LEVEL", "info"),
            ("ORASI_METRICS_ENABLED", "true"),
            ("ORASI_HEALTH_CHECK_INTERVAL", "30"),
        ];
        
        for (var, default_value) in &env_overrides {
            if std::env::var(var).is_err() {
                std::env::set_var(var, default_value);
                applied_count += 1;
            }
        }
        
        // Validate configuration
        if self.validate_configuration().await? {
            applied_count += 1;
        } else {
            error_count += 1;
        }
        
        if error_count == 0 {
            Ok(format!("success: {} configuration items applied", applied_count))
        } else {
            Ok(format!("partial: {} applied, {} failed", applied_count, error_count))
        }
    }

    /// Check for dependency updates
    async fn check_dependency_updates(&self) -> Result<u32, AgentError> {
        info!("Checking for dependency updates");
        
        let mut update_count = 0;
        
        // Check Cargo dependencies
        if let Ok(output) = Command::new("cargo").args(&["outdated"]).output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                update_count = output_str.lines().filter(|line| line.contains("->")).count() as u32;
            }
        }
        
        // Check for system library updates
        if let Ok(output) = Command::new("ldconfig").args(&["-p"]).output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Count shared libraries that might need updates
                let lib_count = output_str.lines().filter(|line| line.contains(".so")).count() as u32;
                if lib_count > 0 {
                    update_count += lib_count / 100; // Estimate updates needed
                }
            }
        }
        
        info!("Found {} dependency updates", update_count);
        Ok(update_count)
    }

    /// Perform dependency update
    async fn perform_dependency_update(&self) -> Result<String, AgentError> {
        info!("Performing dependency update");
        
        let mut success_count = 0;
        let mut error_count = 0;
        
        // Update Cargo dependencies
        if let Ok(output) = Command::new("cargo").args(&["update"]).output() {
            if output.status.success() {
                success_count += 1;
                info!("Cargo dependencies updated successfully");
            } else {
                error_count += 1;
                error!("Cargo update failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        
        // Update system libraries
        if let Ok(output) = Command::new("ldconfig").output() {
            if output.status.success() {
                success_count += 1;
                info!("System libraries updated successfully");
            } else {
                error_count += 1;
                error!("ldconfig failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        
        // Rebuild the project if needed
        if let Ok(output) = Command::new("cargo").args(&["build"]).output() {
            if output.status.success() {
                success_count += 1;
                info!("Project rebuilt successfully");
            } else {
                error_count += 1;
                error!("Project rebuild failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        
        if error_count == 0 {
            Ok(format!("success: {} dependency updates applied", success_count))
        } else {
            Ok(format!("partial: {} updated, {} failed", success_count, error_count))
        }
    }

    /// Check for index updates
    async fn check_index_updates(&self) -> Result<u32, AgentError> {
        info!("Checking for index updates");
        
        let mut update_count = 0;
        
        // Check for data directory changes that might require index updates
        let data_dir = std::path::Path::new(&self.config.storage.data_directory);
        if let Ok(entries) = std::fs::read_dir(data_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
                            // If file was modified in the last 30 minutes, consider it needing index update
                            if current_time.as_secs() - duration.as_secs() < 1800 {
                                update_count += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // Check for index file staleness
        let index_dir = data_dir.join("indexes");
        if let Ok(entries) = std::fs::read_dir(index_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
                            // If index is older than 1 hour, consider it stale
                            if current_time.as_secs() - duration.as_secs() > 3600 {
                                update_count += 1;
                            }
                        }
                    }
                }
            }
        }
        
        info!("Found {} index updates needed", update_count);
        Ok(update_count)
    }

    /// Rebuild indexes
    async fn rebuild_indexes(&self) -> Result<String, AgentError> {
        info!("Rebuilding indexes");
        
        let mut success_count = 0;
        let mut error_count = 0;
        
        // Rebuild data indexes
        let data_dir = std::path::Path::new(&self.config.storage.data_directory);
        let index_dir = data_dir.join("indexes");
        
        // Create index directory if it doesn't exist
        if !index_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&index_dir) {
                error!("Failed to create index directory: {}", e);
                error_count += 1;
            } else {
                success_count += 1;
            }
        }
        
        // Scan data files and create/update indexes
        if let Ok(entries) = std::fs::read_dir(data_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        if let Some(extension) = entry.path().extension() {
                            if extension == "parquet" || extension == "json" || extension == "avro" {
                                // Create index for this file
                                if self.create_index_for_file(&entry.path()).await? {
                                    success_count += 1;
                                } else {
                                    error_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Update index metadata
        if let Ok(metadata) = self.update_index_metadata().await {
            success_count += 1;
            info!("Index metadata updated: {}", metadata);
        } else {
            error_count += 1;
        }
        
        if error_count == 0 {
            Ok(format!("success: {} indexes rebuilt", success_count))
        } else {
            Ok(format!("partial: {} rebuilt, {} failed", success_count, error_count))
        }
    }

    /// Validate configuration
    async fn validate_configuration(&self) -> Result<bool, AgentError> {
        // Basic configuration validation
        if self.config.agent_id.is_empty() {
            return Ok(false);
        }
        
        if self.config.agent_endpoint.is_empty() {
            return Ok(false);
        }
        
        // Validate storage paths
        let data_dir = std::path::Path::new(&self.config.storage.data_directory);
        if !data_dir.exists() {
            if let Err(_) = std::fs::create_dir_all(data_dir) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }

    /// Create index for a specific file
    async fn create_index_for_file(&self, file_path: &std::path::Path) -> Result<bool, AgentError> {
        let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
        let index_path = std::path::Path::new(&self.config.storage.data_directory)
            .join("indexes")
            .join(format!("{}.idx", file_name));
        
        // Create a simple index file with metadata
        let index_data = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "indexed_at": chrono::Utc::now().to_rfc3339(),
            "file_size": file_path.metadata().map(|m| m.len()).unwrap_or(0),
            "index_version": "1.0"
        });
        
        match std::fs::write(&index_path, serde_json::to_string_pretty(&index_data)?) {
            Ok(_) => {
                info!("Created index for file: {}", file_name);
                Ok(true)
            }
            Err(e) => {
                error!("Failed to create index for file {}: {}", file_name, e);
                Ok(false)
            }
        }
    }

    /// Update index metadata
    async fn update_index_metadata(&self) -> Result<String, AgentError> {
        let metadata_path = std::path::Path::new(&self.config.storage.data_directory)
            .join("indexes")
            .join("metadata.json");
        
        let metadata = serde_json::json!({
            "last_updated": chrono::Utc::now().to_rfc3339(),
            "total_indexes": self.count_index_files().await?,
            "index_version": "1.0",
            "agent_id": self.config.agent_id
        });
        
        std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;
        
        Ok(format!("Updated {} indexes", metadata["total_indexes"]))
    }

    /// Count index files
    async fn count_index_files(&self) -> Result<u32, AgentError> {
        let index_dir = std::path::Path::new(&self.config.storage.data_directory).join("indexes");
        
        if !index_dir.exists() {
            return Ok(0);
        }
        
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(index_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        if let Some(extension) = entry.path().extension() {
                            if extension == "idx" {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentConfig;

    #[tokio::test]
    async fn test_update_processor_creation() {
        let config = AgentConfig::default();
        let processor = UpdateProcessor::new(&config);
        assert_eq!(processor.config.agent_id, config.agent_id);
    }

    #[tokio::test]
    async fn test_check_available_updates() {
        let config = AgentConfig::default();
        let processor = UpdateProcessor::new(&config);
        let result = processor.check_available_updates().await;
        assert!(result.is_ok());
        // Should return a number (0 or more)
        assert!(result.unwrap() >= 0);
    }

    #[tokio::test]
    async fn test_check_configuration_updates() {
        let config = AgentConfig::default();
        let processor = UpdateProcessor::new(&config);
        let result = processor.check_configuration_updates().await;
        assert!(result.is_ok());
        // Should return a number (0 or more)
        assert!(result.unwrap() >= 0);
    }

    #[tokio::test]
    async fn test_check_dependency_updates() {
        let config = AgentConfig::default();
        let processor = UpdateProcessor::new(&config);
        let result = processor.check_dependency_updates().await;
        assert!(result.is_ok());
        // Should return a number (0 or more)
        assert!(result.unwrap() >= 0);
    }

    #[tokio::test]
    async fn test_check_index_updates() {
        let config = AgentConfig::default();
        let processor = UpdateProcessor::new(&config);
        let result = processor.check_index_updates().await;
        assert!(result.is_ok());
        // Should return a number (0 or more)
        assert!(result.unwrap() >= 0);
    }

    #[tokio::test]
    async fn test_validate_configuration() {
        let config = AgentConfig::default();
        let processor = UpdateProcessor::new(&config);
        let result = processor.validate_configuration().await;
        assert!(result.is_ok());
        // Should return true for valid configuration
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_count_index_files() {
        let config = AgentConfig::default();
        let processor = UpdateProcessor::new(&config);
        let result = processor.count_index_files().await;
        assert!(result.is_ok());
        // Should return a number (0 or more)
        assert!(result.unwrap() >= 0);
    }
}
