//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration for streaming processor

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Streaming processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Sources configuration
    pub sources: HashMap<String, SourceConfig>,

    /// Processors configuration
    pub processors: Vec<ProcessorConfig>,

    /// Sinks configuration
    pub sinks: HashMap<String, SinkConfig>,

    /// General processing configuration
    pub processing: ProcessingConfig,

    /// State management configuration
    pub state: StateConfig,

    /// Metrics configuration
    pub metrics: MetricsConfig,

    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for StreamingProcessorConfig {
    fn default() -> Self {
        Self {
            name: "streaming-processor".to_string(),
            version: "1.0.0".to_string(),
            sources: HashMap::new(),
            processors: Vec::new(),
            sinks: HashMap::new(),
            processing: ProcessingConfig::default(),
            state: StateConfig::default(),
            metrics: MetricsConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl StreamingProcessorConfig {
    /// Validate the configuration
    pub fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Processor name cannot be empty".to_string(),
            ));
        }

        if self.version.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Processor version cannot be empty".to_string(),
            ));
        }

        // Validate sources
        for (name, source_config) in &self.sources {
            source_config.validate().map_err(|e| {
                bridge_core::error::BridgeError::configuration(format!(
                    "Source '{}' configuration error: {}",
                    name, e
                ))
            })?;
        }

        // Validate processors
        for processor_config in &self.processors {
            processor_config.validate().map_err(|e| {
                bridge_core::error::BridgeError::configuration(format!(
                    "Processor configuration error: {}",
                    e
                ))
            })?;
        }

        // Validate sinks
        for (name, sink_config) in &self.sinks {
            sink_config.validate().map_err(|e| {
                bridge_core::error::BridgeError::configuration(format!(
                    "Sink '{}' configuration error: {}",
                    name, e
                ))
            })?;
        }

        // Validate processing configuration
        self.processing.validate()?;

        // Validate state configuration
        self.state.validate()?;

        // Validate metrics configuration
        self.metrics.validate()?;

        // Validate security configuration
        self.security.validate()?;

        Ok(())
    }
}

/// Source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Source type
    pub source_type: SourceType,

    /// Source name
    pub name: String,

    /// Source version
    pub version: String,

    /// Source-specific configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// Connection configuration
    pub connection: ConnectionConfig,
}

impl SourceConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Source name cannot be empty".to_string(),
            ));
        }

        if self.version.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Source version cannot be empty".to_string(),
            ));
        }

        self.connection.validate()?;

        Ok(())
    }
}

/// Source types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Kafka,
    Http,
    File,
    WebSocket,
    Custom(String),
}

/// Processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Processor type
    pub processor_type: ProcessorType,

    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Processor-specific configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Processing order
    pub order: usize,
}

impl ProcessorConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Processor name cannot be empty".to_string(),
            ));
        }

        if self.version.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Processor version cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Processor types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessorType {
    Filter,
    Transform,
    Aggregate,
    Window,
    Custom(String),
}

/// Sink configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkConfig {
    /// Sink type
    pub sink_type: SinkType,

    /// Sink name
    pub name: String,

    /// Sink version
    pub version: String,

    /// Sink-specific configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// Connection configuration
    pub connection: ConnectionConfig,
}

impl SinkConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Sink name cannot be empty".to_string(),
            ));
        }

        if self.version.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Sink version cannot be empty".to_string(),
            ));
        }

        self.connection.validate()?;

        Ok(())
    }
}

/// Sink types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SinkType {
    Kafka,
    Http,
    File,
    Database,
    Custom(String),
}

/// Processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Batch size
    pub batch_size: usize,

    /// Buffer size
    pub buffer_size: usize,

    /// Processing timeout
    pub timeout: Duration,

    /// Enable parallel processing
    pub enable_parallel: bool,

    /// Number of parallel workers
    pub num_workers: usize,

    /// Enable backpressure
    pub enable_backpressure: bool,

    /// Backpressure threshold (percentage)
    pub backpressure_threshold: u8,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            buffer_size: 10000,
            timeout: Duration::from_secs(30),
            enable_parallel: false,
            num_workers: 1,
            enable_backpressure: true,
            backpressure_threshold: 80,
        }
    }
}

