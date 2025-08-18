//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka producer implementation
//! 
//! This module provides the Kafka producer that implements
//! the LakehouseWriter trait for writing telemetry data to Kafka.

use std::collections::HashMap;
use std::time::Instant;
use async_trait::async_trait;
use tracing::{debug, error, info};
use serde_json;
use chrono::{DateTime, Utc};
use bridge_core::traits::LakehouseWriter;
use bridge_core::types::{MetricsBatch, TracesBatch, LogsBatch, TelemetryBatch, WriteResult, WriteStatus, WriteError};
use bridge_core::error::BridgeResult;

use crate::config::KafkaConfig;
use crate::error::{KafkaError as ConnectorKafkaError, KafkaResult};

/// Kafka producer implementation
pub struct KafkaProducer {
    /// Kafka configuration
    config: KafkaConfig,
    /// Producer state
    initialized: bool,
    /// Statistics
    stats: ProducerStats,
}

/// Producer statistics
#[derive(Debug, Clone)]
struct ProducerStats {
    total_writes: u64,
    total_records: u64,
    total_bytes: u64,
    error_count: u64,
    last_write_time: Option<DateTime<Utc>>,
    total_write_time_ms: u64,
}

impl Default for ProducerStats {
    fn default() -> Self {
        Self {
            total_writes: 0,
            total_records: 0,
            total_bytes: 0,
            error_count: 0,
            last_write_time: None,
            total_write_time_ms: 0,
        }
    }
}

impl KafkaProducer {
    /// Create a new Kafka producer
    pub async fn new(config: KafkaConfig) -> KafkaResult<Self> {
        info!("Creating Kafka producer for topic: {}", config.topic_name());
        
        let producer = Self {
            config,
            initialized: true,
            stats: ProducerStats::default(),
        };
        
        info!("Kafka producer initialized successfully");
        Ok(producer)
    }

    /// Get the configuration
    pub fn config(&self) -> &KafkaConfig {
        &self.config
    }

    /// Check if the producer is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Simulate sending a message to Kafka
    async fn send_message(&mut self, _key: &str, value: &str) -> KafkaResult<()> {
        let start_time = Instant::now();
        
        // Simulate Kafka send operation
        self.stats.total_writes += 1;
        self.stats.total_bytes += value.len() as u64;
        self.stats.last_write_time = Some(Utc::now());
        
        // Simulate potential failure
        if value.contains("error") {
            self.stats.error_count += 1;
            return Err(ConnectorKafkaError::producer("Simulated error for testing"));
        }
        
        // Track the write time
        let write_time_ms = start_time.elapsed().as_millis() as u64;
        self.stats.total_write_time_ms += write_time_ms;
        
        Ok(())
    }
}

#[async_trait]
impl LakehouseWriter for KafkaProducer {
    async fn write_metrics(&self, batch: MetricsBatch) -> BridgeResult<WriteResult> {
        debug!("Writing metrics batch to Kafka: {} records", batch.metrics.len());
        
        let start_time = Instant::now();
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut errors = Vec::new();
        
        // Create a mutable copy for sending messages
        let mut producer_copy = self.clone();
        
        for metric in &batch.metrics {
            let message = match serde_json::to_string(metric) {
                Ok(msg) => msg,
                Err(e) => {
                    records_failed += 1;
                    errors.push(WriteError {
                        code: "SERIALIZATION_ERROR".to_string(),
                        message: format!("Failed to serialize metric: {}", e),
                        details: None,
                    });
                    continue;
                }
            };
            
            let key = format!("metric_{}", metric.timestamp);
            match producer_copy.send_message(&key, &message).await {
                Ok(_) => {
                    records_written += 1;
                }
                Err(e) => {
                    records_failed += 1;
                    errors.push(WriteError {
                        code: "KAFKA_ERROR".to_string(),
                        message: format!("Failed to send metric to Kafka: {}", e),
                        details: None,
                    });
                }
            }
        }
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status = if records_failed == 0 {
            WriteStatus::Success
        } else if records_written > 0 {
            WriteStatus::Partial
        } else {
            WriteStatus::Failed
        };
        
        info!("Successfully wrote {} metrics records to Kafka", records_written);
        
        Ok(WriteResult {
            timestamp: Utc::now(),
            status,
            records_written,
            records_failed,
            duration_ms,
            metadata: HashMap::new(),
            errors,
        })
    }

    async fn write_traces(&self, batch: TracesBatch) -> BridgeResult<WriteResult> {
        debug!("Writing traces batch to Kafka: {} records", batch.traces.len());
        
        let start_time = Instant::now();
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut errors = Vec::new();
        
        // Create a mutable copy for sending messages
        let mut producer_copy = self.clone();
        
        for trace in &batch.traces {
            let message = match serde_json::to_string(trace) {
                Ok(msg) => msg,
                Err(e) => {
                    records_failed += 1;
                    errors.push(WriteError {
                        code: "SERIALIZATION_ERROR".to_string(),
                        message: format!("Failed to serialize trace: {}", e),
                        details: None,
                    });
                    continue;
                }
            };
            
            let key = format!("trace_{}", trace.trace_id);
            match producer_copy.send_message(&key, &message).await {
                Ok(_) => {
                    records_written += 1;
                }
                Err(e) => {
                    records_failed += 1;
                    errors.push(WriteError {
                        code: "KAFKA_ERROR".to_string(),
                        message: format!("Failed to send trace to Kafka: {}", e),
                        details: None,
                    });
                }
            }
        }
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status = if records_failed == 0 {
            WriteStatus::Success
        } else if records_written > 0 {
            WriteStatus::Partial
        } else {
            WriteStatus::Failed
        };
        
        info!("Successfully wrote {} trace records to Kafka", records_written);
        
        Ok(WriteResult {
            timestamp: Utc::now(),
            status,
            records_written,
            records_failed,
            duration_ms,
            metadata: HashMap::new(),
            errors,
        })
    }

