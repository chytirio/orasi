//! Delta Lake exporter configuration and statistics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Delta Lake exporter configuration
#[derive(Debug, Clone)]
pub struct DeltaLakeExporterConfig {
    /// Delta table location (file:// or s3://)
    pub table_location: String,

    /// Table name
    pub table_name: String,

    /// Maximum records per file
    pub max_records_per_file: usize,

    /// Partition columns
    pub partition_columns: Vec<String>,

    /// Whether to create table if it doesn't exist
    pub create_table_if_not_exists: bool,

    /// Write mode
    pub write_mode: DeltaWriteMode,

    /// Storage options (for S3, etc.)
    pub storage_options: HashMap<String, String>,
}

/// Delta Lake write modes
#[derive(Debug, Clone)]
pub enum DeltaWriteMode {
    Append,
    Overwrite,
    ErrorIfExists,
}

impl Default for DeltaLakeExporterConfig {
    fn default() -> Self {
        Self {
            table_location: "file://./data/delta".to_string(),
            table_name: "telemetry".to_string(),
            max_records_per_file: 10000,
            partition_columns: vec!["date".to_string()],
            create_table_if_not_exists: true,
            write_mode: DeltaWriteMode::Append,
            storage_options: HashMap::new(),
        }
    }
}

/// Delta Lake exporter statistics
#[derive(Debug, Clone)]
pub struct DeltaLakeExporterStats {
    /// Total batches exported
    pub total_batches: u64,

    /// Total records exported
    pub total_records: u64,

    /// Total bytes exported
    pub total_bytes: u64,

    /// Batches exported in last minute
    pub batches_per_minute: u64,

    /// Records exported in last minute
    pub records_per_minute: u64,

    /// Average export time in milliseconds
    pub avg_export_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last export timestamp
    pub last_export_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Files written
    pub files_written: u64,
}

impl Default for DeltaLakeExporterStats {
    fn default() -> Self {
        Self {
            total_batches: 0,
            total_records: 0,
            total_bytes: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
            files_written: 0,
        }
    }
}
