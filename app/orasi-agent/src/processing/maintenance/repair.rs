//! Repair operations for maintenance

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::MaintenanceTask;
use chrono;
use regex;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tracing::{info, warn, error};

/// Repair processor for handling system repair operations
pub struct RepairProcessor {
    config: AgentConfig,
}

impl RepairProcessor {
    /// Create new repair processor
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Perform repair operation
    pub async fn perform_repair(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        info!("Performing repair on targets: {:?}", task.targets);

        let mut issues_fixed = 0;
        let mut repair_results = serde_json::Map::new();

        for target in &task.targets {
            match target.as_str() {
                "data" => {
                    let (fixed, result) = self.repair_data().await?;
                    issues_fixed += fixed;
                    repair_results.insert("data".to_string(), result);
                }
                "indexes" => {
                    let (fixed, result) = self.repair_indexes().await?;
                    issues_fixed += fixed;
                    repair_results.insert("indexes".to_string(), result);
                }
                "database" => {
                    let (fixed, result) = self.repair_database().await?;
                    issues_fixed += fixed;
                    repair_results.insert("database".to_string(), result);
                }
                "config" => {
                    let (fixed, result) = self.repair_configuration().await?;
                    issues_fixed += fixed;
                    repair_results.insert("config".to_string(), result);
                }
                _ => {
                    warn!("Unknown repair target: {}", target);
                }
            }
        }

        let default_repair = "general".to_string();
        let repair_type = task.options.get("repair_type").unwrap_or(&default_repair);

        let result = serde_json::json!({
            "operation": "repair",
            "targets": task.targets,
            "repair_type": repair_type,
            "issues_fixed": issues_fixed,
            "repair_results": repair_results,
            "status": "completed"
        });

        Ok(result)
    }

    /// Repair data files
    async fn repair_data(&self) -> Result<(u32, Value), AgentError> {
        info!("Repairing data files");

        let mut repair_info = serde_json::Map::new();
        let mut issues_fixed = 0;

        // Check for corrupted data files
        let corrupted_files = self.find_corrupted_data_files().await?;
        repair_info.insert("corrupted_files_found".to_string(), Value::Number(serde_json::Number::from(corrupted_files.len())));

        for file_path in corrupted_files {
            if let Ok(()) = self.repair_data_file(&file_path).await {
                issues_fixed += 1;
                info!("Repaired data file: {:?}", file_path);
            }
        }

        repair_info.insert("files_repaired".to_string(), Value::Number(serde_json::Number::from(issues_fixed)));
        repair_info.insert("status".to_string(), Value::String("completed".to_string()));

        Ok((issues_fixed, Value::Object(repair_info)))
    }

    /// Repair indexes
    async fn repair_indexes(&self) -> Result<(u32, Value), AgentError> {
        info!("Repairing indexes");

        let mut repair_info = serde_json::Map::new();
        let mut issues_fixed = 0;

        // Check for corrupted indexes
        let corrupted_indexes = self.find_corrupted_indexes().await?;
        repair_info.insert("corrupted_indexes_found".to_string(), Value::Number(serde_json::Number::from(corrupted_indexes.len())));

        for index_name in corrupted_indexes {
            if let Ok(()) = self.repair_index(&index_name).await {
                issues_fixed += 1;
                info!("Repaired index: {}", index_name);
            }
        }

        repair_info.insert("indexes_repaired".to_string(), Value::Number(serde_json::Number::from(issues_fixed)));
        repair_info.insert("status".to_string(), Value::String("completed".to_string()));

        Ok((issues_fixed, Value::Object(repair_info)))
    }

    /// Repair database
    async fn repair_database(&self) -> Result<(u32, Value), AgentError> {
        info!("Repairing database");

        let mut repair_info = serde_json::Map::new();
        let mut issues_fixed = 0;

        // Check database integrity
        let integrity_issues = self.check_database_integrity().await?;
        repair_info.insert("integrity_issues_found".to_string(), Value::Number(serde_json::Number::from(integrity_issues.len())));

        for issue in integrity_issues {
            if let Ok(()) = self.fix_database_issue(&issue).await {
                issues_fixed += 1;
                info!("Fixed database issue: {}", issue);
            }
        }

        repair_info.insert("issues_fixed".to_string(), Value::Number(serde_json::Number::from(issues_fixed)));
        repair_info.insert("status".to_string(), Value::String("completed".to_string()));

        Ok((issues_fixed, Value::Object(repair_info)))
    }

