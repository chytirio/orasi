//! Restore operations for maintenance

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::MaintenanceTask;
use serde_json::Value;
use tracing::{info, warn};

/// Restore processor for handling system restore operations
pub struct RestoreProcessor {
    config: AgentConfig,
}

impl RestoreProcessor {
    /// Create new restore processor
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Perform restore operation
    pub async fn perform_restore(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        info!("Performing restore of targets: {:?}", task.targets);

        let mut restored_files = 0;
        let mut restore_size_bytes = 0;

        let default_source = "latest".to_string();
        let backup_source = task.options.get("backup_source").unwrap_or(&default_source);

        // Get backup ID to restore from
        let backup_id = self.get_backup_id(backup_source).await?;

        for target in &task.targets {
            match target.as_str() {
                "data" => {
                    let (size, count) = self.restore_data(&backup_id).await?;
                    restore_size_bytes += size;
                    restored_files += count;
                }
                "config" => {
                    let (size, count) = self.restore_config(&backup_id).await?;
                    restore_size_bytes += size;
                    restored_files += count;
                }
                "logs" => {
                    let (size, count) = self.restore_logs(&backup_id).await?;
                    restore_size_bytes += size;
                    restored_files += count;
                }
                "indexes" => {
                    let (size, count) = self.restore_indexes(&backup_id).await?;
                    restore_size_bytes += size;
                    restored_files += count;
                }
                _ => {
                    warn!("Unknown restore target: {}", target);
                }
            }
        }

        let result = serde_json::json!({
            "operation": "restore",
            "targets": task.targets,
            "backup_source": backup_source,
            "backup_id": backup_id,
            "restored_files": restored_files,
            "restore_size_bytes": restore_size_bytes,
            "status": "completed"
        });

        Ok(result)
    }

    /// Get backup ID from source specification
    async fn get_backup_id(&self, backup_source: &str) -> Result<String, AgentError> {
        match backup_source {
            "latest" => self.get_latest_backup_id().await,
            _ => {
                // Assume it's a specific backup ID
                if backup_source.starts_with("backup_") {
                    Ok(backup_source.to_string())
                } else {
                    Err(AgentError::InvalidInput(format!(
                        "Invalid backup source: {}",
                        backup_source
                    )))
                }
            }
        }
    }

    /// Get the latest backup ID
    async fn get_latest_backup_id(&self) -> Result<String, AgentError> {
        let backup_dir = std::path::Path::new(&self.config.storage.local_path).join("backups");
        
        if !backup_dir.exists() {
            return Err(AgentError::InvalidInput("No backups found".to_string()));
        }

        let mut latest_backup = None;
        let mut latest_timestamp = 0u64;

        if let Ok(entries) = std::fs::read_dir(&backup_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(dir_name) = path.file_name() {
                            if let Some(name_str) = dir_name.to_str() {
                                if name_str.starts_with("backup_") {
                                    // Extract timestamp from backup ID
                                    if let Some(timestamp_str) = name_str.strip_prefix("backup_") {
                                        if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                                            if timestamp > latest_timestamp {
                                                latest_timestamp = timestamp;
                                                latest_backup = Some(name_str.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        latest_backup.ok_or_else(|| {
            AgentError::InvalidInput("No valid backups found".to_string())
        })
    }

    /// Restore data files
    async fn restore_data(&self, backup_id: &str) -> Result<(u64, u32), AgentError> {
        let backup_dir = std::path::Path::new(&self.config.storage.local_path)
            .join("backups")
            .join(backup_id)
            .join("data");
        let data_dir = std::path::Path::new(&self.config.storage.local_path).join("data");

        let mut total_size = 0;
        let mut file_count = 0;

        if backup_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| AgentError::IoError(format!("Failed to create data directory: {}", e)))?;

            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            let restore_path = data_dir.join(path.file_name().unwrap());
                            if let Ok(_) = std::fs::copy(&path, &restore_path) {
                                if let Ok(metadata) = path.metadata() {
                                    total_size += metadata.len();
                                    file_count += 1;
                                    info!("Restored data file: {:?}", restore_path);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((total_size, file_count))
    }

    /// Restore configuration files
    async fn restore_config(&self, backup_id: &str) -> Result<(u64, u32), AgentError> {
        let backup_dir = std::path::Path::new(&self.config.storage.local_path)
            .join("backups")
            .join(backup_id)
            .join("config");
        let config_dir = std::path::Path::new(&self.config.storage.local_path).join("config");

        let mut total_size = 0;
        let mut file_count = 0;

        if backup_dir.exists() {
            std::fs::create_dir_all(&config_dir)
                .map_err(|e| AgentError::IoError(format!("Failed to create config directory: {}", e)))?;

            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            let restore_path = config_dir.join(path.file_name().unwrap());
                            if let Ok(_) = std::fs::copy(&path, &restore_path) {
                                if let Ok(metadata) = path.metadata() {
                                    total_size += metadata.len();
                                    file_count += 1;
                                    info!("Restored config file: {:?}", restore_path);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((total_size, file_count))
    }

    /// Restore log files
    async fn restore_logs(&self, backup_id: &str) -> Result<(u64, u32), AgentError> {
        let backup_dir = std::path::Path::new(&self.config.storage.local_path)
            .join("backups")
            .join(backup_id)
            .join("logs");
        let log_dir = std::path::Path::new(&self.config.storage.local_path).join("logs");

        let mut total_size = 0;
        let mut file_count = 0;

        if backup_dir.exists() {
            std::fs::create_dir_all(&log_dir)
                .map_err(|e| AgentError::IoError(format!("Failed to create log directory: {}", e)))?;

            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.extension().map_or(false, |ext| ext == "log") {
                            let restore_path = log_dir.join(path.file_name().unwrap());
                            if let Ok(_) = std::fs::copy(&path, &restore_path) {
                                if let Ok(metadata) = path.metadata() {
                                    total_size += metadata.len();
                                    file_count += 1;
                                    info!("Restored log file: {:?}", restore_path);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((total_size, file_count))
    }

    /// Restore index files
    async fn restore_indexes(&self, backup_id: &str) -> Result<(u64, u32), AgentError> {
        let backup_dir = std::path::Path::new(&self.config.storage.local_path)
            .join("backups")
            .join(backup_id)
            .join("indexes");
        let index_dir = std::path::Path::new(&self.config.storage.local_path).join("indexes");

        let mut total_size = 0;
        let mut file_count = 0;

        if backup_dir.exists() {
            std::fs::create_dir_all(&index_dir)
                .map_err(|e| AgentError::IoError(format!("Failed to create index directory: {}", e)))?;

            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            let restore_path = index_dir.join(path.file_name().unwrap());
                            if let Ok(_) = std::fs::copy(&path, &restore_path) {
                                if let Ok(metadata) = path.metadata() {
                                    total_size += metadata.len();
                                    file_count += 1;
                                    info!("Restored index file: {:?}", restore_path);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((total_size, file_count))
    }
}
