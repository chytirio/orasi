//! Agent state management

use crate::{config::AgentConfig, types::*};
use std::collections::HashMap;

/// Agent state management
pub struct AgentState {
    /// Agent information
    agent_info: AgentInfo,

    /// Current health status
    health_status: Option<HealthStatus>,

    /// Current load metrics
    load_metrics: AgentLoad,

    /// Active tasks
    active_tasks: HashMap<String, Task>,

    /// Task queue
    task_queue: Vec<Task>,
}

impl AgentState {
    /// Create new agent state
    pub async fn new(config: &AgentConfig) -> Result<Self, crate::error::AgentError> {
        let capabilities = AgentCapabilities {
            task_types: vec![
                TaskType::Ingestion,
                TaskType::Indexing,
                TaskType::Processing,
            ],
            max_concurrent_tasks: config.capabilities.max_concurrent_tasks,
            supported_formats: config.capabilities.supported_formats.clone(),
            resource_limits: ResourceLimits {
                max_cpu_percent: 80.0,
                max_memory_bytes: 1024 * 1024 * 1024,    // 1GB
                max_disk_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            },
        };

        let agent_info = AgentInfo {
            agent_id: config.agent_id.clone(),
            status: AgentStatus::Starting,
            version: crate::AGENT_VERSION.to_string(),
            capabilities,
            endpoint: config.agent_endpoint.clone(),
            last_heartbeat: current_timestamp(),
            metadata: HashMap::new(),
        };

        Ok(Self {
            agent_info,
            health_status: None,
            load_metrics: AgentLoad::default(),
            active_tasks: HashMap::new(),
            task_queue: Vec::new(),
        })
    }

    /// Get agent information
    pub fn get_agent_info(&self) -> AgentInfo {
        self.agent_info.clone()
    }

    /// Set agent status
    pub fn set_status(&mut self, status: AgentStatus) {
        self.agent_info.status = status;
    }

    /// Get agent status
    pub fn get_status(&self) -> AgentStatus {
        self.agent_info.status.clone()
    }

    /// Update health status
    pub fn update_health_status(&mut self, status: HealthStatus) {
        self.health_status = Some(status);
    }

    /// Get health status
    pub fn get_health_status(&self) -> Option<HealthStatus> {
        self.health_status.clone()
    }

    /// Update load metrics
    pub fn update_load_metrics(&mut self, metrics: AgentLoad) {
        self.load_metrics = metrics;
        self.agent_info.last_heartbeat = current_timestamp();
    }

    /// Get load metrics
    pub fn get_load_metrics(&self) -> AgentLoad {
        self.load_metrics.clone()
    }

    /// Add active task
    pub fn add_active_task(&mut self, task: Task) {
        self.active_tasks.insert(task.task_id.clone(), task);
    }

    /// Remove active task
    pub fn remove_active_task(&mut self, task_id: &str) -> Option<Task> {
        self.active_tasks.remove(task_id)
    }

    /// Get active tasks
    pub fn get_active_tasks(&self) -> Vec<Task> {
        self.active_tasks.values().cloned().collect()
    }

    /// Add task to queue
    pub fn add_task_to_queue(&mut self, task: Task) {
        self.task_queue.push(task);
    }

    /// Get next task from queue
    pub fn get_next_task(&mut self) -> Option<Task> {
        self.task_queue.pop()
    }

    /// Get queue length
    pub fn get_queue_length(&self) -> usize {
        self.task_queue.len()
    }
}