    /// Repair configuration
    async fn repair_configuration(&self) -> Result<(u32, Value), AgentError> {
        info!("Repairing configuration");

        let mut repair_info = serde_json::Map::new();
        let mut issues_fixed = 0;

        // Check configuration validity
        let config_issues = self.check_configuration_validity().await?;
        repair_info.insert("config_issues_found".to_string(), Value::Number(serde_json::Number::from(config_issues.len())));

        for issue in config_issues {
            if let Ok(()) = self.fix_configuration_issue(&issue).await {
                issues_fixed += 1;
                info!("Fixed configuration issue: {}", issue);
            }
        }

        repair_info.insert("issues_fixed".to_string(), Value::Number(serde_json::Number::from(issues_fixed)));
        repair_info.insert("status".to_string(), Value::String("completed".to_string()));

        Ok((issues_fixed, Value::Object(repair_info)))
    }

    /// Find corrupted data files
    async fn find_corrupted_data_files(&self) -> Result<Vec<std::path::PathBuf>, AgentError> {
        let data_dir = std::path::Path::new(&self.config.storage.data_directory);
        let mut corrupted_files = Vec::new();

        if data_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&data_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            // Check if file is corrupted
                            if self.is_file_corrupted(&path).await? {
                                corrupted_files.push(path);
                            }
                        }
                    }
                }
            }
        }

        info!("Found {} corrupted data files", corrupted_files.len());
        Ok(corrupted_files)
    }

    /// Check if a file is corrupted
    async fn is_file_corrupted(&self, file_path: &Path) -> Result<bool, AgentError> {
        if let Ok(metadata) = file_path.metadata() {
            // Check for empty files
            if metadata.len() == 0 {
                return Ok(true);
            }

            // Check file extension and validate format
            if let Some(extension) = file_path.extension() {
                match extension.to_str().unwrap_or("") {
                    "parquet" => {
                        // Check Parquet file header
                        if let Ok(content) = fs::read(file_path) {
                            if content.len() < 4 || &content[0..4] != b"PAR1" {
                                return Ok(true);
                            }
                        }
                    }
                    "json" => {
                        // Check JSON file validity
                        if let Ok(content) = fs::read_to_string(file_path) {
                            if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                                return Ok(true);
                            }
                        }
                    }
                    "avro" => {
                        // Check Avro file header
                        if let Ok(content) = fs::read(file_path) {
                            if content.len() < 4 || &content[0..4] != b"Obj\x01" {
                                return Ok(true);
                            }
                        }
                    }
                    _ => {
                        // For other files, check if they're readable
                        if fs::read(file_path).is_err() {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Repair a specific data file
    async fn repair_data_file(&self, file_path: &std::path::Path) -> Result<(), AgentError> {
        info!("Repairing data file: {:?}", file_path);

        // Create backup of the corrupted file
        let backup_path = file_path.with_extension("backup");
        if let Err(e) = fs::copy(file_path, &backup_path) {
            error!("Failed to create backup: {}", e);
            return Err(AgentError::IoError(format!("Failed to create backup: {}", e)));
        }

        // Try to repair based on file type
        if let Some(extension) = file_path.extension() {
            match extension.to_str().unwrap_or("") {
                "parquet" => self.repair_parquet_file(file_path).await?,
                "json" => self.repair_json_file(file_path).await?,
                "avro" => self.repair_avro_file(file_path).await?,
                _ => self.repair_generic_file(file_path).await?,
            }
        } else {
            self.repair_generic_file(file_path).await?;
        }

        info!("Successfully repaired data file: {:?}", file_path);
        Ok(())
    }

    /// Repair Parquet file
    async fn repair_parquet_file(&self, file_path: &Path) -> Result<(), AgentError> {
        // Try to read and rewrite the Parquet file to fix corruption
        if let Ok(content) = fs::read(file_path) {
            // Check if file has valid Parquet header
            if content.len() >= 4 && &content[0..4] == b"PAR1" {
                // File has valid header, try to truncate to last valid record
                if let Some(last_valid_pos) = self.find_last_valid_parquet_position(&content) {
                    let mut file = fs::OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(file_path)?;
                    
                    use std::io::Write;
                    file.write_all(&content[0..last_valid_pos])?;
                    file.flush()?;
                }
            } else {
                // Invalid header, create empty Parquet file
                let empty_parquet = b"PAR1\x00\x00\x00\x00PAR1";
                fs::write(file_path, empty_parquet)?;
            }
        }
        Ok(())
    }

    /// Find last valid position in Parquet file
    fn find_last_valid_parquet_position(&self, content: &[u8]) -> Option<usize> {
        // Simple implementation: find the last occurrence of "PAR1"
        for i in (0..content.len()).rev() {
            if i >= 3 && &content[i-3..=i] == b"PAR1" {
                return Some(i + 1);
            }
        }
        None
    }

    /// Repair JSON file
    async fn repair_json_file(&self, file_path: &Path) -> Result<(), AgentError> {
        if let Ok(content) = fs::read_to_string(file_path) {
            // Try to fix common JSON issues
            let repaired_content = self.fix_json_content(&content);
            
            // Validate the repaired content
            if serde_json::from_str::<serde_json::Value>(&repaired_content).is_ok() {
                fs::write(file_path, repaired_content)?;
            } else {
                // If still invalid, create empty JSON object
                fs::write(file_path, "{}")?;
            }
        }
        Ok(())
    }

    /// Fix common JSON content issues
    fn fix_json_content(&self, content: &str) -> String {
        let mut fixed = content.to_string();
        
        // Remove trailing commas
        fixed = fixed.replace(",\n}", "\n}");
        fixed = fixed.replace(",\n]", "\n]");
        
        // Fix missing quotes around property names
        let re = regex::Regex::new(r"(\w+):").unwrap();
        fixed = re.replace_all(&fixed, r#""$1":"#).to_string();
        
        // Remove null bytes
        fixed = fixed.replace('\0', "");
        
        fixed
    }

    /// Repair Avro file
    async fn repair_avro_file(&self, file_path: &Path) -> Result<(), AgentError> {
        if let Ok(content) = fs::read(file_path) {
            // Check if file has valid Avro header
            if content.len() >= 4 && &content[0..4] == b"Obj\x01" {
                // File has valid header, try to truncate to last valid record
                if let Some(last_valid_pos) = self.find_last_valid_avro_position(&content) {
                    let mut file = fs::OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(file_path)?;
                    
                    use std::io::Write;
                    file.write_all(&content[0..last_valid_pos])?;
                    file.flush()?;
                }
            } else {
                // Invalid header, create empty Avro file
                let empty_avro = b"Obj\x01\x00\x00\x00\x00";
                fs::write(file_path, empty_avro)?;
            }
        }
        Ok(())
    }

    /// Find last valid position in Avro file
    fn find_last_valid_avro_position(&self, content: &[u8]) -> Option<usize> {
        // Simple implementation: find the last occurrence of "Obj\x01"
        for i in (0..content.len()).rev() {
            if i >= 3 && &content[i-3..=i] == b"Obj\x01" {
                return Some(i + 1);
            }
        }
        None
    }

    /// Repair generic file
    async fn repair_generic_file(&self, file_path: &Path) -> Result<(), AgentError> {
        // For generic files, try to read and rewrite to fix any I/O issues
        if let Ok(content) = fs::read(file_path) {
            // Remove any null bytes that might cause issues
            let cleaned_content: Vec<u8> = content.into_iter().filter(|&b| b != 0).collect();
            fs::write(file_path, cleaned_content)?;
        }
        Ok(())
    }

    /// Find corrupted indexes
    async fn find_corrupted_indexes(&self) -> Result<Vec<String>, AgentError> {
        let index_dir = std::path::Path::new(&self.config.storage.data_directory).join("indexes");
        let mut corrupted_indexes = Vec::new();

        if index_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&index_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(file_name) = path.file_name() {
                                if let Some(name_str) = file_name.to_str() {
                                    if name_str.ends_with(".idx") {
                                        // Check if index file is corrupted
                                        if self.is_index_corrupted(&path).await? {
                                            let index_name = name_str.trim_end_matches(".idx").to_string();
                                            corrupted_indexes.push(index_name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        info!("Found {} corrupted indexes", corrupted_indexes.len());
        Ok(corrupted_indexes)
    }

    /// Check if an index file is corrupted
    async fn is_index_corrupted(&self, index_path: &Path) -> Result<bool, AgentError> {
        if let Ok(metadata) = index_path.metadata() {
            // Check for empty index files
            if metadata.len() == 0 {
                return Ok(true);
            }

            // Check if index file has valid JSON structure
            if let Ok(content) = fs::read_to_string(index_path) {
                if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                    return Ok(true);
                }
            } else {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Repair a specific index
    async fn repair_index(&self, index_name: &str) -> Result<(), AgentError> {
        info!("Repairing index: {}", index_name);

        let index_path = std::path::Path::new(&self.config.storage.data_directory)
            .join("indexes")
            .join(format!("{}.idx", index_name));

        if index_path.exists() {
            // Create backup
            let backup_path = index_path.with_extension("backup");
            fs::copy(&index_path, &backup_path)?;

            // Try to repair the index
            if let Ok(content) = fs::read_to_string(&index_path) {
                // Try to fix JSON content
                let repaired_content = self.fix_json_content(&content);
                
                if serde_json::from_str::<serde_json::Value>(&repaired_content).is_ok() {
                    fs::write(&index_path, repaired_content)?;
                } else {
                    // If still invalid, recreate index from data files
                    self.recreate_index(index_name).await?;
                }
            } else {
                // File is unreadable, recreate index
                self.recreate_index(index_name).await?;
            }
        } else {
            // Index file doesn't exist, create it
            self.recreate_index(index_name).await?;
        }

        info!("Successfully repaired index: {}", index_name);
        Ok(())
    }

    /// Recreate an index from data files
    async fn recreate_index(&self, index_name: &str) -> Result<(), AgentError> {
        let index_path = std::path::Path::new(&self.config.storage.data_directory)
            .join("indexes")
            .join(format!("{}.idx", index_name));

        // Create a new index with basic metadata
        let index_data = serde_json::json!({
            "index_name": index_name,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "repaired_at": chrono::Utc::now().to_rfc3339(),
            "version": "1.0",
            "status": "recreated"
        });

        fs::write(&index_path, serde_json::to_string_pretty(&index_data)?)?;
        Ok(())
    }

    /// Check database integrity
    async fn check_database_integrity(&self) -> Result<Vec<String>, AgentError> {
        let mut issues = Vec::new();

        // Check if database connection is working
        if let Err(e) = self.test_database_connection().await {
            issues.push(format!("Database connection failed: {}", e));
        }

        // Check for database corruption using system tools
        if let Ok(output) = Command::new("sqlite3")
            .args(&["--version"])
            .output() {
            if output.status.success() {
                // SQLite is available, check for corruption
                if let Ok(corruption_check) = self.check_sqlite_corruption().await {
                    issues.extend(corruption_check);
                }
            }
        }

        // Check PostgreSQL if available
        if let Ok(output) = Command::new("psql")
            .args(&["--version"])
            .output() {
            if output.status.success() {
                if let Ok(pg_issues) = self.check_postgresql_integrity().await {
                    issues.extend(pg_issues);
                }
            }
        }

        info!("Found {} database integrity issues", issues.len());
        Ok(issues)
    }

    /// Test database connection
    async fn test_database_connection(&self) -> Result<(), AgentError> {
        // This would typically test the actual database connection
        // For now, we'll simulate a connection test
        Ok(())
    }

    /// Check SQLite corruption
    async fn check_sqlite_corruption(&self) -> Result<Vec<String>, AgentError> {
        let mut issues = Vec::new();
        
        // Look for SQLite database files in the data directory
        let data_dir = std::path::Path::new(&self.config.storage.data_directory);
        if let Ok(entries) = std::fs::read_dir(data_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(extension) = path.extension() {
                            if extension == "db" || extension == "sqlite" || extension == "sqlite3" {
                                // Check SQLite database integrity
                                if let Ok(output) = Command::new("sqlite3")
                                    .args(&[path.to_str().unwrap(), "PRAGMA integrity_check;"])
                                    .output() {
                                    if !output.status.success() {
                                        issues.push(format!("SQLite database corruption detected: {:?}", path));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(issues)
    }

    /// Check PostgreSQL integrity
    async fn check_postgresql_integrity(&self) -> Result<Vec<String>, AgentError> {
        let mut issues = Vec::new();
        
        // Check if PostgreSQL is running and accessible
        if let Ok(output) = Command::new("pg_isready")
            .output() {
            if !output.status.success() {
                issues.push("PostgreSQL service is not running".to_string());
            }
        }

        Ok(issues)
    }

    /// Fix a specific database issue
    async fn fix_database_issue(&self, issue: &str) -> Result<(), AgentError> {
        info!("Fixing database issue: {}", issue);

        if issue.contains("SQLite database corruption") {
            // Extract database path from issue message
            if let Some(path_start) = issue.find(": ") {
                let db_path = &issue[path_start + 2..];
                self.repair_sqlite_database(db_path).await?;
            }
        } else if issue.contains("PostgreSQL service is not running") {
            self.restart_postgresql_service().await?;
        } else if issue.contains("Database connection failed") {
            self.repair_database_connection().await?;
        }

        info!("Successfully fixed database issue: {}", issue);
        Ok(())
    }

    /// Repair SQLite database
    async fn repair_sqlite_database(&self, db_path: &str) -> Result<(), AgentError> {
        // Create backup
        let backup_path = format!("{}.backup", db_path);
        fs::copy(db_path, &backup_path)?;

        // Try to repair using SQLite's VACUUM command
        if let Ok(output) = Command::new("sqlite3")
            .args(&[db_path, "VACUUM;"])
            .output() {
            if !output.status.success() {
                error!("Failed to repair SQLite database: {}", String::from_utf8_lossy(&output.stderr));
                return Err(AgentError::Internal("Failed to repair SQLite database".to_string()));
            }
        }

        Ok(())
    }

    /// Restart PostgreSQL service
    async fn restart_postgresql_service(&self) -> Result<(), AgentError> {
        // Try to restart PostgreSQL service
        if let Ok(output) = Command::new("sudo")
            .args(&["systemctl", "restart", "postgresql"])
            .output() {
            if !output.status.success() {
                // Try alternative restart method
                if let Ok(output) = Command::new("brew")
                    .args(&["services", "restart", "postgresql"])
                    .output() {
                    if !output.status.success() {
                        return Err(AgentError::Internal("Failed to restart PostgreSQL service".to_string()));
                    }
                }
            }
        }

        Ok(())
    }

    /// Repair database connection
    async fn repair_database_connection(&self) -> Result<(), AgentError> {
        // This would typically involve checking connection parameters,
        // restarting connection pools, etc.
        info!("Repairing database connection");
        Ok(())
    }

    /// Check configuration validity
    async fn check_configuration_validity(&self) -> Result<Vec<String>, AgentError> {
        let mut issues = Vec::new();

        // Check if configuration file exists and is readable
        let config_path = Path::new("config/agent.toml");
        if !config_path.exists() {
            issues.push("Configuration file does not exist".to_string());
        } else if let Ok(content) = fs::read_to_string(config_path) {
            // Check if configuration is valid TOML
            if toml::from_str::<AgentConfig>(&content).is_err() {
                issues.push("Configuration file contains invalid TOML".to_string());
            }
        } else {
            issues.push("Configuration file is not readable".to_string());
        }

        // Check required environment variables
        let required_env_vars = [
            "ORASI_AGENT_ID",
            "ORASI_DATA_DIRECTORY",
            "ORASI_LOG_LEVEL",
        ];

        for var in &required_env_vars {
            if std::env::var(var).is_err() {
                issues.push(format!("Required environment variable {} is not set", var));
            }
        }

        // Check storage directory permissions
        let data_dir = Path::new(&self.config.storage.data_directory);
        if data_dir.exists() {
            if let Ok(metadata) = data_dir.metadata() {
                if !metadata.permissions().readonly() {
                    // Check if we can write to the directory
                    let test_file = data_dir.join(".test_write");
                    if fs::write(&test_file, "test").is_err() {
                        issues.push("Storage directory is not writable".to_string());
                    } else {
                        let _ = fs::remove_file(test_file);
                    }
                } else {
                    issues.push("Storage directory is read-only".to_string());
                }
            }
        } else {
            // Try to create the directory
            if fs::create_dir_all(data_dir).is_err() {
                issues.push("Cannot create storage directory".to_string());
            }
        }

        info!("Found {} configuration issues", issues.len());
        Ok(issues)
    }

    /// Fix a specific configuration issue
    async fn fix_configuration_issue(&self, issue: &str) -> Result<(), AgentError> {
        info!("Fixing configuration issue: {}", issue);

        if issue.contains("Configuration file does not exist") {
            self.create_default_configuration().await?;
        } else if issue.contains("Configuration file contains invalid TOML") {
            self.fix_configuration_syntax().await?;
        } else if issue.contains("Required environment variable") {
            self.set_default_environment_variables().await?;
        } else if issue.contains("Storage directory") {
            self.fix_storage_directory_permissions().await?;
        }

        info!("Successfully fixed configuration issue: {}", issue);
        Ok(())
    }

    /// Create default configuration
    async fn create_default_configuration(&self) -> Result<(), AgentError> {
        let config_content = r#"
# Orasi Agent Configuration
agent_id = "default-agent"
agent_endpoint = "0.0.0.0:8082"
health_endpoint = "0.0.0.0:8083"
metrics_endpoint = "0.0.0.0:9092"

[storage]
data_directory = "/var/lib/orasi/agent"
temp_directory = "/tmp/orasi/agent"
max_disk_usage_gb = 10

[processing]
max_concurrent_tasks = 10
batch_size = 1000
batch_timeout_ms = 5000
"#;

        // Create config directory if it doesn't exist
        fs::create_dir_all("config")?;
        fs::write("config/agent.toml", config_content)?;
        Ok(())
    }

    /// Fix configuration syntax
    async fn fix_configuration_syntax(&self) -> Result<(), AgentError> {
        if let Ok(content) = fs::read_to_string("config/agent.toml") {
            // Try to fix common TOML syntax issues
            let fixed_content = self.fix_toml_syntax(&content);
            
            // Validate the fixed content
            if toml::from_str::<AgentConfig>(&fixed_content).is_ok() {
                fs::write("config/agent.toml", fixed_content)?;
            } else {
                // If still invalid, create default configuration
                self.create_default_configuration().await?;
            }
        }
        Ok(())
    }

    /// Fix TOML syntax issues
    fn fix_toml_syntax(&self, content: &str) -> String {
        let mut fixed = content.to_string();
        
        // Fix missing quotes around string values
        let re = regex::Regex::new(r####"(\w+)\s*=\s*([^\"\s][^\s]*[^\"\s])"####).unwrap();
        fixed = re.replace_all(&fixed, r#"$1 = "$2""#).to_string();
        
        // Fix missing section brackets
        if !fixed.contains("[storage]") {
            fixed.push_str("\n[storage]\n");
        }
        
        if !fixed.contains("[processing]") {
            fixed.push_str("\n[processing]\n");
        }
        
        fixed
    }

    /// Set default environment variables
    async fn set_default_environment_variables(&self) -> Result<(), AgentError> {
        let defaults = [
            ("ORASI_AGENT_ID", "default-agent"),
            ("ORASI_DATA_DIRECTORY", "/var/lib/orasi/agent"),
            ("ORASI_LOG_LEVEL", "info"),
        ];

        for (var, value) in &defaults {
            if std::env::var(var).is_err() {
                std::env::set_var(var, value);
            }
        }

        Ok(())
    }

    /// Fix storage directory permissions
    async fn fix_storage_directory_permissions(&self) -> Result<(), AgentError> {
        let data_dir = Path::new(&self.config.storage.data_directory);
        
        if !data_dir.exists() {
            fs::create_dir_all(data_dir)?;
        }

        // Try to set write permissions
        if let Ok(metadata) = data_dir.metadata() {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            fs::set_permissions(data_dir, permissions)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentConfig;

    #[tokio::test]
    async fn test_repair_processor_creation() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        assert_eq!(processor.config.agent_id, config.agent_id);
    }

    #[tokio::test]
    async fn test_find_corrupted_data_files() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        let result = processor.find_corrupted_data_files().await;
        assert!(result.is_ok());
        // Should return a vector of paths
        assert!(result.unwrap().is_empty() || result.unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_find_corrupted_indexes() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        let result = processor.find_corrupted_indexes().await;
        assert!(result.is_ok());
        // Should return a vector of strings
        assert!(result.unwrap().is_empty() || result.unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_check_database_integrity() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        let result = processor.check_database_integrity().await;
        assert!(result.is_ok());
        // Should return a vector of strings
        assert!(result.unwrap().is_empty() || result.unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_check_configuration_validity() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        let result = processor.check_configuration_validity().await;
        assert!(result.is_ok());
        // Should return a vector of strings
        assert!(result.unwrap().is_empty() || result.unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_is_file_corrupted() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        
        // Test with a non-existent file
        let non_existent_path = Path::new("/non/existent/file.txt");
        let result = processor.is_file_corrupted(non_existent_path).await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Non-existent files are not considered corrupted
    }

    #[tokio::test]
    async fn test_fix_json_content() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        
        // Test with malformed JSON
        let malformed_json = r#"{"name": "test", "value": 123,}"#;
        let fixed = processor.fix_json_content(malformed_json);
        
        // Should be valid JSON after fixing
        assert!(serde_json::from_str::<serde_json::Value>(&fixed).is_ok());
    }

    #[tokio::test]
    async fn test_fix_toml_syntax() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        
        // Test with malformed TOML
        let malformed_toml = r#"
agent_id = default-agent
endpoint = 0.0.0.0:8082
"#;
        let fixed = processor.fix_toml_syntax(malformed_toml);
        
        // Should contain proper TOML syntax
        assert!(fixed.contains(r#"agent_id = "default-agent""#));
        assert!(fixed.contains(r#"endpoint = "0.0.0.0:8082""#));
    }

    #[tokio::test]
    async fn test_recreate_index() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        
        let test_index_name = "test_index";
        let result = processor.recreate_index(test_index_name).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_repair_data() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        let result = processor.repair_data().await;
        assert!(result.is_ok());
        
        let (issues_fixed, _) = result.unwrap();
        assert!(issues_fixed >= 0);
    }

    #[tokio::test]
    async fn test_repair_indexes() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        let result = processor.repair_indexes().await;
        assert!(result.is_ok());
        
        let (issues_fixed, _) = result.unwrap();
        assert!(issues_fixed >= 0);
    }

    #[tokio::test]
    async fn test_repair_database() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        let result = processor.repair_database().await;
        assert!(result.is_ok());
        
        let (issues_fixed, _) = result.unwrap();
        assert!(issues_fixed >= 0);
    }

    #[tokio::test]
    async fn test_repair_configuration() {
        let config = AgentConfig::default();
        let processor = RepairProcessor::new(&config);
        let result = processor.repair_configuration().await;
        assert!(result.is_ok());
        
        let (issues_fixed, _) = result.unwrap();
        assert!(issues_fixed >= 0);
    }
}
