//! Cluster types and data structures

use crate::types::{AgentCapabilities, MemberStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Get current timestamp in milliseconds
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
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

/// Cluster state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterState {
    pub members: HashMap<String, ClusterMember>,
    pub leader: Option<String>,
    pub health_status: ClusterHealthStatus,
    pub last_update: u64,
}

/// Cluster health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealthStatus {
    pub total_members: usize,
    pub healthy_members: usize,
    pub degraded_members: usize,
    pub unhealthy_members: usize,
    pub last_check: u64,
}