impl ProcessingConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.batch_size == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Batch size must be greater than 0".to_string(),
            ));
        }

        if self.buffer_size == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Buffer size must be greater than 0".to_string(),
            ));
        }

        if self.num_workers == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Number of workers must be greater than 0".to_string(),
            ));
        }

        if self.backpressure_threshold > 100 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Backpressure threshold must be between 0 and 100".to_string(),
            ));
        }

        Ok(())
    }
}

/// State configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    /// State store type
    pub store_type: StateStoreType,

    /// State store configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Enable state persistence
    pub enable_persistence: bool,

    /// State persistence path
    pub persistence_path: Option<String>,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            store_type: StateStoreType::InMemory,
            config: HashMap::new(),
            enable_persistence: false,
            persistence_path: None,
        }
    }
}

impl StateConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.enable_persistence && self.persistence_path.is_none() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Persistence path must be specified when persistence is enabled".to_string(),
            ));
        }

        Ok(())
    }
}

/// State store types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateStoreType {
    InMemory,
    Redis,
    Database,
    File,
    Custom(String),
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Metrics endpoint
    pub endpoint: Option<String>,

    /// Metrics collection interval
    pub collection_interval: Duration,

    /// Enable health checks
    pub enable_health_checks: bool,

    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            endpoint: Some("0.0.0.0:9090".to_string()),
            collection_interval: Duration::from_secs(15),
            enable_health_checks: true,
            health_check_interval: Duration::from_secs(30),
        }
    }
}

impl MetricsConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.enable_metrics && self.endpoint.is_none() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Metrics endpoint must be specified when metrics are enabled".to_string(),
            ));
        }

        Ok(())
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub enable_auth: bool,

    /// Authentication type
    pub auth_type: AuthType,

    /// TLS configuration
    pub tls: Option<TlsConfig>,

    /// Rate limiting configuration
    pub rate_limiting: Option<RateLimitingConfig>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_auth: false,
            auth_type: AuthType::None,
            tls: None,
            rate_limiting: None,
        }
    }
}

impl SecurityConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if let Some(tls_config) = &self.tls {
            tls_config.validate()?;
        }

        if let Some(rate_limiting_config) = &self.rate_limiting {
            rate_limiting_config.validate()?;
        }

        Ok(())
    }
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    None,
    ApiKey,
    Jwt,
    OAuth,
    Basic,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: AuthType,

    /// Authentication credentials
    pub credentials: HashMap<String, String>,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Certificate file path
    pub cert_file: String,

    /// Private key file path
    pub key_file: String,

    /// CA certificate file path
    pub ca_file: Option<String>,
}

impl TlsConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.cert_file.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "TLS certificate file path cannot be empty".to_string(),
            ));
        }

        if self.key_file.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "TLS private key file path cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Requests per second
    pub requests_per_second: u32,

    /// Burst size
    pub burst_size: u32,

    /// Rate limit by IP
    pub limit_by_ip: bool,

    /// Rate limit by user
    pub limit_by_user: bool,
}

impl RateLimitingConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.requests_per_second == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Requests per second must be greater than 0".to_string(),
            ));
        }

        if self.burst_size == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Burst size must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection timeout
    pub timeout: Duration,

    /// Maximum retries
    pub max_retries: u32,

    /// Retry delay
    pub retry_delay: Duration,

    /// Keep-alive configuration
    pub keep_alive: Option<KeepAliveConfig>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            keep_alive: Some(KeepAliveConfig::default()),
        }
    }
}

impl ConnectionConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.max_retries == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Maximum retries must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Keep-alive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepAliveConfig {
    /// Keep-alive interval
    pub interval: Duration,

    /// Keep-alive timeout
    pub timeout: Duration,
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
        }
    }
}
