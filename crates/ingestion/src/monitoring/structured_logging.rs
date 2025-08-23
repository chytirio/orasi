//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Structured logging system for the ingestion platform
//!
//! This module provides comprehensive structured logging capabilities including
//! log levels, structured fields, log rotation, and integration with external
//! logging systems.

use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, trace, warn, Level};

/// Log level configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    /// Trace level
    Trace,
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warn level
    Warn,
    /// Error level
    Error,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

/// Structured logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLoggingConfig {
    /// Enable structured logging
    pub enabled: bool,

    /// Log level
    pub level: LogLevel,

    /// Enable JSON formatting
    pub json_format: bool,

    /// Enable console output
    pub console_output: bool,

    /// Enable file output
    pub file_output: bool,

    /// Log file path
    pub log_file_path: Option<String>,

    /// Enable log rotation
    pub enable_rotation: bool,

    /// Maximum log file size in MB
    pub max_file_size_mb: u64,

    /// Maximum number of log files to keep
    pub max_files: u32,

    /// Enable timestamp in logs
    pub include_timestamp: bool,

    /// Enable thread ID in logs
    pub include_thread_id: bool,

    /// Enable span information in logs
    pub include_spans: bool,

    /// Custom fields to include in all logs
    pub custom_fields: HashMap<String, String>,

    /// Enable external logging integration
    pub external_integration: ExternalLoggingConfig,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// External logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLoggingConfig {
    /// Enable external logging
    pub enabled: bool,

    /// External logging endpoints
    pub endpoints: Vec<ExternalLoggingEndpoint>,

    /// Batch size for external logging
    pub batch_size: usize,

    /// Flush interval in seconds
    pub flush_interval_secs: u64,

    /// Retry configuration
    pub retry_config: LoggingRetryConfig,
}

/// External logging endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLoggingEndpoint {
    /// Endpoint name
    pub name: String,

    /// Endpoint URL
    pub url: String,

    /// Authentication (optional)
    pub auth: Option<LoggingAuth>,

    /// Headers
    pub headers: HashMap<String, String>,

    /// Timeout in seconds
    pub timeout_secs: u64,

    /// Whether endpoint is required
    pub required: bool,
}

/// Logging authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingAuth {
    /// Authentication type
    pub auth_type: LoggingAuthType,

    /// Username (for basic auth)
    pub username: Option<String>,

    /// Password (for basic auth)
    pub password: Option<String>,

    /// Token (for bearer auth)
    pub token: Option<String>,

    /// API key
    pub api_key: Option<String>,
}

/// Logging authentication type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoggingAuthType {
    None,
    Basic,
    Bearer,
    ApiKey,
}

/// Logging retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingRetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,

    /// Maximum retry delay in milliseconds
    pub max_delay_ms: u64,

    /// Retry backoff multiplier
    pub backoff_multiplier: f64,

    /// Enable jitter
    pub enable_jitter: bool,
}

/// Structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogEntry {
    /// Log timestamp
    pub timestamp: DateTime<Utc>,

    /// Log level
    pub level: LogLevel,

    /// Log message
    pub message: String,

    /// Target module
    pub target: String,

    /// Structured fields
    pub fields: HashMap<String, serde_json::Value>,

    /// Span information
    pub span_info: Option<SpanInfo>,

    /// Thread ID
    pub thread_id: Option<u64>,

    /// File and line information
    pub file_info: Option<FileInfo>,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Span information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanInfo {
    /// Span name
    pub name: String,

    /// Span ID
    pub span_id: String,

    /// Parent span ID
    pub parent_span_id: Option<String>,

    /// Trace ID
    pub trace_id: Option<String>,

    /// Span fields
    pub fields: HashMap<String, serde_json::Value>,
}

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File name
    pub file: String,

    /// Line number
    pub line: u32,

    /// Column number
    pub column: u32,
}

/// Structured logging manager
pub struct StructuredLoggingManager {
    config: StructuredLoggingConfig,
    custom_fields: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    external_senders: Arc<RwLock<Vec<ExternalLogSender>>>,
    log_buffer: Arc<RwLock<Vec<StructuredLogEntry>>>,
    is_initialized: Arc<RwLock<bool>>,
}

