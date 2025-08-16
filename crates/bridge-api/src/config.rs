//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration management for Bridge API

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info};

/// Bridge API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeAPIConfig {
    /// HTTP server configuration
    pub http: HttpConfig,

    /// gRPC server configuration
    pub grpc: GrpcConfig,

    /// Authentication configuration
    pub auth: AuthConfig,

    /// CORS configuration
    pub cors: CorsConfig,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Metrics configuration
    pub metrics: MetricsConfig,

    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for BridgeAPIConfig {
    fn default() -> Self {
        Self {
            http: HttpConfig::default(),
            grpc: GrpcConfig::default(),
            auth: AuthConfig::default(),
            cors: CorsConfig::default(),
            rate_limit: RateLimitConfig::default(),
            logging: LoggingConfig::default(),
            metrics: MetricsConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// HTTP server address
    pub address: String,

    /// HTTP server port
    pub port: u16,

    /// Request timeout
    pub request_timeout: Duration,

    /// Maximum request body size
    pub max_request_size: usize,

    /// Enable compression
    pub enable_compression: bool,

    /// Enable keep-alive
    pub enable_keep_alive: bool,

    /// Keep-alive timeout
    pub keep_alive_timeout: Duration,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            address: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout: Duration::from_secs(30),
            max_request_size: 10 * 1024 * 1024, // 10MB
            enable_compression: true,
            enable_keep_alive: true,
            keep_alive_timeout: Duration::from_secs(60),
        }
    }
}

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// gRPC server address
    pub address: String,

    /// gRPC server port
    pub port: u16,

    /// Enable reflection
    pub enable_reflection: bool,

    /// Enable health checks
    pub enable_health_checks: bool,

    /// Maximum message size
    pub max_message_size: usize,

    /// Maximum concurrent streams
    pub max_concurrent_streams: usize,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            address: "0.0.0.0".to_string(),
            port: 9090,
            enable_reflection: true,
            enable_health_checks: true,
            max_message_size: 4 * 1024 * 1024, // 4MB
            max_concurrent_streams: 1000,
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication
    pub enabled: bool,

    /// Authentication type
    pub auth_type: AuthType,

    /// API key configuration
    pub api_key: ApiKeyConfig,

    /// JWT configuration
    pub jwt: JwtConfig,

    /// OAuth configuration
    pub oauth: OAuthConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auth_type: AuthType::None,
            api_key: ApiKeyConfig::default(),
            jwt: JwtConfig::default(),
            oauth: OAuthConfig::default(),
        }
    }
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    None,
    ApiKey,
    Jwt,
    OAuth,
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// API key header name
    pub header_name: String,

    /// Valid API keys
    pub valid_keys: Vec<String>,

    /// API key validation regex
    pub validation_regex: Option<String>,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            header_name: "X-API-Key".to_string(),
            valid_keys: Vec::new(),
            validation_regex: None,
        }
    }
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret key
    pub secret_key: String,

    /// JWT issuer
    pub issuer: Option<String>,

    /// JWT audience
    pub audience: Option<String>,

    /// JWT expiration time
    pub expiration_time: Duration,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret_key: "your-secret-key".to_string(),
            issuer: None,
            audience: None,
            expiration_time: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth client ID
    pub client_id: String,

    /// OAuth client secret
    pub client_secret: String,

    /// OAuth authorization URL
    pub authorization_url: String,

    /// OAuth token URL
    pub token_url: String,

    /// OAuth user info URL
    pub user_info_url: Option<String>,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            client_id: "".to_string(),
            client_secret: "".to_string(),
            authorization_url: "".to_string(),
            token_url: "".to_string(),
            user_info_url: None,
        }
    }
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Enable CORS
    pub enabled: bool,

    /// Allowed origins
    pub allowed_origins: Vec<String>,

    /// Allowed methods
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    pub allowed_headers: Vec<String>,

    /// Allow credentials
    pub allow_credentials: bool,

    /// Max age
    pub max_age: Duration,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-API-Key".to_string(),
            ],
            allow_credentials: true,
            max_age: Duration::from_secs(3600),
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,

    /// Requests per second
    pub requests_per_second: u32,

    /// Burst size
    pub burst_size: u32,

    /// Rate limit window
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: 100,
            burst_size: 200,
            window_size: Duration::from_secs(60),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,

    /// Enable request logging
    pub enable_request_logging: bool,

    /// Enable response logging
    pub enable_response_logging: bool,

    /// Log format
    pub format: LogFormat,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            enable_request_logging: true,
            enable_response_logging: false,
            format: LogFormat::Json,
        }
    }
}

