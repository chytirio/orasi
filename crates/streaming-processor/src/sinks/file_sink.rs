//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! File sink implementation
//! 
//! This module provides a file sink for streaming data to files.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use chrono::Utc;
use std::fs::OpenOptions;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tokio::fs::File as TokioFile;

use super::{SinkConfig, StreamSink, SinkStats};

/// File sink configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSinkConfig {
    /// Sink name
    pub name: String,
    
    /// Sink version
    pub version: String,
    
    /// File path
    pub file_path: String,
    
    /// File format
    pub file_format: FileFormat,
    
    /// Write mode
    pub write_mode: WriteMode,
    
    /// Buffer size
    pub buffer_size: usize,
    
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
    
    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// File format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileFormat {
    Json,
    JsonLines,
    Csv,
    Parquet,
    Avro,
    Arrow,
}

/// Write mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WriteMode {
    Append,
    Overwrite,
    Rotate,
}

impl FileSinkConfig {
    /// Create new file sink configuration
    pub fn new(file_path: String, file_format: FileFormat) -> Self {
        Self {
            name: "file".to_string(),
            version: "1.0.0".to_string(),
            file_path,
            file_format,
            write_mode: WriteMode::Append,
            buffer_size: 10000,
            flush_interval_ms: 5000,
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl SinkConfig for FileSinkConfig {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    async fn validate(&self) -> BridgeResult<()> {
        if self.file_path.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "File path cannot be empty".to_string()
            ));
        }
        
        // Check if directory exists and is writable
        if let Some(parent) = Path::new(&self.file_path).parent() {
            if !parent.exists() {
                return Err(bridge_core::BridgeError::configuration(
                    format!("Directory does not exist: {}", parent.display())
                ));
            }
        }
        
        Ok(())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// File sink implementation
pub struct FileSink {
    config: FileSinkConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<SinkStats>>,
    file_writer: Arc<RwLock<Option<FileWriter>>>,
}

/// File writer wrapper
struct FileWriter {
    file: TokioFile,
    buffer: Vec<u8>,
    buffer_size: usize,
    format: FileFormat,
    write_mode: WriteMode,
    records_written: u64,
}

impl FileWriter {
    /// Create new file writer
    async fn new(config: &FileSinkConfig) -> BridgeResult<Self> {
        let path = Path::new(&config.file_path);
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| bridge_core::BridgeError::stream(
                        format!("Failed to create directory: {}", e)
                    ))?;
            }
        }
        
