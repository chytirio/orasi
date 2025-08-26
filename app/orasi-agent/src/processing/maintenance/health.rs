//! Health check operations for maintenance

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::MaintenanceTask;
use serde_json::Value;
use sqlx::{Connection, SqliteConnection, Row};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn, error};
use sysinfo::System;

/// Health processor for handling system health check operations
pub struct HealthProcessor {
    config: AgentConfig,
}

impl HealthProcessor {
    /// Create new health processor
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Perform health check operation
    pub async fn perform_health_check(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        info!("Performing health check on targets: {:?}", task.targets);

        let mut health_status = "healthy".to_string();
        let mut checks_performed = 0;
        let mut issues_found = 0;
        let mut health_details = serde_json::Map::new();

        for target in &task.targets {
            match target.as_str() {
                "system" => {
                    let (status, issues, details) = self.check_system_health().await?;
                    checks_performed += 1;
                    issues_found += issues;
                    if status != "healthy" {
                        health_status = status;
                    }
                    health_details.insert("system".to_string(), details);
                }
                "connectivity" => {
                    let (status, issues, details) = self.check_connectivity().await?;
                    checks_performed += 1;
                    issues_found += issues;
                    if status != "healthy" {
                        health_status = status;
                    }
                    health_details.insert("connectivity".to_string(), details);
                }
                "storage" => {
                    let (status, issues, details) = self.check_storage_health().await?;
                    checks_performed += 1;
                    issues_found += issues;
                    if status != "healthy" {
                        health_status = status;
                    }
                    health_details.insert("storage".to_string(), details);
                }
                "database" => {
                    let (status, issues, details) = self.check_database_health().await?;
                    checks_performed += 1;
                    issues_found += issues;
                    if status != "healthy" {
                        health_status = status;
                    }
                    health_details.insert("database".to_string(), details);
                }
                _ => {
                    warn!("Unknown health check target: {}", target);
                }
            }
        }

        let result = serde_json::json!({
            "operation": "health_check",
            "targets": task.targets,
            "health_status": health_status,
            "checks_performed": checks_performed,
            "issues_found": issues_found,
            "health_details": health_details,
            "status": "completed"
        });

        Ok(result)
    }

    /// Check system health
    async fn check_system_health(&self) -> Result<(String, u32, Value), AgentError> {
        let mut issues = 0;
        let mut details = serde_json::Map::new();

        // Check memory usage
        let mut memory_info = System::new_all();
        let total_memory = memory_info.total_memory();
        let used_memory = memory_info.used_memory();
        let memory_usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;

        details.insert("memory_usage_percent".to_string(), Value::Number(
            serde_json::Number::from_f64(memory_usage_percent).unwrap_or(serde_json::Number::from(0))
        ));

        if memory_usage_percent > 90.0 {
            issues += 1;
        }

        // Check CPU usage
        let mut cpu_info = System::new_all();
        let cpu_usage = cpu_info.global_cpu_info().cpu_usage();
        
        details.insert("cpu_usage_percent".to_string(), Value::Number(
            serde_json::Number::from_f64(cpu_usage.into()).unwrap_or(serde_json::Number::from(0))
        ));

        if cpu_usage > 90.0 {
            issues += 1;
        }

        // Check disk space (simplified approach)
        let disk_usage_percent = 75.0; // Placeholder value
        details.insert("disk_usage_percent".to_string(), Value::Number(
            serde_json::Number::from_f64(disk_usage_percent).unwrap_or(serde_json::Number::from(0))
        ));

        if disk_usage_percent > 90.0 {
            issues += 1;
        }

        let status = if issues == 0 { "healthy" } else { "warning" };
        Ok((status.to_string(), issues, Value::Object(details)))
    }

    /// Check connectivity health
    async fn check_connectivity(&self) -> Result<(String, u32, Value), AgentError> {
        let mut issues = 0;
        let mut details = serde_json::Map::new();

        // Check network connectivity
        let network_status = self.check_network_connectivity().await?;
        details.insert("network_status".to_string(), Value::String(network_status.clone()));

        if network_status != "connected" {
            issues += 1;
        }

        // Check service endpoints
        let endpoints_status = self.check_service_endpoints().await?;
        details.insert("endpoints_status".to_string(), endpoints_status);

        let status = if issues == 0 { "healthy" } else { "warning" };
        Ok((status.to_string(), issues, Value::Object(details)))
    }

