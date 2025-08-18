//! Task-related types and functions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Task types that agents can handle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    Ingestion,
    Indexing,
    Processing,
    Query,
    Maintenance,
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
