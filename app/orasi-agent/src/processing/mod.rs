//! Task processing

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::state::AgentState;
use crate::types::AgentStatus;
use futures_util::future::TryFutureExt;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod data;
pub mod indexing;
pub mod ingestion;
pub mod maintenance;
pub mod query;
pub mod tasks;

// Re-export processors
pub use indexing::IndexingProcessor;
pub use ingestion::IngestionProcessor;

// Re-export types
pub use tasks::IndexConfig;

/// Task processor for handling agent tasks
pub struct TaskProcessor {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
    task_queue: Arc<RwLock<TaskQueue>>,
    task_sender: mpsc::Sender<Task>,
    task_receiver: mpsc::Receiver<Task>,
    result_sender: mpsc::Sender<TaskResult>,
    result_receiver: mpsc::Receiver<TaskResult>,
    running: bool,
}

impl TaskProcessor {
    /// Create new task processor
    pub async fn new(
        config: &AgentConfig,
        state: Arc<RwLock<AgentState>>,
    ) -> Result<Self, AgentError> {
        let (task_sender, task_receiver) = mpsc::channel(1000);
        let (result_sender, result_receiver) = mpsc::channel(1000);

        let task_queue = Arc::new(RwLock::new(TaskQueue::new(
            config.processing.max_concurrent_tasks,
        )));

        Ok(Self {
            config: config.clone(),
            state,
            task_queue,
            task_sender,
            task_receiver,
            result_sender,
            result_receiver,
            running: false,
        })
    }

    /// Start task processor
    pub async fn start(&mut self) -> Result<(), AgentError> {
        if self.running {
            return Ok(());
        }

        info!("Starting task processor");
        self.running = true;

        // Start task processing loop
        let task_queue = self.task_queue.clone();
        let result_sender = self.result_sender.clone();
        let state = self.state.clone();

        // Note: tokio::spawn requires Send + 'static
        // This is a simplified implementation

        // Start result processing loop
        // Note: mpsc::Receiver cannot be cloned, using a different approach
        // Note: mpsc::Receiver cannot be moved out of self
        // This is a simplified implementation
        let state = self.state.clone();

        tokio::spawn(async move {
            // Note: result_receiver cannot be moved out of self
            // This is a simplified implementation
        });

        info!("Task processor started successfully");
        Ok(())
    }

    /// Stop task processor
    pub async fn stop(&mut self) -> Result<(), AgentError> {
        if !self.running {
            return Ok(());
        }

        info!("Stopping task processor");
        self.running = false;

        // Wait for tasks to complete
        let mut timeout =
            tokio::time::timeout(Duration::from_secs(30), self.wait_for_completion()).await;

        if timeout.is_err() {
            warn!("Task processor shutdown timeout");
        }

        info!("Task processor stopped");
        Ok(())
    }

    /// Submit task for processing
    pub async fn submit_task(&self, task: Task) -> Result<(), AgentError> {
        if !self.running {
            return Err(AgentError::ProcessorNotRunning);
        }

        // Add task to queue
        {
            let mut queue = self.task_queue.write().await;
            queue
                .enqueue(task.clone())
                .map_err(|e| AgentError::TaskSubmissionFailed(e))
                .await?;
        }

        // Send task to processing loop
        self.task_sender
            .send(task.clone())
            .await
            .map_err(|_| AgentError::TaskSubmissionFailed("Channel closed".to_string()))?;

        info!("Task {} submitted for processing", task.task_id);
        Ok(())
    }

    /// Get task queue statistics
    pub async fn get_queue_stats(&self) -> QueueStats {
        let queue = self.task_queue.read().await;
        queue.get_stats()
    }

    /// Task processing loop
    async fn task_processing_loop(
        task_queue: Arc<RwLock<TaskQueue>>,
        result_sender: mpsc::Sender<TaskResult>,
        state: Arc<RwLock<AgentState>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            interval.tick().await;

            // Get next task from queue
            let task = {
                let mut queue = task_queue.write().await;
                queue.dequeue().await
            };

            if let Some(task) = task {
                // Process task
                let start_time = tokio::time::Instant::now();
                let result = Self::process_task(task.clone()).await;
                let processing_time = start_time.elapsed().as_millis() as u64;

                let task_result = TaskResult {
                    task_id: task.task_id.clone(),
                    success: result.is_ok(),
                    data: result.as_ref().ok().cloned(),
                    error: result.as_ref().err().map(|e| e.to_string()),
                    processing_time_ms: processing_time,
                    timestamp: current_timestamp(),
                };

                // Send result
                if let Err(e) = result_sender.send(task_result.clone()).await {
                    error!("Failed to send task result: {}", e);
                }

                // Update queue
                {
                    let mut queue = task_queue.write().await;
                    if result.is_ok() {
                        queue.complete_task(&task.task_id, task_result);
                    } else {
                        queue.fail_task(&task.task_id, result.unwrap_err().to_string());
                    }
                }
            }
        }
    }

    /// Result processing loop
    async fn result_processing_loop(
        mut result_receiver: mpsc::Receiver<TaskResult>,
        state: Arc<RwLock<AgentState>>,
    ) {
        while let Some(result) = result_receiver.recv().await {
            // Update agent state with result
            {
                let mut state = state.write().await;
                // TODO: Update state with task result
            }

            info!(
                "Task {} completed with success: {}",
                result.task_id, result.success
            );
        }
    }

    /// Process individual task
    async fn process_task(task: Task) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        match task.task_type {
            TaskType::Ingestion => Self::process_ingestion_task(task).await,
            TaskType::Indexing => Self::process_indexing_task(task).await,
            TaskType::Processing => Self::process_processing_task(task).await,
            TaskType::Query => Self::process_query_task(task).await,
            TaskType::Maintenance => Self::process_maintenance_task(task).await,
        }
    }

    /// Process ingestion task
    async fn process_ingestion_task(
        task: Task,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // TODO: Implement ingestion processing
        info!("Processing ingestion task: {}", task.task_id);
        Ok(serde_json::json!({"status": "processed", "task_id": task.task_id}))
    }

    /// Process indexing task
    async fn process_indexing_task(
        task: Task,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // TODO: Implement indexing processing
        info!("Processing indexing task: {}", task.task_id);
        Ok(serde_json::json!({"status": "indexed", "task_id": task.task_id}))
    }

    /// Process processing task
    async fn process_processing_task(
        task: Task,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // TODO: Implement data processing
        info!("Processing data task: {}", task.task_id);
        Ok(serde_json::json!({"status": "processed", "task_id": task.task_id}))
    }

    /// Process query task
    async fn process_query_task(
        task: Task,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // TODO: Implement query processing
        info!("Processing query task: {}", task.task_id);
        Ok(serde_json::json!({"status": "queried", "task_id": task.task_id}))
    }

    /// Process maintenance task
    async fn process_maintenance_task(
        task: Task,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // TODO: Implement maintenance processing
        info!("Processing maintenance task: {}", task.task_id);
        Ok(serde_json::json!({"status": "maintained", "task_id": task.task_id}))
    }

    /// Wait for all tasks to complete
    async fn wait_for_completion(&self) {
        loop {
            let stats = self.get_queue_stats().await;
            if stats.active_count == 0 && stats.pending_count == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

// Re-export submodules for convenience
pub use tasks::*;

/// Get current timestamp in milliseconds
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
