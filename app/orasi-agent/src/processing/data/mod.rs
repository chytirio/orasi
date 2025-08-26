//! Data processing module

pub mod aggregation;
pub mod enrichment;
pub mod filtering;
pub mod ml;
pub mod transformation;
pub mod utils;
pub mod validation;

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::{ProcessingTask, TaskResult};
use crate::state::AgentState;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Data processing processor for handling data transformation and analysis tasks
pub struct DataProcessingProcessor {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
}

impl DataProcessingProcessor {
    /// Create new data processing processor
    pub async fn new(
        config: &AgentConfig,
        state: Arc<RwLock<AgentState>>,
    ) -> Result<Self, AgentError> {
        Ok(Self {
            config: config.clone(),
            state,
        })
    }

    /// Process data processing task
    pub async fn process_data(&self, task: ProcessingTask) -> Result<TaskResult, AgentError> {
        let start_time = Instant::now();
        info!("Processing data processing task: {}", task.pipeline);

        // Validate task parameters
        self.validate_processing_task(&task)?;

        // Execute processing pipeline based on type
        let result = match task.pipeline.to_lowercase().as_str() {
            "transformation" => {
                let processor = transformation::TransformationProcessor::new(&self.config, self.state.clone());
                processor.execute_pipeline(&task).await
            }
            "aggregation" => {
                let processor = aggregation::AggregationProcessor::new(&self.config, self.state.clone());
                processor.execute_pipeline(&task).await
            }
            "filtering" => {
                let processor = filtering::FilteringProcessor::new(&self.config, self.state.clone());
                processor.execute_pipeline(&task).await
            }
            "enrichment" => {
                let processor = enrichment::EnrichmentProcessor::new(&self.config, self.state.clone());
                processor.execute_pipeline(&task).await
            }
            "validation" => {
                let processor = validation::ValidationProcessor::new(&self.config, self.state.clone());
                processor.execute_pipeline(&task).await
            }
            "ml" => {
                let processor = ml::MLProcessor::new(&self.config, self.state.clone());
                processor.execute_pipeline(&task).await
            }
            _ => Err(AgentError::InvalidInput(format!(
                "Unsupported pipeline type: {}",
                task.pipeline
            ))),
        }?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        info!("Data processing task completed in {}ms", duration_ms);

        Ok(TaskResult {
            task_id: task.pipeline.clone(),
            success: true,
            data: Some(result),
            error: None,
            processing_time_ms: duration_ms,
            timestamp: utils::current_timestamp(),
        })
    }

    /// Validate processing task parameters
    fn validate_processing_task(&self, task: &ProcessingTask) -> Result<(), AgentError> {
        if task.pipeline.is_empty() {
            return Err(AgentError::InvalidInput(
                "Pipeline cannot be empty".to_string(),
            ));
        }

        if task.input_location.is_empty() {
            return Err(AgentError::InvalidInput(
                "Input location cannot be empty".to_string(),
            ));
        }

        if task.output_destination.is_empty() {
            return Err(AgentError::InvalidInput(
                "Output destination cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}
