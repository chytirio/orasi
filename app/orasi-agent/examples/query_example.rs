//! Example demonstrating query execution functionality
//!
//! This example shows how to create an agent with query capabilities
//! and execute various types of queries.

use orasi_agent::{
    config::AgentConfig,
    error::AgentError,
    processing::query::QueryProcessor,
    processing::tasks::{QueryTask, TaskResult},
    state::AgentState,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), AgentError> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Orasi Agent Query Example");

    // Create agent configuration with query capabilities enabled
    let mut config = AgentConfig::default();
    config.capabilities.query = true;
    config.agent_id = "query-example-agent".to_string();

    info!("Agent configuration: {:?}", config);

    // Create agent state
    let state = Arc::new(RwLock::new(AgentState::new(&config).await?));

    // Create query processor
    let query_processor = QueryProcessor::new(&config, state).await?;

    // Example 1: Simple SQL query
    info!("=== Example 1: Simple SQL Query ===");
    let sql_task = QueryTask {
        query: "SELECT 1 as test_column, 'hello' as greeting".to_string(),
        parameters: HashMap::new(),
        result_destination: "memory".to_string(),
    };

    let result = execute_query_example(&query_processor, &sql_task, "SQL Query").await?;
    print_query_result(&result);

    // Example 2: Query with parameters
    info!("=== Example 2: Query with Parameters ===");
    let mut params = HashMap::new();
    params.insert("limit".to_string(), "10".to_string());
    params.insert("offset".to_string(), "0".to_string());

    let param_task = QueryTask {
        query: "SELECT * FROM telemetry_data LIMIT {limit} OFFSET {offset}".to_string(),
        parameters: params,
        result_destination: "memory".to_string(),
    };

    let result = execute_query_example(&query_processor, &param_task, "Parameterized Query").await?;
    print_query_result(&result);

    // Example 3: Analytics query
    info!("=== Example 3: Analytics Query ===");
    let analytics_task = QueryTask {
        query: "SELECT 
                    service_name,
                    COUNT(*) as request_count,
                    AVG(response_time) as avg_response_time,
                    MAX(response_time) as max_response_time
                FROM metrics 
                WHERE timestamp >= NOW() - INTERVAL '1 hour'
                GROUP BY service_name
                ORDER BY request_count DESC".to_string(),
        parameters: HashMap::new(),
        result_destination: "analytics_results".to_string(),
    };

    let result = execute_query_example(&query_processor, &analytics_task, "Analytics Query").await?;
    print_query_result(&result);

    // Example 4: Time series query
    info!("=== Example 4: Time Series Query ===");
    let timeseries_task = QueryTask {
        query: "SELECT 
                    DATE_TRUNC('hour', timestamp) as hour,
                    service_name,
                    SUM(error_count) as total_errors,
                    SUM(request_count) as total_requests,
                    (SUM(error_count) * 100.0 / SUM(request_count)) as error_rate
                FROM service_metrics 
                WHERE timestamp >= NOW() - INTERVAL '24 hours'
                GROUP BY DATE_TRUNC('hour', timestamp), service_name
                ORDER BY hour DESC, error_rate DESC".to_string(),
        parameters: HashMap::new(),
        result_destination: "timeseries_results".to_string(),
    };

    let result = execute_query_example(&query_processor, &timeseries_task, "Time Series Query").await?;
    print_query_result(&result);

    // Get query engine statistics
    info!("=== Query Engine Statistics ===");
    let stats = query_processor.get_query_stats().await?;
    info!("Query engine stats: {}", serde_json::to_string_pretty(&stats)?);

    info!("Query example completed successfully");
    Ok(())
}

/// Execute a query example and handle errors gracefully
async fn execute_query_example(
    processor: &QueryProcessor,
    task: &QueryTask,
    description: &str,
) -> Result<TaskResult, AgentError> {
    info!("Executing {}: {}", description, task.query);

    match processor.process_query(task.clone()).await {
        Ok(result) => {
            info!("{} completed successfully", description);
            Ok(result)
        }
        Err(e) => {
            warn!("{} failed: {}", description, e);
            // Return a mock result for demonstration purposes
            Ok(TaskResult {
                task_id: task.query.clone(),
                success: false,
                data: Some(serde_json::json!({
                    "error": e.to_string(),
                    "query": task.query,
                    "description": description,
                    "mock_result": true
                })),
                error: Some(e.to_string()),
                processing_time_ms: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            })
        }
    }
}

/// Print query result in a formatted way
fn print_query_result(result: &TaskResult) {
    info!("Query Result:");
    info!("  Task ID: {}", result.task_id);
    info!("  Success: {}", result.success);
    info!("  Processing Time: {}ms", result.processing_time_ms);
    
    if let Some(data) = &result.data {
        if let Ok(pretty_json) = serde_json::to_string_pretty(data) {
            info!("  Data: {}", pretty_json);
        } else {
            info!("  Data: {:?}", data);
        }
    }
    
    if let Some(error) = &result.error {
        warn!("  Error: {}", error);
    }
    
    info!("");
}
