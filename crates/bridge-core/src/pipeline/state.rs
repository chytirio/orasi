//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Pipeline state and statistics for the OpenTelemetry Data Lake Bridge
//!
//! This module provides state and statistics structures for the telemetry
//! ingestion pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::traits::{ExporterStats, ProcessorStats, ReceiverStats};

/// Pipeline state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    /// Pipeline running status
    pub running: bool,

    /// Pipeline health status
    pub healthy: bool,

    /// Pipeline error count
    pub error_count: u64,

    /// Last error timestamp
    pub last_error_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Last error message
    pub last_error_message: Option<String>,

    /// Pipeline start time
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Pipeline stop time
    pub stop_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Pipeline statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    /// Total batches processed
    pub total_batches: u64,

    /// Total records processed
    pub total_records: u64,

    /// Batches processed in last minute
    pub batches_per_minute: u64,

    /// Records processed in last minute
    pub records_per_minute: u64,

    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,

    /// Total processing time in milliseconds
    pub total_processing_time_ms: u64,

    /// Error count
    pub error_count: u64,

    /// Last processing timestamp
    pub last_process_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Receiver statistics
    pub receiver_stats: HashMap<String, ReceiverStats>,

    /// Processor statistics
    pub processor_stats: HashMap<String, ProcessorStats>,

    /// Exporter statistics
    pub exporter_stats: HashMap<String, ExporterStats>,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self {
            running: false,
            healthy: true,
            error_count: 0,
            last_error_time: None,
            last_error_message: None,
            start_time: None,
            stop_time: None,
        }
    }
}

impl Default for PipelineStats {
    fn default() -> Self {
        Self {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_processing_time_ms: 0.0,
            total_processing_time_ms: 0,
            error_count: 0,
            last_process_time: None,
            receiver_stats: HashMap::new(),
            processor_stats: HashMap::new(),
            exporter_stats: HashMap::new(),
        }
    }
}
