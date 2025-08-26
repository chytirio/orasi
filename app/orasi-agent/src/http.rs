//! HTTP endpoints for Orasi Agent

use crate::agent::OrasiAgent;
use crate::cluster::ClusterState;
use crate::config::AgentConfig;
use crate::health::{HealthChecker, HealthStatus};
use crate::metrics::MetricsCollector;
use crate::processing::tasks::{QueueStats, Task};
use crate::types::{AgentInfo, AgentStatus};
use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// HTTP server for agent endpoints
pub struct HttpServer {
    config: AgentConfig,
    agent: Arc<RwLock<OrasiAgent>>,
    health_checker: Arc<RwLock<HealthChecker>>,
    metrics_collector: Arc<RwLock<MetricsCollector>>,
}

impl HttpServer {
    /// Create new HTTP server
    pub fn new(
        config: AgentConfig,
        agent: Arc<RwLock<OrasiAgent>>,
        health_checker: Arc<RwLock<HealthChecker>>,
        metrics_collector: Arc<RwLock<MetricsCollector>>,
    ) -> Self {
        Self {
            config,
            agent,
            health_checker,
            metrics_collector,
        }
    }

    /// Create router with all endpoints
    pub fn create_router(&self) -> Router {
        Router::new()
            .route("/health", get(Self::health_check))
            .route("/health/live", get(Self::health_live))
            .route("/health/ready", get(Self::health_ready))
            .route("/metrics", get(Self::metrics))
            .route("/metrics/prometheus", get(Self::prometheus_metrics))
            .route("/agent/info", get(Self::agent_info))
            .route("/agent/status", get(Self::agent_status))
            .route("/agent/tasks", get(Self::task_queue_stats))
            .route("/agent/tasks", post(Self::submit_task))
            .route("/cluster/state", get(Self::cluster_state))
            .route("/cluster/members", get(Self::cluster_members))
            .route("/cluster/leader", get(Self::cluster_leader))
            .with_state(Arc::new(self.clone()))
    }

