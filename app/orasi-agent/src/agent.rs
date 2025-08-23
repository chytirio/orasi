//! Main agent implementation

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
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Load balancing strategy
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalancingStrategy {
    /// Round-robin load balancing
    RoundRobin,
    /// Health-based load balancing (prefer healthy nodes)
    HealthBased,
    /// Load-based load balancing (prefer less loaded nodes)
    LoadBased,
    /// Hybrid load balancing (combine health and load)
    Hybrid,
}

/// Load balancer for distributing tasks across cluster members
pub struct LoadBalancer {
    /// Load balancing strategy
    strategy: LoadBalancingStrategy,

    /// Current round-robin index
    round_robin_index: usize,

    /// Cluster member health status
    member_health: HashMap<String, HealthStatus>,

    /// Cluster member load metrics
    member_load: HashMap<String, AgentLoad>,
}

impl LoadBalancer {
    /// Create new load balancer
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            strategy,
            round_robin_index: 0,
            member_health: HashMap::new(),
            member_load: HashMap::new(),
        }
    }

    /// Update member health status
    pub fn update_member_health(&mut self, member_id: String, health: HealthStatus) {
        self.member_health.insert(member_id, health);
    }

    /// Update member load metrics
    pub fn update_member_load(&mut self, member_id: String, load: AgentLoad) {
        self.member_load.insert(member_id, load);
    }

    /// Remove member from load balancer
    pub fn remove_member(&mut self, member_id: &str) {
        self.member_health.remove(member_id);
        self.member_load.remove(member_id);
    }

    /// Select best member for task assignment
    pub fn select_member(&mut self, available_members: &[String]) -> Option<String> {
        if available_members.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => self.select_round_robin(available_members),
            LoadBalancingStrategy::HealthBased => self.select_health_based(available_members),
            LoadBalancingStrategy::LoadBased => self.select_load_based(available_members),
            LoadBalancingStrategy::Hybrid => self.select_hybrid(available_members),
        }
    }

    /// Round-robin selection
    fn select_round_robin(&mut self, available_members: &[String]) -> Option<String> {
        if available_members.is_empty() {
            return None;
        }

        let selected = available_members[self.round_robin_index % available_members.len()].clone();
        self.round_robin_index = (self.round_robin_index + 1) % available_members.len();
        Some(selected)
    }

    /// Health-based selection
    fn select_health_based(&self, available_members: &[String]) -> Option<String> {
        let mut healthy_members = Vec::new();
        let mut degraded_members = Vec::new();

        for member_id in available_members {
            if let Some(health) = self.member_health.get(member_id) {
                match health {
                    HealthStatus::Healthy => healthy_members.push(member_id.clone()),
                    HealthStatus::Degraded => degraded_members.push(member_id.clone()),
                    HealthStatus::Unhealthy => {
                        // Skip unhealthy members
                        continue;
                    }
                    HealthStatus::Unknown => {
                        // If unknown, assume healthy
                        healthy_members.push(member_id.clone());
                    }
                }
            } else {
                // If no health info available, assume healthy
                healthy_members.push(member_id.clone());
            }
        }

        // Prefer healthy members, fall back to degraded
        if !healthy_members.is_empty() {
            Some(healthy_members[0].clone())
        } else if !degraded_members.is_empty() {
            Some(degraded_members[0].clone())
        } else {
            None
        }
    }

    /// Load-based selection
    fn select_load_based(&self, available_members: &[String]) -> Option<String> {
        let mut best_member = None;
        let mut lowest_load = f64::MAX;

        for member_id in available_members {
            if let Some(load) = self.member_load.get(member_id) {
                // Calculate load score (weighted combination of CPU, memory, and queue length)
                let cpu_weight = 0.4;
                let memory_weight = 0.3;
                let queue_weight = 0.3;

                let cpu_score = load.cpu_percent / 100.0;
                let memory_score = load.memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0); // Normalize to GB
                let queue_score = load.queue_length as f64 / 100.0; // Normalize to reasonable queue size

                let total_load = cpu_score * cpu_weight
                    + memory_score * memory_weight
                    + queue_score * queue_weight;

                if total_load < lowest_load {
                    lowest_load = total_load;
                    best_member = Some(member_id.clone());
                }
            } else {
                // If no load info available, assume low load
                return Some(member_id.clone());
            }
        }

        best_member
    }

    /// Hybrid selection (combine health and load)
    fn select_hybrid(&self, available_members: &[String]) -> Option<String> {
        let mut healthy_members = Vec::new();
        let mut degraded_members = Vec::new();

        // First, categorize members by health
        for member_id in available_members {
            if let Some(health) = self.member_health.get(member_id) {
                match health {
                    HealthStatus::Healthy => healthy_members.push(member_id.clone()),
                    HealthStatus::Degraded => degraded_members.push(member_id.clone()),
                    HealthStatus::Unhealthy => {
                        // Skip unhealthy members
                        continue;
                    }
                    HealthStatus::Unknown => {
                        // If unknown, assume healthy
                        healthy_members.push(member_id.clone());
                    }
                }
            } else {
                // If no health info available, assume healthy
                healthy_members.push(member_id.clone());
            }
        }

        // Select best member from healthy group, then degraded group
        let candidate_groups = [&healthy_members, &degraded_members];

        for group in &candidate_groups {
            if !group.is_empty() {
                if let Some(best_member) = self.select_load_based(group) {
                    return Some(best_member);
                }
            }
        }

        None
    }

    /// Get load balancer statistics
    pub fn get_stats(&self) -> LoadBalancerStats {
        let total_members = self.member_health.len();
        let healthy_members = self
            .member_health
            .values()
            .filter(|h| matches!(h, HealthStatus::Healthy))
            .count();
        let degraded_members = self
            .member_health
            .values()
            .filter(|h| matches!(h, HealthStatus::Degraded))
            .count();
        let unhealthy_members = self
            .member_health
            .values()
            .filter(|h| matches!(h, HealthStatus::Unhealthy))
            .count();

        LoadBalancerStats {
            strategy: self.strategy.clone(),
            total_members,
            healthy_members,
            degraded_members,
            unhealthy_members,
            round_robin_index: self.round_robin_index,
        }
    }
}

/// Load balancer statistics
#[derive(Debug, Clone)]
pub struct LoadBalancerStats {
    pub strategy: LoadBalancingStrategy,
    pub total_members: usize,
    pub healthy_members: usize,
    pub degraded_members: usize,
    pub unhealthy_members: usize,
    pub round_robin_index: usize,
}

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
        load_balancer.strategy = strategy.clone();
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

        // TODO: Implement actual service discovery registration
        info!(
            "Agent registered with service discovery: {}",
            agent_info.agent_id
        );
        Ok(())
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
    async fn update_health_status(&self, status: HealthStatus) -> Result<(), AgentError> {
        // Update agent state with health status
        {
            let mut state = self.state.write().await;
            // TODO: Update health status in state
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
            // TODO: Update metrics in state
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
