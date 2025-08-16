//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Lakehouse configuration for the OpenTelemetry Data Lake Bridge
//!
//! This module provides configuration structures for various lakehouse systems.

use serde::{Deserialize, Serialize};

use super::AuthenticationConfig;

/// Lakehouse configuration variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LakehouseConfig {
    /// Delta Lake configuration
    DeltaLake {
        /// Storage path for Delta Lake
        storage_path: String,
        /// Catalog configuration
        catalog: Option<String>,
        /// Table format version
        table_format_version: Option<u32>,
        /// Enable transaction support
        enable_transactions: bool,
        /// Partition columns
        partition_columns: Option<Vec<String>>,
        /// Compression codec
        compression: Option<String>,
    },

    /// Apache Iceberg configuration
    Iceberg {
        /// Catalog URI
        catalog_uri: String,
        /// Warehouse path
        warehouse: String,
        /// Table format version
        table_format_version: Option<u32>,
        /// Enable schema evolution
        enable_schema_evolution: bool,
        /// Partition columns
        partition_columns: Option<Vec<String>>,
        /// Compression codec
        compression: Option<String>,
    },

    /// Apache Hudi configuration
    Hudi {
        /// Storage path
        storage_path: String,
        /// Table type (COPY_ON_WRITE, MERGE_ON_READ)
        table_type: String,
        /// Enable incremental processing
        enable_incremental: bool,
        /// Partition columns
        partition_columns: Option<Vec<String>>,
        /// Compression codec
        compression: Option<String>,
    },

    /// Snowflake configuration
    Snowflake {
        /// Account identifier
        account: String,
        /// Database name
        database: String,
        /// Schema name
        schema: String,
        /// Warehouse name
        warehouse: Option<String>,
        /// Role name
        role: Option<String>,
        /// Enable bulk loading
        enable_bulk_loading: bool,
        /// Authentication configuration
        authentication: AuthenticationConfig,
    },

    /// S3/Parquet configuration
    S3Parquet {
        /// S3 bucket name
        bucket: String,
        /// S3 prefix/path
        prefix: String,
        /// AWS region
        region: String,
        /// Enable partitioning
        enable_partitioning: bool,
        /// Partition columns
        partition_columns: Option<Vec<String>>,
        /// Compression codec
        compression: Option<String>,
        /// Authentication configuration
        authentication: Option<AuthenticationConfig>,
    },
}