    /// Health check endpoint
    async fn health_check(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let health_checker = server.health_checker.read().await;

        match health_checker.check_health().await {
            Ok(status) => {
                let status_code = match status {
                    HealthStatus::Healthy => StatusCode::OK,
                    HealthStatus::Degraded => StatusCode::OK,
                    HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
                    HealthStatus::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
                };

                let response = json!({
                    "status": status,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "service": "orasi-agent"
                });

                (status_code, Json(response))
            }
            Err(e) => {
                error!("Health check failed: {}", e);
                let response = json!({
                    "status": "unhealthy",
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "service": "orasi-agent"
                });
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
    }

    /// Liveness probe endpoint
    async fn health_live(State(_server): State<Arc<Self>>) -> impl IntoResponse {
        let response = json!({
            "status": "alive",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        (StatusCode::OK, Json(response))
    }

    /// Readiness probe endpoint
    async fn health_ready(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let agent = server.agent.read().await;
        let status = agent.get_agent_status().await;

        let is_ready = matches!(status, AgentStatus::Running);
        let status_code = if is_ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };

        let response = json!({
            "status": if is_ready { "ready" } else { "not_ready" },
            "agent_status": status,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        (status_code, Json(response))
    }

    /// Metrics endpoint (JSON format)
    async fn metrics(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let metrics_collector = server.metrics_collector.read().await;

        match metrics_collector.collect_metrics().await {
            Ok(metrics) => {
                let response = json!({
                    "metrics": metrics,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                (StatusCode::OK, Json(response))
            }
            Err(e) => {
                error!("Failed to collect metrics: {}", e);
                let response = json!({
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
    }

    /// Prometheus metrics endpoint
    async fn prometheus_metrics(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let agent = server.agent.read().await;
        
        // Collect current metrics
        let metrics_collector = agent.get_metrics_collector();
        let current_metrics = match metrics_collector.read().await.collect_metrics().await {
            Ok(metrics) => metrics,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to collect metrics").into_response(),
        };
        
        // Build Prometheus metrics format
        let mut prometheus_metrics = String::new();
        
        // Agent status
        prometheus_metrics.push_str("# HELP orasi_agent_up Agent is running\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_up gauge\n");
        prometheus_metrics.push_str("orasi_agent_up 1\n\n");
        
        // Task metrics
        prometheus_metrics.push_str("# HELP orasi_agent_tasks_total Total tasks processed\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_tasks_total counter\n");
        prometheus_metrics.push_str(&format!("orasi_agent_tasks_total {}\n", current_metrics.tasks.total_processed));
        
        prometheus_metrics.push_str("# HELP orasi_agent_tasks_successful Tasks processed successfully\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_tasks_successful counter\n");
        prometheus_metrics.push_str(&format!("orasi_agent_tasks_successful {}\n", current_metrics.tasks.successful));
        
        prometheus_metrics.push_str("# HELP orasi_agent_tasks_failed Tasks that failed\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_tasks_failed counter\n");
        prometheus_metrics.push_str(&format!("orasi_agent_tasks_failed {}\n", current_metrics.tasks.failed));
        
        prometheus_metrics.push_str("# HELP orasi_agent_tasks_pending Tasks currently pending\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_tasks_pending gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_tasks_pending {}\n", current_metrics.tasks.pending));
        
        prometheus_metrics.push_str("# HELP orasi_agent_tasks_active Tasks currently active\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_tasks_active gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_tasks_active {}\n", current_metrics.tasks.active));
        
        prometheus_metrics.push_str("# HELP orasi_agent_tasks_processing_time_ms Average task processing time\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_tasks_processing_time_ms gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_tasks_processing_time_ms {}\n", current_metrics.tasks.avg_processing_time_ms));
        
        // Resource metrics
        prometheus_metrics.push_str("\n# HELP orasi_agent_cpu_percent CPU usage percentage\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_cpu_percent gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_cpu_percent {}\n", current_metrics.resources.cpu_percent));
        
        prometheus_metrics.push_str("# HELP orasi_agent_memory_bytes Memory usage in bytes\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_memory_bytes gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_memory_bytes {}\n", current_metrics.resources.memory_bytes));
        
        prometheus_metrics.push_str("# HELP orasi_agent_memory_percent Memory usage percentage\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_memory_percent gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_memory_percent {}\n", current_metrics.resources.memory_percent));
        
        prometheus_metrics.push_str("# HELP orasi_agent_disk_bytes Disk usage in bytes\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_disk_bytes gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_disk_bytes {}\n", current_metrics.resources.disk_bytes));
        
        prometheus_metrics.push_str("# HELP orasi_agent_disk_percent Disk usage percentage\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_disk_percent gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_disk_percent {}\n", current_metrics.resources.disk_percent));
        
        prometheus_metrics.push_str("# HELP orasi_agent_network_rx_bytes Network received bytes\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_network_rx_bytes counter\n");
        prometheus_metrics.push_str(&format!("orasi_agent_network_rx_bytes {}\n", current_metrics.resources.network_rx_bytes));
        
        prometheus_metrics.push_str("# HELP orasi_agent_network_tx_bytes Network transmitted bytes\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_network_tx_bytes counter\n");
        prometheus_metrics.push_str(&format!("orasi_agent_network_tx_bytes {}\n", current_metrics.resources.network_tx_bytes));
        
        // Performance metrics
        prometheus_metrics.push_str("\n# HELP orasi_agent_requests_per_second Requests per second\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_requests_per_second gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_requests_per_second {}\n", current_metrics.performance.requests_per_second));
        
        prometheus_metrics.push_str("# HELP orasi_agent_response_time_ms Average response time\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_response_time_ms gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_response_time_ms {}\n", current_metrics.performance.avg_response_time_ms));
        
        prometheus_metrics.push_str("# HELP orasi_agent_response_time_p95_ms 95th percentile response time\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_response_time_p95_ms gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_response_time_p95_ms {}\n", current_metrics.performance.p95_response_time_ms));
        
        prometheus_metrics.push_str("# HELP orasi_agent_response_time_p99_ms 99th percentile response time\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_response_time_p99_ms gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_response_time_p99_ms {}\n", current_metrics.performance.p99_response_time_ms));
        
