//! Basic Orasi Agent example

use orasi_agent::{
    agent::OrasiAgent,
    config::AgentConfig,
    processing::tasks::{Task, TaskPayload, TaskPriority, TaskStatus, TaskType},
    types::AgentStatus,
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Orasi Agent example");

    // Create agent configuration
    let config = AgentConfig {
        agent_id: "example-agent-1".to_string(),
        agent_endpoint: "0.0.0.0:8082".to_string(),
        health_endpoint: "0.0.0.0:8083".to_string(),
        metrics_endpoint: "0.0.0.0:9092".to_string(),
        cluster_endpoint: "0.0.0.0:8084".to_string(),
        ..Default::default()
    };

    // Create and start agent
    let mut agent = OrasiAgent::new(config).await?;
    agent.start().await?;

    info!("Agent started successfully");

    // Create some example tasks
    let tasks = vec![
        create_ingestion_task("task-1"),
        create_indexing_task("task-2"),
        create_processing_task("task-3"),
    ];

    // Submit tasks to the agent
    for task in tasks {
        match agent.submit_task(task).await {
            Ok(_) => info!("Task submitted successfully"),
            Err(e) => error!("Failed to submit task: {}", e),
        }
    }

    // Monitor agent for a while
    let mut counter = 0;
    while counter < 30 {
        sleep(Duration::from_secs(2)).await;

        // Get agent status
        let status = agent.get_agent_status().await;
        info!("Agent status: {:?}", status);

        // Get task queue statistics
        let queue_stats = agent.get_task_queue_stats().await;
        info!("Queue stats: {:?}", queue_stats);

        // Get cluster state
        let cluster_state = agent.get_cluster_state().await;
        info!("Cluster members: {}", cluster_state.members.len());

        counter += 1;
    }

    // Request shutdown
    info!("Requesting agent shutdown");
    agent.request_shutdown().await?;

    // Wait for shutdown to complete
    sleep(Duration::from_secs(5)).await;

    info!("Example completed successfully");
    Ok(())
}

/// Create an example ingestion task
fn create_ingestion_task(task_id: &str) -> Task {
    Task {
        task_id: task_id.to_string(),
        task_type: TaskType::Ingestion,
        priority: TaskPriority::Normal,
        payload: TaskPayload::Ingestion(orasi_agent::processing::tasks::IngestionTask {
            source_id: "example-source".to_string(),
            format: "json".to_string(),
            location: "file:///tmp/example-data.json".to_string(),
            schema: None,
            options: std::collections::HashMap::new(),
        }),
        metadata: std::collections::HashMap::new(),
        created_at: current_timestamp(),
        expires_at: Some(current_timestamp() + 300000), // 5 minutes
        retry_count: 0,
        max_retries: 3,
        last_error: None,
        status: TaskStatus::Pending,
    }
}

/// Create an example indexing task
fn create_indexing_task(task_id: &str) -> Task {
    Task {
        task_id: task_id.to_string(),
        task_type: TaskType::Indexing,
        priority: TaskPriority::High,
        payload: TaskPayload::Indexing(orasi_agent::processing::tasks::IndexingTask {
            data_location: "s3://bucket/data/".to_string(),
            index_config: orasi_agent::processing::tasks::IndexConfig {
                index_type: "btree".to_string(),
                fields: vec!["timestamp".to_string(), "user_id".to_string()],
                options: std::collections::HashMap::new(),
            },
            destination: "s3://bucket/indexes/".to_string(),
        }),
        metadata: std::collections::HashMap::new(),
        created_at: current_timestamp(),
        expires_at: Some(current_timestamp() + 600000), // 10 minutes
        retry_count: 0,
        max_retries: 3,
        last_error: None,
        status: TaskStatus::Pending,
    }
}

/// Create an example processing task
fn create_processing_task(task_id: &str) -> Task {
    Task {
        task_id: task_id.to_string(),
        task_type: TaskType::Processing,
        priority: TaskPriority::Low,
        payload: TaskPayload::Processing(orasi_agent::processing::tasks::ProcessingTask {
            input_location: "s3://bucket/raw-data/".to_string(),
            pipeline: "data_processing".to_string(),
            output_destination: "s3://bucket/processed-data/".to_string(),
            options: std::collections::HashMap::new(),
        }),
        metadata: std::collections::HashMap::new(),
        created_at: current_timestamp(),
        expires_at: Some(current_timestamp() + 900000), // 15 minutes
        retry_count: 0,
        max_retries: 3,
        last_error: None,
        status: TaskStatus::Pending,
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
