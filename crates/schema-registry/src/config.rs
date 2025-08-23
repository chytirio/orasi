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
    /// Configuration version
    pub version: String,
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
            version: "1.0.0".to_string(),
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

    /// Schema TTL in seconds (0 = no TTL)
    pub schema_ttl: u64,

    /// Enable Redis clustering
    pub enable_clustering: bool,

    /// Redis persistence mode
    pub persistence_mode: RedisPersistenceMode,

    /// Redis memory policy
    pub memory_policy: RedisMemoryPolicy,

    /// Enable Redis Sentinel
    pub enable_sentinel: bool,

    /// Sentinel master name
    pub sentinel_master_name: Option<String>,

    /// Sentinel hosts
    pub sentinel_hosts: Vec<String>,

    /// Enable SSL/TLS
    pub enable_ssl: bool,

    /// SSL certificate path
    pub ssl_cert_path: Option<String>,

    /// SSL key path
    pub ssl_key_path: Option<String>,

    /// SSL CA path
    pub ssl_ca_path: Option<String>,

    /// Enable Redis compression
    pub enable_compression: bool,

    /// Compression threshold in bytes
    pub compression_threshold: usize,
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
            read_timeout: 30,
            write_timeout: 30,
            key_prefix: "schema_registry:".to_string(),
            schema_ttl: 0, // No TTL by default
            enable_clustering: false,
            persistence_mode: RedisPersistenceMode::Rdb,
            memory_policy: RedisMemoryPolicy::AllKeysLru,
            enable_sentinel: false,
            sentinel_master_name: None,
            sentinel_hosts: Vec::new(),
            enable_ssl: false,
            ssl_cert_path: None,
            ssl_key_path: None,
            ssl_ca_path: None,
            enable_compression: false,
            compression_threshold: 1024,
        }
    }
}

/// Redis persistence mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedisPersistenceMode {
    /// No persistence
    None,
    /// RDB persistence
    Rdb,
    /// AOF persistence
    Aof,
    /// RDB + AOF persistence
    Both,
}

/// Redis memory policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedisMemoryPolicy {
    /// No eviction
    NoEviction,
    /// All keys LRU
    AllKeysLru,
    /// Volatile LRU
    VolatileLru,
    /// All keys random
    AllKeysRandom,
    /// Volatile random
    VolatileRandom,
    /// Volatile TTL
    VolatileTtl,
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

    /// Enable response caching
    pub enable_cache: bool,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,

    /// Maximum cache entries
    pub cache_max_entries: usize,

    /// API versioning configuration
    pub versioning: ApiVersioningConfig,
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
            enable_cache: true,
            cache_ttl_seconds: 30,
            cache_max_entries: 1024,
            versioning: ApiVersioningConfig::default(),
        }
    }
}

/// API versioning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVersioningConfig {
    /// Enable API versioning
    pub enabled: bool,

    /// Supported API versions
    pub supported_versions: Vec<String>,

    /// Default API version
    pub default_version: String,

    /// Whether to allow version negotiation
    pub allow_version_negotiation: bool,

    /// Version deprecation warnings
    pub deprecated_versions: HashMap<String, String>,

    /// Version compatibility matrix
    pub compatibility_matrix: HashMap<String, Vec<String>>,
}

impl Default for ApiVersioningConfig {
    fn default() -> Self {
        let mut deprecated_versions = HashMap::new();
        deprecated_versions.insert(
            "v1.0.0".to_string(),
            "API v1.0.0 is deprecated. Please upgrade to v2.0.0.".to_string(),
        );

        let mut compatibility_matrix = HashMap::new();
        compatibility_matrix.insert(
            "v1.0.0".to_string(),
            vec!["v1.0.0".to_string()],
        );
        compatibility_matrix.insert(
            "v2.0.0".to_string(),
            vec!["v2.0.0".to_string()],
        );

        Self {
            enabled: true,
            supported_versions: vec!["v1.0.0".to_string(), "v2.0.0".to_string()],
            default_version: "v2.0.0".to_string(),
            allow_version_negotiation: true,
            deprecated_versions,
            compatibility_matrix,
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

    /// Enable authorization (RBAC)
    pub enable_authorization: bool,

    /// Map of API key -> permissions
    /// Example: { "key1": ["schemas:read", "schemas:write"], "key2": ["schemas:read"] }
    pub api_key_permissions: std::collections::HashMap<String, Vec<String>>,

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
            enable_authorization: false,
            api_key_permissions: std::collections::HashMap::new(),
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

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFormat::Json => write!(f, "json"),
            LogFormat::Text => write!(f, "text"),
        }
    }
}

