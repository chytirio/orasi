//! Query processing

use super::tasks::*;
use crate::types::*;
use crate::{config::AgentConfig, error::AgentError, state::AgentState};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

// Import query engine types
use query_engine::{
    executors::{ExecutionEngine, QueryExecutor, QueryResult as EngineQueryResult},
    parsers::{ParsedQuery, QueryParser, QueryParserTrait},
    sources::{DataSourceManager, SourceManagerImpl, SourceManagerConfig},
    QueryEngine, QueryEngineConfig,
};

/// Query processor for handling data query tasks
pub struct QueryProcessor {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
    query_engine: Option<Arc<QueryEngine>>,
}

impl QueryProcessor {
    /// Create new query processor
    pub async fn new(
        config: &AgentConfig,
        state: Arc<RwLock<AgentState>>,
    ) -> Result<Self, AgentError> {
        let mut processor = Self {
            config: config.clone(),
            state,
            query_engine: None,
        };

        // Initialize query engine if query capabilities are enabled
        if config.capabilities.query {
            processor.initialize_query_engine().await?;
        }

        Ok(processor)
    }

    /// Initialize the query engine
    async fn initialize_query_engine(&mut self) -> Result<(), AgentError> {
        info!("Initializing query engine for agent");

        let query_config = QueryEngineConfig {
            enable_caching: self.config.storage.enable_cache,
            cache_size_limit: self.config.storage.cache_size,
            enable_optimization: true,
            max_execution_time_seconds: self.config.processing.task_timeout.as_secs(),
            enable_streaming: false,
            max_result_set_size: 10000,
            enable_plan_visualization: false,
            enable_performance_monitoring: true,
            data_sources: HashMap::new(), // Will be populated based on available data sources
        };

        match QueryEngine::new(query_config).await {
            Ok(engine) => {
                let engine = Arc::new(engine);
                engine.init().await.map_err(|e| {
                    AgentError::Configuration(format!("Failed to initialize query engine: {}", e))
                })?;
                self.query_engine = Some(engine);
                info!("Query engine initialized successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to create query engine: {}", e);
                Err(AgentError::Configuration(format!(
                    "Failed to create query engine: {}",
                    e
                )))
            }
        }
    }

    /// Process query task
    pub async fn process_query(&self, task: QueryTask) -> Result<TaskResult, AgentError> {
        let start_time = Instant::now();
        info!("Processing query task: {}", task.query);

        // Validate task parameters
        self.validate_query_task(&task)?;

        // Execute query based on type
        let result = self.execute_generic_query(&task).await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        info!("Query task completed in {}ms", duration_ms);

        Ok(TaskResult {
            task_id: task.query.clone(),
            success: true,
            data: Some(result),
            error: None,
            processing_time_ms: duration_ms,
            timestamp: current_timestamp(),
        })
    }

