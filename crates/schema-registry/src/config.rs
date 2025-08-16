//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration management for the Schema Registry
//!
//! This module provides configuration structures and validation for
//! the schema registry service.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Schema Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistryConfig {
    /// Storage configuration
    pub storage: StorageConfig,

    /// API configuration
    pub api: ApiConfig,

    /// Validation configuration
    pub validation: ValidationConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

impl Default for SchemaRegistryConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            api: ApiConfig::default(),
            validation: ValidationConfig::default(),
            security: SecurityConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: StorageBackendType,

    /// Storage connection string or path
    pub connection_string: String,

    /// Storage options
    pub options: HashMap<String, String>,

    /// Maximum number of connections
    pub max_connections: u32,

    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// PostgreSQL specific configuration
    pub postgres: PostgresConfig,

    /// SQLite specific configuration
    pub sqlite: SqliteConfig,

    /// Redis specific configuration
    pub redis: RedisConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackendType::Memory,
            connection_string: "memory://".to_string(),
            options: HashMap::new(),
            max_connections: 10,
            connection_timeout: 30,
            postgres: PostgresConfig::default(),
            sqlite: SqliteConfig::default(),
            redis: RedisConfig::default(),
        }
    }
}

/// PostgreSQL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Database URL
    pub url: String,
    
    /// Database name
    pub database: String,
    
    /// Username
    pub username: String,
    
    /// Password
    pub password: String,
    
    /// Host
    pub host: String,
    
    /// Port
    pub port: u16,
    
    /// SSL mode
    pub ssl_mode: String,
    
    /// Connection pool size
    pub pool_size: u32,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost:5432/schema_registry".to_string(),
            database: "schema_registry".to_string(),
            username: "postgres".to_string(),
            password: "".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            ssl_mode: "disable".to_string(),
            pool_size: 10,
        }
    }
}

/// SQLite configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// Database path
    pub database_path: std::path::PathBuf,
    
    /// Enable WAL mode
    pub enable_wal: bool,
    
    /// Connection timeout
    pub connection_timeout: u64,
    
    /// Journal mode
    pub journal_mode: String,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            database_path: std::path::PathBuf::from("schema_registry.db"),
            enable_wal: true,
            connection_timeout: 30,
            journal_mode: "WAL".to_string(),
        }
    }
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL
    pub url: String,
    
    /// Redis host
    pub host: String,
    
    /// Redis port
    pub port: u16,
    
    /// Redis password
    pub password: Option<String>,
    
    /// Redis database number
    pub database: u8,
    
    /// Connection pool size
    pub pool_size: u32,
    
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    
    /// Read timeout in seconds
    pub read_timeout: u64,
    
    /// Write timeout in seconds
    pub write_timeout: u64,
    
    /// Key prefix for schema storage
    pub key_prefix: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            host: "localhost".to_string(),
            port: 6379,
            password: None,
            database: 0,
            pool_size: 10,
            connection_timeout: 30,
            read_timeout: 10,
            write_timeout: 10,
            key_prefix: "schema_registry:".to_string(),
        }
    }
}

/// Storage backend types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StorageBackendType {
    /// In-memory storage (for testing)
    Memory,

    /// SQLite storage
    Sqlite,

    /// PostgreSQL storage
    Postgres,

    /// Redis storage
    Redis,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API host
    pub host: String,

    /// API port
    pub port: u16,

    /// API base path
    pub base_path: String,

    /// Maximum request size in bytes
    pub max_request_size: usize,

    /// Request timeout in seconds
    pub request_timeout: u64,

    /// Enable CORS
    pub enable_cors: bool,

    /// CORS origins
    pub cors_origins: Vec<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            base_path: "/api/v1".to_string(),
            max_request_size: 10 * 1024 * 1024, // 10MB
            request_timeout: 30,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable schema validation
    pub enable_validation: bool,

    /// Strict validation mode
    pub strict_mode: bool,

    /// Maximum schema size in bytes
    pub max_schema_size: usize,

    /// Allowed schema formats
    pub allowed_formats: Vec<SchemaFormat>,

    /// Custom validation rules
    pub custom_rules: HashMap<String, String>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enable_validation: true,
            strict_mode: false,
            max_schema_size: 1024 * 1024, // 1MB
            allowed_formats: vec![SchemaFormat::Json, SchemaFormat::Yaml],
            custom_rules: HashMap::new(),
        }
    }
}