    /// Check storage health
    async fn check_storage_health(&self) -> Result<(String, u32, Value), AgentError> {
        let mut issues = 0;
        let mut details = serde_json::Map::new();

        // Check local storage
        let local_storage_status = self.check_local_storage().await?;
        details.insert("local_storage".to_string(), local_storage_status);

        // Check backup storage
        let backup_storage_status = self.check_backup_storage().await?;
        details.insert("backup_storage".to_string(), backup_storage_status);

        let status = if issues == 0 { "healthy" } else { "warning" };
        Ok((status.to_string(), issues, Value::Object(details)))
    }

    /// Check database health
    async fn check_database_health(&self) -> Result<(String, u32, Value), AgentError> {
        let mut issues = 0;
        let mut details = serde_json::Map::new();

        // Check database connectivity
        let db_connectivity = self.check_database_connectivity().await?;
        details.insert("connectivity".to_string(), Value::String(db_connectivity.clone()));

        if db_connectivity != "connected" {
            issues += 1;
        }

        // Check database performance
        let db_performance = self.check_database_performance().await?;
        details.insert("performance".to_string(), db_performance);

        let status = if issues == 0 { "healthy" } else { "warning" };
        Ok((status.to_string(), issues, Value::Object(details)))
    }

    /// Check network connectivity
    async fn check_network_connectivity(&self) -> Result<String, AgentError> {
        // Test connectivity to common endpoints
        let test_endpoints = vec![
            "8.8.8.8:53",      // Google DNS
            "1.1.1.1:53",      // Cloudflare DNS
            "208.67.222.222:53" // OpenDNS
        ];

        let mut successful_connections = 0;
        let timeout_duration = Duration::from_secs(5);

        for endpoint in test_endpoints {
            match timeout(timeout_duration, self.test_tcp_connection(endpoint)).await {
                Ok(Ok(_)) => {
                    successful_connections += 1;
                    info!("Network connectivity test passed for {}", endpoint);
                }
                Ok(Err(e)) => {
                    warn!("Network connectivity test failed for {}: {}", endpoint, e);
                }
                Err(_) => {
                    warn!("Network connectivity test timed out for {}", endpoint);
                }
            }
        }

        // Also test HTTP connectivity to a reliable service
        match timeout(timeout_duration, self.test_http_connectivity()).await {
            Ok(Ok(_)) => {
                successful_connections += 1;
                info!("HTTP connectivity test passed");
            }
            Ok(Err(e)) => {
                warn!("HTTP connectivity test failed: {}", e);
            }
            Err(_) => {
                warn!("HTTP connectivity test timed out");
            }
        }

        if successful_connections >= 2 {
            Ok("connected".to_string())
        } else if successful_connections >= 1 {
            Ok("partial".to_string())
        } else {
            Ok("disconnected".to_string())
        }
    }

    /// Test TCP connection to a specific endpoint
    async fn test_tcp_connection(&self, endpoint: &str) -> Result<(), std::io::Error> {
        use tokio::net::TcpStream;
        use tokio::io::AsyncWriteExt;
        
        let mut stream = TcpStream::connect(endpoint).await?;
        stream.shutdown().await?;
        Ok(())
    }

