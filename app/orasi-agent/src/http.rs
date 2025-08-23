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
    async fn prometheus_metrics(State(_server): State<Arc<Self>>) -> impl IntoResponse {
        // TODO: Implement Prometheus metrics export
        let metrics = "# HELP orasi_agent_up Agent is running\n";
        let metrics = format!("{}# TYPE orasi_agent_up gauge\n", metrics);
        let metrics = format!("{}orasi_agent_up 1\n", metrics);

        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            "text/plain; version=0.0.4; charset=utf-8".parse().unwrap(),
        );

        (StatusCode::OK, headers, metrics)
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
