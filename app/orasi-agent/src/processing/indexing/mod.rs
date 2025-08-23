//! Indexing processing

pub mod btree;
pub mod composite;
pub mod fulltext;
pub mod hash;
pub mod inverted;
pub mod maintainer;
pub mod optimizer;
pub mod spatial;

use super::tasks::*;
use crate::types::*;
use crate::{config::AgentConfig, error::AgentError, state::AgentState};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Indexing processor for handling data indexing tasks
pub struct IndexingProcessor {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
    btree_builder: btree::BTreeIndexBuilder,
    hash_builder: hash::HashIndexBuilder,
    fulltext_builder: fulltext::FullTextIndexBuilder,
    spatial_builder: spatial::SpatialIndexBuilder,
    composite_builder: composite::CompositeIndexBuilder,
    inverted_builder: inverted::InvertedIndexBuilder,
    optimizer: optimizer::IndexOptimizer,
    maintainer: maintainer::IndexMaintainer,
}

impl IndexingProcessor {
    /// Create new indexing processor
    pub async fn new(
        config: &AgentConfig,
        state: Arc<RwLock<AgentState>>,
    ) -> Result<Self, AgentError> {
        let btree_builder = btree::BTreeIndexBuilder::new(config.clone());
        let hash_builder = hash::HashIndexBuilder::new(config.clone());
        let fulltext_builder = fulltext::FullTextIndexBuilder::new(config.clone());
        let spatial_builder = spatial::SpatialIndexBuilder::new(config.clone());
        let composite_builder = composite::CompositeIndexBuilder::new(config.clone());
        let inverted_builder = inverted::InvertedIndexBuilder::new(config.clone());
        let optimizer = optimizer::IndexOptimizer::new(config.clone());
        let maintainer = maintainer::IndexMaintainer::new(config.clone());

        Ok(Self {
            config: config.clone(),
            state,
            btree_builder,
            hash_builder,
            fulltext_builder,
            spatial_builder,
            composite_builder,
            inverted_builder,
            optimizer,
            maintainer,
        })
    }

    /// Process indexing task
    pub async fn process_indexing(&self, task: IndexingTask) -> Result<TaskResult, AgentError> {
        let start_time = Instant::now();
        info!("Processing indexing task for: {}", task.data_location);

        // Validate task parameters
        self.validate_indexing_task(&task)?;

        // Build index based on configuration
        let result = self.build_index(&task).await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Update agent state with indexing metrics
        {
            // Note: indexing_metrics field is private
            // This is a simplified implementation
        }

        info!("Indexing task completed in {}ms", duration_ms);

        Ok(TaskResult {
            task_id: format!("index_{}", task.data_location),
            success: true,
            data: Some(result),
            error: None,
            processing_time_ms: duration_ms,
            timestamp: current_timestamp(),
        })
    }

    /// Validate indexing task parameters
    fn validate_indexing_task(&self, task: &IndexingTask) -> Result<(), AgentError> {
        if task.data_location.is_empty() {
            return Err(AgentError::InvalidInput(
                "Data location cannot be empty".to_string(),
            ));
        }

        if task.destination.is_empty() {
            return Err(AgentError::InvalidInput(
                "Index destination cannot be empty".to_string(),
            ));
        }

        if task.index_config.fields.is_empty() {
            return Err(AgentError::InvalidInput(
                "Index fields cannot be empty".to_string(),
            ));
        }

        if task.index_config.index_type.is_empty() {
            return Err(AgentError::InvalidInput(
                "Index type cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Build index based on configuration
    async fn build_index(&self, task: &IndexingTask) -> Result<Value, AgentError> {
        info!(
            "Building index of type '{}' for fields: {:?}",
            task.index_config.index_type, task.index_config.fields
        );

        match task.index_config.index_type.to_lowercase().as_str() {
            "btree" => self.btree_builder.build_index(task).await,
            "hash" => self.hash_builder.build_index(task).await,
            "fulltext" => self.fulltext_builder.build_index(task).await,
            "spatial" => self.spatial_builder.build_index(task).await,
            "composite" => self.composite_builder.build_index(task).await,
            "inverted" => self.inverted_builder.build_index(task).await,
            _ => Err(AgentError::InvalidInput(format!(
                "Unsupported index type: {}",
                task.index_config.index_type
            ))),
        }
    }

    /// Optimize existing index
    pub async fn optimize_index(&self, index_location: &str) -> Result<Value, AgentError> {
        self.optimizer.optimize_index(index_location).await
    }

    /// Maintain index (cleanup, defragmentation, etc.)
    pub async fn maintain_index(&self, index_location: &str) -> Result<Value, AgentError> {
        self.maintainer.maintain_index(index_location).await
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