/// External log sender
struct ExternalLogSender {
    name: String,
    endpoint: ExternalLoggingEndpoint,
    client: reqwest::Client,
}

impl StructuredLoggingConfig {
    /// Create new structured logging configuration
    pub fn new() -> Self {
        Self {
            enabled: true,
            level: LogLevel::Info,
            json_format: true,
            console_output: true,
            file_output: false,
            log_file_path: None,
            enable_rotation: false,
            max_file_size_mb: 100,
            max_files: 5,
            include_timestamp: true,
            include_thread_id: true,
            include_spans: true,
            custom_fields: HashMap::new(),
            external_integration: ExternalLoggingConfig {
                enabled: false,
                endpoints: Vec::new(),
                batch_size: 100,
                flush_interval_secs: 5,
                retry_config: LoggingRetryConfig {
                    max_attempts: 3,
                    initial_delay_ms: 1000,
                    max_delay_ms: 10000,
                    backoff_multiplier: 2.0,
                    enable_jitter: true,
                },
            },
            additional_config: HashMap::new(),
        }
    }

    /// Create configuration with custom level
    pub fn with_level(level: LogLevel) -> Self {
        let mut config = Self::new();
        config.level = level;
        config
    }

    /// Create configuration for production
    pub fn production() -> Self {
        let mut config = Self::new();
        config.level = LogLevel::Info;
        config.json_format = true;
        config.console_output = false;
        config.file_output = true;
        config.log_file_path = Some("/var/log/ingestion/ingestion.log".to_string());
        config.enable_rotation = true;
        config.max_file_size_mb = 100;
        config.max_files = 10;
        config
    }

    /// Create configuration for development
    pub fn development() -> Self {
        let mut config = Self::new();
        config.level = LogLevel::Debug;
        config.json_format = false;
        config.console_output = true;
        config.file_output = false;
        config.include_spans = true;
        config
    }
}

impl StructuredLoggingManager {
    /// Create new structured logging manager
    pub fn new(config: StructuredLoggingConfig) -> Self {
        Self {
            config,
            custom_fields: Arc::new(RwLock::new(HashMap::new())),
            external_senders: Arc::new(RwLock::new(Vec::new())),
            log_buffer: Arc::new(RwLock::new(Vec::new())),
            is_initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize structured logging
    pub async fn initialize(&self) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut is_initialized = self.is_initialized.write().await;
        if *is_initialized {
            return Ok(());
        }

        // Initialize tracing subscriber
        self.initialize_tracing_subscriber().await?;

        // Initialize external logging
        if self.config.external_integration.enabled {
            self.initialize_external_logging().await?;
        }

        // Start log flushing task
        if self.config.external_integration.enabled {
            let manager = self.clone();
            tokio::spawn(async move {
                manager.run_log_flushing().await;
            });
        }

        *is_initialized = true;
        info!("Structured logging initialized");
        Ok(())
    }

    /// Initialize tracing subscriber
    async fn initialize_tracing_subscriber(&self) -> BridgeResult<()> {
        info!("Initializing structured logging");

        // For now, just log that structured logging is initialized
        // In a real implementation, this would set up the actual tracing infrastructure
        info!("Structured logging initialized (simplified implementation)");

        Ok(())
    }

    /// Initialize external logging
    async fn initialize_external_logging(&self) -> BridgeResult<()> {
        let mut senders = Vec::new();

        for endpoint in &self.config.external_integration.endpoints {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(endpoint.timeout_secs))
                .build()
                .map_err(|e| {
                    bridge_core::BridgeError::configuration(format!(
                        "Failed to create HTTP client: {}",
                        e
                    ))
                })?;

            let sender = ExternalLogSender {
                name: endpoint.name.clone(),
                endpoint: endpoint.clone(),
                client,
            };

            senders.push(sender);
        }

        let mut external_senders = self.external_senders.write().await;
        *external_senders = senders;

        Ok(())
    }

    /// Add custom field
    pub async fn add_custom_field(
        &self,
        key: String,
        value: serde_json::Value,
    ) -> BridgeResult<()> {
        let mut custom_fields = self.custom_fields.write().await;
        custom_fields.insert(key, value);
        Ok(())
    }

    /// Remove custom field
    pub async fn remove_custom_field(&self, key: &str) -> BridgeResult<bool> {
        let mut custom_fields = self.custom_fields.write().await;
        Ok(custom_fields.remove(key).is_some())
    }

