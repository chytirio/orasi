//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Trace data structures for the OpenTelemetry Data Lake Bridge
//!
//! This module provides trace-specific data structures including trace data,
//! spans, events, and batches used throughout the bridge.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trace data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceData {
    /// Trace ID
    pub trace_id: String,

    /// Span ID
    pub span_id: String,

    /// Parent span ID
    pub parent_span_id: Option<String>,

    /// Span name
    pub name: String,

    /// Span kind
    pub kind: SpanKind,

    /// Span start time
    pub start_time: DateTime<Utc>,

    /// Span end time
    pub end_time: Option<DateTime<Utc>>,

    /// Span duration in nanoseconds
    pub duration_ns: Option<u64>,

    /// Span status
    pub status: SpanStatus,

    /// Span attributes
    pub attributes: HashMap<String, String>,

    /// Span events
    pub events: Vec<SpanEvent>,

    /// Span links
    pub links: Vec<SpanLink>,
}

/// Span kinds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanKind {
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

/// Span status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanStatus {
    /// Status code
    pub code: StatusCode,

    /// Status message
    pub message: Option<String>,
}

/// Status codes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusCode {
    Ok,
    Error,
    Unset,
}

/// Span event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name
    pub name: String,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Event attributes
    pub attributes: HashMap<String, String>,
}

/// Span link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLink {
    /// Linked trace ID
    pub trace_id: String,

    /// Linked span ID
    pub span_id: String,

    /// Link attributes
    pub attributes: HashMap<String, String>,
}

/// Traces batch for lakehouse operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracesBatch {
    /// Batch ID
    pub id: uuid::Uuid,

    /// Batch timestamp
    pub timestamp: DateTime<Utc>,

    /// Traces data
    pub traces: Vec<TraceData>,

    /// Batch metadata
    pub metadata: HashMap<String, String>,
}