impl SchemaRegistryConfig {
    /// Load configuration from file
    pub fn from_file(path: &PathBuf) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .add_source(config::Environment::with_prefix("SCHEMA_REGISTRY"))
            .build()?;

        settings.try_deserialize().map_err(|e| {
            // Enhance error message with config file path for common error types
            match e {
                config::ConfigError::NotFound(key) => config::ConfigError::NotFound(format!(
                    "{} (in config file: {})",
                    key,
                    path.display()
                )),
                config::ConfigError::FileParse { uri, cause } => config::ConfigError::FileParse {
                    uri: Some(format!(
                        "{} (config file: {})",
                        uri.as_deref().unwrap_or("unknown"),
                        path.display()
                    )),
                    cause,
                },
                _ => e,
            }
        })
    }

    /// Load configuration from multiple sources with precedence
    pub fn from_sources(
        config_file: Option<&PathBuf>,
        env_prefix: &str,
    ) -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder();

        // Add defaults first
        builder = builder.add_source(config::File::from_str(
            &Self::generate_example(),
            config::FileFormat::Toml,
        ));

        // Add config file if provided
        if let Some(path) = config_file {
            builder = builder.add_source(config::File::from(path.as_ref()));
        }

        // Add environment variables with separator
        builder = builder.add_source(
            config::Environment::with_prefix(env_prefix)
                .separator("__")
                .try_parsing(true),
        );

        let settings = builder.build()?;
        settings.try_deserialize()
    }

    /// Load configuration with defaults
    pub fn load_with_defaults() -> Result<Self, config::ConfigError> {
        // Try to load from common config file locations
        let config_paths = vec![
            PathBuf::from("config/schema-registry.toml"),
            PathBuf::from("schema-registry.toml"),
            PathBuf::from("config/schema-registry.yaml"),
            PathBuf::from("schema-registry.yaml"),
            PathBuf::from("config/schema-registry.json"),
            PathBuf::from("schema-registry.json"),
        ];

        for path in config_paths {
            if path.exists() {
                return Self::from_file(&path);
            }
        }

        // If no config file found, load from environment variables with defaults
        Self::from_sources(None, "SCHEMA_REGISTRY")
    }

    /// Generate example configuration
    pub fn generate_example() -> String {
        r#"# Schema Registry Configuration Example
# This file shows all available configuration options

version = "1.0.0"

[storage]
# Storage backend type: Memory, Sqlite, Postgres, Redis
backend = "Memory"
connection_string = "memory://"
max_connections = 10
connection_timeout = 30
options = {}

[storage.postgres]
url = "postgresql://localhost:5432/schema_registry"
database = "schema_registry"
username = "postgres"
password = ""
host = "localhost"
port = 5432
ssl_mode = "disable"
pool_size = 10

[storage.sqlite]
database_path = "schema_registry.db"
enable_wal = true
connection_timeout = 30
journal_mode = "WAL"

[storage.redis]
url = "redis://localhost:6379"
host = "localhost"
port = 6379
password = ""
database = 0
pool_size = 10
connection_timeout = 30
read_timeout = 10
write_timeout = 10
key_prefix = "schema_registry:"

[api]
host = "127.0.0.1"
port = 8080
base_path = "/api/v1"
max_request_size = 10485760  # 10MB
request_timeout = 30
enable_cors = true
cors_origins = ["*"]

[validation]
enable_validation = true
strict_mode = false
max_schema_size = 1048576  # 1MB
allowed_formats = ["Json", "Yaml"]
custom_rules = {}

[security]
enable_auth = false
auth_type = "None"  # None, ApiKey, Jwt, OAuth2
api_key_header = "X-API-Key"
allowed_api_keys = ["test-api-key"]
enable_authorization = false
api_key_permissions = {}
rate_limiting = false
rate_limit_per_minute = 1000

[monitoring]
enable_metrics = true
metrics_endpoint = "/metrics"
enable_health_checks = true
health_check_endpoint = "/health"
enable_structured_logging = true
log_level = "info"
log_format = "Json"
"#
        .to_string()
    }

    /// Get configuration documentation
    pub fn get_documentation() -> String {
        r#"# Schema Registry Configuration Documentation

## Overview
The Schema Registry can be configured using TOML, YAML, or JSON files, as well as environment variables.

## Configuration Sources (in order of precedence)
1. Environment variables (prefixed with SCHEMA_REGISTRY_)
2. Configuration file
3. Default values

## Environment Variables
All configuration options can be set using environment variables with the SCHEMA_REGISTRY_ prefix:
- SCHEMA_REGISTRY_API_PORT=8080
- SCHEMA_REGISTRY_STORAGE_BACKEND=Postgres
- SCHEMA_REGISTRY_SECURITY_ENABLE_AUTH=true

## Storage Configuration
### Backend Types
- **Memory**: In-memory storage (for testing)
- **Sqlite**: SQLite database storage
- **Postgres**: PostgreSQL database storage
- **Redis**: Redis key-value storage

### Connection Strings
- Memory: "memory://"
- SQLite: "sqlite:///path/to/database.db"
- PostgreSQL: "postgresql://user:pass@host:port/database"
- Redis: "redis://host:port/database"

## API Configuration
- **host**: API server host (default: 127.0.0.1)
- **port**: API server port (default: 8080)
- **base_path**: API base path (default: /api/v1)
- **max_request_size**: Maximum request size in bytes (default: 10MB)
- **request_timeout**: Request timeout in seconds (default: 30)
- **enable_cors**: Enable CORS (default: true)
- **cors_origins**: Allowed CORS origins (default: ["*"])

## Security Configuration
- **enable_auth**: Enable authentication (default: false)
- **auth_type**: Authentication type (None, ApiKey, Jwt, OAuth2)
- **api_key_header**: API key header name (default: X-API-Key)
- **allowed_api_keys**: List of valid API keys
- **enable_authorization**: Enable authorization (RBAC) (default: false)
- **api_key_permissions**: Map of API key -> permissions (default: {})
- **rate_limiting**: Enable rate limiting (default: false)
- **rate_limit_per_minute**: Rate limit requests per minute (default: 1000)

## Validation Configuration
- **enable_validation**: Enable schema validation (default: true)
- **strict_mode**: Enable strict validation mode (default: false)
- **max_schema_size**: Maximum schema size in bytes (default: 1MB)
- **allowed_formats**: Allowed schema formats (Json, Yaml, Avro, Protobuf, OpenApi)

## Monitoring Configuration
- **enable_metrics**: Enable Prometheus metrics (default: true)
- **metrics_endpoint**: Metrics endpoint (default: /metrics)
- **enable_health_checks**: Enable health checks (default: true)
- **health_check_endpoint**: Health check endpoint (default: /health)
- **enable_structured_logging**: Enable structured logging (default: true)
- **log_level**: Log level (debug, info, warn, error) (default: info)
- **log_format**: Log format (Json, Text) (default: Json)

## Examples

### Basic Configuration (TOML)
```toml
[api]
host = "0.0.0.0"
port = 8080

[storage]
backend = "Memory"
```

### Production Configuration (YAML)
```yaml
storage:
  backend: Postgres
  postgres:
    url: "postgresql://user:pass@localhost:5432/schema_registry"
    pool_size: 20

security:
  enable_auth: true
  auth_type: ApiKey
  allowed_api_keys:
    - "prod-api-key-1"
    - "prod-api-key-2"
  rate_limiting: true
  rate_limit_per_minute: 100

monitoring:
  log_level: "info"
  enable_metrics: true
```

### Environment Variables Only
```bash
export SCHEMA_REGISTRY_API_HOST=0.0.0.0
export SCHEMA_REGISTRY_API_PORT=8080
export SCHEMA_REGISTRY_STORAGE_BACKEND=Postgres
export SCHEMA_REGISTRY_STORAGE_POSTGRES_URL="postgresql://user:pass@localhost:5432/schema_registry"
export SCHEMA_REGISTRY_SECURITY_ENABLE_AUTH=true
export SCHEMA_REGISTRY_SECURITY_AUTH_TYPE=ApiKey
export SCHEMA_REGISTRY_SECURITY_ALLOWED_API_KEYS="key1,key2,key3"
```
"#.to_string()
    }

    /// Validate configuration with detailed error reporting
    pub fn validate(&self) -> Result<(), String> {
        let mut errors = Vec::new();

        // Validate storage configuration
        self.validate_storage(&mut errors);

        // Validate API configuration
        self.validate_api(&mut errors);

        // Validate validation configuration
        self.validate_validation_config(&mut errors);

        // Validate security configuration
        self.validate_security(&mut errors);

        // Validate monitoring configuration
        self.validate_monitoring(&mut errors);

        // Validate versioning configuration
        self.validate_versioning(&mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Configuration validation failed with {} error(s):\n{}",
                errors.len(),
                errors.join("\n")
            ))
        }
    }

    /// Validate storage configuration
    fn validate_storage(&self, errors: &mut Vec<String>) {
        // Validate connection string
        if self.storage.connection_string.is_empty() {
            errors.push("storage.connection_string: cannot be empty".to_string());
        }

        // Validate max connections
        if self.storage.max_connections == 0 {
            errors.push("storage.max_connections: must be greater than 0".to_string());
        }

        // Validate connection timeout
        if self.storage.connection_timeout == 0 {
            errors.push("storage.connection_timeout: must be greater than 0".to_string());
        }

        // Validate backend-specific configuration
        match self.storage.backend {
            StorageBackendType::Postgres => {
                self.validate_postgres_config(errors);
            }
            StorageBackendType::Sqlite => {
                self.validate_sqlite_config(errors);
            }
            StorageBackendType::Redis => {
                self.validate_redis_config(errors);
            }
            StorageBackendType::Memory => {
                // Memory backend doesn't need additional validation
            }
        }
    }

    /// Validate PostgreSQL configuration
    fn validate_postgres_config(&self, errors: &mut Vec<String>) {
        let pg = &self.storage.postgres;

        if pg.url.is_empty() {
            errors.push("storage.postgres.url: cannot be empty".to_string());
        }

        if pg.database.is_empty() {
            errors.push("storage.postgres.database: cannot be empty".to_string());
        }

        if pg.username.is_empty() {
            errors.push("storage.postgres.username: cannot be empty".to_string());
        }

        if pg.host.is_empty() {
            errors.push("storage.postgres.host: cannot be empty".to_string());
        }

        if pg.port == 0 {
            errors.push("storage.postgres.port: must be greater than 0".to_string());
        }

        if pg.pool_size == 0 {
            errors.push("storage.postgres.pool_size: must be greater than 0".to_string());
        }

        // Validate SSL mode
        let valid_ssl_modes = ["disable", "require", "verify-ca", "verify-full"];
        if !valid_ssl_modes.contains(&pg.ssl_mode.as_str()) {
            errors.push(format!(
                "storage.postgres.ssl_mode: must be one of {:?}",
                valid_ssl_modes
            ));
        }
    }

    /// Validate SQLite configuration
    fn validate_sqlite_config(&self, errors: &mut Vec<String>) {
        let sqlite = &self.storage.sqlite;

        if sqlite.database_path.to_string_lossy().is_empty() {
            errors.push("storage.sqlite.database_path: cannot be empty".to_string());
        }

        if sqlite.connection_timeout == 0 {
            errors.push("storage.sqlite.connection_timeout: must be greater than 0".to_string());
        }

        // Validate journal mode
        let valid_journal_modes = ["DELETE", "WAL", "PERSIST", "MEMORY", "OFF"];
        if !valid_journal_modes.contains(&sqlite.journal_mode.as_str()) {
            errors.push(format!(
                "storage.sqlite.journal_mode: must be one of {:?}",
                valid_journal_modes
            ));
        }
    }

    /// Validate Redis configuration
    fn validate_redis_config(&self, errors: &mut Vec<String>) {
        let redis = &self.storage.redis;

        if redis.url.is_empty() {
            errors.push("storage.redis.url: cannot be empty".to_string());
        }

        if redis.host.is_empty() {
            errors.push("storage.redis.host: cannot be empty".to_string());
        }

        if redis.port == 0 {
            errors.push("storage.redis.port: must be greater than 0".to_string());
        }

        if redis.pool_size == 0 {
            errors.push("storage.redis.pool_size: must be greater than 0".to_string());
        }

        if redis.connection_timeout == 0 {
            errors.push("storage.redis.connection_timeout: must be greater than 0".to_string());
        }

        if redis.read_timeout == 0 {
            errors.push("storage.redis.read_timeout: must be greater than 0".to_string());
        }

        if redis.write_timeout == 0 {
            errors.push("storage.redis.write_timeout: must be greater than 0".to_string());
        }

        if redis.key_prefix.is_empty() {
            errors.push("storage.redis.key_prefix: cannot be empty".to_string());
        }

        // Validate SSL configuration
        if redis.enable_ssl {
            if redis.ssl_cert_path.is_none() {
                errors
                    .push("storage.redis.ssl_cert_path: required when SSL is enabled".to_string());
            }
            if redis.ssl_key_path.is_none() {
                errors.push("storage.redis.ssl_key_path: required when SSL is enabled".to_string());
            }
        }

        // Validate Sentinel configuration
        if redis.enable_sentinel {
            if redis.sentinel_master_name.is_none() {
                errors.push(
                    "storage.redis.sentinel_master_name: required when Sentinel is enabled"
                        .to_string(),
                );
            }
            if redis.sentinel_hosts.is_empty() {
                errors.push("storage.redis.sentinel_hosts: must contain at least one host when Sentinel is enabled".to_string());
            }
        }

        // Validate compression configuration
        if redis.enable_compression && redis.compression_threshold == 0 {
            errors.push("storage.redis.compression_threshold: must be greater than 0 when compression is enabled".to_string());
        }
    }

    /// Validate API configuration
    fn validate_api(&self, errors: &mut Vec<String>) {
        if self.api.host.is_empty() {
            errors.push("api.host: cannot be empty".to_string());
        }

        if self.api.port == 0 {
            errors.push("api.port: must be greater than 0".to_string());
        }

        if self.api.port > 65535 {
            errors.push("api.port: must be less than or equal to 65535".to_string());
        }

        if self.api.base_path.is_empty() {
            errors.push("api.base_path: cannot be empty".to_string());
        }

        if !self.api.base_path.starts_with('/') {
            errors.push("api.base_path: must start with '/'".to_string());
        }

        if self.api.max_request_size == 0 {
            errors.push("api.max_request_size: must be greater than 0".to_string());
        }

        if self.api.request_timeout == 0 {
            errors.push("api.request_timeout: must be greater than 0".to_string());
        }

        if self.api.cache_ttl_seconds == 0 {
            errors.push("api.cache_ttl_seconds: must be greater than 0".to_string());
        }

        if self.api.cache_max_entries == 0 {
            errors.push("api.cache_max_entries: must be greater than 0".to_string());
        }

        // Validate CORS configuration
        if self.api.enable_cors && self.api.cors_origins.is_empty() {
            errors.push(
                "api.cors_origins: must contain at least one origin when CORS is enabled"
                    .to_string(),
            );
        }
    }

    /// Validate validation configuration
    fn validate_validation_config(&self, errors: &mut Vec<String>) {
        if self.validation.max_schema_size == 0 {
            errors.push("validation.max_schema_size: must be greater than 0".to_string());
        }

        if self.validation.allowed_formats.is_empty() {
            errors.push("validation.allowed_formats: must contain at least one format".to_string());
        }
    }

    /// Validate security configuration
    fn validate_security(&self, errors: &mut Vec<String>) {
        if self.security.enable_auth {
            match self.security.auth_type {
                AuthType::Jwt => {
                    if self.security.jwt_secret.is_none() {
                        errors.push(
                            "security.jwt_secret: required when JWT auth is enabled".to_string(),
                        );
                    }
                }
                AuthType::ApiKey => {
                    if self.security.allowed_api_keys.is_empty() {
                        errors.push("security.allowed_api_keys: must contain at least one key when API key auth is enabled".to_string());
                    }
                }
                AuthType::OAuth2 => {
                    // OAuth2 configuration validation would go here if implemented
                    // For now, just note that OAuth2 is enabled
                }
                AuthType::None => {
                    errors.push(
                        "security.auth_type: cannot be 'None' when auth is enabled".to_string(),
                    );
                }
            }
        }

        if self.security.rate_limiting {
            if self.security.rate_limit_per_minute == 0 {
                errors.push("security.rate_limit_per_minute: must be greater than 0 when rate limiting is enabled".to_string());
            }
        }

        if self.security.enable_authorization && !self.security.enable_auth {
            errors.push(
                "security.enable_authorization: requires authentication to be enabled".to_string(),
            );
        }
    }

    /// Validate monitoring configuration
    fn validate_monitoring(&self, errors: &mut Vec<String>) {
        // Validate log level
        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&self.monitoring.log_level.to_lowercase().as_str()) {
            errors.push(format!(
                "monitoring.log_level: must be one of {:?}",
                valid_log_levels
            ));
        }

        // Validate log format
        let valid_log_formats = ["text", "json"];
        if !valid_log_formats.contains(&self.monitoring.log_format.to_string().as_str()) {
            errors.push(format!(
                "monitoring.log_format: must be one of {:?}",
                valid_log_formats
            ));
        }

        // Validate metrics configuration
        if self.monitoring.enable_metrics {
            if self.monitoring.metrics_endpoint.is_empty() {
                errors.push(
                    "monitoring.metrics_endpoint: cannot be empty when metrics are enabled"
                        .to_string(),
                );
            }
            if !self.monitoring.metrics_endpoint.starts_with('/') {
                errors.push("monitoring.metrics_endpoint: must start with '/'".to_string());
            }
        }

        // Validate health check configuration
        if self.monitoring.enable_health_checks {
            if self.monitoring.health_check_endpoint.is_empty() {
                errors.push("monitoring.health_check_endpoint: cannot be empty when health checks are enabled".to_string());
            }
            if !self.monitoring.health_check_endpoint.starts_with('/') {
                errors.push("monitoring.health_check_endpoint: must start with '/'".to_string());
            }
        }
    }

    /// Validate versioning configuration
    fn validate_versioning(&self, errors: &mut Vec<String>) {
        let versioning = &self.api.versioning;

        if versioning.enabled {
            if versioning.supported_versions.is_empty() {
                errors.push("api.versioning.supported_versions: must contain at least one version when versioning is enabled".to_string());
            }

            if versioning.default_version.is_empty() {
                errors.push(
                    "api.versioning.default_version: cannot be empty when versioning is enabled"
                        .to_string(),
                );
            }

            if !versioning
                .supported_versions
                .contains(&versioning.default_version)
            {
                errors.push(
                    "api.versioning.default_version: must be one of the supported versions"
                        .to_string(),
                );
            }

            // Validate deprecated versions
            for deprecated_version in versioning.deprecated_versions.keys() {
                if !versioning.supported_versions.contains(deprecated_version) {
                    errors.push(format!("api.versioning.deprecated_versions: version '{}' is not in supported_versions", deprecated_version));
                }
            }

            // Validate compatibility matrix
            for (version, compatible_versions) in &versioning.compatibility_matrix {
                if !versioning.supported_versions.contains(version) {
                    errors.push(format!("api.versioning.compatibility_matrix: version '{}' is not in supported_versions", version));
                }

                for compatible_version in compatible_versions {
                    if !versioning.supported_versions.contains(compatible_version) {
                        errors.push(format!("api.versioning.compatibility_matrix: compatible version '{}' for '{}' is not in supported_versions", compatible_version, version));
                    }
                }
            }
        }
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
