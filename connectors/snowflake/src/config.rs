//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration management for the Snowflake connector
//!
//! This module provides type-safe configuration structures with validation
//! and hierarchical configuration management for Snowflake operations.

use crate::error::{SnowflakeError, SnowflakeResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use validator::Validate;

/// Snowflake configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnowflakeConfig {
    /// Connection configuration
    pub connection: SnowflakeConnectionConfig,
    /// Warehouse configuration
    pub warehouse: SnowflakeWarehouseConfig,
    /// Database configuration
    pub database: SnowflakeDatabaseConfig,
    /// Writer configuration
    pub writer: SnowflakeWriterConfig,
    /// Reader configuration
    pub reader: SnowflakeReaderConfig,
    /// Schema configuration
    pub schema: SnowflakeSchemaConfig,
    /// Performance configuration
    pub performance: SnowflakePerformanceConfig,
    /// Security configuration
    pub security: SnowflakeSecurityConfig,
}

/// Snowflake connection configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnowflakeConnectionConfig {
    /// Account identifier
    #[validate(length(min = 1))]
    pub account: String,
    /// Username
    #[validate(length(min = 1))]
    pub username: String,
    /// Password (should be encrypted in production)
    #[validate(length(min = 1))]
    pub password: String,
    /// Role
    pub role: Option<String>,
    /// Warehouse name
    #[validate(length(min = 1))]
    pub warehouse: String,
    /// Database name
    #[validate(length(min = 1))]
    pub database: String,
    /// Schema name
    #[validate(length(min = 1))]
    pub schema: String,
    /// Connection timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub connection_timeout_secs: u64,
    /// Query timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub query_timeout_secs: u64,
}

/// Snowflake warehouse configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnowflakeWarehouseConfig {
    /// Warehouse name
    #[validate(length(min = 1))]
    pub warehouse_name: String,
    /// Warehouse size
    #[validate(length(min = 1, max = 20))]
    pub warehouse_size: String,
    /// Auto-suspend in seconds
    #[validate(range(min = 0, max = 3600))]
    pub auto_suspend_secs: u64,
    /// Auto-resume
    pub auto_resume: bool,
    /// Warehouse properties
    pub properties: HashMap<String, String>,
}

/// Snowflake database configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnowflakeDatabaseConfig {
    /// Database name
    #[validate(length(min = 1))]
    pub database_name: String,
    /// Schema name
    #[validate(length(min = 1))]
    pub schema_name: String,
    /// Table format
    #[validate(length(min = 1, max = 20))]
    pub table_format: String,
    /// Copy options
    pub copy_options: String,
}

/// Snowflake writer configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnowflakeWriterConfig {
    /// Batch size for writing
    #[validate(range(min = 1, max = 100000))]
    pub batch_size: usize,
    /// Flush interval in milliseconds
    #[validate(range(min = 100, max = 60000))]
    pub flush_interval_ms: u64,
    /// Enable auto-scaling
    pub auto_scaling: bool,
    /// Enable clustering
    pub enable_clustering: bool,
    /// Clustering keys
    pub clustering_keys: Vec<String>,
}

/// Snowflake reader configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnowflakeReaderConfig {
    /// Read batch size
    #[validate(range(min = 1, max = 100000))]
    pub read_batch_size: usize,
    /// Enable result caching
    pub enable_result_caching: bool,
    /// Cache timeout in seconds
    #[validate(range(min = 0, max = 3600))]
    pub cache_timeout_secs: u64,
    /// Enable query acceleration
    pub enable_query_acceleration: bool,
    /// Read timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub read_timeout_secs: u64,
}

/// Snowflake schema configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnowflakeSchemaConfig {
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

/// Snowflake performance configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnowflakePerformanceConfig {
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

/// Snowflake security configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnowflakeSecurityConfig {
    /// Enable encryption at rest
    pub enable_encryption_at_rest: bool,
    /// Enable encryption in transit
    pub enable_encryption_in_transit: bool,
    /// Enable access control
    pub enable_access_control: bool,
    /// Access control mode (rbac, abac, custom)
    pub access_control_mode: Option<String>,
}

impl SnowflakeConfig {
    /// Create a new Snowflake configuration with defaults
    pub fn new(account: String, username: String, password: String) -> Self {
        Self {
            connection: SnowflakeConnectionConfig {
                account,
                username,
                password,
                ..Default::default()
            },
            warehouse: SnowflakeWarehouseConfig::default(),
            database: SnowflakeDatabaseConfig::default(),
            writer: SnowflakeWriterConfig::default(),
            reader: SnowflakeReaderConfig::default(),
            schema: SnowflakeSchemaConfig::default(),
            performance: SnowflakePerformanceConfig::default(),
            security: SnowflakeSecurityConfig::default(),
        }
    }

