//! Orasi Agent - Distributed data processing agent

pub mod agent;
pub mod cluster;
pub mod config;
pub mod discovery;
pub mod error;
pub mod health;
pub mod http;
pub mod metrics;
pub mod processing;
pub mod state;
pub mod types;

/// Agent version
pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Re-export main types for convenience
pub use agent::OrasiAgent;
pub use config::AgentConfig;
pub use error::AgentError;
pub use types::*;

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

/// Agent name
pub const AGENT_NAME: &str = "orasi-agent";

/// Default agent endpoint
pub const DEFAULT_AGENT_ENDPOINT: &str = "0.0.0.0:8082";

/// Default health check endpoint
pub const DEFAULT_HEALTH_ENDPOINT: &str = "0.0.0.0:8083";

/// Default metrics endpoint
pub const DEFAULT_METRICS_ENDPOINT: &str = "0.0.0.0:9092";

/// Default cluster coordination endpoint
pub const DEFAULT_CLUSTER_ENDPOINT: &str = "0.0.0.0:8084";

/// Default heartbeat interval in seconds
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Default task timeout in seconds
pub const DEFAULT_TASK_TIMEOUT_SECS: u64 = 300;

/// Initialize orasi agent
pub async fn init_agent(config: AgentConfig) -> AgentResult<OrasiAgent> {
    tracing::info!("Initializing Orasi agent v{}", AGENT_VERSION);

    let agent = OrasiAgent::new(config).await?;
    tracing::info!("Orasi agent initialization completed");

    Ok(agent)
}

/// Shutdown orasi agent
pub async fn shutdown_agent(agent: OrasiAgent) -> AgentResult<()> {
    tracing::info!("Shutting down Orasi agent");

    agent.shutdown().await?;
    tracing::info!("Orasi agent shutdown completed");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::AgentConfig,
        processing::tasks::{
            IngestionTask, ProcessingTask, Task, TaskPayload, TaskPriority, TaskStatus, TaskType,
        },
        types::AgentStatus,
        OrasiAgent,
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_agent_creation() {
        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            agent_endpoint: "0.0.0.0:8082".to_string(),
            health_endpoint: "0.0.0.0:8083".to_string(),
            metrics_endpoint: "0.0.0.0:9092".to_string(),
            cluster_endpoint: "0.0.0.0:8084".to_string(),
            ..Default::default()
        };

        let agent = OrasiAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_task_creation() {
        let task = Task {
            task_id: "test-task".to_string(),
            task_type: TaskType::Ingestion,
            priority: TaskPriority::Normal,
            payload: TaskPayload::Ingestion(crate::processing::tasks::IngestionTask {
                source_id: "test-source".to_string(),
                format: "json".to_string(),
                location: "file:///tmp/test.json".to_string(),
                schema: None,
                options: HashMap::new(),
            }),
            metadata: HashMap::new(),
            created_at: current_timestamp(),
            expires_at: Some(current_timestamp() + 300000),
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            status: TaskStatus::Pending,
        };

        assert_eq!(task.task_id, "test-task");
        assert_eq!(task.task_type, TaskType::Ingestion);
        assert_eq!(task.priority, TaskPriority::Normal);
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_task_queue() {
        use crate::processing::tasks::TaskQueue;

        let mut queue = TaskQueue::new(5);

        let task = Task {
            task_id: "test-task".to_string(),
            task_type: TaskType::Processing,
            priority: TaskPriority::High,
            payload: TaskPayload::Processing(ProcessingTask {
                input_location: "s3://bucket/input/".to_string(),
                pipeline: "test_pipeline".to_string(),
                output_destination: "s3://bucket/output/".to_string(),
                options: HashMap::new(),
            }),
            metadata: HashMap::new(),
            created_at: current_timestamp(),
            expires_at: None,
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            status: TaskStatus::Pending,
        };

        // Test enqueue
        let result = queue.enqueue(task).await;
        assert!(result.is_ok());

        // Test dequeue
        let dequeued_task = queue.dequeue().await;
        assert!(dequeued_task.is_some());

        // Test stats
        let stats = queue.get_stats();
        assert_eq!(stats.pending_count, 0);
        assert_eq!(stats.active_count, 1);
    }

    #[tokio::test]
    async fn test_health_status() {
        use crate::health::{CheckResult, HealthStatus};

        let check = CheckResult {
            name: "test-check".to_string(),
            status: HealthStatus::Healthy,
            message: "Test check passed".to_string(),
            data: None,
            last_check: current_timestamp(),
        };

        assert_eq!(check.name, "test-check");
        assert_eq!(check.status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        use crate::metrics::{AgentMetrics, ResourceMetrics, TaskMetrics};

        let metrics = AgentMetrics {
            tasks: TaskMetrics {
                total_processed: 100,
                successful: 95,
                failed: 5,
                pending: 10,
                active: 3,
                avg_processing_time_ms: 150.0,
                by_type: HashMap::new(),
            },
            resources: ResourceMetrics {
                cpu_percent: 25.0,
                memory_bytes: 512 * 1024 * 1024,
                memory_percent: 50.0,
                disk_bytes: 5 * 1024 * 1024 * 1024,
                disk_percent: 30.0,
                network_rx_bytes: 1024 * 1024,
                network_tx_bytes: 512 * 1024,
            },
            performance: crate::metrics::PerformanceMetrics {
                requests_per_second: 10.5,
                avg_response_time_ms: 150.0,
                p95_response_time_ms: 300.0,
                p99_response_time_ms: 500.0,
                throughput_bytes_per_sec: 1024.0 * 1024.0,
            },
            errors: crate::metrics::ErrorMetrics {
                total_errors: 5,
                by_type: HashMap::new(),
                error_rate: 0.05,
            },
            timestamp: current_timestamp(),
        };

        assert_eq!(metrics.tasks.total_processed, 100);
        assert_eq!(metrics.tasks.successful, 95);
        assert_eq!(metrics.tasks.failed, 5);
        assert_eq!(metrics.resources.cpu_percent, 25.0);
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}
