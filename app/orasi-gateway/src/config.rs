//! Configuration for Orasi Gateway

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the Orasi Gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Gateway identifier
    pub gateway_id: String,
    
    /// Gateway endpoint for external traffic
    pub gateway_endpoint: String,
    
    /// Health check endpoint
    pub health_endpoint: String,
    
    /// Metrics endpoint
    pub metrics_endpoint: String,
    
    /// Admin endpoint
    pub admin_endpoint: String,
    
    /// Service discovery settings
    pub service_discovery: ServiceDiscoveryConfig,
    
    /// Load balancing settings
    pub load_balancing: LoadBalancingConfig,
    
    /// Routing settings
    pub routing: RoutingConfig,
    
    /// Security settings
    pub security: SecurityConfig,
    
    /// Rate limiting settings
    pub rate_limiting: RateLimitingConfig,
    
    /// TLS settings
    pub tls: TlsConfig,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            gateway_id: uuid::Uuid::new_v4().to_string(),
            gateway_endpoint: "0.0.0.0:8080".to_string(),
            health_endpoint: "0.0.0.0:8081".to_string(),
            metrics_endpoint: "0.0.0.0:9090".to_string(),
            admin_endpoint: "0.0.0.0:8082".to_string(),
            service_discovery: ServiceDiscoveryConfig::default(),
            load_balancing: LoadBalancingConfig::default(),
            routing: RoutingConfig::default(),
            security: SecurityConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
            tls: TlsConfig::default(),
        }
    }
}

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryConfig {
    /// Service discovery backend (etcd, consul, etc.)
    pub backend: ServiceDiscoveryBackend,
    
    /// Service discovery endpoints
    pub endpoints: Vec<String>,
    
    /// Service refresh interval
    pub refresh_interval: Duration,
    
    /// Service health check interval
    pub health_check_interval: Duration,
}

impl Default for ServiceDiscoveryConfig {
    fn default() -> Self {
        Self {
            backend: ServiceDiscoveryBackend::Etcd,
            endpoints: vec!["http://localhost:2379".to_string()],
            refresh_interval: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
        }
    }
}

/// Service discovery backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceDiscoveryBackend {
    Etcd,
    Consul,
    Static,
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// Load balancing algorithm
    pub algorithm: LoadBalancingAlgorithm,
    
    /// Health check settings
    pub health_check: HealthCheckConfig,
    
    /// Circuit breaker settings
    pub circuit_breaker: CircuitBreakerConfig,
    
    /// Retry settings
    pub retry: RetryConfig,
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            algorithm: LoadBalancingAlgorithm::RoundRobin,
            health_check: HealthCheckConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
            retry: RetryConfig::default(),
        }
    }
}

/// Load balancing algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    IpHash,
    Random,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    pub enabled: bool,
    
    /// Health check path
    pub path: String,
    
    /// Health check interval
    pub interval: Duration,
    
    /// Health check timeout
    pub timeout: Duration,
    
    /// Unhealthy threshold
    pub unhealthy_threshold: u32,
    
    /// Healthy threshold
    pub healthy_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: "/health".to_string(),
            interval: Duration::from_secs(10),
            timeout: Duration::from_secs(5),
            unhealthy_threshold: 3,
            healthy_threshold: 2,
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    
    /// Failure threshold
    pub failure_threshold: u32,
    
    /// Recovery timeout
    pub recovery_timeout: Duration,
    
    /// Half-open state max requests
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Enable retries
    pub enabled: bool,
    
    /// Maximum retry attempts
    pub max_attempts: u32,
    
    /// Retry delay
    pub delay: Duration,
    
    /// Retry backoff multiplier
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
        }
    }
}

/// Routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Default service
    pub default_service: String,
    
    /// Route rules
    pub routes: Vec<RouteRule>,
    
    /// Request timeout
    pub request_timeout: Duration,
    
    /// Max request size
    pub max_request_size: usize,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            default_service: "orasi-api".to_string(),
            routes: Vec::new(),
            request_timeout: Duration::from_secs(30),
            max_request_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Route rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    /// Route path pattern
    pub path: String,
    
    /// Target service
    pub service: String,
    
    /// HTTP methods
    pub methods: Vec<String>,
    
    /// Route priority
    pub priority: u32,
    
    /// Route metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable CORS
    pub enable_cors: bool,
    
    /// CORS origins
    pub cors_origins: Vec<String>,
    
    /// Enable authentication
    pub enable_auth: bool,
    
    /// Authentication providers
    pub auth_providers: Vec<String>,
    
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            enable_auth: false,
            auth_providers: Vec::new(),
            enable_rate_limiting: true,
        }
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

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 1000,
            burst_size: 100,
            limit_by_ip: true,
            limit_by_user: false,
        }
    }
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,
    
    /// Certificate file path
    pub cert_file: Option<String>,
    
    /// Private key file path
    pub key_file: Option<String>,
    
    /// CA certificate file path
    pub ca_file: Option<String>,
    
    /// TLS version
    pub min_tls_version: TlsVersion,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_file: None,
            key_file: None,
            ca_file: None,
            min_tls_version: TlsVersion::Tls12,
        }
    }
}

/// TLS version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}
