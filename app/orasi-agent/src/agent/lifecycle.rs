//! Agent lifecycle management implementation

use crate::cluster::{ClusterCoordinator, TaskDistributionMessage};
use crate::config::AgentConfig;
use crate::discovery::ServiceDiscovery;
use crate::error::AgentError;
use crate::health::{HealthChecker, HealthStatus};
use crate::metrics::{AgentMetrics, MetricsCollector};
use crate::processing::tasks::TaskType;
use crate::processing::{IndexingProcessor, IngestionProcessor, TaskProcessor};
use crate::state::AgentState;
use crate::types::{
    AgentInfo, AgentLoad, AgentLoad as TypesAgentLoad, AgentStatus, ClusterMessage, HealthState,
    Heartbeat, MemberStatus, Task, TaskResult,
};
use super::load_balancer::{LoadBalancer, LoadBalancingStrategy, LoadBalancerStats};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Main Orasi Agent implementation
pub struct OrasiAgent {
    /// Agent configuration
    config: AgentConfig,

    /// Agent state
    state: Arc<RwLock<AgentState>>,

    /// Cluster coordinator
    cluster_coordinator: Arc<RwLock<ClusterCoordinator>>,

    /// Service discovery
    service_discovery: Arc<RwLock<ServiceDiscovery>>,

    /// Task processor
    task_processor: Arc<RwLock<TaskProcessor>>,

    /// Health checker
    health_checker: Arc<RwLock<HealthChecker>>,

    /// Metrics collector
    metrics_collector: Arc<RwLock<MetricsCollector>>,

    /// Ingestion processor
    ingestion_processor: Arc<IngestionProcessor>,

    /// Indexing processor
    indexing_processor: Arc<IndexingProcessor>,

    /// Load balancer
    load_balancer: Arc<RwLock<LoadBalancer>>,

    /// Shutdown signal receiver
    shutdown_rx: mpsc::Receiver<()>,

    /// Shutdown signal sender
    shutdown_tx: mpsc::Sender<()>,
}

impl OrasiAgent {
    /// Create a new Orasi Agent
    pub async fn new(config: AgentConfig) -> Result<Self, AgentError> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        // Initialize agent state
        let state = Arc::new(RwLock::new(AgentState::new(&config).await?));

        // Initialize service discovery
        let service_discovery = Arc::new(RwLock::new(ServiceDiscovery::new(&config).await?));

        // Initialize cluster coordinator
        let cluster_coordinator = Arc::new(RwLock::new(
            ClusterCoordinator::new(&config, state.clone()).await?,
        ));

        // Initialize task processor
        let task_processor = Arc::new(RwLock::new(
            TaskProcessor::new(&config, state.clone()).await?,
        ));

        // Initialize health checker
        let health_checker = Arc::new(RwLock::new(
            HealthChecker::new(&config, state.clone()).await?,
        ));

        // Initialize metrics collector
        let metrics_collector = Arc::new(RwLock::new(MetricsCollector::new(&config).await?));

        // Initialize ingestion processor
        let ingestion_processor = Arc::new(IngestionProcessor::new(&config, state.clone()).await?);

        // Initialize indexing processor
        let indexing_processor = Arc::new(IndexingProcessor::new(&config, state.clone()).await?);

        // Initialize load balancer
        let load_balancer = Arc::new(RwLock::new(LoadBalancer::new(
            LoadBalancingStrategy::Hybrid,
        )));

