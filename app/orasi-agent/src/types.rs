//! Types for Orasi Agent

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Re-export task types from processing module for backward compatibility
pub use crate::processing::tasks::*;

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

    /// Member join notification
    MemberJoin(ClusterMember),

    /// Member leave notification
    MemberLeave(String),

    /// Leader election message
    LeaderElection(LeaderElectionMessage),

    /// Task distribution message
    TaskDistribution(TaskDistributionMessage),

    /// State synchronization message
    StateSync(StateSyncMessage),
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

/// Member status in cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemberStatus {
    Active,
    Inactive,
    Unhealthy,
}

/// Cluster member information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    pub member_id: String,
    pub endpoint: String,
    pub status: MemberStatus,
    pub capabilities: AgentCapabilities,
    pub last_heartbeat: u64,
    pub metadata: HashMap<String, String>,
}

/// Leader election message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElectionMessage {
    pub election_id: String,
    pub candidate_id: String,
    pub term: u64,
    pub timestamp: u64,
}

/// Task distribution message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDistributionMessage {
    pub task: Task,
    pub target_member: String,
    pub priority: u8,
    pub timestamp: u64,
}

/// State synchronization message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSyncMessage {
    pub sync_type: String,
    pub data: Value,
    pub timestamp: u64,
}
