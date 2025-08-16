//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration management for the Apache Iceberg connector
//!
//! This module provides type-safe configuration structures with validation
//! and hierarchical configuration management for Apache Iceberg operations.

use crate::error::{IcebergError, IcebergResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use validator::Validate;

/// Apache Iceberg configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Default)]
pub struct IcebergConfig {
    /// Storage configuration
    pub storage: IcebergStorageConfig,
    /// Table configuration
    pub table: IcebergTableConfig,
    /// Writer configuration
    pub writer: IcebergWriterConfig,
    /// Reader configuration
    pub reader: IcebergReaderConfig,
    /// Catalog configuration
    pub catalog: IcebergCatalogConfig,
    /// Schema configuration
    pub schema: IcebergSchemaConfig,
    /// Performance configuration
    pub performance: IcebergPerformanceConfig,
    /// Security configuration
    pub security: IcebergSecurityConfig,
}

/// Apache Iceberg storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IcebergStorageConfig {
    /// Storage path for Iceberg
    #[validate(length(min = 1))]
    pub storage_path: String,
    /// Storage type (local, s3, azure, gcs)
    #[validate(length(min = 1, max = 10))]
    pub storage_type: String,
    /// Storage options
    pub options: HashMap<String, String>,
}

/// Apache Iceberg table configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IcebergTableConfig {
    /// Table name
    #[validate(length(min = 1, max = 255))]
    pub table_name: String,
    /// Table format version
    #[validate(range(min = 1, max = 2))]
    pub table_format_version: u32,
    /// Partition columns
    pub partition_columns: Vec<String>,
    /// Compression codec
    #[validate(length(min = 1, max = 20))]
    pub compression: String,
    /// Table properties
    pub properties: HashMap<String, String>,
}

/// Apache Iceberg writer configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IcebergWriterConfig {
    /// Batch size for writing
    #[validate(range(min = 1, max = 100000))]
    pub batch_size: usize,
    /// Flush interval in milliseconds
    #[validate(range(min = 100, max = 60000))]
    pub flush_interval_ms: u64,
    /// Enable auto-compaction
    pub auto_compact: bool,
    /// Compaction interval in hours
    #[validate(range(min = 1, max = 168))]
    pub compact_interval_hours: u32,
}

/// Apache Iceberg reader configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IcebergReaderConfig {
    /// Read batch size
    #[validate(range(min = 1, max = 100000))]
    pub read_batch_size: usize,
    /// Enable predicate pushdown
    pub enable_predicate_pushdown: bool,
    /// Enable column pruning
    pub enable_column_pruning: bool,
    /// Enable partition pruning
    pub enable_partition_pruning: bool,
    /// Read timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub read_timeout_secs: u64,
}

/// Apache Iceberg catalog configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IcebergCatalogConfig {
    /// Catalog type (hive, nessie, custom)
    #[validate(length(min = 1, max = 20))]
    pub catalog_type: String,
    /// Catalog URI
    pub catalog_uri: Option<String>,
    /// Warehouse location
    #[validate(length(min = 1))]
    pub warehouse_location: String,
    /// Catalog properties
    pub properties: HashMap<String, String>,
}

/// Apache Iceberg schema configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IcebergSchemaConfig {
    /// Enable schema evolution
    pub enable_schema_evolution: bool,
    /// Schema validation mode (strict, lenient, none)
    #[validate(length(min = 1, max = 20))]
    pub validation_mode: String,
    /// Enable column mapping
    pub enable_column_mapping: bool,
    /// Column mapping mode (name, id)
    #[validate(length(min = 1, max = 10))]
    pub column_mapping_mode: String,
}

/// Apache Iceberg performance configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IcebergPerformanceConfig {
    /// Enable parallel processing
    pub enable_parallel_processing: bool,
    /// Number of parallel threads
    #[validate(range(min = 1, max = 64))]
    pub parallel_threads: usize,
    /// Enable memory optimization
    pub enable_memory_optimization: bool,
    /// Memory limit in MB
    #[validate(range(min = 100, max = 100000))]
    pub memory_limit_mb: usize,
}

/// Apache Iceberg security configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IcebergSecurityConfig {
    /// Enable encryption at rest
    pub enable_encryption_at_rest: bool,
    /// Encryption algorithm
    pub encryption_algorithm: Option<String>,
    /// Enable access control
    pub enable_access_control: bool,
    /// Access control mode (ranger, unity, custom)
    pub access_control_mode: Option<String>,
}