        prometheus_metrics.push_str("# HELP orasi_agent_throughput_bytes_per_sec Throughput in bytes per second\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_throughput_bytes_per_sec gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_throughput_bytes_per_sec {}\n", current_metrics.performance.throughput_bytes_per_sec));
        
        // Error metrics
        prometheus_metrics.push_str("\n# HELP orasi_agent_errors_total Total errors\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_errors_total counter\n");
        prometheus_metrics.push_str(&format!("orasi_agent_errors_total {}\n", current_metrics.errors.total_errors));
        
        prometheus_metrics.push_str("# HELP orasi_agent_error_rate Error rate\n");
        prometheus_metrics.push_str("# TYPE orasi_agent_error_rate gauge\n");
        prometheus_metrics.push_str(&format!("orasi_agent_error_rate {}\n", current_metrics.errors.error_rate));
        
        // Error types
        for (error_type, count) in &current_metrics.errors.by_type {
            prometheus_metrics.push_str(&format!("# HELP orasi_agent_errors_by_type_total Errors by type\n"));
            prometheus_metrics.push_str(&format!("# TYPE orasi_agent_errors_by_type_total counter\n"));
            prometheus_metrics.push_str(&format!("orasi_agent_errors_by_type_total{{type=\"{}\"}} {}\n", error_type, count));
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            "text/plain; version=0.0.4; charset=utf-8".parse().unwrap(),
        );

        (StatusCode::OK, headers, prometheus_metrics).into_response()
    }

    /// Agent information endpoint
    async fn agent_info(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let agent = server.agent.read().await;

        let info = agent.get_agent_info().await;
        let response = json!({
            "agent_info": info,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        (StatusCode::OK, Json(response))
    }

    /// Agent status endpoint
    async fn agent_status(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let agent = server.agent.read().await;
        let status = agent.get_agent_status().await;

        let response = json!({
            "status": status,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        (StatusCode::OK, Json(response))
    }

    /// Task queue statistics endpoint
    async fn task_queue_stats(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let agent = server.agent.read().await;

        let stats = agent.get_task_queue_stats().await;
        let response = json!({
            "queue_stats": stats,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        (StatusCode::OK, Json(response))
    }

    /// Submit task endpoint
    async fn submit_task(
        State(server): State<Arc<Self>>,
        Json(task): Json<crate::processing::tasks::Task>,
    ) -> impl IntoResponse {
        let agent = server.agent.read().await;

        let task_id = task.task_id.clone();
        match agent.submit_task(task).await {
            Ok(_) => {
                let response = json!({
                    "message": "Task submitted successfully",
                    "task_id": task_id,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                (StatusCode::ACCEPTED, Json(response))
            }
            Err(e) => {
                error!("Failed to submit task: {}", e);
                let response = json!({
                    "error": e.to_string(),
                    "task_id": task_id,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                (StatusCode::BAD_REQUEST, Json(response))
            }
        }
    }

    /// Cluster state endpoint
    async fn cluster_state(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let agent = server.agent.read().await;

        let state = agent.get_cluster_state().await;
        let response = json!({
            "cluster_state": state,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        (StatusCode::OK, Json(response))
    }

    /// Cluster members endpoint
    async fn cluster_members(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let agent = server.agent.read().await;

        let state = agent.get_cluster_state().await;
        let members: Vec<_> = state.members.values().collect();
        let response = json!({
            "members": members,
            "total_members": members.len(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        (StatusCode::OK, Json(response))
    }

    /// Cluster leader endpoint
    async fn cluster_leader(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let agent = server.agent.read().await;

        let is_leader = agent.is_leader().await;
        let response = json!({
            "is_leader": is_leader,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        (StatusCode::OK, Json(response))
    }
}

impl Clone for HttpServer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            agent: self.agent.clone(),
            health_checker: self.health_checker.clone(),
            metrics_collector: self.metrics_collector.clone(),
        }
    }
}
