//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake configuration
//! 
//! This module provides configuration structures for Delta Lake connector.
//! 
//! NOTE: This is a simplified implementation for Phase 2 - validation
//! and advanced features will be added in Phase 3.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Delta Lake configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaLakeConfig {
    /// Storage path (e.g., "s3://bucket/path" or "/local/path")
    pub storage_path: String,
    
    /// Table name
    pub table_name: String,
    
    /// Writer configuration
    pub writer_config: DeltaLakeWriterConfig,
    
    /// Reader configuration
    pub reader_config: DeltaLakeReaderConfig,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Delta Lake writer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaLakeWriterConfig {
    /// Batch size for writing
    pub batch_size: usize,
    
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
    
    /// Compression codec
    pub compression_codec: String,
    
    /// Partition columns
    pub partition_columns: Vec<String>,
    
    /// Table format version
    pub table_format_version: u32,
    
    /// Transaction timeout in seconds
    pub transaction_timeout_secs: u64,
    
    /// Vacuum retention hours
    pub vacuum_retention_hours: u64,
}

/// Delta Lake reader configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaLakeReaderConfig {
    /// Query timeout in seconds
    pub query_timeout_secs: u64,
    
    /// Max records per query
    pub max_records_per_query: usize,
    
    /// Parallel threads for reading
    pub parallel_threads: usize,
    
    /// Cache size in bytes
    pub cache_size_bytes: usize,
    
    /// Enable predicate pushdown
    pub enable_predicate_pushdown: bool,
    
    /// Enable column pruning
    pub enable_column_pruning: bool,
}

impl DeltaLakeConfig {
    /// Create new Delta Lake configuration
    pub fn new(storage_path: String, table_name: String) -> Self {
        Self {
            storage_path,
            table_name,
            writer_config: DeltaLakeWriterConfig::default(),
            reader_config: DeltaLakeReaderConfig::default(),
            metadata: HashMap::new(),
        }
    }
    
    /// Get table name
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
    
    /// Get storage path
    pub fn storage_path(&self) -> &str {
        &self.storage_path
    }
    
    /// Validate configuration (simplified for Phase 2)
    pub fn validate_config(&self) -> Result<(), String> {
        if self.storage_path.is_empty() {
            return Err("Storage path cannot be empty".to_string());
        }
        
        if self.table_name.is_empty() {
            return Err("Table name cannot be empty".to_string());
        }
        
        if self.writer_config.batch_size == 0 {
            return Err("Batch size must be greater than 0".to_string());
        }
        
        if self.writer_config.flush_interval_ms == 0 {
            return Err("Flush interval must be greater than 0".to_string());
        }
        
        Ok(())
    }
}

impl Default for DeltaLakeWriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 10000,
            flush_interval_ms: 5000,
            compression_codec: "snappy".to_string(),
            partition_columns: vec!["year".to_string(), "month".to_string(), "day".to_string(), "hour".to_string()],
            table_format_version: 2,
            transaction_timeout_secs: 300,
            vacuum_retention_hours: 168, // 7 days
        }
    }
}

impl Default for DeltaLakeReaderConfig {
    fn default() -> Self {
        Self {
            query_timeout_secs: 300,
            max_records_per_query: 100000,
            parallel_threads: 4,
            cache_size_bytes: 1024 * 1024 * 100, // 100MB
            enable_predicate_pushdown: true,
            enable_column_pruning: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = DeltaLakeConfig::new(
            "s3://test-bucket".to_string(),
            "test_table".to_string(),
        );
        
        assert_eq!(config.table_name(), "test_table");
        assert_eq!(config.storage_path(), "s3://test-bucket");
        assert_eq!(config.writer_config.batch_size, 10000);
        assert_eq!(config.reader_config.parallel_threads, 4);
    }

    #[test]
    fn test_config_validation() {
        let mut config = DeltaLakeConfig::new(
            "s3://test-bucket".to_string(),
            "test_table".to_string(),
        );
        
        // Valid config
        assert!(config.validate_config().is_ok());
        
        // Invalid storage path
        config.storage_path = "".to_string();
        assert!(config.validate_config().is_err());
        
        // Reset and test invalid table name
        config.storage_path = "s3://test-bucket".to_string();
        config.table_name = "".to_string();
        assert!(config.validate_config().is_err());
        
        // Reset and test invalid batch size
        config.table_name = "test_table".to_string();
        config.writer_config.batch_size = 0;
        assert!(config.validate_config().is_err());
    }

    #[test]
    fn test_writer_config_default() {
        let config = DeltaLakeWriterConfig::default();
        assert_eq!(config.batch_size, 10000);
        assert_eq!(config.flush_interval_ms, 5000);
        assert_eq!(config.compression_codec, "snappy");
        assert_eq!(config.table_format_version, 2);
    }

    #[test]
    fn test_reader_config_default() {
        let config = DeltaLakeReaderConfig::default();
        assert_eq!(config.query_timeout_secs, 300);
        assert_eq!(config.max_records_per_query, 100000);
        assert_eq!(config.parallel_threads, 4);
        assert!(config.enable_predicate_pushdown);
        assert!(config.enable_column_pruning);
    }
}
