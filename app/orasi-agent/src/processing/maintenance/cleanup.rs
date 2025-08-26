//! Cleanup operations for maintenance

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::MaintenanceTask;
use serde_json::Value;
use sqlx::{Connection, SqliteConnection, Row};
use std::path::Path;
use tracing::{info, warn, error};

/// Cleanup processor for handling file system cleanup operations
pub struct CleanupProcessor {
    config: AgentConfig,
}

impl CleanupProcessor {
    /// Create new cleanup processor
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Perform cleanup operation
    pub async fn perform_cleanup(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        info!("Performing cleanup on targets: {:?}", task.targets);

        let mut files_removed = 0;
        let mut space_freed_bytes = 0;

        for target in &task.targets {
            match target.as_str() {
                "temp_files" => {
                    let (removed, freed) = self.cleanup_temp_files().await?;
                    files_removed += removed;
                    space_freed_bytes += freed;
                }
                "old_logs" => {
                    let (removed, freed) = self.cleanup_old_logs().await?;
                    files_removed += removed;
                    space_freed_bytes += freed;
                }
                "cache" => {
                    let (removed, freed) = self.cleanup_cache().await?;
                    files_removed += removed;
                    space_freed_bytes += freed;
                }
                "orphaned_data" => {
                    let (removed, freed) = self.cleanup_orphaned_data().await?;
                    files_removed += removed;
                    space_freed_bytes += freed;
                }
                _ => {
                    warn!("Unknown cleanup target: {}", target);
                }
            }
        }

        let result = serde_json::json!({
            "operation": "cleanup",
            "targets": task.targets,
            "files_removed": files_removed,
            "space_freed_bytes": space_freed_bytes,
            "status": "completed"
        });

        Ok(result)
    }

