//! Task-related types and functions

use crate::config::AgentConfig;
use crate::error::AgentError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

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

    /// Retry count
    pub retry_count: u32,

    /// Maximum retries
    pub max_retries: u32,

    /// Last error message
    pub last_error: Option<String>,

    /// Task status
    pub status: TaskStatus,
}

/// Task priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
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
    Retrying,
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task identifier
    pub task_id: String,

    /// Success status
    pub success: bool,

    /// Result data
    pub data: Option<serde_json::Value>,

    /// Error message
    pub error: Option<String>,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Timestamp
    pub timestamp: u64,
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
        .unwrap()
        .as_secs()
}

pub fn is_task_expired(task: &Task) -> bool {
    if let Some(expires_at) = task.expires_at {
        current_timestamp() > expires_at
    } else {
        false
    }
}

/// Task queue for managing and scheduling tasks
pub struct TaskQueue {
    /// Priority queue for pending tasks
    pending_tasks: BinaryHeap<QueuedTask>,

    /// Active tasks
    active_tasks: HashMap<String, Task>,

    /// Completed tasks
    completed_tasks: HashMap<String, TaskResult>,

    /// Failed tasks
    failed_tasks: HashMap<String, Task>,

    /// Maximum concurrent tasks
    max_concurrent_tasks: usize,

    /// Persistence directory
    persistence_dir: PathBuf,

    /// Recovery enabled
    recovery_enabled: bool,
}

/// Wrapper for tasks in the priority queue
#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task: Task,
    pub priority_score: u64,
}

impl PartialEq for QueuedTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority_score == other.priority_score
    }
}

impl Eq for QueuedTask {}

impl PartialOrd for QueuedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority scores come first
        other.priority_score.cmp(&self.priority_score)
    }
}

impl TaskQueue {
    /// Create a new task queue
    pub fn new(max_concurrent_tasks: usize) -> Self {
        Self {
            pending_tasks: BinaryHeap::new(),
            active_tasks: HashMap::new(),
            completed_tasks: HashMap::new(),
            failed_tasks: HashMap::new(),
            max_concurrent_tasks,
            persistence_dir: PathBuf::from("./data/tasks"),
            recovery_enabled: true,
        }
    }

    /// Create a new task queue with persistence
    pub fn with_persistence(max_concurrent_tasks: usize, persistence_dir: PathBuf) -> Self {
        Self {
            pending_tasks: BinaryHeap::new(),
            active_tasks: HashMap::new(),
            completed_tasks: HashMap::new(),
            failed_tasks: HashMap::new(),
            max_concurrent_tasks,
            persistence_dir,
            recovery_enabled: true,
        }
    }

    /// Initialize persistence directory
    pub async fn init_persistence(&self) -> Result<(), String> {
        if !self.recovery_enabled {
            return Ok(());
        }

        // Create persistence directory if it doesn't exist
        if !self.persistence_dir.exists() {
            fs::create_dir_all(&self.persistence_dir)
                .map_err(|e| format!("Failed to create persistence directory: {}", e))?;
        }

        // Create subdirectories for different task states
        let subdirs = ["pending", "active", "completed", "failed"];
        for subdir in &subdirs {
            let subdir_path = self.persistence_dir.join(subdir);
            if !subdir_path.exists() {
                fs::create_dir(&subdir_path)
                    .map_err(|e| format!("Failed to create subdirectory {}: {}", subdir, e))?;
            }
        }

        info!(
            "Task persistence initialized at: {:?}",
            self.persistence_dir
        );
        Ok(())
    }

    /// Save task to persistent storage
    pub async fn save_task(&self, task: &Task, state: &str) -> Result<(), String> {
        if !self.recovery_enabled {
            return Ok(());
        }

        let file_path = self
            .persistence_dir
            .join(state)
            .join(format!("{}.json", task.task_id));

        // Create directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path)
            .map_err(|e| format!("Failed to create task file: {}", e))?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string_pretty(task)
            .map_err(|e| format!("Failed to serialize task: {}", e))?;

        writer
            .write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write task file: {}", e))?;

        writer
            .flush()
            .map_err(|e| format!("Failed to flush task file: {}", e))?;

        info!("Saved task {} to persistent storage", task.task_id);
        Ok(())
    }