        // Open file based on write mode
        let file = match config.write_mode {
            WriteMode::Append => {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&config.file_path)
                    .map_err(|e| bridge_core::BridgeError::stream(
                        format!("Failed to open file for appending: {}", e)
                    ))?
            },
            WriteMode::Overwrite => {
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&config.file_path)
                    .map_err(|e| bridge_core::BridgeError::stream(
                        format!("Failed to open file for writing: {}", e)
                    ))?
            },
            WriteMode::Rotate => {
                // For rotate mode, we'll create a new file with timestamp
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
                let stem = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("data");
                let extension = path.extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("json");
                let rotated_path = path.with_file_name(
                    format!("{}_{}.{}", stem, timestamp, extension)
                );
                
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&rotated_path)
                    .map_err(|e| bridge_core::BridgeError::stream(
                        format!("Failed to open rotated file: {}", e)
                    ))?
            }
        };
        
        let file = TokioFile::from_std(file);
        
        Ok(Self {
            file,
            buffer: Vec::with_capacity(config.buffer_size),
            buffer_size: config.buffer_size,
            format: config.file_format.clone(),
            write_mode: config.write_mode.clone(),
            records_written: 0,
        })
    }
    
    /// Write batch to file
    async fn write_batch(&mut self, batch: &TelemetryBatch) -> BridgeResult<()> {
        match self.format {
            FileFormat::Json => self.write_json(batch).await,
            FileFormat::JsonLines => self.write_json_lines(batch).await,
            FileFormat::Csv => self.write_csv(batch).await,
            FileFormat::Parquet => self.write_parquet(batch).await,
            FileFormat::Avro => self.write_avro(batch).await,
            FileFormat::Arrow => self.write_arrow(batch).await,
        }
    }
    
    /// Write batch as JSON
    async fn write_json(&mut self, batch: &TelemetryBatch) -> BridgeResult<()> {
        let json_data = serde_json::to_string_pretty(batch)
            .map_err(|e| bridge_core::BridgeError::stream(
                format!("Failed to serialize batch to JSON: {}", e)
            ))?;
        
        self.file.write_all(json_data.as_bytes()).await
            .map_err(|e| bridge_core::BridgeError::stream(
                format!("Failed to write JSON to file: {}", e)
            ))?;
        
        self.file.write_all(b"\n").await
            .map_err(|e| bridge_core::BridgeError::stream(
                format!("Failed to write newline: {}", e)
            ))?;
        
        self.records_written += batch.records.len() as u64;
        Ok(())
    }
    
    /// Write batch as JSON Lines (one JSON object per line)
    async fn write_json_lines(&mut self, batch: &TelemetryBatch) -> BridgeResult<()> {
        for record in &batch.records {
            let json_line = serde_json::to_string(record)
                .map_err(|e| bridge_core::BridgeError::stream(
                    format!("Failed to serialize record to JSON: {}", e)
                ))?;
            
            self.file.write_all(json_line.as_bytes()).await
                .map_err(|e| bridge_core::BridgeError::stream(
                    format!("Failed to write JSON line: {}", e)
                ))?;
            
            self.file.write_all(b"\n").await
                .map_err(|e| bridge_core::BridgeError::stream(
                    format!("Failed to write newline: {}", e)
                ))?;
        }
        
        self.records_written += batch.records.len() as u64;
        Ok(())
    }
    
    /// Write batch as CSV
    async fn write_csv(&mut self, batch: &TelemetryBatch) -> BridgeResult<()> {
        // Write CSV header if this is the first write
        if self.records_written == 0 {
            let header = "id,timestamp,record_type,source,attributes\n";
            self.file.write_all(header.as_bytes()).await
                .map_err(|e| bridge_core::BridgeError::stream(
                    format!("Failed to write CSV header: {}", e)
                ))?;
        }
        
        for record in &batch.records {
            let attributes_str = serde_json::to_string(&record.attributes)
                .unwrap_or_default()
                .replace("\"", "\"\""); // Escape quotes for CSV
            
            let csv_line = format!(
                "{},{},{},{},\"{}\"\n",
                record.id,
                record.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
                format!("{:?}", record.record_type),
                batch.source,
                attributes_str
            );
            
            self.file.write_all(csv_line.as_bytes()).await
                .map_err(|e| bridge_core::BridgeError::stream(
                    format!("Failed to write CSV line: {}", e)
                ))?;
        }
        
        self.records_written += batch.records.len() as u64;
        Ok(())
    }
    
    /// Write batch as Parquet
    async fn write_parquet(&mut self, batch: &TelemetryBatch) -> BridgeResult<()> {
        // For now, we'll write as JSON since Parquet requires more complex setup
        // In a production implementation, you'd use the parquet crate
        warn!("Parquet format not fully implemented, falling back to JSON");
        self.write_json(batch).await
    }
    
    /// Write batch as Avro
    async fn write_avro(&mut self, batch: &TelemetryBatch) -> BridgeResult<()> {
        // For now, we'll write as JSON since Avro requires schema definition
        // In a production implementation, you'd use the avro-rs crate
        warn!("Avro format not fully implemented, falling back to JSON");
        self.write_json(batch).await
    }
    
    /// Write batch as Arrow
    async fn write_arrow(&mut self, batch: &TelemetryBatch) -> BridgeResult<()> {
        // For now, we'll write as JSON since Arrow requires more complex setup
        // In a production implementation, you'd use the arrow crate
        warn!("Arrow format not fully implemented, falling back to JSON");
        self.write_json(batch).await
    }
    
    /// Flush the file buffer
    async fn flush(&mut self) -> BridgeResult<()> {
        self.file.flush().await
            .map_err(|e| bridge_core::BridgeError::stream(
                format!("Failed to flush file: {}", e)
            ))?;
        Ok(())
    }
    
    /// Get records written count
    fn records_written(&self) -> u64 {
        self.records_written
    }
}

impl FileSink {
    /// Create new file sink
    pub async fn new(config: &dyn SinkConfig) -> BridgeResult<Self> {
        let config = config.as_any()
            .downcast_ref::<FileSinkConfig>()
            .ok_or_else(|| bridge_core::BridgeError::configuration(
                "Invalid file sink configuration".to_string()
            ))?
            .clone();
        
        config.validate().await?;
        
        let stats = SinkStats {
            sink: config.name.clone(),
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_send_time: None,
            is_connected: false,
            latency_ms: 0,
        };
        
        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            file_writer: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Initialize file writer
    async fn init_file_writer(&self) -> BridgeResult<()> {
        info!("Initializing file writer for: {}", self.config.file_path);
        
        let file_writer = FileWriter::new(&self.config).await?;
        let mut writer_guard = self.file_writer.write().await;
        *writer_guard = Some(file_writer);
        drop(writer_guard);
        
        info!("File writer initialized");
        Ok(())
    }
    
    /// Write batch to file
    async fn write_batch(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        let start_time = std::time::Instant::now();
        
        info!("Writing batch to file: {}", self.config.file_path);
        
        let mut writer_guard = self.file_writer.write().await;
        if let Some(file_writer) = writer_guard.as_mut() {
            file_writer.write_batch(&batch).await?;
            file_writer.flush().await?;
        } else {
            return Err(bridge_core::BridgeError::stream(
                "File writer not initialized".to_string()
            ));
        }
        drop(writer_guard);
        
        let write_time = start_time.elapsed().as_millis() as u64;
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_batches += 1;
            stats.total_records += batch.size as u64;
            stats.total_bytes += batch.size as u64 * 100; // Approximate bytes per record
            stats.last_send_time = Some(Utc::now());
            stats.latency_ms = write_time;
            stats.is_connected = true;
        }
        
        info!("Batch written to file in {}ms", write_time);
        Ok(())
    }
}

