//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Data source manager for the query engine
//!
//! This module provides centralized management of data sources,
//! including registration, discovery, and lifecycle management.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{DataSource, DataSourceConfig, DataSourceManager, DataSourceResult, DataSourceStats};

/// Source manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceManagerConfig {
    /// Manager name
    pub name: String,

    /// Manager version
    pub version: String,

    /// Auto-discovery enabled
    pub auto_discovery: bool,

    /// Discovery interval in seconds
    pub discovery_interval_seconds: u64,

    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,

    /// Enable debug logging
    pub debug_logging: bool,
}

impl SourceManagerConfig {
    /// Create new configuration with defaults
    pub fn new() -> Self {
        Self {
            name: "source_manager".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            auto_discovery: true,
            discovery_interval_seconds: 300,   // 5 minutes
            health_check_interval_seconds: 60, // 1 minute
            debug_logging: false,
        }
    }
}

/// Source manager implementation
pub struct SourceManagerImpl {
    config: SourceManagerConfig,
    data_source_manager: Arc<RwLock<DataSourceManager>>,
    source_registry: Arc<RwLock<HashMap<String, SourceInfo>>>,
    is_initialized: bool,
}

/// Source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Source ID
    pub id: Uuid,

    /// Source name
    pub name: String,

    /// Source type
    pub source_type: String,

    /// Source configuration
    pub config: String, // Serialized configuration

    /// Registration timestamp
    pub registered_at: DateTime<Utc>,

    /// Last health check timestamp
    pub last_health_check: Option<DateTime<Utc>>,

    /// Health status
    pub is_healthy: bool,

    /// Source metadata
    pub metadata: HashMap<String, String>,
}

impl SourceManagerImpl {
    /// Create new source manager
    pub fn new(config: SourceManagerConfig) -> Self {
        Self {
            config,
            data_source_manager: Arc::new(RwLock::new(DataSourceManager::new())),
            source_registry: Arc::new(RwLock::new(HashMap::new())),
            is_initialized: false,
        }
    }

    /// Register a data source
    pub async fn register_source(
        &self,
        name: String,
        source_type: String,
        config: String,
        metadata: HashMap<String, String>,
    ) -> BridgeResult<()> {
        info!("Registering data source: {} (type: {})", name, source_type);

        let source_info = SourceInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            source_type,
            config,
            registered_at: Utc::now(),
            last_health_check: None,
            is_healthy: false,
            metadata,
        };

        let mut registry = self.source_registry.write().await;
        registry.insert(name.clone(), source_info);

        if self.config.debug_logging {
            info!("Data source registered: {}", name);
        }

