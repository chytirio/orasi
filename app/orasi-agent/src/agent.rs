//! Main agent implementation

use crate::{
    cluster::ClusterCoordinator, config::AgentConfig, error::AgentError, health::HealthChecker,
    metrics::MetricsCollector, processing::{TaskProcessor, IndexingProcessor, IngestionProcessor},
    discovery::ServiceDiscovery, state::AgentState, types::*,
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

/// Main Orasi Agent implementation
pub struct OrasiAgent {
    /// Agent configuration
    config: AgentConfig,

    /// Agent state
    state: Arc<RwLock<AgentState>>,

    /// Cluster coordinator
    cluster_coordinator: Arc<ClusterCoordinator>,

    /// Service discovery
    service_discovery: Arc<ServiceDiscovery>,

    /// Task processor
    task_processor: Arc<TaskProcessor>,

    /// Health checker
    health_checker: Arc<HealthChecker>,

    /// Metrics collector
    metrics_collector: Arc<MetricsCollector>,

    /// Ingestion processor
    ingestion_processor: Arc<IngestionProcessor>,

    /// Indexing processor
    indexing_processor: Arc<IndexingProcessor>,

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
        let service_discovery = Arc::new(ServiceDiscovery::new(&config).await?);

        // Initialize cluster coordinator
        let cluster_coordinator = Arc::new(ClusterCoordinator::new(&config, state.clone()).await?);

        // Initialize task processor
        let task_processor = Arc::new(TaskProcessor::new(&config, state.clone()).await?);

        // Initialize health checker
        let health_checker = Arc::new(HealthChecker::new(&config).await?);

        // Initialize metrics collector
        let metrics_collector = Arc::new(MetricsCollector::new(&config).await?);

        // Initialize ingestion processor
        let ingestion_processor = Arc::new(IngestionProcessor::new(&config, state.clone()).await?);

        // Initialize indexing processor
        let indexing_processor = Arc::new(IndexingProcessor::new(&config, state.clone()).await?);

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
        self.service_discovery.start().await?;

        // Start cluster coordination
        self.cluster_coordinator.start().await?;

        // Start health checker
        self.health_checker.start().await?;

        // Start metrics collector
        self.metrics_collector.start().await?;

        // Start task processor
        self.task_processor.start().await?;

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
            tokio::select! {
                // Handle shutdown signal
                _ = self.shutdown_rx.recv() => {
                    info!("Received shutdown signal");
                    break;
                }

                // Handle cluster messages
                result = self.cluster_coordinator.receive_message() => {
                    match result {
                        Ok(Some(message)) => {
                            if let Err(e) = self.handle_cluster_message(message).await {
                                error!("Error handling cluster message: {}", e);
                            }
                        }
                        Ok(None) => {
                            // No message available, continue
                        }
                        Err(e) => {
                            error!("Error receiving cluster message: {}", e);
                        }
                    }
                }

                // Handle health check requests
                result = self.health_checker.check_health() => {
                    match result {
                        Ok(status) => {
                            if let Err(e) = self.update_health_status(status).await {
                                error!("Error updating health status: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error during health check: {}", e);
                        }
                    }
                }

                // Handle metrics collection
                result = self.metrics_collector.collect_metrics() => {
                    match result {
                        Ok(metrics) => {
                            if let Err(e) = self.update_metrics(metrics).await {
                                error!("Error updating metrics: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error collecting metrics: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Shutdown the agent
    pub async fn shutdown(&self) -> Result<(), AgentError> {
        info!("Shutting down Orasi agent");

        // Update agent status
        {
            let mut state = self.state.write().await;
            state.set_status(AgentStatus::Stopping);
        }

        // Deregister from service discovery
        self.deregister_agent().await?;

        // Stop task processor
        self.task_processor.stop().await?;

        // Stop metrics collector
        self.metrics_collector.stop().await?;

        // Stop health checker
        self.health_checker.stop().await?;

        // Stop cluster coordination
        self.cluster_coordinator.stop().await?;

        // Stop service discovery
        self.service_discovery.stop().await?;

        // Update agent status
        {
            let mut state = self.state.write().await;
            state.set_status(AgentStatus::Stopped);
        }

        info!("Orasi agent shutdown completed");
        Ok(())
    }

    /// Send shutdown signal
    pub async fn send_shutdown_signal(&self) -> Result<(), AgentError> {
        self.shutdown_tx
            .send(())
            .await
            .map_err(|e| AgentError::Shutdown(e.to_string()))?;
        Ok(())
    }

    /// Register agent with service discovery
    async fn register_agent(&self) -> Result<(), AgentError> {
        let state = self.state.read().await;
        let agent_info = state.get_agent_info();

        self.service_discovery
            .register_agent(agent_info)
            .await
            .map_err(|e| AgentError::ServiceDiscovery(e.to_string()))?;

        Ok(())
    }

    /// Deregister agent from service discovery
    async fn deregister_agent(&self) -> Result<(), AgentError> {
        self.service_discovery
            .deregister_agent(&self.config.agent_id)
            .await
            .map_err(|e| AgentError::ServiceDiscovery(e.to_string()))?;

        Ok(())
    }

    /// Handle cluster messages
    async fn handle_cluster_message(&self, message: ClusterMessage) -> Result<(), AgentError> {
        match message {
            ClusterMessage::TaskAssignment(task) => {
                self.handle_task_assignment(task).await?;
            }
            ClusterMessage::HealthCheck(agent_id) => {
                if agent_id == self.config.agent_id {
                    self.handle_health_check().await?;
                }
            }
            _ => {
                // Handle other message types as needed
                warn!(
                    "Unhandled cluster message type: {:?}",
                    std::mem::discriminant(&message)
                );
            }
        }

        Ok(())
    }

    /// Handle task assignment
    async fn handle_task_assignment(&self, task: Task) -> Result<(), AgentError> {
        info!("Received task assignment: {}", task.task_id);

        // Check if task is expired
        if is_task_expired(&task) {
            warn!("Task {} is expired, skipping", task.task_id);
            return Ok(());
        }

        // Check if we can handle this task type
        if !self.can_handle_task_type(&task.task_type) {
            warn!("Cannot handle task type: {:?}", task.task_type);
            return Ok(());
        }

        // Submit task to processor
        self.task_processor
            .submit_task(task)
            .await
            .map_err(|e| AgentError::TaskProcessing(e.to_string()))?;

        Ok(())
    }

    /// Handle health check request
    async fn handle_health_check(&self) -> Result<(), AgentError> {
        let status = self.health_checker.get_health_status().await?;

        // Send health check response to cluster
        self.cluster_coordinator
            .send_message(ClusterMessage::HealthCheckResponse(status))
            .await
            .map_err(|e| AgentError::Cluster(e.to_string()))?;

        Ok(())
    }

    /// Update health status
    async fn update_health_status(&self, status: HealthStatus) -> Result<(), AgentError> {
        let mut state = self.state.write().await;
        state.update_health_status(status);
        Ok(())
    }

    /// Update metrics
    async fn update_metrics(&self, metrics: AgentLoad) -> Result<(), AgentError> {
        let mut state = self.state.write().await;
        state.update_load_metrics(metrics);
        Ok(())
    }

    /// Check if agent can handle a specific task type
    fn can_handle_task_type(&self, task_type: &TaskType) -> bool {
        let state = self.state.blocking_read();
        let capabilities = &state.get_agent_info().capabilities;
        capabilities.task_types.contains(task_type)
    }

    /// Get agent information
    pub async fn get_agent_info(&self) -> AgentInfo {
        let state = self.state.read().await;
        state.get_agent_info()
    }

    /// Get agent status
    pub async fn get_status(&self) -> AgentStatus {
        let state = self.state.read().await;
        state.get_status()
    }
}
