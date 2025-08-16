//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Hudi metrics and monitoring
//!
//! This module provides metrics collection and monitoring capabilities
//! for Apache Hudi operations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Apache Hudi metrics collector
#[derive(Debug, Clone)]
pub struct HudiMetrics {
    /// Total write operations
    total_writes: Arc<AtomicU64>,
    /// Total read operations
    total_reads: Arc<AtomicU64>,
    /// Total records written
    total_records_written: Arc<AtomicU64>,
    /// Total records read
    total_records_read: Arc<AtomicU64>,
    /// Total bytes written
    total_bytes_written: Arc<AtomicU64>,
    /// Total bytes read
    total_bytes_read: Arc<AtomicU64>,
    /// Write errors
    write_errors: Arc<AtomicU64>,
    /// Read errors
    read_errors: Arc<AtomicU64>,
    /// Average write time in milliseconds
    avg_write_time_ms: Arc<AtomicU64>,
    /// Average read time in milliseconds
    avg_read_time_ms: Arc<AtomicU64>,
}

impl HudiMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            total_writes: Arc::new(AtomicU64::new(0)),
            total_reads: Arc::new(AtomicU64::new(0)),
            total_records_written: Arc::new(AtomicU64::new(0)),
            total_records_read: Arc::new(AtomicU64::new(0)),
            total_bytes_written: Arc::new(AtomicU64::new(0)),
            total_bytes_read: Arc::new(AtomicU64::new(0)),
            write_errors: Arc::new(AtomicU64::new(0)),
            read_errors: Arc::new(AtomicU64::new(0)),
            avg_write_time_ms: Arc::new(AtomicU64::new(0)),
            avg_read_time_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a write operation
    pub fn record_write(&self, records: u64, bytes: u64, time_ms: u64) {
        self.total_writes.fetch_add(1, Ordering::Relaxed);
        self.total_records_written
            .fetch_add(records, Ordering::Relaxed);
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);

        // Update average write time
        let current_avg = self.avg_write_time_ms.load(Ordering::Relaxed);
        let total_writes = self.total_writes.load(Ordering::Relaxed);
        let new_avg = if total_writes > 0 {
            ((current_avg * (total_writes - 1)) + time_ms) / total_writes
        } else {
            time_ms
        };
        self.avg_write_time_ms.store(new_avg, Ordering::Relaxed);
    }

    /// Record a read operation
    pub fn record_read(&self, records: u64, bytes: u64, time_ms: u64) {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        self.total_records_read
            .fetch_add(records, Ordering::Relaxed);
        self.total_bytes_read.fetch_add(bytes, Ordering::Relaxed);

        // Update average read time
        let current_avg = self.avg_read_time_ms.load(Ordering::Relaxed);
        let total_reads = self.total_reads.load(Ordering::Relaxed);
        let new_avg = if total_reads > 0 {
            ((current_avg * (total_reads - 1)) + time_ms) / total_reads
        } else {
            time_ms
        };
        self.avg_read_time_ms.store(new_avg, Ordering::Relaxed);
    }

    /// Record a write error
    pub fn record_write_error(&self) {
        self.write_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a read error
    pub fn record_read_error(&self) {
        self.read_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get metrics snapshot
    pub fn get_snapshot(&self) -> HudiMetricsSnapshot {
        HudiMetricsSnapshot {
            total_writes: self.total_writes.load(Ordering::Relaxed),
            total_reads: self.total_reads.load(Ordering::Relaxed),
            total_records_written: self.total_records_written.load(Ordering::Relaxed),
            total_records_read: self.total_records_read.load(Ordering::Relaxed),
            total_bytes_written: self.total_bytes_written.load(Ordering::Relaxed),
            total_bytes_read: self.total_bytes_read.load(Ordering::Relaxed),
            write_errors: self.write_errors.load(Ordering::Relaxed),
            read_errors: self.read_errors.load(Ordering::Relaxed),
            avg_write_time_ms: self.avg_write_time_ms.load(Ordering::Relaxed),
            avg_read_time_ms: self.avg_read_time_ms.load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.total_writes.store(0, Ordering::Relaxed);
        self.total_reads.store(0, Ordering::Relaxed);
        self.total_records_written.store(0, Ordering::Relaxed);
        self.total_records_read.store(0, Ordering::Relaxed);
        self.total_bytes_written.store(0, Ordering::Relaxed);
        self.total_bytes_read.store(0, Ordering::Relaxed);
        self.write_errors.store(0, Ordering::Relaxed);
        self.read_errors.store(0, Ordering::Relaxed);
        self.avg_write_time_ms.store(0, Ordering::Relaxed);
        self.avg_read_time_ms.store(0, Ordering::Relaxed);
    }
}

impl Default for HudiMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Apache Hudi metrics snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HudiMetricsSnapshot {
    /// Total write operations
    pub total_writes: u64,
    /// Total read operations
    pub total_reads: u64,
    /// Total records written
    pub total_records_written: u64,
    /// Total records read
    pub total_records_read: u64,
    /// Total bytes written
    pub total_bytes_written: u64,
    /// Total bytes read
    pub total_bytes_read: u64,
    /// Write errors
    pub write_errors: u64,
    /// Read errors
    pub read_errors: u64,
    /// Average write time in milliseconds
    pub avg_write_time_ms: u64,
    /// Average read time in milliseconds
    pub avg_read_time_ms: u64,
}

/// Apache Hudi metrics manager
pub struct HudiMetricsManager {
    /// Metrics collector
    metrics: HudiMetrics,
}

impl HudiMetricsManager {
    /// Create a new metrics manager
    pub fn new() -> Self {
        Self {
            metrics: HudiMetrics::new(),
        }
    }

    /// Get the metrics collector
    pub fn metrics(&self) -> &HudiMetrics {
        &self.metrics
    }

    /// Get metrics snapshot
    pub fn get_snapshot(&self) -> HudiMetricsSnapshot {
        self.metrics.get_snapshot()
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let snapshot = self.metrics.get_snapshot();

        format!(
            r#"# HELP hudi_total_writes Total number of write operations
# TYPE hudi_total_writes counter
hudi_total_writes {total_writes}

# HELP hudi_total_reads Total number of read operations
# TYPE hudi_total_reads counter
hudi_total_reads {total_reads}

# HELP hudi_total_records_written Total number of records written
# TYPE hudi_total_records_written counter
hudi_total_records_written {total_records_written}

# HELP hudi_total_records_read Total number of records read
# TYPE hudi_total_records_read counter
hudi_total_records_read {total_records_read}

# HELP hudi_total_bytes_written Total bytes written
# TYPE hudi_total_bytes_written counter
hudi_total_bytes_written {total_bytes_written}

# HELP hudi_total_bytes_read Total bytes read
# TYPE hudi_total_bytes_read counter
hudi_total_bytes_read {total_bytes_read}

# HELP hudi_write_errors Total write errors
# TYPE hudi_write_errors counter
hudi_write_errors {write_errors}

# HELP hudi_read_errors Total read errors
# TYPE hudi_read_errors counter
hudi_read_errors {read_errors}

# HELP hudi_avg_write_time_ms Average write time in milliseconds
# TYPE hudi_avg_write_time_ms gauge
hudi_avg_write_time_ms {avg_write_time_ms}

# HELP hudi_avg_read_time_ms Average read time in milliseconds
# TYPE hudi_avg_read_time_ms gauge
hudi_avg_read_time_ms {avg_read_time_ms}
"#,
            total_writes = snapshot.total_writes,
            total_reads = snapshot.total_reads,
            total_records_written = snapshot.total_records_written,
            total_records_read = snapshot.total_records_read,
            total_bytes_written = snapshot.total_bytes_written,
            total_bytes_read = snapshot.total_bytes_read,
            write_errors = snapshot.write_errors,
            read_errors = snapshot.read_errors,
            avg_write_time_ms = snapshot.avg_write_time_ms,
            avg_read_time_ms = snapshot.avg_read_time_ms,
        )
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.metrics.reset();
    }
}

impl Default for HudiMetricsManager {
    fn default() -> Self {
        Self::new()
    }
}
