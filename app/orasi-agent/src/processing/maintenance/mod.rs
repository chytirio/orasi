//! Maintenance processing module
//! 
//! This module provides comprehensive system maintenance capabilities including:
//! - Cleanup operations (temp files, logs, cache, orphaned data)
//! - Optimization operations (indexes, database, cache, compression)
//! - Backup and restore operations
//! - Health checks and monitoring
//! - Update management
//! - Repair operations

pub mod cleanup;
pub mod optimization;
pub mod backup;
pub mod restore;
pub mod health;
pub mod update;
pub mod repair;

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::{MaintenanceTask, TaskResult};
use crate::state::AgentState;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::info;

/// Maintenance processor for handling system maintenance tasks
pub struct MaintenanceProcessor {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
}

impl MaintenanceProcessor {
    /// Create new maintenance processor
    pub async fn new(
        config: &AgentConfig,
        state: Arc<RwLock<AgentState>>,
    ) -> Result<Self, AgentError> {
        Ok(Self {
            config: config.clone(),
            state,
        })
    }

    /// Process maintenance task
    pub async fn process_maintenance(
        &self,
        task: MaintenanceTask,
    ) -> Result<TaskResult, AgentError> {
        let start_time = Instant::now();
        info!("Processing maintenance task: {}", task.maintenance_type);

        // Validate task parameters
        self.validate_maintenance_task(&task)?;

        // Execute maintenance operation based on type
        let result = match task.maintenance_type.to_lowercase().as_str() {
            "cleanup" => self.perform_cleanup(&task).await,
            "optimize" => self.perform_optimization(&task).await,
            "backup" => self.perform_backup(&task).await,
            "restore" => self.perform_restore(&task).await,
            "health_check" => self.perform_health_check(&task).await,
            "update" => self.perform_update(&task).await,
            "repair" => self.perform_repair(&task).await,
            _ => Err(AgentError::InvalidInput(format!(
                "Unsupported maintenance operation: {}",
                task.maintenance_type
            ))),
        }?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        info!("Maintenance task completed in {}ms", duration_ms);

        Ok(TaskResult {
            task_id: format!("maintenance_{}", task.maintenance_type),
            success: true,
            data: Some(result),
            error: None,
            processing_time_ms: duration_ms,
            timestamp: current_timestamp(),
        })
    }

    /// Validate maintenance task parameters
    fn validate_maintenance_task(&self, task: &MaintenanceTask) -> Result<(), AgentError> {
        if task.maintenance_type.is_empty() {
            return Err(AgentError::InvalidInput(
                "Maintenance type cannot be empty".to_string(),
            ));
        }

        if task.targets.is_empty() {
            return Err(AgentError::InvalidInput(
                "Maintenance targets cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Perform cleanup operation
    async fn perform_cleanup(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        cleanup::CleanupProcessor::new(&self.config).perform_cleanup(task).await
    }

    /// Perform optimization operation
    async fn perform_optimization(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        optimization::OptimizationProcessor::new(&self.config).perform_optimization(task).await
    }

    /// Perform backup operation
    async fn perform_backup(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        backup::BackupProcessor::new(&self.config).perform_backup(task).await
    }

    /// Perform restore operation
    async fn perform_restore(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        restore::RestoreProcessor::new(&self.config).perform_restore(task).await
    }

    /// Perform health check operation
    async fn perform_health_check(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        health::HealthProcessor::new(&self.config).perform_health_check(task).await
    }

    /// Perform update operation
    async fn perform_update(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        update::UpdateProcessor::new(&self.config).perform_update(task).await
    }

    /// Perform repair operation
    async fn perform_repair(&self, task: &MaintenanceTask) -> Result<Value, AgentError> {
        repair::RepairProcessor::new(&self.config).perform_repair(task).await
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