/// Schema formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaFormat {
    /// JSON Schema
    Json,

    /// YAML Schema
    Yaml,

    /// Avro Schema
    Avro,

    /// Protocol Buffers
    Protobuf,

    /// OpenAPI/Swagger
    OpenApi,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub enable_auth: bool,

    /// Authentication type
    pub auth_type: AuthType,

    /// JWT secret key
    pub jwt_secret: Option<String>,

    /// API key header name
    pub api_key_header: String,

    /// Allowed API keys
    pub allowed_api_keys: Vec<String>,

    /// Rate limiting enabled
    pub rate_limiting: bool,

    /// Rate limit requests per minute
    pub rate_limit_per_minute: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_auth: false,
            auth_type: AuthType::None,
            jwt_secret: None,
            api_key_header: "X-API-Key".to_string(),
            allowed_api_keys: Vec::new(),
            rate_limiting: false,
            rate_limit_per_minute: 1000,
        }
    }
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    /// No authentication
    None,

    /// API key authentication
    ApiKey,

    /// JWT authentication
    Jwt,

    /// OAuth2 authentication
    OAuth2,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics
    pub enable_metrics: bool,

    /// Metrics endpoint
    pub metrics_endpoint: String,

    /// Enable health checks
    pub enable_health_checks: bool,

    /// Health check endpoint
    pub health_check_endpoint: String,

    /// Enable structured logging
    pub enable_structured_logging: bool,

    /// Log level
    pub log_level: String,

    /// Log format
    pub log_format: LogFormat,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            metrics_endpoint: "/metrics".to_string(),
            enable_health_checks: true,
            health_check_endpoint: "/health".to_string(),
            enable_structured_logging: true,
            log_level: "info".to_string(),
            log_format: LogFormat::Json,
        }
    }
}

/// Log format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    /// JSON format
    Json,

    /// Text format
    Text,
}

impl SchemaRegistryConfig {
    /// Load configuration from file
    pub fn from_file(path: &PathBuf) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .add_source(config::Environment::with_prefix("SCHEMA_REGISTRY"))
            .build()?;

        settings.try_deserialize()
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate storage configuration
        if self.storage.connection_string.is_empty() {
            return Err("Storage connection string cannot be empty".to_string());
        }

        // Validate API configuration
        if self.api.port == 0 {
            return Err("API port cannot be 0".to_string());
        }

        if self.api.max_request_size == 0 {
            return Err("Max request size cannot be 0".to_string());
        }

        // Validate validation configuration
        if self.validation.max_schema_size == 0 {
            return Err("Max schema size cannot be 0".to_string());
        }

        // Validate security configuration
        if self.security.enable_auth {
            match self.security.auth_type {
                AuthType::Jwt => {
                    if self.security.jwt_secret.is_none() {
                        return Err("JWT secret is required when JWT auth is enabled".to_string());
                    }
                }
                AuthType::ApiKey => {
                    if self.security.allowed_api_keys.is_empty() {
                        return Err(
                            "At least one API key is required when API key auth is enabled"
                                .to_string(),
                        );
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SchemaRegistryConfig::default();
        assert_eq!(config.api.port, 8080);
        assert_eq!(config.storage.backend, StorageBackendType::Memory);
        assert!(config.validation.enable_validation);
    }

    #[test]
    fn test_config_validation() {
        let mut config = SchemaRegistryConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid port
        config.api.port = 0;
        assert!(config.validate().is_err());

        // Test invalid max request size
        config.api.port = 8080;
        config.api.max_request_size = 0;
        assert!(config.validate().is_err());
    }
}
