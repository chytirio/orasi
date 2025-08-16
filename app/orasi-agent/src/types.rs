//! Types for Orasi Agent

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent identifier
    pub agent_id: String,

    /// Agent status
    pub status: AgentStatus,

    /// Agent version
    pub version: String,

    /// Agent capabilities
    pub capabilities: AgentCapabilities,

    /// Agent endpoint
    pub endpoint: String,

    /// Last heartbeat timestamp
    pub last_heartbeat: u64,

    /// Agent metadata
    pub metadata: HashMap<String, String>,
}

/// Agent capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Supported task types
    pub task_types: Vec<TaskType>,

    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,

    /// Supported data formats
    pub supported_formats: Vec<String>,

    /// Resource limits
    pub resource_limits: ResourceLimits,
}

/// Task types that agents can handle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    Ingestion,
    Indexing,
    Processing,
    Query,
    Maintenance,
}

/// Resource limits for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU usage (percentage)
    pub max_cpu_percent: f64,

    /// Maximum memory usage (bytes)
    pub max_memory_bytes: u64,

    /// Maximum disk usage (bytes)
    pub max_disk_bytes: u64,
}

/// Task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task identifier
    pub task_id: String,

    /// Task type
    pub task_type: TaskType,

    /// Task priority
    pub priority: TaskPriority,

    /// Task payload
    pub payload: TaskPayload,

    /// Task metadata
    pub metadata: HashMap<String, String>,

    /// Created timestamp
    pub created_at: u64,

    /// Expires at timestamp
    pub expires_at: Option<u64>,
}

/// Task priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum TaskPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPayload {
    /// Ingestion task
    Ingestion(IngestionTask),

    /// Indexing task
    Indexing(IndexingTask),

    /// Processing task
    Processing(ProcessingTask),

    /// Query task
    Query(QueryTask),

    /// Maintenance task
    Maintenance(MaintenanceTask),
}

/// Ingestion task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionTask {
    /// Data source identifier
    pub source_id: String,

    /// Data format
    pub format: String,

    /// Data location
    pub location: String,

    /// Schema information
    pub schema: Option<String>,

    /// Processing options
    pub options: HashMap<String, String>,
}

/// Indexing task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingTask {
    /// Data to index
    pub data_location: String,

    /// Index configuration
    pub index_config: IndexConfig,

    /// Index destination
    pub destination: String,
}

/// Index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Index type
    pub index_type: String,

    /// Index fields
    pub fields: Vec<String>,

    /// Index options
    pub options: HashMap<String, String>,
}

/// Processing task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingTask {
    /// Input data location
    pub input_location: String,

    /// Processing pipeline
    pub pipeline: String,

    /// Output destination
    pub output_destination: String,

    /// Processing options
    pub options: HashMap<String, String>,
}

/// Query task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTask {
    /// Query string
    pub query: String,

    /// Query parameters
    pub parameters: HashMap<String, String>,

    /// Result destination
    pub result_destination: String,
}

/// Maintenance task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceTask {
    /// Maintenance type
    pub maintenance_type: String,

    /// Target resources
    pub targets: Vec<String>,

    /// Maintenance options
    pub options: HashMap<String, String>,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task identifier
    pub task_id: String,

    /// Task status
    pub status: TaskStatus,

    /// Task output
    pub output: Option<TaskOutput>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Processing duration
    pub duration_ms: u64,

    /// Completed timestamp
    pub completed_at: u64,
}

/// Task output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    /// Output location
    pub location: String,

    /// Output format
    pub format: String,

    /// Output size in bytes
    pub size_bytes: u64,

    /// Output metadata
    pub metadata: HashMap<String, String>,
}

/// Heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Agent identifier
    pub agent_id: String,

    /// Agent status
    pub status: AgentStatus,

    /// Current load
    pub current_load: AgentLoad,

    /// Timestamp
    pub timestamp: u64,
}

/// Agent load information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoad {
    /// CPU usage percentage
    pub cpu_percent: f64,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// Disk usage in bytes
    pub disk_bytes: u64,

    /// Active task count
    pub active_tasks: usize,

    /// Queue length
    pub queue_length: usize,
}

impl Default for AgentLoad {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_bytes: 0,
            disk_bytes: 0,
            active_tasks: 0,
            queue_length: 0,
        }
    }
}

/// Cluster message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    /// Heartbeat message
    Heartbeat(Heartbeat),

    /// Task assignment
    TaskAssignment(Task),

    /// Task result
    TaskResult(TaskResult),

    /// Agent registration
    AgentRegistration(AgentInfo),

    /// Agent deregistration
    AgentDeregistration(String),

    /// Health check request
    HealthCheck(String),

    /// Health check response
    HealthCheckResponse(HealthStatus),
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Service name
    pub service: String,

    /// Health status
    pub status: HealthState,

    /// Health details
    pub details: HashMap<String, String>,

    /// Timestamp
    pub timestamp: u64,
}

/// Health state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Utility functions
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn is_task_expired(task: &Task) -> bool {
    if let Some(expires_at) = task.expires_at {
        current_timestamp() > expires_at
    } else {
        false
    }
}
