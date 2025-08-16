//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Log data structures for the OpenTelemetry Data Lake Bridge
//!
//! This module provides log-specific data structures including log data,
//! levels, and batches used throughout the bridge.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Log data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogData {
    /// Log timestamp
    pub timestamp: DateTime<Utc>,

    /// Log level
    pub level: LogLevel,

    /// Log message
    pub message: String,

    /// Log attributes
    pub attributes: HashMap<String, String>,

    /// Log body
    pub body: Option<String>,

    /// Log severity number
    pub severity_number: Option<u32>,

    /// Log severity text
    pub severity_text: Option<String>,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Logs batch for lakehouse operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsBatch {
    /// Batch ID
    pub id: uuid::Uuid,

    /// Batch timestamp
    pub timestamp: DateTime<Utc>,

    /// Logs data
    pub logs: Vec<LogData>,

    /// Batch metadata
    pub metadata: HashMap<String, String>,
}