/// Log format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics
    pub enabled: bool,

    /// Metrics address
    pub address: String,

    /// Metrics port
    pub port: u16,

    /// Metrics path
    pub path: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            address: "0.0.0.0".to_string(),
            port: 9091,
            path: "/metrics".to_string(),
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable security headers
    pub enable_security_headers: bool,

    /// Content security policy
    pub content_security_policy: Option<String>,

    /// Strict transport security
    pub strict_transport_security: Option<String>,

    /// X-Frame-Options
    pub x_frame_options: Option<String>,

    /// X-Content-Type-Options
    pub x_content_type_options: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_security_headers: true,
            content_security_policy: Some("default-src 'self'".to_string()),
            strict_transport_security: Some("max-age=31536000; includeSubDomains".to_string()),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
        }
    }
}

impl BridgeAPIConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix("BRIDGE_API"))
            .build()?;

        settings.try_deserialize()
    }

    /// Get HTTP server address
    pub fn http_address(&self) -> String {
        format!("{}:{}", self.http.address, self.http.port)
    }

    /// Get gRPC server address
    pub fn grpc_address(&self) -> String {
        format!("{}:{}", self.grpc.address, self.grpc.port)
    }

    /// Get metrics server address
    pub fn metrics_address(&self) -> String {
        format!("{}:{}", self.metrics.address, self.metrics.port)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        let mut errors = Vec::new();

        // Validate HTTP configuration
        if let Err(e) = self.http.validate() {
            errors.push(format!("HTTP config: {}", e));
        }

        // Validate gRPC configuration
        if let Err(e) = self.grpc.validate() {
            errors.push(format!("gRPC config: {}", e));
        }

        // Validate authentication configuration
        if let Err(e) = self.auth.validate() {
            errors.push(format!("Auth config: {}", e));
        }

        // Validate rate limiting configuration
        if let Err(e) = self.rate_limit.validate() {
            errors.push(format!("Rate limit config: {}", e));
        }

        // Validate metrics configuration
        if let Err(e) = self.metrics.validate() {
            errors.push(format!("Metrics config: {}", e));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigValidationError::ValidationFailed(errors))
        }
    }

    /// Get configuration hash for change detection
    pub fn hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        format!("{:?}", self).hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

// Configuration validation errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigValidationError {
    #[error("Validation failed: {}", .0.join(", "))]
    ValidationFailed(Vec<String>),

    #[error("Invalid port number: {0}")]
    InvalidPort(u16),

    #[error("Invalid timeout: {0}")]
    InvalidTimeout(String),

    #[error("Invalid size: {0}")]
    InvalidSize(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid authentication configuration: {0}")]
    InvalidAuth(String),
}

// Validation implementations for sub-configs
impl HttpConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        // Validate port
        if self.port == 0 {
            return Err(ConfigValidationError::InvalidPort(self.port));
        }

        // Validate address
        if self.address.is_empty() {
            return Err(ConfigValidationError::MissingField("address".to_string()));
        }

        // Validate timeout
        if self.request_timeout.as_secs() == 0 {
            return Err(ConfigValidationError::InvalidTimeout(
                "request_timeout cannot be zero".to_string(),
            ));
        }

        // Validate max request size
        if self.max_request_size == 0 {
            return Err(ConfigValidationError::InvalidSize(
                "max_request_size cannot be zero".to_string(),
            ));
        }

        Ok(())
    }
}

impl GrpcConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        // Validate port
        if self.port == 0 {
            return Err(ConfigValidationError::InvalidPort(self.port));
        }

        // Validate address
        if self.address.is_empty() {
            return Err(ConfigValidationError::MissingField("address".to_string()));
        }

        // Validate max message size
        if self.max_message_size == 0 {
            return Err(ConfigValidationError::InvalidSize(
                "max_message_size cannot be zero".to_string(),
            ));
        }

        // Validate max concurrent streams
        if self.max_concurrent_streams == 0 {
            return Err(ConfigValidationError::InvalidSize(
                "max_concurrent_streams cannot be zero".to_string(),
            ));
        }

        Ok(())
    }
}

