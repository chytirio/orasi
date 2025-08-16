//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka metrics and monitoring
//! 
//! This module provides metrics collection and monitoring capabilities
//! for Kafka operations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Kafka metrics collector
#[derive(Debug, Clone)]
pub struct KafkaMetrics {
    /// Total produce operations
    total_produces: Arc<AtomicU64>,
    /// Total consume operations
    total_consumes: Arc<AtomicU64>,
    /// Total messages produced
    total_messages_produced: Arc<AtomicU64>,
    /// Total messages consumed
    total_messages_consumed: Arc<AtomicU64>,
    /// Total bytes produced
    total_bytes_produced: Arc<AtomicU64>,
    /// Total bytes consumed
    total_bytes_consumed: Arc<AtomicU64>,
    /// Produce errors
    produce_errors: Arc<AtomicU64>,
    /// Consume errors
    consume_errors: Arc<AtomicU64>,
    /// Average produce time in milliseconds
    avg_produce_time_ms: Arc<AtomicU64>,
    /// Average consume time in milliseconds
    avg_consume_time_ms: Arc<AtomicU64>,
}

impl KafkaMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            total_produces: Arc::new(AtomicU64::new(0)),
            total_consumes: Arc::new(AtomicU64::new(0)),
            total_messages_produced: Arc::new(AtomicU64::new(0)),
            total_messages_consumed: Arc::new(AtomicU64::new(0)),
            total_bytes_produced: Arc::new(AtomicU64::new(0)),
            total_bytes_consumed: Arc::new(AtomicU64::new(0)),
            produce_errors: Arc::new(AtomicU64::new(0)),
            consume_errors: Arc::new(AtomicU64::new(0)),
            avg_produce_time_ms: Arc::new(AtomicU64::new(0)),
            avg_consume_time_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a produce operation
    pub fn record_produce(&self, messages: u64, bytes: u64, time_ms: u64) {
        self.total_produces.fetch_add(1, Ordering::Relaxed);
        self.total_messages_produced.fetch_add(messages, Ordering::Relaxed);
        self.total_bytes_produced.fetch_add(bytes, Ordering::Relaxed);
        
        // Update average produce time
        let current_avg = self.avg_produce_time_ms.load(Ordering::Relaxed);
        let total_produces = self.total_produces.load(Ordering::Relaxed);
        let new_avg = if total_produces > 0 {
            ((current_avg * (total_produces - 1)) + time_ms) / total_produces
        } else {
            time_ms
        };
        self.avg_produce_time_ms.store(new_avg, Ordering::Relaxed);
    }

    /// Record a consume operation
    pub fn record_consume(&self, messages: u64, bytes: u64, time_ms: u64) {
        self.total_consumes.fetch_add(1, Ordering::Relaxed);
        self.total_messages_consumed.fetch_add(messages, Ordering::Relaxed);
        self.total_bytes_consumed.fetch_add(bytes, Ordering::Relaxed);
        
        // Update average consume time
        let current_avg = self.avg_consume_time_ms.load(Ordering::Relaxed);
        let total_consumes = self.total_consumes.load(Ordering::Relaxed);
        let new_avg = if total_consumes > 0 {
            ((current_avg * (total_consumes - 1)) + time_ms) / total_consumes
        } else {
            time_ms
        };
        self.avg_consume_time_ms.store(new_avg, Ordering::Relaxed);
    }

    /// Record a produce error
    pub fn record_produce_error(&self) {
        self.produce_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a consume error
    pub fn record_consume_error(&self) {
        self.consume_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get metrics snapshot
    pub fn get_snapshot(&self) -> KafkaMetricsSnapshot {
        KafkaMetricsSnapshot {
            total_produces: self.total_produces.load(Ordering::Relaxed),
            total_consumes: self.total_consumes.load(Ordering::Relaxed),
            total_messages_produced: self.total_messages_produced.load(Ordering::Relaxed),
            total_messages_consumed: self.total_messages_consumed.load(Ordering::Relaxed),
            total_bytes_produced: self.total_bytes_produced.load(Ordering::Relaxed),
            total_bytes_consumed: self.total_bytes_consumed.load(Ordering::Relaxed),
            produce_errors: self.produce_errors.load(Ordering::Relaxed),
            consume_errors: self.consume_errors.load(Ordering::Relaxed),
            avg_produce_time_ms: self.avg_produce_time_ms.load(Ordering::Relaxed),
            avg_consume_time_ms: self.avg_consume_time_ms.load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.total_produces.store(0, Ordering::Relaxed);
        self.total_consumes.store(0, Ordering::Relaxed);
        self.total_messages_produced.store(0, Ordering::Relaxed);
        self.total_messages_consumed.store(0, Ordering::Relaxed);
        self.total_bytes_produced.store(0, Ordering::Relaxed);
        self.total_bytes_consumed.store(0, Ordering::Relaxed);
        self.produce_errors.store(0, Ordering::Relaxed);
        self.consume_errors.store(0, Ordering::Relaxed);
        self.avg_produce_time_ms.store(0, Ordering::Relaxed);
        self.avg_consume_time_ms.store(0, Ordering::Relaxed);
    }
}

impl Default for KafkaMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Kafka metrics snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaMetricsSnapshot {
    /// Total produce operations
    pub total_produces: u64,
    /// Total consume operations
    pub total_consumes: u64,
    /// Total messages produced
    pub total_messages_produced: u64,
    /// Total messages consumed
    pub total_messages_consumed: u64,
    /// Total bytes produced
    pub total_bytes_produced: u64,
    /// Total bytes consumed
    pub total_bytes_consumed: u64,
    /// Produce errors
    pub produce_errors: u64,
    /// Consume errors
    pub consume_errors: u64,
    /// Average produce time in milliseconds
    pub avg_produce_time_ms: u64,
    /// Average consume time in milliseconds
    pub avg_consume_time_ms: u64,
}

/// Kafka metrics manager
pub struct KafkaMetricsManager {
    /// Metrics collector
    metrics: KafkaMetrics,
}

impl KafkaMetricsManager {
    /// Create a new metrics manager
    pub fn new() -> Self {
        Self {
            metrics: KafkaMetrics::new(),
        }
    }

    /// Get the metrics collector
    pub fn metrics(&self) -> &KafkaMetrics {
        &self.metrics
    }

    /// Get metrics snapshot
    pub fn get_snapshot(&self) -> KafkaMetricsSnapshot {
        self.metrics.get_snapshot()
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let snapshot = self.metrics.get_snapshot();
        
        format!(
            r#"# HELP kafka_total_produces Total number of produce operations
# TYPE kafka_total_produces counter
kafka_total_produces {total_produces}

# HELP kafka_total_consumes Total number of consume operations
# TYPE kafka_total_consumes counter
kafka_total_consumes {total_consumes}

# HELP kafka_total_messages_produced Total number of messages produced
# TYPE kafka_total_messages_produced counter
kafka_total_messages_produced {total_messages_produced}

# HELP kafka_total_messages_consumed Total number of messages consumed
# TYPE kafka_total_messages_consumed counter
kafka_total_messages_consumed {total_messages_consumed}

# HELP kafka_total_bytes_produced Total bytes produced
# TYPE kafka_total_bytes_produced counter
kafka_total_bytes_produced {total_bytes_produced}

# HELP kafka_total_bytes_consumed Total bytes consumed
# TYPE kafka_total_bytes_consumed counter
kafka_total_bytes_consumed {total_bytes_consumed}

# HELP kafka_produce_errors Total produce errors
# TYPE kafka_produce_errors counter
kafka_produce_errors {produce_errors}

# HELP kafka_consume_errors Total consume errors
# TYPE kafka_consume_errors counter
kafka_consume_errors {consume_errors}

# HELP kafka_avg_produce_time_ms Average produce time in milliseconds
# TYPE kafka_avg_produce_time_ms gauge
kafka_avg_produce_time_ms {avg_produce_time_ms}

# HELP kafka_avg_consume_time_ms Average consume time in milliseconds
# TYPE kafka_avg_consume_time_ms gauge
kafka_avg_consume_time_ms {avg_consume_time_ms}
"#,
            total_produces = snapshot.total_produces,
            total_consumes = snapshot.total_consumes,
            total_messages_produced = snapshot.total_messages_produced,
            total_messages_consumed = snapshot.total_messages_consumed,
            total_bytes_produced = snapshot.total_bytes_produced,
            total_bytes_consumed = snapshot.total_bytes_consumed,
            produce_errors = snapshot.produce_errors,
            consume_errors = snapshot.consume_errors,
            avg_produce_time_ms = snapshot.avg_produce_time_ms,
            avg_consume_time_ms = snapshot.avg_consume_time_ms,
        )
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.metrics.reset();
    }
}

impl Default for KafkaMetricsManager {
    fn default() -> Self {
        Self::new()
    }
}