        Ok(Self {
            config,
            state,
            cluster_coordinator,
            service_discovery,
            task_processor,
            health_checker,
            metrics_collector,
            ingestion_processor,
            indexing_processor,
            load_balancer,
            shutdown_rx,
            shutdown_tx,
        })
    }

    /// Start the agent
    pub async fn start(&mut self) -> Result<(), AgentError> {
        info!("Starting Orasi agent {}", self.config.agent_id);

        // Update agent status
        {
            let mut state = self.state.write().await;
            state.set_status(AgentStatus::Starting);
        }

        // Start service discovery
        self.service_discovery.write().await.start().await?;

        // Start cluster coordination
        self.cluster_coordinator.write().await.start().await?;

        // Start health checker
        {
            let mut health_checker = self.health_checker.write().await;
            health_checker.start().await?;
        }

        // Start metrics collector
        {
            let mut metrics_collector = self.metrics_collector.write().await;
            metrics_collector.start().await?;
        }

        // Start task processor
        self.task_processor.write().await.start().await?;

        // Join cluster
        self.cluster_coordinator
            .write()
            .await
            .join_cluster()
            .await?;

        // Register with service discovery
        self.register_agent().await?;

        // Update agent status
        {
            let mut state = self.state.write().await;
            state.set_status(AgentStatus::Running);
        }

        info!("Orasi agent started successfully");
        Ok(())
    }

    /// Run the agent main loop
    pub async fn run(&mut self) -> Result<(), AgentError> {
        info!("Running Orasi agent main loop");

        loop {
            // Handle shutdown signal
            if let Ok(_) = self.shutdown_rx.try_recv() {
                info!("Received shutdown signal");
                break;
            }

            // Handle cluster messages
            if let Ok(Some(message)) = self
                .cluster_coordinator
                .write()
                .await
                .receive_message()
                .await
            {
                if let Err(e) = self.handle_cluster_message(message).await {
                    error!("Error handling cluster message: {}", e);
                }
            }

            // Handle health check requests
            if let Ok(status) = self.perform_health_check().await {
                if let Err(e) = self.update_health_status(status).await {
                    error!("Error updating health status: {}", e);
                }
            }

            // Handle metrics collection
            if let Ok(metrics) = self.collect_metrics().await {
                if let Err(e) = self.update_metrics(metrics).await {
                    error!("Error updating metrics: {}", e);
                }
            }

            // Handle load balancing updates
            if let Err(e) = self.update_load_balancer().await {
                error!("Error updating load balancer: {}", e);
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Update load balancer with cluster member information
    async fn update_load_balancer(&self) -> Result<(), AgentError> {
        // Get cluster members
        let members = self
            .cluster_coordinator
            .read()
            .await
            .get_cluster_members()
            .await?;

        let mut load_balancer = self.load_balancer.write().await;

        for member in members {
            // Update health status based on member status
            let health = match member.status {
                MemberStatus::Active => HealthStatus::Healthy,
                MemberStatus::Inactive => HealthStatus::Degraded,
                MemberStatus::Unhealthy => HealthStatus::Unhealthy,
            };
            load_balancer.update_member_health(member.member_id.clone(), health);

            // Create default load metrics for now
            let load = TypesAgentLoad {
                active_tasks: 0,
                queue_length: 0,
                cpu_percent: 0.0,
                memory_bytes: 0,
                disk_bytes: 0,
            };
            load_balancer.update_member_load(member.member_id.clone(), load);
        }

        Ok(())
    }

    /// Select best member for task assignment using load balancing
    pub async fn select_task_member(&self, available_members: &[String]) -> Option<String> {
        let mut load_balancer = self.load_balancer.write().await;
        load_balancer.select_member(available_members)
    }

    /// Get load balancer statistics
    pub async fn get_load_balancer_stats(&self) -> LoadBalancerStats {
        let load_balancer = self.load_balancer.read().await;
        load_balancer.get_stats()
    }

    /// Set load balancing strategy
    pub async fn set_load_balancing_strategy(&self, strategy: LoadBalancingStrategy) {
        let mut load_balancer = self.load_balancer.write().await;
        load_balancer.set_strategy(strategy.clone());
        info!("Load balancing strategy changed to: {:?}", strategy);
    }

    /// Shutdown the agent
    pub async fn shutdown(&self) -> Result<(), AgentError> {
        info!("Shutting down Orasi agent");

        // Update agent status
        {
            let mut state = self.state.write().await;
            state.set_status(AgentStatus::Stopping);
        }

        // Stop task processor
        self.task_processor.write().await.stop().await?;

        // Stop metrics collector
        {
            let mut metrics_collector = self.metrics_collector.write().await;
            metrics_collector.stop().await?;
        }

        // Stop health checker
        {
            let mut health_checker = self.health_checker.write().await;
            health_checker.stop().await?;
        }

        // Stop cluster coordinator
        self.cluster_coordinator.write().await.stop().await?;

        // Stop service discovery
        self.service_discovery.write().await.stop().await?;

        // Update agent status
        {
            let mut state = self.state.write().await;
            state.set_status(AgentStatus::Stopped);
        }

        info!("Orasi agent shutdown completed");
        Ok(())
    }

    /// Request shutdown
    pub async fn request_shutdown(&self) -> Result<(), AgentError> {
        self.shutdown_tx
            .send(())
            .await
            .map_err(|_| AgentError::Shutdown("Failed to send shutdown signal".to_string()))?;
        Ok(())
    }

    /// Register agent with service discovery
    async fn register_agent(&self) -> Result<(), AgentError> {
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        // Register with service discovery
        let mut service_discovery = self.service_discovery.write().await;
        match service_discovery.register_agent(agent_info.clone()).await {
            Ok(_) => {
                info!(
                    "Agent successfully registered with service discovery: {}",
                    agent_info.agent_id
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to register agent with service discovery: {} - {}",
                    agent_info.agent_id, e
                );
                Err(AgentError::ServiceDiscovery(format!(
                    "Registration failed: {}",
                    e
                )))
            }
        }
    }

    /// Handle cluster message
    async fn handle_cluster_message(&self, message: ClusterMessage) -> Result<(), AgentError> {
        match message {
            ClusterMessage::TaskDistribution(distribution) => {
                // Handle task distribution
                // Convert types::TaskDistributionMessage to cluster::TaskDistributionMessage
                let cluster_distribution = TaskDistributionMessage {
                    task: distribution.task,
                    target_member: Some(distribution.target_member),
                    timestamp: distribution.timestamp,
                };
                self.handle_task_distribution(cluster_distribution).await?;
            }
            ClusterMessage::Heartbeat(heartbeat) => {
                // Handle heartbeat
                self.handle_heartbeat(heartbeat).await?;
            }
            _ => {
                // Handle other message types
                self.cluster_coordinator
                    .write()
                    .await
                    .handle_message(message)
                    .await?;
            }
        }

        Ok(())
    }

    /// Handle task distribution
    async fn handle_task_distribution(
        &self,
        distribution: crate::cluster::TaskDistributionMessage,
    ) -> Result<(), AgentError> {
        info!("Received task distribution: {}", distribution.task.task_id);

        // Submit task to processor
        self.task_processor
            .write()
            .await
            .submit_task(distribution.task)
            .await?;

        Ok(())
    }

    /// Handle heartbeat
    async fn handle_heartbeat(&self, heartbeat: Heartbeat) -> Result<(), AgentError> {
        // Update agent state with heartbeat information
        {
            let mut state = self.state.write().await;
            state.update_load_metrics(heartbeat.current_load);
        }

        Ok(())
    }

    /// Perform health check
    async fn perform_health_check(&self) -> Result<HealthStatus, AgentError> {
        let health_checker = self.health_checker.read().await;
        health_checker.check_health().await
    }

    /// Update health status
    async fn update_health_status(&self, status: crate::health::HealthStatus) -> Result<(), AgentError> {
        // Convert health::HealthStatus to types::HealthStatus
        let health_state = match status {
            crate::health::HealthStatus::Healthy => crate::types::HealthState::Healthy,
            crate::health::HealthStatus::Degraded => crate::types::HealthState::Degraded,
            crate::health::HealthStatus::Unhealthy => crate::types::HealthState::Unhealthy,
            crate::health::HealthStatus::Unknown => crate::types::HealthState::Degraded,
        };

        let types_health_status = crate::types::HealthStatus {
            service: "orasi-agent".to_string(),
            status: health_state,
            details: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        // Update agent state with health status
        {
            let mut state = self.state.write().await;
            state.update_health_status(types_health_status);
        }

        // Send heartbeat to cluster
        self.cluster_coordinator
            .write()
            .await
            .send_heartbeat()
            .await?;

        Ok(())
    }

    /// Collect metrics
    async fn collect_metrics(&self) -> Result<crate::metrics::AgentMetrics, AgentError> {
        let metrics_collector = self.metrics_collector.read().await;
        metrics_collector.collect_metrics().await
    }

    /// Update metrics
    async fn update_metrics(
        &self,
        metrics: crate::metrics::AgentMetrics,
    ) -> Result<(), AgentError> {
        // Update agent state with metrics
        {
            let mut state = self.state.write().await;
            // Convert AgentMetrics to AgentLoad for state update
            let load_metrics = AgentLoad {
                active_tasks: metrics.tasks.active as usize,
                queue_length: metrics.tasks.pending as usize,
                cpu_percent: metrics.resources.cpu_percent,
                memory_bytes: metrics.resources.memory_bytes,
                disk_bytes: metrics.resources.disk_bytes,
            };
            state.update_load_metrics(load_metrics);
        }

        Ok(())
    }

    /// Get agent information
    pub async fn get_agent_info(&self) -> AgentInfo {
        let state = self.state.read().await;
        state.get_agent_info()
    }

    /// Get agent status
    pub async fn get_agent_status(&self) -> AgentStatus {
        let state = self.state.read().await;
        state.get_status()
    }

    /// Submit task for processing
    pub async fn submit_task(&self, task: Task) -> Result<(), AgentError> {
        self.task_processor.write().await.submit_task(task).await
    }

    /// Get task queue statistics
    pub async fn get_task_queue_stats(&self) -> crate::processing::tasks::QueueStats {
        self.task_processor.read().await.get_queue_stats().await
    }

    /// Get cluster state
    pub async fn get_cluster_state(&self) -> crate::cluster::ClusterState {
        self.cluster_coordinator
            .read()
            .await
            .get_cluster_state()
            .await
    }

    /// Check if agent is leader
    pub async fn is_leader(&self) -> bool {
        self.cluster_coordinator.read().await.is_leader().await
    }

    /// Distribute task to cluster
    pub async fn distribute_task(
        &self,
        task: Task,
        target_member: Option<String>,
    ) -> Result<(), AgentError> {
        self.cluster_coordinator
            .write()
            .await
            .distribute_task(task, target_member)
            .await
    }

    /// Get health checker
    pub fn get_health_checker(&self) -> Arc<RwLock<HealthChecker>> {
        self.health_checker.clone()
    }

    /// Get metrics collector
    pub fn get_metrics_collector(&self) -> Arc<RwLock<MetricsCollector>> {
        self.metrics_collector.clone()
    }
}