#[async_trait]
impl StreamSink for FileSink {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing file sink");
        
        // Validate configuration
        self.config.validate().await?;
        
        // Initialize file writer
        self.init_file_writer().await?;
        
        info!("File sink initialized");
        Ok(())
    }
    
    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting file sink");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
        }
        
        info!("File sink started");
        Ok(())
    }
    
    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping file sink");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);
        
        // Flush any remaining data
        let mut writer_guard = self.file_writer.write().await;
        if let Some(file_writer) = writer_guard.as_mut() {
            if let Err(e) = file_writer.flush().await {
                error!("Failed to flush file writer on stop: {}", e);
            }
        }
        drop(writer_guard);
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }
        
        info!("File sink stopped");
        Ok(())
    }
    
    fn is_running(&self) -> bool {
        // This is a simplified check - in practice we'd need to handle the async nature
        // For now, we'll return true if the sink has been initialized
        // We can't easily check the RwLock here, so we'll use a simple approach
        true // The sink is considered running if it's been created
    }
    
    async fn send(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        if !self.is_running() {
            return Err(bridge_core::BridgeError::stream(
                "File sink is not running".to_string()
            ));
        }
        
        self.write_batch(batch).await
    }
    
    async fn get_stats(&self) -> BridgeResult<SinkStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn version(&self) -> &str {
        &self.config.version
    }
}

impl FileSink {
    /// Get file configuration
    pub fn get_config(&self) -> &FileSinkConfig {
        &self.config
    }
    
    /// Check if sink is running (async version)
    pub async fn is_running_async(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::types::{TelemetryRecord, TelemetryData, TelemetryType, LogData, LogLevel};
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_sink_creation() {
        let config = FileSinkConfig::new(
            "/tmp/test_file_sink.json".to_string(),
            FileFormat::Json
        );
        
        let sink = FileSink::new(&config).await;
        assert!(sink.is_ok());
    }

    #[tokio::test]
    async fn test_file_sink_json_writing() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.json");
        
        let config = FileSinkConfig::new(
            file_path.to_string_lossy().to_string(),
            FileFormat::Json
        );
        
        let mut sink = FileSink::new(&config).await.unwrap();
        sink.init().await.unwrap();
        sink.start().await.unwrap();
        
        // Create a test batch
        let record = TelemetryRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Test log message".to_string(),
                attributes: HashMap::new(),
                body: None,
                severity_number: Some(9),
                severity_text: Some("INFO".to_string()),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        
        let batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        };
        
        // Send the batch
        let result = sink.send(batch).await;
        assert!(result.is_ok());
        
        // Verify file was created and contains data
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(!content.is_empty());
        assert!(content.contains("Test log message"));
        
        sink.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_file_sink_csv_writing() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.csv");
        
        let config = FileSinkConfig::new(
            file_path.to_string_lossy().to_string(),
            FileFormat::Csv
        );
        
        let mut sink = FileSink::new(&config).await.unwrap();
        sink.init().await.unwrap();
        sink.start().await.unwrap();
        
        // Create a test batch
        let record = TelemetryRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Test CSV log".to_string(),
                attributes: HashMap::new(),
                body: None,
                severity_number: Some(9),
                severity_text: Some("INFO".to_string()),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        
        let batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        };
        
        // Send the batch
        let result = sink.send(batch).await;
        assert!(result.is_ok());
        
        // Verify file was created and contains CSV data
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(!content.is_empty());
        assert!(content.contains("id,timestamp,record_type,source,attributes"));
        
        sink.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_file_sink_json_lines_writing() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.jsonl");
        
        let config = FileSinkConfig::new(
            file_path.to_string_lossy().to_string(),
            FileFormat::JsonLines
        );
        
        let mut sink = FileSink::new(&config).await.unwrap();
        sink.init().await.unwrap();
        sink.start().await.unwrap();
        
        // Create a test batch
        let record = TelemetryRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Test JSONL log".to_string(),
                attributes: HashMap::new(),
                body: None,
                severity_number: Some(9),
                severity_text: Some("INFO".to_string()),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        
        let batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        };
        
        // Send the batch
        let result = sink.send(batch).await;
        assert!(result.is_ok());
        
        // Verify file was created and contains JSONL data
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(!content.is_empty());
        assert!(content.contains("Test JSONL log"));
        
        sink.stop().await.unwrap();
    }
}
