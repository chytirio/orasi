//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! S3/Parquet configuration
//!
//! This module provides configuration structures for S3/Parquet connector.
//!
//! NOTE: This is a simplified implementation for Phase 2 - validation
//! and advanced features will be added in Phase 3.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// S3/Parquet configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct S3ParquetConfig {
    /// S3 bucket name
    pub bucket: String,

    /// S3 prefix/path within the bucket
    pub prefix: String,

    /// AWS region
    pub region: String,

    /// AWS credentials configuration
    pub credentials: S3Credentials,

    /// Writer configuration
    pub writer_config: S3ParquetWriterConfig,

    /// Reader configuration
    pub reader_config: S3ParquetReaderConfig,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// S3 credentials configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Credentials {
    /// AWS access key ID
    pub access_key_id: Option<String>,

    /// AWS secret access key
    pub secret_access_key: Option<String>,

    /// AWS session token (for temporary credentials)
    pub session_token: Option<String>,

    /// Use IAM role (default: true)
    pub use_iam_role: bool,

    /// Profile name for AWS credentials file
    pub profile: Option<String>,
}

/// S3/Parquet writer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ParquetWriterConfig {
    /// Batch size for writing
    pub batch_size: usize,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,

    /// Compression codec
    pub compression_codec: String,

    /// Partition columns
    pub partition_columns: Vec<String>,

    /// Row group size
    pub row_group_size: usize,

    /// Max file size in bytes
    pub max_file_size_bytes: usize,

    /// Enable dictionary encoding
    pub enable_dictionary_encoding: bool,

    /// Dictionary page size limit
    pub dictionary_page_size_limit: usize,

    /// Write statistics
    pub write_statistics: bool,

    /// S3 multipart upload configuration
    pub multipart_config: S3MultipartConfig,
}

/// S3 multipart upload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3MultipartConfig {
    /// Minimum part size in bytes
    pub min_part_size: usize,

    /// Maximum part size in bytes
    pub max_part_size: usize,

    /// Maximum number of parts
    pub max_parts: usize,

    /// Upload timeout in seconds
    pub upload_timeout_secs: u64,
}

/// S3/Parquet reader configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ParquetReaderConfig {
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

    /// S3 client configuration
    pub s3_config: S3ClientConfig,
}

/// S3 client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ClientConfig {
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,

    /// Request timeout in seconds
    pub request_timeout_secs: u64,

    /// Max retries
    pub max_retries: u32,

    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,

    /// Enable request compression
    pub enable_compression: bool,

    /// Buffer size in bytes
    pub buffer_size_bytes: usize,
}

impl S3ParquetConfig {
    /// Create new S3/Parquet configuration
    pub fn new(bucket: String, prefix: String, region: String) -> Self {
        Self {
            bucket,
            prefix,
            region,
            credentials: S3Credentials::default(),
            writer_config: S3ParquetWriterConfig::default(),
            reader_config: S3ParquetReaderConfig::default(),
            metadata: HashMap::new(),
        }
    }

    /// Get bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Get prefix
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Get region
    pub fn region(&self) -> &str {
        &self.region
    }

    /// Validate configuration (simplified for Phase 2)
    pub fn validate_config(&self) -> Result<(), String> {
        if self.bucket.is_empty() {
            return Err("Bucket name cannot be empty".to_string());
        }

        if self.region.is_empty() {
            return Err("Region cannot be empty".to_string());
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

impl Default for S3Credentials {
    fn default() -> Self {
        Self {
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
            use_iam_role: true,
            profile: None,
        }
    }
}

impl Default for S3ParquetWriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 10000,
            flush_interval_ms: 5000,
            compression_codec: "snappy".to_string(),
            partition_columns: vec![
                "year".to_string(),
                "month".to_string(),
                "day".to_string(),
                "hour".to_string(),
            ],
            row_group_size: 100000,
            max_file_size_bytes: 128 * 1024 * 1024, // 128MB
            enable_dictionary_encoding: true,
            dictionary_page_size_limit: 1024 * 1024, // 1MB
            write_statistics: true,
            multipart_config: S3MultipartConfig::default(),
        }
    }
}

impl Default for S3MultipartConfig {
    fn default() -> Self {
        Self {
            min_part_size: 5 * 1024 * 1024,        // 5MB
            max_part_size: 5 * 1024 * 1024 * 1024, // 5GB
            max_parts: 10000,
            upload_timeout_secs: 3600, // 1 hour
        }
    }
}

impl Default for S3ParquetReaderConfig {
    fn default() -> Self {
        Self {
            query_timeout_secs: 300,
            max_records_per_query: 100000,
            parallel_threads: 4,
            cache_size_bytes: 1024 * 1024 * 100, // 100MB
            enable_predicate_pushdown: true,
            enable_column_pruning: true,
            s3_config: S3ClientConfig::default(),
        }
    }
}

impl Default for S3ClientConfig {
    fn default() -> Self {
        Self {
            connection_timeout_secs: 30,
            request_timeout_secs: 60,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_compression: true,
            buffer_size_bytes: 1024 * 1024, // 1MB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = S3ParquetConfig::new(
            "test-bucket".to_string(),
            "telemetry/".to_string(),
            "us-west-2".to_string(),
        );

        assert_eq!(config.bucket(), "test-bucket");
        assert_eq!(config.prefix(), "telemetry/");
        assert_eq!(config.region(), "us-west-2");
        assert_eq!(config.writer_config.batch_size, 10000);
        assert_eq!(config.reader_config.parallel_threads, 4);
    }

    #[test]
    fn test_config_validation() {
        let mut config = S3ParquetConfig::new(
            "test-bucket".to_string(),
            "telemetry/".to_string(),
            "us-west-2".to_string(),
        );

        // Valid config
        assert!(config.validate_config().is_ok());

        // Invalid bucket name
        config.bucket = "".to_string();
        assert!(config.validate_config().is_err());

        // Reset and test invalid region
        config.bucket = "test-bucket".to_string();
        config.region = "".to_string();
        assert!(config.validate_config().is_err());

        // Reset and test invalid batch size
        config.region = "us-west-2".to_string();
        config.writer_config.batch_size = 0;
        assert!(config.validate_config().is_err());
    }

    #[test]
    fn test_writer_config_default() {
        let config = S3ParquetWriterConfig::default();
        assert_eq!(config.batch_size, 10000);
        assert_eq!(config.flush_interval_ms, 5000);
        assert_eq!(config.compression_codec, "snappy");
        assert_eq!(config.row_group_size, 100000);
        assert!(config.enable_dictionary_encoding);
    }

    #[test]
    fn test_reader_config_default() {
        let config = S3ParquetReaderConfig::default();
        assert_eq!(config.query_timeout_secs, 300);
        assert_eq!(config.max_records_per_query, 100000);
        assert_eq!(config.parallel_threads, 4);
        assert!(config.enable_predicate_pushdown);
        assert!(config.enable_column_pruning);
    }

    #[test]
    fn test_credentials_default() {
        let config = S3Credentials::default();
        assert!(config.use_iam_role);
        assert!(config.access_key_id.is_none());
        assert!(config.secret_access_key.is_none());
    }
}