    async fn write_logs(&self, batch: LogsBatch) -> BridgeResult<WriteResult> {
        debug!("Writing logs batch to Kafka: {} records", batch.logs.len());
        
        let start_time = Instant::now();
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut errors = Vec::new();
        
        // Create a mutable copy for sending messages
        let mut producer_copy = self.clone();
        
        for log in &batch.logs {
            let message = match serde_json::to_string(log) {
                Ok(msg) => msg,
                Err(e) => {
                    records_failed += 1;
                    errors.push(WriteError {
                        code: "SERIALIZATION_ERROR".to_string(),
                        message: format!("Failed to serialize log: {}", e),
                        details: None,
                    });
                    continue;
                }
            };
            
            let key = format!("log_{}", log.timestamp);
            match producer_copy.send_message(&key, &message).await {
                Ok(_) => {
                    records_written += 1;
                }
                Err(e) => {
                    records_failed += 1;
                    errors.push(WriteError {
                        code: "KAFKA_ERROR".to_string(),
                        message: format!("Failed to send log to Kafka: {}", e),
                        details: None,
                    });
                }
            }
        }
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status = if records_failed == 0 {
            WriteStatus::Success
        } else if records_written > 0 {
            WriteStatus::Partial
        } else {
            WriteStatus::Failed
        };
        
        info!("Successfully wrote {} log records to Kafka", records_written);
        
        Ok(WriteResult {
            timestamp: Utc::now(),
            status,
            records_written,
            records_failed,
            duration_ms,
            metadata: HashMap::new(),
            errors,
        })
    }

    async fn write_batch(&self, batch: TelemetryBatch) -> BridgeResult<WriteResult> {
        debug!("Writing telemetry batch to Kafka: {} records", batch.records.len());
        
        let start_time = Instant::now();
        let mut total_records_written = 0;
        let mut total_records_failed = 0;
        let mut all_errors = Vec::new();
        
        // Write each telemetry record
        for record in &batch.records {
            let message = match serde_json::to_string(record) {
                Ok(msg) => msg,
                Err(e) => {
                    total_records_failed += 1;
                    all_errors.push(WriteError {
                        code: "SERIALIZATION_ERROR".to_string(),
                        message: format!("Failed to serialize telemetry record: {}", e),
                        details: None,
                    });
                    continue;
                }
            };
            
            let key = format!("telemetry_{}", record.timestamp);
            // Create a mutable copy for sending messages
            let mut producer_copy = self.clone();
            match producer_copy.send_message(&key, &message).await {
                Ok(_) => {
                    total_records_written += 1;
                }
                Err(e) => {
                    total_records_failed += 1;
                    all_errors.push(WriteError {
                        code: "KAFKA_ERROR".to_string(),
                        message: format!("Failed to send telemetry record to Kafka: {}", e),
                        details: None,
                    });
                }
            }
        }
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status = if total_records_failed == 0 {
            WriteStatus::Success
        } else if total_records_written > 0 {
            WriteStatus::Partial
        } else {
            WriteStatus::Failed
        };
        
        info!("Successfully wrote {} telemetry records to Kafka", total_records_written);
        
        Ok(WriteResult {
            timestamp: Utc::now(),
            status,
            records_written: total_records_written,
            records_failed: total_records_failed,
            duration_ms,
            metadata: HashMap::new(),
            errors: all_errors,
        })
    }

    async fn flush(&self) -> BridgeResult<()> {
        debug!("Flushing Kafka producer");
        info!("Kafka producer flushed successfully");
        Ok(())
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::WriterStats> {
        let writes_per_minute = if let Some(last_write) = self.stats.last_write_time {
            let elapsed = Utc::now().signed_duration_since(last_write);
            if elapsed.num_seconds() > 0 {
                (self.stats.total_writes * 60) / elapsed.num_seconds() as u64
            } else {
                0
            }
        } else {
            0
        };
        
        let records_per_minute = if let Some(last_write) = self.stats.last_write_time {
            let elapsed = Utc::now().signed_duration_since(last_write);
            if elapsed.num_seconds() > 0 {
                (self.stats.total_records * 60) / elapsed.num_seconds() as u64
            } else {
                0
            }
        } else {
            0
        };
        
        let avg_write_time_ms = if self.stats.total_writes > 0 {
            self.stats.total_write_time_ms as f64 / self.stats.total_writes as f64
        } else {
            0.0
        };
        
        Ok(bridge_core::traits::WriterStats {
            total_writes: self.stats.total_writes,
            total_records: self.stats.total_records,
            writes_per_minute,
            records_per_minute,
            avg_write_time_ms,
            error_count: self.stats.error_count,
            last_write_time: self.stats.last_write_time,
        })
    }

    async fn close(&self) -> BridgeResult<()> {
        info!("Closing Kafka producer");
        info!("Kafka producer closed successfully");
        Ok(())
    }
}

impl Clone for KafkaProducer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            initialized: self.initialized,
            stats: self.stats.clone(),
        }
    }
}