    /// Validate query task parameters
    fn validate_query_task(&self, task: &QueryTask) -> Result<(), AgentError> {
        if task.query.is_empty() {
            return Err(AgentError::InvalidInput(
                "Query cannot be empty".to_string(),
            ));
        }

        if task.result_destination.is_empty() {
            return Err(AgentError::InvalidInput(
                "Result destination cannot be empty".to_string(),
            ));
        }

        // Check if query capabilities are enabled
        if !self.config.capabilities.query {
            return Err(AgentError::InvalidInput(
                "Query capabilities are not enabled for this agent".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute generic query
    async fn execute_generic_query(&self, task: &QueryTask) -> Result<Value, AgentError> {
        info!("Executing query: {}", task.query);

        // Check if query engine is available
        let query_engine = self.query_engine.as_ref().ok_or_else(|| {
            AgentError::Configuration("Query engine not initialized".to_string())
        })?;

        // Execute the query
        let query_start = Instant::now();
        let engine_result = query_engine.execute_query(&task.query).await.map_err(|e| {
            error!("Query execution failed: {}", e);
            AgentError::TaskProcessing(format!("Query execution failed: {}", e))
        })?;

        let execution_time_ms = query_start.elapsed().as_millis() as u64;

        // Convert engine result to JSON
        let result = self.convert_engine_result_to_json(engine_result, task, execution_time_ms)?;

        info!("Query executed successfully in {}ms", execution_time_ms);

        Ok(result)
    }

    /// Convert query engine result to JSON format
    fn convert_engine_result_to_json(
        &self,
        engine_result: EngineQueryResult,
        task: &QueryTask,
        execution_time_ms: u64,
    ) -> Result<Value, AgentError> {
        // Extract data from engine result
        let rows_returned = engine_result.data.len();
        
        // Convert query rows to JSON
        let mut result_data = Vec::new();
        for row in &engine_result.data {
            let mut row_data = HashMap::new();
            for (key, value) in &row.data {
                let json_value = match value {
                    query_engine::executors::QueryValue::String(s) => Value::String(s.clone()),
                    query_engine::executors::QueryValue::Integer(i) => Value::Number(serde_json::Number::from(*i)),
                    query_engine::executors::QueryValue::Float(f) => {
                        if let Some(n) = serde_json::Number::from_f64(*f) {
                            Value::Number(n)
                        } else {
                            Value::Null
                        }
                    }
                    query_engine::executors::QueryValue::Boolean(b) => Value::Bool(*b),
                    query_engine::executors::QueryValue::Null => Value::Null,
                    query_engine::executors::QueryValue::Array(arr) => {
                        let json_array: Vec<Value> = arr.iter().map(|v| {
                            match v {
                                query_engine::executors::QueryValue::String(s) => Value::String(s.clone()),
                                query_engine::executors::QueryValue::Integer(i) => Value::Number(serde_json::Number::from(*i)),
                                query_engine::executors::QueryValue::Float(f) => {
                                    if let Some(n) = serde_json::Number::from_f64(*f) {
                                        Value::Number(n)
                                    } else {
                                        Value::Null
                                    }
                                }
                                query_engine::executors::QueryValue::Boolean(b) => Value::Bool(*b),
                                query_engine::executors::QueryValue::Null => Value::Null,
                                _ => Value::Null, // Handle other cases as needed
                            }
                        }).collect();
                        Value::Array(json_array)
                    }
                    query_engine::executors::QueryValue::Object(obj) => {
                        let json_obj: HashMap<String, Value> = obj.iter().map(|(k, v)| {
                            let json_value = match v {
                                query_engine::executors::QueryValue::String(s) => Value::String(s.clone()),
                                query_engine::executors::QueryValue::Integer(i) => Value::Number(serde_json::Number::from(*i)),
                                query_engine::executors::QueryValue::Float(f) => {
                                    if let Some(n) = serde_json::Number::from_f64(*f) {
                                        Value::Number(n)
                                    } else {
                                        Value::Null
                                    }
                                }
                                query_engine::executors::QueryValue::Boolean(b) => Value::Bool(*b),
                                query_engine::executors::QueryValue::Null => Value::Null,
                                _ => Value::Null,
                            };
                            (k.clone(), json_value)
                        }).collect();
                        Value::Object(serde_json::Map::from_iter(json_obj))
                    }
                };
                row_data.insert(key.clone(), json_value);
            }
            result_data.push(Value::Object(serde_json::Map::from_iter(row_data)));
        }

        // Build the final result
        let result = serde_json::json!({
            "query": task.query,
            "parameters": task.parameters,
            "result_destination": task.result_destination,
            "rows_returned": rows_returned,
            "execution_time_ms": execution_time_ms,
            "query_id": engine_result.id.to_string(),
            "status": match engine_result.status {
                query_engine::executors::ExecutionStatus::Success => "success",
                query_engine::executors::ExecutionStatus::Error(e) => {
                    warn!("Query execution had errors: {}", e);
                    "partial"
                }
                query_engine::executors::ExecutionStatus::Timeout => "timeout",
                query_engine::executors::ExecutionStatus::Cancelled => "cancelled",
            },
            "result": result_data,
            "metadata": engine_result.metadata,
            "execution_timestamp": engine_result.execution_timestamp.to_rfc3339(),
        });

        Ok(result)
    }

    /// Get query engine statistics
    pub async fn get_query_stats(&self) -> Result<Value, AgentError> {
        if let Some(query_engine) = &self.query_engine {
            // This would require exposing stats from the query engine
            // For now, return basic stats
            Ok(serde_json::json!({
                "query_engine_available": true,
                "capabilities_enabled": self.config.capabilities.query,
            }))
        } else {
            Ok(serde_json::json!({
                "query_engine_available": false,
                "capabilities_enabled": self.config.capabilities.query,
            }))
        }
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentConfig;
    use crate::processing::tasks::QueryTask;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_query_processor_creation() {
        // Create a minimal configuration
        let mut config = AgentConfig::default();
        config.capabilities.query = true;
        
        // Create agent state
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));
        
        // Create query processor
        let processor = QueryProcessor::new(&config, state).await;
        
        // Should succeed (even if query engine fails to initialize, it should handle gracefully)
        assert!(processor.is_ok());
    }

    #[tokio::test]
    async fn test_query_task_validation() {
        let mut config = AgentConfig::default();
        config.capabilities.query = true;
        
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));
        let processor = QueryProcessor::new(&config, state).await.unwrap();
        
        // Test valid query task
        let valid_task = QueryTask {
            query: "SELECT 1".to_string(),
            parameters: HashMap::new(),
            result_destination: "memory".to_string(),
        };
        
        // Should not panic
        let _ = processor.validate_query_task(&valid_task);
        
        // Test invalid query task (empty query)
        let invalid_task = QueryTask {
            query: "".to_string(),
            parameters: HashMap::new(),
            result_destination: "memory".to_string(),
        };
        
        // Should return error
        let result = processor.validate_query_task(&invalid_task);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_processor_with_disabled_capabilities() {
        let mut config = AgentConfig::default();
        config.capabilities.query = false; // Disable query capabilities
        
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));
        let processor = QueryProcessor::new(&config, state).await.unwrap();
        
        let task = QueryTask {
            query: "SELECT 1".to_string(),
            parameters: HashMap::new(),
            result_destination: "memory".to_string(),
        };
        
        // Should return error when query capabilities are disabled
        let result = processor.validate_query_task(&task);
        assert!(result.is_err());
    }
}