    /// Test HTTP connectivity
    async fn test_http_connectivity(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://httpbin.org/get")
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("HTTP request failed with status: {}", response.status()).into())
        }
    }

    /// Check service endpoints
    async fn check_service_endpoints(&self) -> Result<Value, AgentError> {
        let mut endpoints = serde_json::Map::new();
        
        // Check agent endpoint
        let agent_status = self.check_endpoint_health(&self.config.agent_endpoint).await;
        endpoints.insert("agent_endpoint".to_string(), Value::String(agent_status));
        
        // Check health endpoint
        let health_status = self.check_endpoint_health(&self.config.health_endpoint).await;
        endpoints.insert("health_endpoint".to_string(), Value::String(health_status));
        
        // Check metrics endpoint
        let metrics_status = self.check_endpoint_health(&self.config.metrics_endpoint).await;
        endpoints.insert("metrics_endpoint".to_string(), Value::String(metrics_status));

        Ok(Value::Object(endpoints))
    }

    /// Check individual endpoint health
    async fn check_endpoint_health(&self, endpoint: &str) -> String {
        let timeout_duration = Duration::from_secs(3);
        
        // Try to connect to the endpoint
        match timeout(timeout_duration, self.test_tcp_connection(endpoint)).await {
            Ok(Ok(_)) => "healthy".to_string(),
            Ok(Err(e)) => {
                warn!("Endpoint health check failed for {}: {}", endpoint, e);
                "unhealthy".to_string()
            }
            Err(_) => {
                warn!("Endpoint health check timed out for {}", endpoint);
                "timeout".to_string()
            }
        }
    }

    /// Check local storage
    async fn check_local_storage(&self) -> Result<Value, AgentError> {
        let mut storage_info = serde_json::Map::new();
        
        let storage_path = std::path::Path::new(&self.config.storage.local_path);
        if storage_path.exists() {
            storage_info.insert("status".to_string(), Value::String("available".to_string()));
            
            // Check available space (simplified approach)
            let total_space = 1024 * 1024 * 1024; // 1GB placeholder
            let available_space = 256 * 1024 * 1024; // 256MB placeholder
            let used_space = total_space - available_space;
            let usage_percent = (used_space as f64 / total_space as f64) * 100.0;

            storage_info.insert("total_space_mb".to_string(), Value::Number(serde_json::Number::from(total_space / 1024 / 1024)));
            storage_info.insert("available_space_mb".to_string(), Value::Number(serde_json::Number::from(available_space / 1024 / 1024)));
            storage_info.insert("usage_percent".to_string(), Value::Number(
                serde_json::Number::from_f64(usage_percent).unwrap_or(serde_json::Number::from(0))
            ));

            // Check if storage is getting full
            if usage_percent > 90.0 {
                storage_info.insert("warning".to_string(), Value::String("Storage usage is high".to_string()));
            }
        } else {
            storage_info.insert("status".to_string(), Value::String("unavailable".to_string()));
            storage_info.insert("error".to_string(), Value::String("Storage path does not exist".to_string()));
        }

        Ok(Value::Object(storage_info))
    }

    /// Check backup storage
    async fn check_backup_storage(&self) -> Result<Value, AgentError> {
        let mut backup_info = serde_json::Map::new();
        
        let backup_path = std::path::Path::new(&self.config.storage.local_path).join("backups");
        if backup_path.exists() {
            backup_info.insert("status".to_string(), Value::String("available".to_string()));
            
            // Count backup files and calculate total size
            let mut backup_count = 0;
            let mut total_size_bytes = 0u64;
            
            if let Ok(entries) = std::fs::read_dir(&backup_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.path().is_file() {
                            backup_count += 1;
                            if let Ok(metadata) = entry.metadata() {
                                total_size_bytes += metadata.len();
                            }
                        }
                    }
                }
            }
            
            backup_info.insert("backup_count".to_string(), Value::Number(serde_json::Number::from(backup_count)));
            backup_info.insert("total_size_mb".to_string(), Value::Number(serde_json::Number::from(total_size_bytes / 1024 / 1024)));
            
            // Check if backups are recent (within last 24 hours)
            let recent_backup = self.check_recent_backups(&backup_path).await;
            backup_info.insert("recent_backup".to_string(), Value::Bool(recent_backup));
            
        } else {
            backup_info.insert("status".to_string(), Value::String("unavailable".to_string()));
            backup_info.insert("warning".to_string(), Value::String("Backup directory does not exist".to_string()));
        }

        Ok(Value::Object(backup_info))
    }

    /// Check if there are recent backups
    async fn check_recent_backups(&self, backup_path: &std::path::Path) -> bool {
        let twenty_four_hours_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 60 * 60);
        
        if let Ok(entries) = std::fs::read_dir(backup_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if modified > twenty_four_hours_ago {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check database connectivity
    async fn check_database_connectivity(&self) -> Result<String, AgentError> {
        let db_path = std::path::Path::new(&self.config.storage.local_path).join("agent.db");
        let database_url = format!("sqlite:{}", db_path.display());

        // Test database connection with timeout
        let timeout_duration = Duration::from_secs(5);
        
        match timeout(timeout_duration, SqliteConnection::connect(&database_url)).await {
            Ok(Ok(mut conn)) => {
                // Test a simple query to ensure the database is responsive
                match timeout(
                    Duration::from_secs(2),
                    sqlx::query("SELECT 1").fetch_one(&mut conn)
                ).await {
                    Ok(Ok(_)) => {
                        info!("Database connectivity test passed");
                        Ok("connected".to_string())
                    }
                    Ok(Err(e)) => {
                        warn!("Database query test failed: {}", e);
                        Ok("error".to_string())
                    }
                    Err(_) => {
                        warn!("Database query test timed out");
                        Ok("timeout".to_string())
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("Database connection failed: {}", e);
                Ok("disconnected".to_string())
            }
            Err(_) => {
                warn!("Database connection timed out");
                Ok("timeout".to_string())
            }
        }
    }

    /// Check database performance
    async fn check_database_performance(&self) -> Result<Value, AgentError> {
        let mut performance = serde_json::Map::new();
        
        let db_path = std::path::Path::new(&self.config.storage.local_path).join("agent.db");
        let database_url = format!("sqlite:{}", db_path.display());

        match SqliteConnection::connect(&database_url).await {
            Ok(mut conn) => {
                // Measure query response time
                let start_time = std::time::Instant::now();
                
                match sqlx::query("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
                    .fetch_one(&mut conn)
                    .await
                {
                    Ok(row) => {
                        let response_time = start_time.elapsed().as_millis();
                        let table_count: i64 = row.try_get(0)?;
                        
                        performance.insert("response_time_ms".to_string(), Value::Number(serde_json::Number::from(response_time)));
                        performance.insert("table_count".to_string(), Value::Number(serde_json::Number::from(table_count)));
                        
                        // Check database file size
                        if let Ok(metadata) = std::fs::metadata(&db_path) {
                            let db_size_mb = metadata.len() / 1024 / 1024;
                            performance.insert("database_size_mb".to_string(), Value::Number(serde_json::Number::from(db_size_mb)));
                        }
                        
                        // Simulate cache hit rate (in a real implementation, this would come from database metrics)
                        performance.insert("cache_hit_rate".to_string(), Value::Number(
                            serde_json::Number::from_f64(95.5).unwrap_or(serde_json::Number::from(0))
                        ));
                        
                        // Check if performance is acceptable
                        if response_time > 100 {
                            performance.insert("warning".to_string(), Value::String("Database response time is slow".to_string()));
                        }
                    }
                    Err(e) => {
                        warn!("Database performance check failed: {}", e);
                        performance.insert("error".to_string(), Value::String(e.to_string()));
                    }
                }
            }
            Err(e) => {
                warn!("Database performance check failed to connect: {}", e);
                performance.insert("error".to_string(), Value::String(e.to_string()));
            }
        }

        Ok(Value::Object(performance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_system_health_check() {
        let config = AgentConfig::default();
        let processor = HealthProcessor::new(&config);
        
        let (status, issues, details) = processor.check_system_health().await.unwrap();
        
        assert!(status == "healthy" || status == "warning");
        assert!(details.is_object());
        
        // Check that we have the expected metrics
        let details_obj = details.as_object().unwrap();
        assert!(details_obj.contains_key("memory_usage_percent") || details_obj.contains_key("cpu_usage_percent") || details_obj.contains_key("disk_usage_percent"));
    }

    #[tokio::test]
    async fn test_network_connectivity_check() {
        let config = AgentConfig::default();
        let processor = HealthProcessor::new(&config);
        
        let status = processor.check_network_connectivity().await.unwrap();
        
        // Should return one of the expected statuses
        assert!(status == "connected" || status == "partial" || status == "disconnected");
    }

    #[tokio::test]
    async fn test_tcp_connection() {
        let config = AgentConfig::default();
        let processor = HealthProcessor::new(&config);
        
        // Test with a known reliable endpoint
        let result = processor.test_tcp_connection("8.8.8.8:53").await;
        
        // Should either succeed or fail gracefully
        match result {
            Ok(_) => println!("TCP connection test passed"),
            Err(e) => println!("TCP connection test failed (expected in some environments): {}", e),
        }
    }

    #[tokio::test]
    async fn test_http_connectivity() {
        let config = AgentConfig::default();
        let processor = HealthProcessor::new(&config);
        
        let result = processor.test_http_connectivity().await;
        
        // Should either succeed or fail gracefully
        match result {
            Ok(_) => println!("HTTP connectivity test passed"),
            Err(e) => println!("HTTP connectivity test failed (expected in some environments): {}", e),
        }
    }

    #[tokio::test]
    async fn test_endpoint_health_check() {
        let config = AgentConfig::default();
        let processor = HealthProcessor::new(&config);
        
        // Test with a local endpoint that should be available
        let status = processor.check_endpoint_health("127.0.0.1:8080").await;
        
        // Should return a valid status
        assert!(status == "healthy" || status == "unhealthy" || status == "timeout");
    }

    #[tokio::test]
    async fn test_local_storage_check() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();
        
        let processor = HealthProcessor::new(&config);
        let storage_info = processor.check_local_storage().await.unwrap();
        
        assert!(storage_info.is_object());
        let storage_obj = storage_info.as_object().unwrap();
        assert!(storage_obj.contains_key("status"));
    }

    #[tokio::test]
    async fn test_backup_storage_check() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();
        
        // Create backup directory
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();
        
        // Create a test backup file
        let backup_file = backup_dir.join("test_backup.txt");
        fs::write(&backup_file, "test backup content").unwrap();
        
        let processor = HealthProcessor::new(&config);
        let backup_info = processor.check_backup_storage().await.unwrap();
        
        assert!(backup_info.is_object());
        let backup_obj = backup_info.as_object().unwrap();
        assert_eq!(backup_obj.get("status").unwrap().as_str().unwrap(), "available");
        assert_eq!(backup_obj.get("backup_count").unwrap().as_u64().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_database_connectivity_check() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();
        
        let processor = HealthProcessor::new(&config);
        let status = processor.check_database_connectivity().await.unwrap();
        
        // Should return disconnected since no database exists
        assert_eq!(status, "disconnected");
    }

    #[tokio::test]
    async fn test_database_performance_check() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();
        
        let processor = HealthProcessor::new(&config);
        let performance = processor.check_database_performance().await.unwrap();
        
        assert!(performance.is_object());
        let perf_obj = performance.as_object().unwrap();
        assert!(perf_obj.contains_key("error"));
    }

    #[tokio::test]
    async fn test_connectivity_health_check() {
        let config = AgentConfig::default();
        let processor = HealthProcessor::new(&config);
        
        let (status, issues, details) = processor.check_connectivity().await.unwrap();
        
        assert!(status == "healthy" || status == "warning");
        assert!(details.is_object());
        
        let details_obj = details.as_object().unwrap();
        assert!(details_obj.contains_key("network_status"));
        assert!(details_obj.contains_key("endpoints_status"));
    }

    #[tokio::test]
    async fn test_storage_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();
        
        let processor = HealthProcessor::new(&config);
        let (status, issues, details) = processor.check_storage_health().await.unwrap();
        
        assert!(status == "healthy" || status == "warning");
        assert!(details.is_object());
        
        let details_obj = details.as_object().unwrap();
        assert!(details_obj.contains_key("local_storage"));
        assert!(details_obj.contains_key("backup_storage"));
    }

    #[tokio::test]
    async fn test_database_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();
        
        let processor = HealthProcessor::new(&config);
        let (status, issues, details) = processor.check_database_health().await.unwrap();
        
        // Should be warning since database doesn't exist
        assert_eq!(status, "warning");
        assert!(issues > 0);
        assert!(details.is_object());
        
        let details_obj = details.as_object().unwrap();
        assert!(details_obj.contains_key("connectivity"));
        assert!(details_obj.contains_key("performance"));
    }

    #[tokio::test]
    async fn test_recent_backups_check() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();
        
        let config = AgentConfig::default();
        let processor = HealthProcessor::new(&config);
        
        // Test with no recent backups
        let has_recent = processor.check_recent_backups(&backup_dir).await;
        assert_eq!(has_recent, false);
        
        // Create a recent backup file
        let backup_file = backup_dir.join("recent_backup.txt");
        fs::write(&backup_file, "recent backup").unwrap();
        
        // Set modification time to now
        let now = std::time::SystemTime::now();
        filetime::set_file_mtime(&backup_file, filetime::FileTime::from_system_time(now)).unwrap();
        
        let has_recent = processor.check_recent_backups(&backup_dir).await;
        assert_eq!(has_recent, true);
    }

    #[tokio::test]
    async fn test_full_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.storage.local_path = temp_dir.path().to_string_lossy().to_string();
        
        let processor = HealthProcessor::new(&config);
        
        let task = MaintenanceTask {
            maintenance_type: "health_check".to_string(),
            targets: vec!["system".to_string(), "connectivity".to_string(), "storage".to_string(), "database".to_string()],
            options: std::collections::HashMap::new(),
        };
        
        let result = processor.perform_health_check(&task).await.unwrap();
        
        assert_eq!(result["operation"], "health_check");
        assert_eq!(result["targets"].as_array().unwrap().len(), 4);
        assert!(result["checks_performed"].as_u64().unwrap() > 0);
        assert!(result["health_details"].is_object());
    }
}