impl AuthConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if !self.enabled {
            return Ok(());
        }

        match self.auth_type {
            AuthType::ApiKey => {
                if self.api_key.valid_keys.is_empty() {
                    return Err(ConfigValidationError::InvalidAuth(
                        "API key authentication enabled but no valid keys provided".to_string(),
                    ));
                }
            }
            AuthType::Jwt => {
                if self.jwt.secret_key.is_empty() {
                    return Err(ConfigValidationError::InvalidAuth(
                        "JWT authentication enabled but no secret key provided".to_string(),
                    ));
                }
            }
            AuthType::OAuth => {
                if self.oauth.client_id.is_empty() || self.oauth.client_secret.is_empty() {
                    return Err(ConfigValidationError::InvalidAuth(
                        "OAuth authentication enabled but missing client_id or client_secret"
                            .to_string(),
                    ));
                }
            }
            AuthType::None => {
                return Err(ConfigValidationError::InvalidAuth(
                    "Authentication enabled but auth_type is None".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl RateLimitConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if !self.enabled {
            return Ok(());
        }

        if self.requests_per_second == 0 {
            return Err(ConfigValidationError::InvalidSize(
                "requests_per_second cannot be zero".to_string(),
            ));
        }

        if self.burst_size == 0 {
            return Err(ConfigValidationError::InvalidSize(
                "burst_size cannot be zero".to_string(),
            ));
        }

        Ok(())
    }
}

impl MetricsConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if !self.enabled {
            return Ok(());
        }

        if self.port == 0 {
            return Err(ConfigValidationError::InvalidPort(self.port));
        }

        if self.address.is_empty() {
            return Err(ConfigValidationError::MissingField("address".to_string()));
        }

        if self.path.is_empty() {
            return Err(ConfigValidationError::MissingField("path".to_string()));
        }

        Ok(())
    }
}

// Hot reloading support
/// Configuration manager with hot reloading support
pub struct ConfigManager {
    config: Arc<RwLock<BridgeAPIConfig>>,
    config_path: String,
    watchers: Arc<RwLock<HashMap<String, Box<dyn Fn(&BridgeAPIConfig) + Send + Sync>>>>,
    is_watching: Arc<RwLock<bool>>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config: BridgeAPIConfig, config_path: String) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
            watchers: Arc::new(RwLock::new(HashMap::new())),
            is_watching: Arc::new(RwLock::new(false)),
        }
    }

    /// Get current configuration
    pub async fn get_config(&self) -> BridgeAPIConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn update_config(
        &self,
        new_config: BridgeAPIConfig,
    ) -> Result<(), ConfigValidationError> {
        // Validate new configuration
        new_config.validate()?;

        // Update configuration
        {
            let mut config = self.config.write().await;
            *config = new_config;
        }

        // Notify watchers
        self.notify_watchers().await;

        info!("Configuration updated successfully");
        Ok(())
    }

    /// Load configuration from file
    pub async fn load_from_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = BridgeAPIConfig::from_file(&self.config_path)?;
        self.update_config(config).await?;
        Ok(())
    }

    /// Add configuration change watcher
    pub async fn add_watcher<F>(&self, name: String, callback: F)
    where
        F: Fn(&BridgeAPIConfig) + Send + Sync + 'static,
    {
        let mut watchers = self.watchers.write().await;
        watchers.insert(name, Box::new(callback));
    }

    /// Remove configuration change watcher
    pub async fn remove_watcher(&self, name: &str) {
        let mut watchers = self.watchers.write().await;
        watchers.remove(name);
    }

    /// Start watching for configuration file changes
    pub async fn start_watching(&self) {
        let mut is_watching = self.is_watching.write().await;
        if *is_watching {
            return;
        }
        *is_watching = true;
        drop(is_watching);

        let config_path = self.config_path.clone();
        let config_manager = self.clone();

        tokio::spawn(async move {
            let mut interval = interval(TokioDuration::from_secs(5));

            loop {
                interval.tick().await;

                if let Err(e) = config_manager.check_file_changes().await {
                    error!("Error checking configuration file changes: {}", e);
                }
            }
        });
    }

    /// Stop watching for configuration file changes
    pub async fn stop_watching(&self) {
        let mut is_watching = self.is_watching.write().await;
        *is_watching = false;
    }

    /// Check for configuration file changes
    async fn check_file_changes(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Read file metadata to check for changes
        let metadata = fs::metadata(&self.config_path).await?;
        let current_modified = metadata.modified()?;

        // Store last modified time (simplified - in production you'd want to persist this)
        static mut LAST_MODIFIED: Option<std::time::SystemTime> = None;

        unsafe {
            if let Some(last_modified) = LAST_MODIFIED {
                if current_modified > last_modified {
                    info!("Configuration file changed, reloading...");
                    self.load_from_file().await?;
                }
            }
            LAST_MODIFIED = Some(current_modified);
        }

        Ok(())
    }

    /// Notify all watchers of configuration changes
    async fn notify_watchers(&self) {
        let config = self.config.read().await;
        let watchers = self.watchers.read().await;

        for (name, callback) in watchers.iter() {
            info!("Notifying configuration watcher: {}", name);
            callback(&config);
        }
    }
}

impl Clone for ConfigManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            config_path: self.config_path.clone(),
            watchers: self.watchers.clone(),
            is_watching: self.is_watching.clone(),
        }
    }
}