    /// Log structured message
    pub async fn log(
        &self,
        level: LogLevel,
        message: &str,
        fields: HashMap<String, serde_json::Value>,
    ) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let custom_fields = self.custom_fields.read().await;
        let mut all_fields = fields.clone();
        all_fields.extend(custom_fields.clone());

        let entry = StructuredLogEntry {
            timestamp: Utc::now(),
            level: level.clone(),
            message: message.to_string(),
            target: "ingestion".to_string(),
            fields: all_fields,
            span_info: None, // Would be populated by tracing
            thread_id: None, // Removed unstable API call
            file_info: None, // Would be populated by tracing
            metadata: HashMap::new(),
        };

        // Add to buffer for external logging
        if self.config.external_integration.enabled {
            let mut log_buffer = self.log_buffer.write().await;
            log_buffer.push(entry.clone());
        }

        // Log using tracing
        match level {
            LogLevel::Trace => trace!(target: "ingestion", ?fields, "{}", message),
            LogLevel::Debug => debug!(target: "ingestion", ?fields, "{}", message),
            LogLevel::Info => info!(target: "ingestion", ?fields, "{}", message),
            LogLevel::Warn => warn!(target: "ingestion", ?fields, "{}", message),
            LogLevel::Error => error!(target: "ingestion", ?fields, "{}", message),
        }