        Ok(())
    }

    /// Unregister a data source
    pub async fn unregister_source(&self, name: &str) -> BridgeResult<()> {
        info!("Unregistering data source: {}", name);

        let mut registry = self.source_registry.write().await;
        let mut data_source_manager = self.data_source_manager.write().await;

        // Remove from registry
        registry.remove(name);

        // Remove from data source manager
        data_source_manager.remove_source(name);

        if self.config.debug_logging {
            info!("Data source unregistered: {}", name);
        }

        Ok(())
    }

    /// Get source information
    pub async fn get_source_info(&self, name: &str) -> BridgeResult<Option<SourceInfo>> {
        let registry = self.source_registry.read().await;
        Ok(registry.get(name).cloned())
    }

    /// List all registered sources
    pub async fn list_sources(&self) -> BridgeResult<Vec<SourceInfo>> {
        let registry = self.source_registry.read().await;
        Ok(registry.values().cloned().collect())
    }

    /// Execute query on a specific source
    pub async fn execute_query(
        &self,
        source_name: &str,
        query: &str,
    ) -> BridgeResult<DataSourceResult> {
        let mut data_source_manager = self.data_source_manager.write().await;
        data_source_manager.execute_query(source_name, query).await
    }

    /// Get source statistics
    pub async fn get_source_stats(
        &self,
        source_name: &str,
    ) -> BridgeResult<Option<DataSourceStats>> {
        let data_source_manager = self.data_source_manager.read().await;
        let stats = data_source_manager.get_stats();
        Ok(stats.get(source_name).cloned())
    }

    /// Get all source statistics
    pub async fn get_all_source_stats(&self) -> BridgeResult<HashMap<String, DataSourceStats>> {
        let data_source_manager = self.data_source_manager.read().await;
        Ok(data_source_manager.get_stats().clone())
    }

    /// Perform health check on a source
    pub async fn health_check_source(&self, source_name: &str) -> BridgeResult<bool> {
        info!("Performing health check on source: {}", source_name);

        let mut registry = self.source_registry.write().await;
        let data_source_manager = self.data_source_manager.write().await;

        let source_info = if let Some(info) = registry.get_mut(source_name) {
            info
        } else {
            return Err(bridge_core::BridgeError::configuration(format!(
                "Source not found: {}",
                source_name
            )));
        };

        let mut is_healthy = true;
        let mut health_errors = Vec::new();

        // 1. Check if the source is accessible
        if let Some(source) = data_source_manager.get_source(source_name) {
            // 2. Verify schema information
            match source.get_schema().await {
                Ok(schema) => {
                    if self.config.debug_logging {
                        info!(
                            "Schema verification successful for {}: {} columns",
                            source_name,
                            schema.columns.len()
                        );
                    }
                }
                Err(e) => {
                    health_errors.push(format!("Schema verification failed: {}", e));
                    is_healthy = false;
                }
            }

            // 3. Test a simple query (LIMIT 1 to minimize impact)
            let test_query = match source_info.source_type.as_str() {
                "delta_lake" | "iceberg" | "s3_parquet" => "SELECT 1 as test LIMIT 1",
                _ => "SELECT 1 as test LIMIT 1", // Generic fallback
            };

            match source.execute_query(test_query).await {
                Ok(result) => {
                    if self.config.debug_logging {
                        info!(
                            "Query test successful for {}: {} rows returned",
                            source_name,
                            result.data.len()
                        );
                    }
                }
                Err(e) => {
                    health_errors.push(format!("Query test failed: {}", e));
                    is_healthy = false;
                }
            }
        } else {
            health_errors.push("Source not found in data source manager".to_string());
            is_healthy = false;
        }

        // Update health status and timestamp
        source_info.last_health_check = Some(Utc::now());
        source_info.is_healthy = is_healthy;

        // Update connection status in data source manager stats
        if let Some(stats) = data_source_manager.get_stats().get(source_name) {
            let mut updated_stats = stats.clone();
            updated_stats.is_connected = is_healthy;
            // Note: We can't directly update stats here due to borrowing rules,
            // but the actual connection status will be updated during query execution
        }

        if !is_healthy {
            warn!(
                "Health check failed for source {}: {}",
                source_name,
                health_errors.join("; ")
            );
        } else {
            info!("Health check passed for source: {}", source_name);
        }

        Ok(is_healthy)
    }

    /// Perform health check on all sources
    pub async fn health_check_all_sources(&self) -> BridgeResult<HashMap<String, bool>> {
        info!("Performing health check on all sources");

        let registry = self.source_registry.read().await;
        let source_names: Vec<String> = registry.keys().cloned().collect();

        let mut results = HashMap::new();
        for source_name in source_names {
            let is_healthy = self.health_check_source(&source_name).await?;
            results.insert(source_name, is_healthy);
        }

        Ok(results)
    }

    /// Discover new sources
    pub async fn discover_sources(&self) -> BridgeResult<Vec<String>> {
        if !self.config.auto_discovery {
            return Ok(Vec::new());
        }

        info!("Discovering new data sources");

        let mut discovered_sources = Vec::new();
        let registry = self.source_registry.read().await;
        let existing_sources: std::collections::HashSet<_> = registry.keys().collect();

        // Scan for Delta Lake tables
        if let Ok(delta_sources) = self.discover_delta_lake_sources().await {
            for source_name in delta_sources {
                if !existing_sources.contains(&source_name) {
                    discovered_sources.push(source_name);
                }
            }
        }

        // Scan for Iceberg tables
        if let Ok(iceberg_sources) = self.discover_iceberg_sources().await {
            for source_name in iceberg_sources {
                if !existing_sources.contains(&source_name) {
                    discovered_sources.push(source_name);
                }
            }
        }

        // Scan for S3 Parquet files
        if let Ok(s3_sources) = self.discover_s3_parquet_sources().await {
            for source_name in s3_sources {
                if !existing_sources.contains(&source_name) {
                    discovered_sources.push(source_name);
                }
            }
        }

        if self.config.debug_logging {
            info!("Discovered {} new sources", discovered_sources.len());
        }

        Ok(discovered_sources)
    }

    /// Auto-register discovered sources
    pub async fn auto_register_discovered_sources(&self) -> BridgeResult<Vec<String>> {
        let discovered_sources = self.discover_sources().await?;
        let mut registered_sources = Vec::new();

        for source_name in discovered_sources {
            // Determine source type from name
            let (source_type, config) = if source_name.starts_with("delta_") {
                let table_name = source_name.trim_start_matches("delta_");
                let config = serde_json::json!({
                    "name": "delta_lake",
                    "version": env!("CARGO_PKG_VERSION"),
                    "table_path": format!("s3://data-lake/delta/{}", table_name),
                    "table_name": table_name,
                    "storage_config": {},
                    "query_timeout_seconds": 300,
                    "debug_logging": false
                });
                ("delta_lake".to_string(), config.to_string())
            } else if source_name.starts_with("iceberg_") {
                let table_name = source_name.trim_start_matches("iceberg_");
                let config = serde_json::json!({
                    "name": "iceberg",
                    "version": env!("CARGO_PKG_VERSION"),
                    "table_path": format!("s3://data-lake/iceberg/{}", table_name),
                    "table_name": table_name,
                    "storage_config": {},
                    "query_timeout_seconds": 300,
                    "debug_logging": false
                });
                ("iceberg".to_string(), config.to_string())
            } else if source_name.starts_with("s3_parquet_") {
                let table_name = source_name.trim_start_matches("s3_parquet_");
                let config = serde_json::json!({
                    "name": "s3_parquet",
                    "version": env!("CARGO_PKG_VERSION"),
                    "bucket": "data-lake",
                    "prefix": format!("parquet/{}", table_name),
                    "storage_config": {},
                    "query_timeout_seconds": 300,
                    "debug_logging": false
                });
                ("s3_parquet".to_string(), config.to_string())
            } else {
                continue; // Skip unknown source types
            };

            let metadata = HashMap::new();
            if let Ok(()) = self
                .register_source(source_name.clone(), source_type, config, metadata)
                .await
            {
                registered_sources.push(source_name);
            }
        }

        if self.config.debug_logging {
            info!(
                "Auto-registered {} discovered sources",
                registered_sources.len()
            );
        }

        Ok(registered_sources)
    }

    /// Discover Delta Lake sources
    async fn discover_delta_lake_sources(&self) -> BridgeResult<Vec<String>> {
        let mut discovered = Vec::new();

        // Common Delta Lake paths to scan
        let delta_paths = vec![
            "s3://data-lake/delta/",
            "gs://data-lake/delta/",
            "abfss://container@storage.dfs.core.windows.net/delta/",
            "/data/delta/",
        ];

        for base_path in delta_paths {
            if let Ok(entries) = self.scan_directory(base_path).await {
                for entry in entries {
                    if entry.ends_with("/_delta_log") {
                        // This is a Delta Lake table
                        let table_name = entry.trim_end_matches("/_delta_log");
                        let source_name = format!(
                            "delta_{}",
                            table_name.split('/').last().unwrap_or("unknown")
                        );
                        discovered.push(source_name);
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// Discover Iceberg sources
    async fn discover_iceberg_sources(&self) -> BridgeResult<Vec<String>> {
        let mut discovered = Vec::new();

        // Common Iceberg paths to scan
        let iceberg_paths = vec![
            "s3://data-lake/iceberg/",
            "gs://data-lake/iceberg/",
            "abfss://container@storage.dfs.core.windows.net/iceberg/",
            "/data/iceberg/",
        ];

        for base_path in iceberg_paths {
            if let Ok(entries) = self.scan_directory(base_path).await {
                for entry in entries {
                    if entry.ends_with("/metadata") {
                        // This is an Iceberg table
                        let table_name = entry.trim_end_matches("/metadata");
                        let source_name = format!(
                            "iceberg_{}",
                            table_name.split('/').last().unwrap_or("unknown")
                        );
                        discovered.push(source_name);
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// Discover S3 Parquet sources
    async fn discover_s3_parquet_sources(&self) -> BridgeResult<Vec<String>> {
        let mut discovered = Vec::new();

        // Common S3 Parquet paths to scan
        let s3_paths = vec![
            "s3://data-lake/parquet/",
            "gs://data-lake/parquet/",
            "abfss://container@storage.dfs.core.windows.net/parquet/",
        ];

        for base_path in s3_paths {
            if let Ok(entries) = self.scan_directory(base_path).await {
                for entry in entries {
                    if entry.ends_with(".parquet") || entry.contains("/part-") {
                        // This is a Parquet file or partition
                        let source_name = format!(
                            "s3_parquet_{}",
                            entry
                                .split('/')
                                .last()
                                .unwrap_or("unknown")
                                .replace(".parquet", "")
                        );
                        discovered.push(source_name);
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// Scan directory for entries (mock implementation)
    async fn scan_directory(&self, path: &str) -> BridgeResult<Vec<String>> {
        // This is a mock implementation
        // In a real implementation, this would use the appropriate storage client
        // (S3, GCS, Azure Blob Storage, local filesystem) to list objects

        match path {
            "s3://data-lake/delta/" => Ok(vec![
                "s3://data-lake/delta/users/_delta_log".to_string(),
                "s3://data-lake/delta/orders/_delta_log".to_string(),
            ]),
            "gs://data-lake/delta/" => {
                Ok(vec!["gs://data-lake/delta/products/_delta_log".to_string()])
            }
            "s3://data-lake/iceberg/" => {
                Ok(vec!["s3://data-lake/iceberg/customers/metadata".to_string()])
            }
            "s3://data-lake/parquet/" => Ok(vec![
                "s3://data-lake/parquet/logs/part-00000.parquet".to_string()
            ]),
            _ => Ok(Vec::new()),
        }
    }

    /// Get source manager statistics
    pub async fn get_manager_stats(&self) -> BridgeResult<SourceManagerStats> {
        let registry = self.source_registry.read().await;
        let data_source_manager = self.data_source_manager.read().await;

        let total_sources = registry.len();
        let healthy_sources = registry.values().filter(|s| s.is_healthy).count();
        let total_queries = data_source_manager
            .get_stats()
            .values()
            .map(|s| s.total_queries)
            .sum();

        // Find the most recent health check and discovery timestamps
        let last_health_check = registry.values().filter_map(|s| s.last_health_check).max();

        let last_discovery = registry.values().map(|s| s.registered_at).max();

        Ok(SourceManagerStats {
            manager: self.config.name.clone(),
            total_sources,
            healthy_sources,
            total_queries,
            last_discovery,
            last_health_check,
        })
    }
}

/// Source manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceManagerStats {
    /// Manager name
    pub manager: String,

    /// Total number of sources
    pub total_sources: usize,

    /// Number of healthy sources
    pub healthy_sources: usize,

    /// Total queries executed
    pub total_queries: u64,

    /// Last discovery timestamp
    pub last_discovery: Option<DateTime<Utc>>,

    /// Last health check timestamp
    pub last_health_check: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_source_manager_creation() {
        let config = SourceManagerConfig::new();
        let manager = SourceManagerImpl::new(config);

        let stats = manager.get_manager_stats().await.unwrap();
        assert_eq!(stats.total_sources, 0);
        assert_eq!(stats.healthy_sources, 0);
    }

    #[tokio::test]
    async fn test_source_registration() {
        let config = SourceManagerConfig::new();
        let manager = SourceManagerImpl::new(config);

        let metadata = HashMap::new();
        let result = manager
            .register_source(
                "test-source".to_string(),
                "delta_lake".to_string(),
                "config".to_string(),
                metadata,
            )
            .await;

        assert!(result.is_ok());

        let sources = manager.list_sources().await.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "test-source");
    }

    #[tokio::test]
    async fn test_source_unregistration() {
        let config = SourceManagerConfig::new();
        let manager = SourceManagerImpl::new(config);

        // Register a source
        let metadata = HashMap::new();
        manager
            .register_source(
                "test-source".to_string(),
                "delta_lake".to_string(),
                "config".to_string(),
                metadata,
            )
            .await
            .unwrap();

        // Unregister the source
        let result = manager.unregister_source("test-source").await;
        assert!(result.is_ok());

        let sources = manager.list_sources().await.unwrap();
        assert_eq!(sources.len(), 0);
    }

    #[tokio::test]
    async fn test_source_discovery() {
        let config = SourceManagerConfig::new();
        let manager = SourceManagerImpl::new(config);

        // Test discovery functionality
        let discovered = manager.discover_sources().await.unwrap();

        // Should discover some mock sources
        assert!(!discovered.is_empty());

        // Verify discovered source names follow expected patterns
        for source_name in discovered {
            assert!(
                source_name.starts_with("delta_")
                    || source_name.starts_with("iceberg_")
                    || source_name.starts_with("s3_parquet_")
            );
        }
    }

    #[tokio::test]
    async fn test_health_check_on_nonexistent_source() {
        let config = SourceManagerConfig::new();
        let manager = SourceManagerImpl::new(config);

        // Health check on non-existent source should return an error
        let result = manager.health_check_source("nonexistent-source").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_auto_registration_disabled() {
        let mut config = SourceManagerConfig::new();
        config.auto_discovery = false;
        let manager = SourceManagerImpl::new(config);

        // With auto-discovery disabled, should return empty list
        let discovered = manager.discover_sources().await.unwrap();
        assert_eq!(discovered.len(), 0);
    }

    #[tokio::test]
    async fn test_manager_stats_with_sources() {
        let config = SourceManagerConfig::new();
        let manager = SourceManagerImpl::new(config);

        // Register a source
        let metadata = HashMap::new();
        manager
            .register_source(
                "test-source".to_string(),
                "delta_lake".to_string(),
                "config".to_string(),
                metadata,
            )
            .await
            .unwrap();

        let stats = manager.get_manager_stats().await.unwrap();
        assert_eq!(stats.total_sources, 1);
        assert_eq!(stats.healthy_sources, 0); // Not healthy until health check
        assert_eq!(stats.total_queries, 0);
    }
}