    /// Clean up temporary files
    async fn cleanup_temp_files(&self) -> Result<(u32, u64), AgentError> {
        let temp_dir = std::env::temp_dir().join("orasi-agent");
        let mut files_removed = 0;
        let mut space_freed = 0;

        if temp_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&temp_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            // Check if file is older than 24 hours
                            if let Ok(metadata) = path.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    let age = std::time::SystemTime::now()
                                        .duration_since(modified)
                                        .unwrap_or_default();
                                    
                                    if age.as_secs() > 24 * 60 * 60 {
                                        // Get file size before removal
                                        let file_size = metadata.len();
                                        
                                        if let Ok(()) = std::fs::remove_file(&path) {
                                            files_removed += 1;
                                            space_freed += file_size;
                                            info!("Removed old temp file: {:?}", path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((files_removed, space_freed))
    }

    /// Clean up old log files
    async fn cleanup_old_logs(&self) -> Result<(u32, u64), AgentError> {
        let log_dir = std::path::Path::new(&self.config.storage.local_path).join("logs");
        let mut files_removed = 0;
        let mut space_freed = 0;

        if log_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&log_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.extension().map_or(false, |ext| ext == "log") {
                            // Check if log file is older than 7 days
                            if let Ok(metadata) = path.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    let age = std::time::SystemTime::now()
                                        .duration_since(modified)
                                        .unwrap_or_default();
                                    
                                    if age.as_secs() > 7 * 24 * 60 * 60 {
                                        let file_size = metadata.len();
                                        
                                        if let Ok(()) = std::fs::remove_file(&path) {
                                            files_removed += 1;
                                            space_freed += file_size;
                                            info!("Removed old log file: {:?}", path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((files_removed, space_freed))
    }

    /// Clean up cache files
    async fn cleanup_cache(&self) -> Result<(u32, u64), AgentError> {
        let cache_dir = std::path::Path::new(&self.config.storage.local_path).join("cache");
        let mut files_removed = 0;
        let mut space_freed = 0;

        if cache_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            // Check if cache file is older than 1 hour
                            if let Ok(metadata) = path.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    let age = std::time::SystemTime::now()
                                        .duration_since(modified)
                                        .unwrap_or_default();
                                    
                                    if age.as_secs() > 60 * 60 {
                                        let file_size = metadata.len();
                                        
                                        if let Ok(()) = std::fs::remove_file(&path) {
                                            files_removed += 1;
                                            space_freed += file_size;
                                            info!("Removed old cache file: {:?}", path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((files_removed, space_freed))
    }

    /// Clean up orphaned data files
    async fn cleanup_orphaned_data(&self) -> Result<(u32, u64), AgentError> {
        let data_dir = std::path::Path::new(&self.config.storage.local_path).join("data");
        let mut files_removed = 0;
        let mut space_freed = 0;

        if data_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&data_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            // Check if data file is older than 30 days and has no references
                            if let Ok(metadata) = path.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    let age = std::time::SystemTime::now()
                                        .duration_since(modified)
                                        .unwrap_or_default();
                                    
                                    if age.as_secs() > 30 * 24 * 60 * 60 {
                                        // Check if file is referenced in the database
                                        if !self.is_file_referenced(&path).await? {
                                            let file_size = metadata.len();
                                            
                                            if let Ok(()) = std::fs::remove_file(&path) {
                                                files_removed += 1;
                                                space_freed += file_size;
                                                info!("Removed orphaned data file: {:?}", path);
                                            }
                                        } else {
                                            info!("Skipping referenced data file: {:?}", path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((files_removed, space_freed))
    }

    /// Check if a file is referenced in the database
    async fn is_file_referenced(&self, path: &Path) -> Result<bool, AgentError> {
        let file_path = path.to_string_lossy();
        let file_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        // Try to connect to the database
        let db_path = std::path::Path::new(&self.config.storage.local_path).join("agent.db");
        let database_url = format!("sqlite:{}", db_path.display());

        match SqliteConnection::connect(&database_url).await {
            Ok(mut conn) => {
                // Check multiple tables for file references
                let is_referenced = self.check_file_references_in_database(&mut conn, &file_path, file_name).await?;
                Ok(is_referenced)
            }
            Err(e) => {
                // If database connection fails, assume file is referenced to be safe
                warn!("Failed to connect to database for file reference check: {}", e);
                warn!("Assuming file is referenced to be safe: {:?}", path);
                Ok(true)
            }
        }
    }

    /// Check file references across multiple database tables
    async fn check_file_references_in_database(
        &self,
        conn: &mut SqliteConnection,
        file_path: &str,
        file_name: &str,
    ) -> Result<bool, AgentError> {
        // Check various tables that might reference files
        let tables_to_check = vec![
            "data_files",
            "processed_files", 
            "index_files",
            "backup_files",
            "export_files",
            "cache_files",
            "temp_files",
            "file_metadata",
            "file_references",
            "data_sources",
            "processing_tasks",
            "indexing_tasks",
            "export_tasks"
        ];

        for table in tables_to_check {
            if self.check_table_for_file_reference(conn, table, file_path, file_name).await? {
                info!("File reference found in table '{}': {}", table, file_path);
                return Ok(true);
            }
        }

        // Also check for partial path matches (in case of relative paths)
        if self.check_partial_path_references(conn, file_path, file_name).await? {
            info!("Partial file reference found: {}", file_path);
            return Ok(true);
        }

        Ok(false)
    }

    /// Check a specific table for file references
    async fn check_table_for_file_reference(
        &self,
        conn: &mut SqliteConnection,
        table: &str,
        file_path: &str,
        file_name: &str,
    ) -> Result<bool, AgentError> {
        // Common column names that might contain file paths
        let path_columns = vec![
            "file_path", "path", "location", "source_path", "target_path",
            "input_path", "output_path", "data_path", "file_location",
            "file_name", "filename", "name"
        ];

        for column in path_columns {
            let query = format!(
                "SELECT COUNT(*) as count FROM {} WHERE {} LIKE ? OR {} = ?",
                table, column, column
            );

            match sqlx::query(&query)
                .bind(format!("%{}%", file_name))
                .bind(file_path)
                .fetch_one(&mut *conn)
                .await
            {
                Ok(row) => {
                    let count: i64 = row.try_get("count")?;
                    if count > 0 {
                        return Ok(true);
                    }
                }
                Err(e) => {
                    // Table or column might not exist, continue to next
                    if !e.to_string().contains("no such table") && !e.to_string().contains("no such column") {
                        warn!("Error checking table {} column {}: {}", table, column, e);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check for partial path references (useful for relative paths)
    async fn check_partial_path_references(
        &self,
        conn: &mut SqliteConnection,
        file_path: &str,
        file_name: &str,
    ) -> Result<bool, AgentError> {
        // Get the file's directory path
        let file_dir = std::path::Path::new(file_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");

        // Check for directory-based references
        let tables_to_check = vec![
            "data_files", "processed_files", "file_metadata"
        ];

        for table in tables_to_check {
            let query = format!(
                "SELECT COUNT(*) as count FROM {} WHERE file_path LIKE ? OR path LIKE ?",
                table
            );

            match sqlx::query(&query)
                .bind(format!("%{}%", file_dir))
                .bind(format!("%{}%", file_name))
                .fetch_one(&mut *conn)
                .await
            {
                Ok(row) => {
                    let count: i64 = row.try_get("count")?;
                    if count > 0 {
                        return Ok(true);
                    }
                }
                Err(e) => {
                    // Table might not exist, continue
                    if !e.to_string().contains("no such table") {
                        warn!("Error checking partial path in table {}: {}", table, e);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get database statistics for cleanup reporting
    pub async fn get_database_stats(&self) -> Result<Value, AgentError> {
        let db_path = std::path::Path::new(&self.config.storage.local_path).join("agent.db");
        let database_url = format!("sqlite:{}", db_path.display());

        match SqliteConnection::connect(&database_url).await {
            Ok(mut conn) => {
                let mut stats = serde_json::Map::new();
                
                // Get table sizes
                let tables = vec![
                    "data_files", "processed_files", "index_files", "file_metadata"
                ];

                for table in tables {
                    let query = format!("SELECT COUNT(*) as count FROM {}", table);
                    match sqlx::query(&query).fetch_one(&mut conn).await {
                        Ok(row) => {
                            let count: i64 = row.try_get("count")?;
                            stats.insert(table.to_string(), serde_json::Value::Number(count.into()));
                        }
                        Err(_) => {
                            // Table doesn't exist
                            stats.insert(table.to_string(), serde_json::Value::Number(0.into()));
                        }
                    }
                }

                Ok(serde_json::Value::Object(stats))
            }
            Err(e) => {
                warn!("Failed to get database stats: {}", e);
                Ok(serde_json::json!({
                    "error": "Database connection failed",
                    "message": e.to_string()
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_file_reference_check() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();

        let processor = CleanupProcessor::new(&config);
        
        // Create a test database
        let db_path = temp_dir.path().join("agent.db");
        let database_url = format!("sqlite:{}", db_path.display());
        
        // Create database and tables
        let mut conn = SqliteConnection::connect(&database_url).await.unwrap();
        
        // Create test tables
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS data_files (
                id INTEGER PRIMARY KEY,
                file_path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )"
        ).execute(&mut conn).await.unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS processed_files (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL,
                status TEXT NOT NULL,
                processed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )"
        ).execute(&mut conn).await.unwrap();

        // Insert test data
        sqlx::query(
            "INSERT INTO data_files (file_path, file_name) VALUES (?, ?)"
        )
        .bind("/path/to/test/file.txt")
        .bind("file.txt")
        .execute(&mut conn)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO processed_files (path, status) VALUES (?, ?)"
        )
        .bind("/path/to/another/file.csv")
        .bind("completed")
        .execute(&mut conn)
        .await
        .unwrap();

        // Test file reference checking
        let test_file = Path::new("/path/to/test/file.txt");
        let is_referenced = processor.is_file_referenced(test_file).await.unwrap();
        assert!(is_referenced, "File should be referenced in database");

        let unreferenced_file = Path::new("/path/to/unreferenced/file.txt");
        let is_referenced = processor.is_file_referenced(unreferenced_file).await.unwrap();
        assert!(!is_referenced, "File should not be referenced in database");

        // Test database stats
        let stats = processor.get_database_stats().await.unwrap();
        assert!(stats.is_object());
        
        if let Some(data_files_count) = stats.get("data_files") {
            assert_eq!(data_files_count.as_u64().unwrap(), 1);
        }
    }

    #[tokio::test]
    async fn test_cleanup_orphaned_data() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();

        let processor = CleanupProcessor::new(&config);
        
        // Create test data directory
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();
        
        // Create a test file
        let test_file = data_dir.join("test_file.txt");
        fs::write(&test_file, "test content").unwrap();
        
        // Set file modification time to 31 days ago (older than 30 days)
        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(31 * 24 * 60 * 60);
        filetime::set_file_mtime(&test_file, filetime::FileTime::from_system_time(old_time)).unwrap();
        
        // Create maintenance task
        let task = MaintenanceTask {
            maintenance_type: "cleanup".to_string(),
            targets: vec!["orphaned_data".to_string()],
            options: std::collections::HashMap::new(),
        };
        
        // Perform cleanup
        let result = processor.perform_cleanup(&task).await.unwrap();
        
        // Verify result
        assert_eq!(result["operation"], "cleanup");
        assert_eq!(result["targets"][0], "orphaned_data");
        
        // Since there's no database, the file should be considered referenced (safe default)
        // and not removed
        assert!(test_file.exists(), "File should not be removed when database is not available");
    }

    #[tokio::test]
    async fn test_cleanup_temp_files() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();

        let processor = CleanupProcessor::new(&config);
        
        // Create test temp directory
        let temp_dir_path = std::env::temp_dir().join("orasi-agent");
        fs::create_dir_all(&temp_dir_path).unwrap();
        
        // Create a test file
        let test_file = temp_dir_path.join("test_temp.txt");
        fs::write(&test_file, "temp content").unwrap();
        
        // Set file modification time to 25 hours ago (older than 24 hours)
        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(25 * 60 * 60);
        filetime::set_file_mtime(&test_file, filetime::FileTime::from_system_time(old_time)).unwrap();
        
        // Create maintenance task
        let task = MaintenanceTask {
            maintenance_type: "cleanup".to_string(),
            targets: vec!["temp_files".to_string()],
            options: std::collections::HashMap::new(),
        };
        
        // Perform cleanup
        let result = processor.perform_cleanup(&task).await.unwrap();
        
        // Verify result
        assert_eq!(result["operation"], "cleanup");
        assert_eq!(result["targets"][0], "temp_files");
        assert!(result["files_removed"].as_u64().unwrap() > 0);
        assert!(result["space_freed_bytes"].as_u64().unwrap() > 0);
        
        // Clean up test temp directory
        fs::remove_dir_all(&temp_dir_path).ok();
    }
}