        Ok(())
    }

    /// Log info message
    pub async fn info(
        &self,
        message: &str,
        fields: HashMap<String, serde_json::Value>,
    ) -> BridgeResult<()> {
        self.log(LogLevel::Info, message, fields).await
    }

    /// Log warning message
    pub async fn warn(
        &self,
        message: &str,
        fields: HashMap<String, serde_json::Value>,
    ) -> BridgeResult<()> {
        self.log(LogLevel::Warn, message, fields).await
    }

    /// Log error message
    pub async fn error(
        &self,
        message: &str,
        fields: HashMap<String, serde_json::Value>,
    ) -> BridgeResult<()> {
        self.log(LogLevel::Error, message, fields).await
    }

    /// Log debug message
    pub async fn debug(
        &self,
        message: &str,
        fields: HashMap<String, serde_json::Value>,
    ) -> BridgeResult<()> {
        self.log(LogLevel::Debug, message, fields).await
    }

    /// Log trace message
    pub async fn trace(
        &self,
        message: &str,
        fields: HashMap<String, serde_json::Value>,
    ) -> BridgeResult<()> {
        self.log(LogLevel::Trace, message, fields).await
    }

    /// Run log flushing task
    async fn run_log_flushing(&self) {
        let flush_interval =
            std::time::Duration::from_secs(self.config.external_integration.flush_interval_secs);
        let mut interval = tokio::time::interval(flush_interval);

        while *self.is_initialized.read().await {
            interval.tick().await;
            if let Err(e) = self.flush_logs().await {
                error!("Failed to flush logs: {}", e);
            }
        }
    }

    /// Flush logs to external endpoints
    async fn flush_logs(&self) -> BridgeResult<()> {
        let mut log_buffer = self.log_buffer.write().await;
        if log_buffer.is_empty() {
            return Ok(());
        }

        let batch_size = self.config.external_integration.batch_size;
        let logs_to_send: Vec<StructuredLogEntry> = if log_buffer.len() <= batch_size {
            log_buffer.drain(..).collect()
        } else {
            log_buffer.drain(..batch_size).collect()
        };

        drop(log_buffer);

        let external_senders = self.external_senders.read().await;
        for sender in external_senders.iter() {
            if let Err(e) = self.send_logs_to_endpoint(sender, &logs_to_send).await {
                error!("Failed to send logs to endpoint {}: {}", sender.name, e);
            }
        }

        Ok(())
    }

    /// Send logs to specific endpoint
    async fn send_logs_to_endpoint(
        &self,
        sender: &ExternalLogSender,
        logs: &[StructuredLogEntry],
    ) -> BridgeResult<()> {
        let retry_config = &self.config.external_integration.retry_config;
        let mut attempt = 0;

        loop {
            let result = self.send_logs_with_retry(sender, logs).await;

            match result {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempt += 1;
                    if attempt >= retry_config.max_attempts {
                        return Err(e);
                    }

                    let delay = self.calculate_retry_delay(attempt, retry_config);
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }

    /// Send logs with retry logic
    async fn send_logs_with_retry(
        &self,
        sender: &ExternalLogSender,
        logs: &[StructuredLogEntry],
    ) -> BridgeResult<()> {
        let mut request = sender.client.post(&sender.endpoint.url);

        // Add headers
        for (key, value) in &sender.endpoint.headers {
            request = request.header(key, value);
        }

        // Add authentication
        if let Some(auth) = &sender.endpoint.auth {
            match auth.auth_type {
                LoggingAuthType::Basic => {
                    if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
                        request = request.basic_auth(username, Some(password));
                    }
                }
                LoggingAuthType::Bearer => {
                    if let Some(token) = &auth.token {
                        request = request.bearer_auth(token);
                    }
                }
                LoggingAuthType::ApiKey => {
                    if let Some(api_key) = &auth.api_key {
                        request = request.header("X-API-Key", api_key);
                    }
                }
                LoggingAuthType::None => {}
            }
        }

        let serialized_logs = serde_json::to_string(&logs).map_err(|e| {
            bridge_core::BridgeError::serialization(format!("Failed to serialize logs: {}", e))
        })?;

        let response = request
            .header("Content-Type", "application/json")
            .body(serialized_logs)
            .send()
            .await
            .map_err(|e| {
                bridge_core::BridgeError::network(format!("Failed to send logs: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(bridge_core::BridgeError::network(format!(
                "External logging endpoint returned error status: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Calculate retry delay
    fn calculate_retry_delay(&self, attempt: u32, retry_config: &LoggingRetryConfig) -> u64 {
        let base_delay = retry_config.initial_delay_ms as f64
            * retry_config.backoff_multiplier.powi(attempt as i32 - 1);
        let delay = base_delay.min(retry_config.max_delay_ms as f64) as u64;

        if retry_config.enable_jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let jitter = rng.gen_range(0.8..1.2);
            (delay as f64 * jitter) as u64
        } else {
            delay
        }
    }

    /// Get logging statistics
    pub async fn get_statistics(&self) -> LoggingStatistics {
        let log_buffer = self.log_buffer.read().await;
        let external_senders = self.external_senders.read().await;

        LoggingStatistics {
            buffer_size: log_buffer.len(),
            external_endpoints: external_senders.len(),
            is_initialized: *self.is_initialized.read().await,
            config: self.config.clone(),
        }
    }
}

impl Clone for StructuredLoggingManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            custom_fields: Arc::clone(&self.custom_fields),
            external_senders: Arc::clone(&self.external_senders),
            log_buffer: Arc::clone(&self.log_buffer),
            is_initialized: Arc::clone(&self.is_initialized),
        }
    }
}

/// Logging statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingStatistics {
    /// Current buffer size
    pub buffer_size: usize,

    /// Number of external endpoints
    pub external_endpoints: usize,

    /// Whether logging is initialized
    pub is_initialized: bool,

    /// Current configuration
    pub config: StructuredLoggingConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_structured_logging_config_creation() {
        let config = StructuredLoggingConfig::new();

        assert!(config.enabled);
        assert_eq!(config.level, LogLevel::Info);
        assert!(config.json_format);
        assert!(config.console_output);
    }

    #[tokio::test]
    async fn test_structured_logging_manager_creation() {
        let config = StructuredLoggingConfig::new();
        let manager = StructuredLoggingManager::new(config);

        let stats = manager.get_statistics().await;
        assert_eq!(stats.buffer_size, 0);
        assert_eq!(stats.external_endpoints, 0);
        assert!(!stats.is_initialized);
    }

    #[tokio::test]
    async fn test_custom_field_management() {
        let config = StructuredLoggingConfig::new();
        let manager = StructuredLoggingManager::new(config);

        // Add custom field
        manager
            .add_custom_field("test_key".to_string(), serde_json::json!("test_value"))
            .await
            .unwrap();

        // Remove custom field
        let removed = manager.remove_custom_field("test_key").await.unwrap();
        assert!(removed);

        // Try to remove non-existent field
        let removed = manager.remove_custom_field("non_existent").await.unwrap();
        assert!(!removed);
    }
}