    /// Load configuration from file
    pub fn from_file(path: &std::path::Path) -> SnowflakeResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SnowflakeError::configuration_with_source("Failed to read config file", e)
        })?;

        Self::from_str(&content)
    }

    /// Load configuration from string
    pub fn from_str(content: &str) -> SnowflakeResult<Self> {
        let config: SnowflakeConfig = toml::from_str(content)
            .map_err(|e| SnowflakeError::configuration_with_source("Failed to parse config", e))?;

        config.validate_config()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate_config(&self) -> SnowflakeResult<()> {
        self.validate().map_err(|e| {
            SnowflakeError::validation_with_source("Configuration validation failed", e)
        })?;

        // Additional custom validation
        if self.writer.batch_size == 0 {
            return Err(SnowflakeError::validation(
                "Batch size must be greater than 0",
            ));
        }

        Ok(())
    }

    /// Get account identifier
    pub fn account(&self) -> &str {
        &self.connection.account
    }

    /// Get username
    pub fn username(&self) -> &str {
        &self.connection.username
    }

    /// Get database name
    pub fn database_name(&self) -> &str {
        &self.connection.database
    }

    /// Get database name (alias for database_name)
    pub fn database(&self) -> &str {
        &self.connection.database
    }

    /// Get schema name
    pub fn schema_name(&self) -> &str {
        &self.connection.schema
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

impl Default for SnowflakeConnectionConfig {
    fn default() -> Self {
        Self {
            account: "your-account".to_string(),
            username: "your-username".to_string(),
            password: "your-password".to_string(),
            role: None,
            warehouse: "COMPUTE_WH".to_string(),
            database: "OPENTELEMETRY".to_string(),
            schema: "PUBLIC".to_string(),
            connection_timeout_secs: 30,
            query_timeout_secs: 300,
        }
    }
}

impl Default for SnowflakeWarehouseConfig {
    fn default() -> Self {
        Self {
            warehouse_name: "COMPUTE_WH".to_string(),
            warehouse_size: "X-SMALL".to_string(),
            auto_suspend_secs: 300,
            auto_resume: true,
            properties: HashMap::new(),
        }
    }
}

impl Default for SnowflakeDatabaseConfig {
    fn default() -> Self {
        Self {
            database_name: "OPENTELEMETRY".to_string(),
            schema_name: "PUBLIC".to_string(),
            table_format: "PARQUET".to_string(),
            copy_options: "FILE_FORMAT = (TYPE = 'PARQUET')".to_string(),
        }
    }
}

impl Default for SnowflakeWriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 10000,
            flush_interval_ms: 5000,
            auto_scaling: true,
            enable_clustering: false,
            clustering_keys: Vec::new(),
        }
    }
}

impl Default for SnowflakeReaderConfig {
    fn default() -> Self {
        Self {
            read_batch_size: 10000,
            enable_result_caching: true,
            cache_timeout_secs: 300,
            enable_query_acceleration: false,
            read_timeout_secs: 300,
        }
    }
}

impl Default for SnowflakeSchemaConfig {
    fn default() -> Self {
        Self {
            enable_schema_evolution: true,
            validation_mode: "lenient".to_string(),
            enable_column_mapping: false,
            column_mapping_mode: "name".to_string(),
        }
    }
}

impl Default for SnowflakePerformanceConfig {
    fn default() -> Self {
        Self {
            enable_parallel_processing: true,
            parallel_threads: num_cpus::get(),
            enable_memory_optimization: true,
            memory_limit_mb: 1000,
        }
    }
}

impl Default for SnowflakeSecurityConfig {
    fn default() -> Self {
        Self {
            enable_encryption_at_rest: true,
            enable_encryption_in_transit: true,
            enable_access_control: true,
            access_control_mode: Some("rbac".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = SnowflakeConfig::new(
            "test-account".to_string(),
            "test-user".to_string(),
            "test-password".to_string(),
        );

        assert_eq!(config.account(), "test-account");
        assert_eq!(config.username(), "test-user");
        assert_eq!(config.batch_size(), 10000);
    }

    #[test]
    fn test_config_validation() {
        let config = SnowflakeConfig::new(
            "test-account".to_string(),
            "test-user".to_string(),
            "test-password".to_string(),
        );

        // Should be valid
        assert!(config.validate_config().is_ok());
    }
}