    /// Load task from persistent storage
    pub async fn load_task(&self, task_id: &str, state: &str) -> Result<Option<Task>, String> {
        if !self.recovery_enabled {
            return Ok(None);
        }

        let file_path = self
            .persistence_dir
            .join(state)
            .join(format!("{}.json", task_id));

        if !file_path.exists() {
            return Ok(None);
        }

        let file =
            File::open(&file_path).map_err(|e| format!("Failed to open task file: {}", e))?;

        let mut reader = BufReader::new(file);
        let mut json = String::new();
        reader
            .read_to_string(&mut json)
            .map_err(|e| format!("Failed to read task file: {}", e))?;

        let task: Task = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize task: {}", e))?;

        info!("Loaded task {} from persistent storage", task_id);
        Ok(Some(task))
    }

    /// Delete task from persistent storage
    pub async fn delete_task(&self, task_id: &str, state: &str) -> Result<(), String> {
        if !self.recovery_enabled {
            return Ok(());
        }

        let file_path = self
            .persistence_dir
            .join(state)
            .join(format!("{}.json", task_id));

        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| format!("Failed to delete task file: {}", e))?;
            info!("Deleted task {} from persistent storage", task_id);
        }

        Ok(())
    }

    /// Save task result to persistent storage
    pub async fn save_task_result(&self, task_id: &str, result: &TaskResult) -> Result<(), String> {
        if !self.recovery_enabled {
            return Ok(());
        }

        let file_path = self
            .persistence_dir
            .join("completed")
            .join(format!("{}_result.json", task_id));

        // Create directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path)
            .map_err(|e| format!("Failed to create result file: {}", e))?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string_pretty(result)
            .map_err(|e| format!("Failed to serialize task result: {}", e))?;

        writer
            .write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write result file: {}", e))?;

        writer
            .flush()
            .map_err(|e| format!("Failed to flush result file: {}", e))?;

        info!("Saved task result {} to persistent storage", task_id);
        Ok(())
    }

    /// Load task result from persistent storage
    pub async fn load_task_result(&self, task_id: &str) -> Result<Option<TaskResult>, String> {
        if !self.recovery_enabled {
            return Ok(None);
        }

        let file_path = self
            .persistence_dir
            .join("completed")
            .join(format!("{}_result.json", task_id));

        if !file_path.exists() {
            return Ok(None);
        }

        let file =
            File::open(&file_path).map_err(|e| format!("Failed to open result file: {}", e))?;

        let mut reader = BufReader::new(file);
        let mut json = String::new();
        reader
            .read_to_string(&mut json)
            .map_err(|e| format!("Failed to read result file: {}", e))?;

        let result: TaskResult = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize task result: {}", e))?;

        info!("Loaded task result {} from persistent storage", task_id);
        Ok(Some(result))
    }

    /// Recover tasks from persistent storage
    pub async fn recover_tasks(&mut self) -> Result<(), String> {
        if !self.recovery_enabled {
            return Ok(());
        }

        info!("Starting task recovery from persistent storage");

        // Recover pending tasks
        self.recover_tasks_from_state("pending").await?;

        // Recover active tasks
        self.recover_tasks_from_state("active").await?;

        // Recover completed tasks
        self.recover_completed_tasks().await?;

        // Recover failed tasks
        self.recover_tasks_from_state("failed").await?;

        info!(
            "Task recovery completed. Pending: {}, Active: {}, Completed: {}, Failed: {}",
            self.pending_tasks.len(),
            self.active_tasks.len(),
            self.completed_tasks.len(),
            self.failed_tasks.len()
        );

        Ok(())
    }

    /// Recover tasks from a specific state directory
    async fn recover_tasks_from_state(&mut self, state: &str) -> Result<(), String> {
        let state_dir = self.persistence_dir.join(state);

        if !state_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&state_dir)
            .map_err(|e| format!("Failed to read state directory {}: {}", state, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !file_stem.ends_with("_result") {
                        if let Some(task) = self.load_task(file_stem, state).await? {
                            match state {
                                "pending" => {
                                    self.pending_tasks.push(QueuedTask {
                                        task: task.clone(),
                                        priority_score: Self::calculate_priority_score(&task),
                                    });
                                }
                                "active" => {
                                    self.active_tasks.insert(task.task_id.clone(), task);
                                }
                                "failed" => {
                                    self.failed_tasks.insert(task.task_id.clone(), task);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Recover completed tasks and their results
    async fn recover_completed_tasks(&mut self) -> Result<(), String> {
        let completed_dir = self.persistence_dir.join("completed");

        if !completed_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&completed_dir)
            .map_err(|e| format!("Failed to read completed directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if file_stem.ends_with("_result") {
                        let task_id = file_stem.trim_end_matches("_result");
                        if let Some(result) = self.load_task_result(task_id).await? {
                            self.completed_tasks.insert(task_id.to_string(), result);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Add a task to the queue
    pub async fn enqueue(&mut self, task: Task) -> Result<(), String> {
        // Check if task has expired
        if let Some(expires_at) = task.expires_at {
            if current_timestamp() > expires_at {
                return Err("Task has expired".to_string());
            }
        }

        let priority_score = Self::calculate_priority_score(&task);
        let queued_task = QueuedTask {
            task: task.clone(),
            priority_score,
        };

        self.pending_tasks.push(queued_task);

        // Save to persistent storage
        self.save_task(&task, "pending").await?;

        Ok(())
    }

    /// Get next task to process
    pub async fn dequeue(&mut self) -> Option<Task> {
        if self.active_tasks.len() >= self.max_concurrent_tasks {
            return None;
        }

        while let Some(queued_task) = self.pending_tasks.pop() {
            let task = queued_task.task;

            // Check if task has expired
            if let Some(expires_at) = task.expires_at {
                if current_timestamp() > expires_at {
                    // Delete expired task from persistent storage
                    self.delete_task(&task.task_id, "pending").await.ok();
                    continue;
                }
            }

            // Check if task should be retried
            if task.status == TaskStatus::Retrying {
                if task.retry_count >= task.max_retries {
                    self.failed_tasks.insert(task.task_id.clone(), task.clone());
                    // Move to failed storage
                    self.delete_task(&task.task_id, "pending").await.ok();
                    self.save_task(&task, "failed").await.ok();
                    continue;
                }
            }

            self.active_tasks.insert(task.task_id.clone(), task.clone());

            // Move from pending to active storage
            self.delete_task(&task.task_id, "pending").await.ok();
            self.save_task(&task, "active").await.ok();

            return Some(task);
        }

        None
    }

    /// Mark task as completed
    pub async fn complete_task(&mut self, task_id: &str, result: TaskResult) {
        if let Some(task) = self.active_tasks.remove(task_id) {
            self.completed_tasks
                .insert(task_id.to_string(), result.clone());

            // Move from active to completed storage
            self.delete_task(task_id, "active").await.ok();
            self.save_task_result(task_id, &result).await.ok();
        }
    }

    /// Mark task as failed
    pub async fn fail_task(&mut self, task_id: &str, error: String) {
        if let Some(mut task) = self.active_tasks.remove(task_id) {
            task.last_error = Some(error);
            task.retry_count += 1;

            if task.retry_count >= task.max_retries {
                task.status = TaskStatus::Failed;
                self.failed_tasks.insert(task_id.to_string(), task.clone());

                // Move to failed storage
                self.delete_task(task_id, "active").await.ok();
                self.save_task(&task, "failed").await.ok();
            } else {
                task.status = TaskStatus::Retrying;
                // Re-queue with exponential backoff
                let backoff_delay = Self::calculate_backoff_delay(task.retry_count);
                task.created_at = current_timestamp() + backoff_delay;

                // Move back to pending storage
                self.delete_task(task_id, "active").await.ok();
                self.enqueue(task).await.ok();
            }
        }
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> QueueStats {
        QueueStats {
            pending_count: self.pending_tasks.len(),
            active_count: self.active_tasks.len(),
            completed_count: self.completed_tasks.len(),
            failed_count: self.failed_tasks.len(),
            max_concurrent_tasks: self.max_concurrent_tasks,
        }
    }

    /// Calculate priority score for task scheduling
    fn calculate_priority_score(task: &Task) -> u64 {
        let base_priority = match task.priority {
            TaskPriority::Low => 1,
            TaskPriority::Normal => 2,
            TaskPriority::High => 3,
            TaskPriority::Critical => 4,
        };

        // Factor in age (older tasks get higher priority)
        let age = current_timestamp().saturating_sub(task.created_at);
        let age_factor = age / 1000; // Convert to seconds

        // Factor in retry count (failed tasks get lower priority)
        let retry_penalty = task.retry_count * 1000;

        (base_priority * 1000000) + age_factor - retry_penalty as u64
    }

    /// Calculate exponential backoff delay
    fn calculate_backoff_delay(retry_count: u32) -> u64 {
        let base_delay = 1000; // 1 second
        let max_delay = 300000; // 5 minutes
        let delay = base_delay * (2_u64.pow(retry_count));
        delay.min(max_delay)
    }
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending_count: usize,
    pub active_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub max_concurrent_tasks: usize,
}
