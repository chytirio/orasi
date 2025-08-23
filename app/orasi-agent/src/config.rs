//! Configuration for Orasi Agent

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for the Orasi Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent identifier
    pub agent_id: String,

    /// Agent endpoint for receiving tasks
    pub agent_endpoint: String,

    /// Health check endpoint
    pub health_endpoint: String,

    /// Metrics endpoint
    pub metrics_endpoint: String,

    /// Cluster coordination endpoint
    pub cluster_endpoint: String,

    /// Cluster coordination settings
    pub cluster: ClusterConfig,

    /// Processing capabilities
    pub capabilities: CapabilitiesConfig,

    /// Task processing settings
    pub processing: ProcessingConfig,

    /// Storage settings
    pub storage: StorageConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent_id: uuid::Uuid::new_v4().to_string(),
            agent_endpoint: "0.0.0.0:8082".to_string(),
            health_endpoint: "0.0.0.0:8083".to_string(),
            metrics_endpoint: "0.0.0.0:9092".to_string(),
            cluster_endpoint: "0.0.0.0:8084".to_string(),
            cluster: ClusterConfig::default(),
            capabilities: CapabilitiesConfig::default(),
            processing: ProcessingConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

/// Cluster coordination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Service discovery backend (etcd, consul, etc.)
    pub service_discovery: ServiceDiscoveryBackend,

    /// etcd endpoints for service discovery
    pub etcd_endpoints: Vec<String>,

    /// Consul URL for service discovery
    pub consul_url: String,

    /// Static services for service discovery
    pub static_services: Vec<StaticServiceConfig>,

    /// Service discovery endpoints
    pub service_discovery_endpoints: Vec<String>,

    /// Heartbeat interval
    pub heartbeat_interval: Duration,

    /// Session timeout
    pub session_timeout: Duration,

    /// Leader election settings
    pub leader_election: LeaderElectionConfig,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            service_discovery: ServiceDiscoveryBackend::Etcd,
            etcd_endpoints: vec!["http://localhost:2379".to_string()],
            consul_url: "http://localhost:8500".to_string(),
            static_services: Vec::new(),
            service_discovery_endpoints: vec!["http://localhost:2379".to_string()],
            heartbeat_interval: Duration::from_secs(30),
            session_timeout: Duration::from_secs(90),
            leader_election: LeaderElectionConfig::default(),
        }
    }
}

/// Static service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticServiceConfig {
    /// Service ID
    pub id: String,

    /// Service name
    pub name: String,

    /// Service endpoint
    pub endpoint: String,

    /// Service metadata
    pub metadata: HashMap<String, String>,

    /// Health check endpoint
    pub health_endpoint: Option<String>,
}

/// Service discovery backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceDiscoveryBackend {
    Etcd,
    Consul,
    Static,
}

/// Leader election configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElectionConfig {
    /// Enable leader election
    pub enabled: bool,

    /// Election timeout
    pub election_timeout: Duration,

    /// Campaign timeout
    pub campaign_timeout: Duration,
}

impl Default for LeaderElectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            election_timeout: Duration::from_secs(60),
            campaign_timeout: Duration::from_secs(30),
        }
    }
}

/// Agent capabilities configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesConfig {
    /// Enable ingestion capabilities
    pub ingestion: bool,

    /// Enable indexing capabilities
    pub indexing: bool,

    /// Enable processing capabilities
    pub processing: bool,

    /// Enable query capabilities
    pub query: bool,

    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,

    /// Supported data formats
    pub supported_formats: Vec<String>,
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        Self {
            ingestion: true,
            indexing: true,
            processing: true,
            query: false,
            max_concurrent_tasks: 10,
            supported_formats: vec![
                "json".to_string(),
                "protobuf".to_string(),
                "avro".to_string(),
            ],
        }
    }
}

/// Task processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Task timeout
    pub task_timeout: Duration,

    /// Retry attempts
    pub retry_attempts: u32,

    /// Retry delay
    pub retry_delay: Duration,

    /// Batch size for processing
    pub batch_size: usize,

    /// Enable parallel processing
    pub parallel_processing: bool,

    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            task_timeout: Duration::from_secs(300),
            retry_attempts: 3,
            retry_delay: Duration::from_secs(5),
            batch_size: 1000,
            parallel_processing: true,
            max_concurrent_tasks: 10,
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Local storage path
    pub local_path: String,

    /// Database URL for local state
    pub database_url: String,

    /// Enable local caching
    pub enable_cache: bool,

    /// Cache size in bytes
    pub cache_size: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            local_path: "/tmp/orasi-agent".to_string(),
            database_url: "sqlite:/tmp/orasi-agent/agent.db".to_string(),
            enable_cache: true,
            cache_size: 100 * 1024 * 1024, // 100MB
        }
    }
}
