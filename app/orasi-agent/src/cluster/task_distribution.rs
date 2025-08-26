//! Task distribution implementation for cluster coordination

use crate::processing::tasks::Task;
use serde::{Deserialize, Serialize};

/// Task distribution message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDistributionMessage {
    /// Task to distribute
    pub task: Task,

    /// Target member ID (None for broadcast)
    pub target_member: Option<String>,

    /// Distribution timestamp
    pub timestamp: u64,
}

/// Task distribution strategy
#[derive(Debug, Clone, PartialEq)]
pub enum TaskDistributionStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Load-based distribution
    LoadBased,
    /// Capability-based distribution
    CapabilityBased,
    /// Random distribution
    Random,
}

/// Task distribution result
#[derive(Debug, Clone)]
pub struct TaskDistributionResult {
    /// Whether the task was successfully distributed
    pub success: bool,
    /// Target member ID
    pub target_member: Option<String>,
    /// Error message if distribution failed
    pub error: Option<String>,
    /// Distribution timestamp
    pub timestamp: u64,
}

impl TaskDistributionMessage {
    /// Create a new task distribution message
    pub fn new(task: Task, target_member: Option<String>) -> Self {
        Self {
            task,
            target_member,
            timestamp: super::types::current_timestamp(),
        }
    }

    /// Check if this is a broadcast message
    pub fn is_broadcast(&self) -> bool {
        self.target_member.is_none()
    }

    /// Get the target member ID
    pub fn get_target_member(&self) -> Option<&String> {
        self.target_member.as_ref()
    }
}

impl TaskDistributionResult {
    /// Create a successful distribution result
    pub fn success(target_member: Option<String>) -> Self {
        Self {
            success: true,
            target_member,
            error: None,
            timestamp: super::types::current_timestamp(),
        }
    }

    /// Create a failed distribution result
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            target_member: None,
            error: Some(error),
            timestamp: super::types::current_timestamp(),
        }
    }
}