impl IcebergConfig {
    /// Create a new Apache Iceberg configuration with defaults
    pub fn new(storage_path: String, table_name: String) -> Self {
        Self {
            storage: IcebergStorageConfig {
                storage_path,
                ..Default::default()
            },
            table: IcebergTableConfig {
                table_name,
                ..Default::default()
            },
            writer: IcebergWriterConfig::default(),
            reader: IcebergReaderConfig::default(),
            catalog: IcebergCatalogConfig::default(),
            schema: IcebergSchemaConfig::default(),
            performance: IcebergPerformanceConfig::default(),
            security: IcebergSecurityConfig::default(),
        }
    }

    /// Load configuration from file
    pub fn from_file(path: &std::path::Path) -> IcebergResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            IcebergError::configuration_with_source("Failed to read config file", e)
        })?;

        Self::from_str(&content)
    }

    /// Load configuration from string
    pub fn from_str(content: &str) -> IcebergResult<Self> {
        let config: IcebergConfig = toml::from_str(content)
            .map_err(|e| IcebergError::configuration_with_source("Failed to parse config", e))?;

        config.validate_config()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate_config(&self) -> IcebergResult<()> {
        self.validate().map_err(|e| {
            IcebergError::validation_with_source("Configuration validation failed", e)
        })?;

        // Additional custom validation
        if self.table.partition_columns.is_empty() {
            return Err(IcebergError::validation(
                "At least one partition column is required",
            ));
        }

        if self.writer.batch_size == 0 {
            return Err(IcebergError::validation(
                "Batch size must be greater than 0",
            ));
        }

        Ok(())
    }

    /// Get storage path
    pub fn storage_path(&self) -> &str {
        &self.storage.storage_path
    }

    /// Get table name
    pub fn table_name(&self) -> &str {
        &self.table.table_name
    }

    /// Get batch size
    pub fn batch_size(&self) -> usize {
        self.writer.batch_size
    }

    /// Get flush interval
    pub fn flush_interval(&self) -> Duration {
        Duration::from_millis(self.writer.flush_interval_ms)
    }
}

impl Default for IcebergStorageConfig {
    fn default() -> Self {
        Self {
            storage_path: "s3://iceberg-tables".to_string(),
            storage_type: "s3".to_string(),
            options: HashMap::new(),
        }
    }
}

impl Default for IcebergTableConfig {
    fn default() -> Self {
        Self {
            table_name: "telemetry_data".to_string(),
            table_format_version: 2,
            partition_columns: vec!["year".to_string(), "month".to_string(), "day".to_string()],
            compression: "snappy".to_string(),
            properties: HashMap::new(),
        }
    }
}

impl Default for IcebergWriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 10000,
            flush_interval_ms: 5000,
            auto_compact: true,
            compact_interval_hours: 24,
        }
    }
}

impl Default for IcebergReaderConfig {
    fn default() -> Self {
        Self {
            read_batch_size: 10000,
            enable_predicate_pushdown: true,
            enable_column_pruning: true,
            enable_partition_pruning: true,
            read_timeout_secs: 300,
        }
    }
}

impl Default for IcebergCatalogConfig {
    fn default() -> Self {
        Self {
            catalog_type: "hive".to_string(),
            catalog_uri: None,
            warehouse_location: "s3://iceberg-warehouse".to_string(),
            properties: HashMap::new(),
        }
    }
}

impl Default for IcebergSchemaConfig {
    fn default() -> Self {
        Self {
            enable_schema_evolution: true,
            validation_mode: "lenient".to_string(),
            enable_column_mapping: false,
            column_mapping_mode: "name".to_string(),
        }
    }
}

impl Default for IcebergPerformanceConfig {
    fn default() -> Self {
        Self {
            enable_parallel_processing: true,
            parallel_threads: num_cpus::get(),
            enable_memory_optimization: true,
            memory_limit_mb: 1000,
        }
    }
}

impl Default for IcebergSecurityConfig {
    fn default() -> Self {
        Self {
            enable_encryption_at_rest: false,
            encryption_algorithm: None,
            enable_access_control: false,
            access_control_mode: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = IcebergConfig::new("s3://test-bucket".to_string(), "test_table".to_string());

        assert_eq!(config.storage_path(), "s3://test-bucket");
        assert_eq!(config.table_name(), "test_table");
        assert_eq!(config.batch_size(), 10000);
    }

    #[test]
    fn test_config_validation() {
        let mut config =
            IcebergConfig::new("s3://test-bucket".to_string(), "test_table".to_string());

        // Should be valid
        assert!(config.validate_config().is_ok());

        // Should be invalid with empty partition columns
        config.table.partition_columns.clear();
        assert!(config.validate_config().is_err());
    }
}
