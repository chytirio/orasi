//! Backup operations for maintenance

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::MaintenanceTask;
use serde_json::Value;
use tracing::{info, warn};

/// Backup processor for handling system backup operations
pub struct BackupProcessor {
    config: AgentConfig,
}

impl BackupProcessor {
    /// Create new backup processor
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Perform backup operation
    pub async fn perform_backup(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        info!("Performing backup of targets: {:?}", task.targets);

        let mut backup_size_bytes = 0;
        let mut files_backed_up = 0;
        let backup_id = self.generate_backup_id().await?;

        for target in &task.targets {
            match target.as_str() {
                "data" => {
                    let (size, count) = self.backup_data(&backup_id).await?;
                    backup_size_bytes += size;
                    files_backed_up += count;
                }
                "config" => {
                    let (size, count) = self.backup_config(&backup_id).await?;
                    backup_size_bytes += size;
                    files_backed_up += count;
                }
                "logs" => {
                    let (size, count) = self.backup_logs(&backup_id).await?;
                    backup_size_bytes += size;
                    files_backed_up += count;
                }
                "indexes" => {
                    let (size, count) = self.backup_indexes(&backup_id).await?;
                    backup_size_bytes += size;
                    files_backed_up += count;
                }
                _ => {
                    warn!("Unknown backup target: {}", target);
                }
            }
        }

        let default_backup = "default".to_string();
        let backup_location = task
            .options
            .get("backup_location")
            .unwrap_or(&default_backup);

        // Store backup metadata
        self.store_backup_metadata(&backup_id, backup_size_bytes, files_backed_up).await?;

        let result = serde_json::json!({
            "operation": "backup",
            "targets": task.targets,
            "backup_location": backup_location,
            "backup_id": backup_id,
            "backup_size_bytes": backup_size_bytes,
            "files_backed_up": files_backed_up,
            "status": "completed"
        });

        Ok(result)
    }

    /// Generate a unique backup ID
    async fn generate_backup_id(&self) -> Result<String, AgentError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let backup_id = format!("backup_{}", timestamp);
        Ok(backup_id)
    }

    /// Backup data files
    async fn backup_data(&self, backup_id: &str) -> Result<(u64, u32), AgentError> {
        let data_dir = std::path::Path::new(&self.config.storage.local_path).join("data");
        let backup_dir = std::path::Path::new(&self.config.storage.local_path)
            .join("backups")
            .join(backup_id)
            .join("data");

        let mut total_size = 0;
        let mut file_count = 0;

        if data_dir.exists() {
            std::fs::create_dir_all(&backup_dir)
                .map_err(|e| AgentError::IoError(format!("Failed to create backup directory: {}", e)))?;

            if let Ok(entries) = std::fs::read_dir(&data_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            let backup_path = backup_dir.join(path.file_name().unwrap());
                            if let Ok(_) = std::fs::copy(&path, &backup_path) {
                                if let Ok(metadata) = path.metadata() {
                                    total_size += metadata.len();
                                    file_count += 1;
                                    info!("Backed up data file: {:?}", path);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((total_size, file_count))
    }

    /// Backup configuration files
    async fn backup_config(&self, backup_id: &str) -> Result<(u64, u32), AgentError> {
        let config_dir = std::path::Path::new(&self.config.storage.local_path).join("config");
        let backup_dir = std::path::Path::new(&self.config.storage.local_path)
            .join("backups")
            .join(backup_id)
            .join("config");

        let mut total_size = 0;
        let mut file_count = 0;

        if config_dir.exists() {
            std::fs::create_dir_all(&backup_dir)
                .map_err(|e| AgentError::IoError(format!("Failed to create backup directory: {}", e)))?;

            if let Ok(entries) = std::fs::read_dir(&config_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            let backup_path = backup_dir.join(path.file_name().unwrap());
                            if let Ok(_) = std::fs::copy(&path, &backup_path) {
                                if let Ok(metadata) = path.metadata() {
                                    total_size += metadata.len();
                                    file_count += 1;
                                    info!("Backed up config file: {:?}", path);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((total_size, file_count))
    }

    /// Backup log files
    async fn backup_logs(&self, backup_id: &str) -> Result<(u64, u32), AgentError> {
        let log_dir = std::path::Path::new(&self.config.storage.local_path).join("logs");
        let backup_dir = std::path::Path::new(&self.config.storage.local_path)
            .join("backups")
            .join(backup_id)
            .join("logs");

        let mut total_size = 0;
        let mut file_count = 0;

        if log_dir.exists() {
            std::fs::create_dir_all(&backup_dir)
                .map_err(|e| AgentError::IoError(format!("Failed to create backup directory: {}", e)))?;

            if let Ok(entries) = std::fs::read_dir(&log_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.extension().map_or(false, |ext| ext == "log") {
                            let backup_path = backup_dir.join(path.file_name().unwrap());
                            if let Ok(_) = std::fs::copy(&path, &backup_path) {
                                if let Ok(metadata) = path.metadata() {
                                    total_size += metadata.len();
                                    file_count += 1;
                                    info!("Backed up log file: {:?}", path);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((total_size, file_count))
    }

    /// Backup index files
    async fn backup_indexes(&self, backup_id: &str) -> Result<(u64, u32), AgentError> {
        let index_dir = std::path::Path::new(&self.config.storage.local_path).join("indexes");
        let backup_dir = std::path::Path::new(&self.config.storage.local_path)
            .join("backups")
            .join(backup_id)
            .join("indexes");

        let mut total_size = 0;
        let mut file_count = 0;

        if index_dir.exists() {
            std::fs::create_dir_all(&backup_dir)
                .map_err(|e| AgentError::IoError(format!("Failed to create backup directory: {}", e)))?;

            if let Ok(entries) = std::fs::read_dir(&index_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            let backup_path = backup_dir.join(path.file_name().unwrap());
                            if let Ok(_) = std::fs::copy(&path, &backup_path) {
                                if let Ok(metadata) = path.metadata() {
                                    total_size += metadata.len();
                                    file_count += 1;
                                    info!("Backed up index file: {:?}", path);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((total_size, file_count))
    }

    /// Store backup metadata
    async fn store_backup_metadata(&self, backup_id: &str, size: u64, file_count: u32) -> Result<(), AgentError> {
        let metadata = serde_json::json!({
            "backup_id": backup_id,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "size_bytes": size,
            "file_count": file_count,
            "version": "1.0.0"
        });

        let metadata_path = std::path::Path::new(&self.config.storage.local_path)
            .join("backups")
            .join(backup_id)
            .join("metadata.json");

        if let Some(parent) = metadata_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AgentError::IoError(format!("Failed to create metadata directory: {}", e)))?;
        }

        std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata).unwrap())
            .map_err(|e| AgentError::IoError(format!("Failed to write backup metadata: {}", e)))?;

        Ok(())
    }
}
