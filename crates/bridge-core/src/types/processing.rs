//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Processing result data structures for the OpenTelemetry Data Lake Bridge
//!
//! This module provides processing result data structures including processed
//! batches, records, and export results used throughout the bridge.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::TelemetryData;

/// Processed batch with transformation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedBatch {
    /// Original batch ID
    pub original_batch_id: Uuid,

    /// Processing timestamp
    pub timestamp: DateTime<Utc>,

    /// Processing status
    pub status: ProcessingStatus,

    /// Processed records
    pub records: Vec<ProcessedRecord>,

    /// Processing metadata
    pub metadata: HashMap<String, String>,

    /// Processing errors
    pub errors: Vec<ProcessingError>,
}

/// Processing status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingStatus {
    Success,
    Partial,
    Failed,
}

/// Processed record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedRecord {
    /// Original record ID
    pub original_id: Uuid,

    /// Processing status
    pub status: ProcessingStatus,

    /// Transformed data
    pub transformed_data: Option<TelemetryData>,

    /// Processing metadata
    pub metadata: HashMap<String, String>,

    /// Processing errors
    pub errors: Vec<ProcessingError>,
}

/// Processing error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error details
    pub details: Option<serde_json::Value>,
}

/// Export result from lakehouse operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// Export timestamp
    pub timestamp: DateTime<Utc>,

    /// Export status
    pub status: ExportStatus,

    /// Number of records exported
    pub records_exported: usize,

    /// Number of records failed
    pub records_failed: usize,

    /// Export duration in milliseconds
    pub duration_ms: u64,

    /// Export metadata
    pub metadata: HashMap<String, String>,

    /// Export errors
    pub errors: Vec<ExportError>,
}

/// Export status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportStatus {
    Success,
    Partial,
    Failed,
}

/// Export error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error details
    pub details: Option<serde_json::Value>,
}

/// Write result from lakehouse write operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    /// Write timestamp
    pub timestamp: DateTime<Utc>,

    /// Write status
    pub status: WriteStatus,

    /// Number of records written
    pub records_written: usize,

    /// Number of records failed
    pub records_failed: usize,

    /// Write duration in milliseconds
    pub duration_ms: u64,

    /// Write metadata
    pub metadata: HashMap<String, String>,

    /// Write errors
    pub errors: Vec<WriteError>,
}

/// Write status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WriteStatus {
    Success,
    Partial,
    Failed,
}

/// Write error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error details
    pub details: Option<serde_json::Value>,
}
