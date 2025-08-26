//! Cluster coordinator implementation

use crate::agent::LoadBalancingStrategy;
use crate::config::{AgentConfig, ServiceDiscoveryBackend};
use crate::error::AgentError;
use crate::processing::data::DataProcessingProcessor;
use crate::processing::maintenance::MaintenanceProcessor;
use crate::processing::query::QueryProcessor;
use crate::processing::tasks::{
    IndexingTask, MaintenanceTask, ProcessingTask, QueryTask, Task, TaskPayload, TaskResult,
};
use crate::processing::{IndexingProcessor, IngestionProcessor};
use crate::state::AgentState;
use crate::types::{
    AgentCapabilities, AgentLoad, AgentStatus, ClusterMember as TypesClusterMember, ClusterMessage,
    Heartbeat, LeaderElectionMessage as TypesLeaderElectionMessage, MemberStatus,
    StateSyncMessage as TypesStateSyncMessage,
    TaskDistributionMessage as TypesTaskDistributionMessage,
};
use super::types::{ClusterMember, ClusterState, ClusterHealthStatus, current_timestamp};
use super::leader_election::{LeaderElectionState, ElectionStatus};
use super::task_distribution::TaskDistributionMessage;
use super::state_sync::StateSyncMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Cluster coordinator for managing distributed agent coordination
pub struct ClusterCoordinator {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
    cluster_state: Arc<RwLock<ClusterState>>,
    leader_election_state: Arc<RwLock<LeaderElectionState>>,
    message_sender: mpsc::Sender<ClusterMessage>,
    message_receiver: mpsc::Receiver<ClusterMessage>,
    running: bool,
}

impl ClusterCoordinator {
    /// Create new cluster coordinator
    pub async fn new(
        config: &AgentConfig,
        state: Arc<RwLock<AgentState>>,
    ) -> Result<Self, AgentError> {
        let (message_sender, message_receiver) = mpsc::channel(1000);

        let cluster_state = Arc::new(RwLock::new(ClusterState {
            members: HashMap::new(),
            leader: None,
            health_status: ClusterHealthStatus {
                total_members: 0,
                healthy_members: 0,
                degraded_members: 0,
                unhealthy_members: 0,
                last_check: 0,
            },
            last_update: current_timestamp(),
        }));

        let leader_election_state = Arc::new(RwLock::new(LeaderElectionState::default()));

        Ok(Self {
            config: config.clone(),
            state,
            cluster_state,
            message_sender,
            message_receiver,
            leader_election_state,
            running: false,
        })
    }

    /// Start cluster coordinator
    pub async fn start(&mut self) -> Result<(), AgentError> {
        if self.running {
            return Ok(());
        }

        info!("Starting cluster coordinator");
        self.running = true;

        // Start cluster coordination loop
        let cluster_state = self.cluster_state.clone();
        let leader_election_state = self.leader_election_state.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            Self::cluster_coordination_loop(cluster_state, leader_election_state, config).await;
        });

        // Start leader election if enabled
        if self.config.cluster.leader_election.enabled {
            self.start_leader_election().await?;
        }

        info!("Cluster coordinator started successfully");
        Ok(())
    }

    /// Stop cluster coordinator
    pub async fn stop(&mut self) -> Result<(), AgentError> {
        if !self.running {
            return Ok(());
        }

        info!("Stopping cluster coordinator");
        self.running = false;

        // Resign as leader if we are the leader
        if self.is_leader().await {
            self.resign_leadership().await?;
        }

        // Leave cluster
        self.leave_cluster().await?;

        info!("Cluster coordinator stopped");
        Ok(())
    }

    /// Join cluster
    pub async fn join_cluster(&self) -> Result<(), AgentError> {
        info!("Joining cluster");

        // Register with service discovery
        self.register_with_service_discovery().await?;

        // Send join notification to other members
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        let member = ClusterMember {
            member_id: agent_info.agent_id.clone(),
            endpoint: agent_info.endpoint.clone(),
            status: MemberStatus::Active,
            capabilities: agent_info.capabilities.clone(),
            last_heartbeat: current_timestamp(),
            metadata: agent_info.metadata.clone(),
        };

        // Add self to cluster state
        {
            let mut cluster_state = self.cluster_state.write().await;
            cluster_state
                .members
                .insert(agent_info.agent_id.clone(), member.clone());
        }

        // Convert cluster::ClusterMember to types::ClusterMember
        let types_member = TypesClusterMember {
            member_id: member.member_id,
            endpoint: member.endpoint,
            status: member.status,
            capabilities: member.capabilities,
            last_heartbeat: member.last_heartbeat,
            metadata: member.metadata,
        };
        let message = ClusterMessage::MemberJoin(types_member);
        self.send_message(message).await?;

        info!("Successfully joined cluster");
        Ok(())
    }

    /// Leave cluster
    pub async fn leave_cluster(&self) -> Result<(), AgentError> {
        info!("Leaving cluster");

        // Send leave notification
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        let message = ClusterMessage::MemberLeave(agent_info.agent_id.clone());
        self.send_message(message).await?;

        // Remove self from cluster state
        {
            let mut cluster_state = self.cluster_state.write().await;
            cluster_state.members.remove(&agent_info.agent_id);
        }

        // Unregister from service discovery
        self.unregister_from_service_discovery().await?;

        info!("Successfully left cluster");
        Ok(())
    }

    /// Send heartbeat
    pub async fn send_heartbeat(&self) -> Result<(), AgentError> {
        let heartbeat = {
            let state = self.state.read().await;
            Heartbeat {
                agent_id: state.get_agent_info().agent_id,
                status: state.get_status(),
                current_load: state.get_load_metrics(),
                timestamp: current_timestamp(),
            }
        };

        let message = ClusterMessage::Heartbeat(heartbeat);
        self.send_message(message).await?;

        Ok(())
    }

    /// Receive cluster message
    pub async fn receive_message(&mut self) -> Result<Option<ClusterMessage>, AgentError> {
        if !self.running {
            return Ok(None);
        }

        match self.message_receiver.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(e) => Err(AgentError::Cluster(format!(
                "Failed to receive message: {}",
                e
            ))),
        }
    }

    /// Get cluster state
    pub async fn get_cluster_state(&self) -> ClusterState {
        let cluster_state = self.cluster_state.read().await;
        cluster_state.clone()
    }

    /// Get cluster members
    pub async fn get_cluster_members(&self) -> Result<Vec<ClusterMember>, AgentError> {
        let cluster_state = self.cluster_state.read().await;
        Ok(cluster_state.members.values().cloned().collect())
    }

    /// Check if this agent is the leader
    pub async fn is_leader(&self) -> bool {
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        let cluster_state = self.cluster_state.read().await;
        cluster_state.leader.as_ref() == Some(&agent_info.agent_id)
    }

    /// Get current leader
    pub async fn get_current_leader(&self) -> Option<String> {
        let cluster_state = self.cluster_state.read().await;
        cluster_state.leader.clone()
    }

    /// Distribute task to cluster
    pub async fn distribute_task(
        &self,
        task: Task,
        target_member: Option<String>,
    ) -> Result<(), AgentError> {
        let message = ClusterMessage::TaskDistribution(TypesTaskDistributionMessage {
            task,
            target_member: target_member.unwrap_or_default(),
            timestamp: current_timestamp(),
            priority: 1, // Default priority
        });

        self.send_message(message).await?;
        Ok(())
    }

    /// Register agent with service discovery
    pub async fn register_agent(&self, agent_info: &crate::types::AgentInfo) -> Result<(), AgentError> {
        // Create service discovery instance
        let mut service_discovery = crate::discovery::ServiceDiscovery::new(&self.config).await?;

        // Register the agent
        service_discovery.register_agent(agent_info.clone()).await?;

        info!("Registered agent with service discovery: {}", agent_info.agent_id);
        Ok(())
    }

    /// Send message to cluster
    async fn send_message(&self, message: ClusterMessage) -> Result<(), AgentError> {
        self.message_sender
            .send(message)
            .await
            .map_err(|e| AgentError::Cluster(format!("Failed to send message: {}", e)))?;
        Ok(())
    }

    /// Register with service discovery
    async fn register_with_service_discovery(&self) -> Result<(), AgentError> {
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        // Create service discovery instance
        let mut service_discovery = crate::discovery::ServiceDiscovery::new(&self.config).await?;

        // Register the agent
        service_discovery.register_agent(agent_info).await?;

        info!("Registered with service discovery");
        Ok(())
    }

    /// Unregister from service discovery
    async fn unregister_from_service_discovery(&self) -> Result<(), AgentError> {
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        // Create service discovery instance
        let mut service_discovery = crate::discovery::ServiceDiscovery::new(&self.config).await?;

        // Deregister the agent
        service_discovery
            .deregister_agent(&agent_info.agent_id)
            .await?;

        info!("Unregistered from service discovery");
        Ok(())
    }

    /// Start leader election
    async fn start_leader_election(&self) -> Result<(), AgentError> {
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        let election_id = Uuid::new_v4().to_string();
        let now = current_timestamp();

        // Update election state
        {
            let mut election_state = self.leader_election_state.write().await;
            election_state.start_election(election_id.clone());
        }

        let message = ClusterMessage::LeaderElection(TypesLeaderElectionMessage {
            election_id,
            candidate_id: agent_info.agent_id,
            term: 1, // Default term
            timestamp: now,
        });

        info!("Starting leader election campaign");
        self.send_message(message).await?;
        Ok(())
    }

    /// Resign leadership
    async fn resign_leadership(&self) -> Result<(), AgentError> {
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        let message = ClusterMessage::LeaderElection(TypesLeaderElectionMessage {
            election_id: Uuid::new_v4().to_string(),
            candidate_id: agent_info.agent_id,
            term: 1, // Default term
            timestamp: current_timestamp(),
        });

        info!("Resigning leadership");
        self.send_message(message).await?;
        Ok(())
    }

    /// Cluster coordination loop
    async fn cluster_coordination_loop(
        cluster_state: Arc<RwLock<ClusterState>>,
        leader_election_state: Arc<RwLock<LeaderElectionState>>,
        config: AgentConfig,
    ) {
        let mut heartbeat_interval = tokio::time::interval(config.cluster.heartbeat_interval);
        let mut cleanup_interval = tokio::time::interval(Duration::from_secs(60));
        let mut election_check_interval = tokio::time::interval(Duration::from_secs(10));

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    // Send heartbeat
                    Self::send_heartbeat_to_cluster(cluster_state.clone()).await;
                }
                _ = cleanup_interval.tick() => {
                    // Clean up stale members
                    Self::cleanup_stale_members(cluster_state.clone()).await;
                }
                _ = election_check_interval.tick() => {
                    // Check election timeout
                    Self::check_election_timeout(leader_election_state.clone()).await;
                }
            }
        }
    }

    /// Send heartbeat to cluster
    async fn send_heartbeat_to_cluster(cluster_state: Arc<RwLock<ClusterState>>) {
        // Create a mock heartbeat since we don't have access to the full agent state here
        // In a real implementation, this would get the actual agent state
        let heartbeat = Heartbeat {
            agent_id: "mock-agent-id".to_string(), // This would be the actual agent ID
            status: AgentStatus::Running,
            current_load: AgentLoad::default(),
            timestamp: current_timestamp(),
        };

        // Update cluster state with heartbeat
        let mut state = cluster_state.write().await;
        if let Some(member) = state.members.get_mut(&heartbeat.agent_id) {
            member.last_heartbeat = heartbeat.timestamp;
            member.status = match heartbeat.status {
                AgentStatus::Running => MemberStatus::Active,
                AgentStatus::Starting => MemberStatus::Active,
                AgentStatus::Stopping => MemberStatus::Inactive,
                AgentStatus::Stopped => MemberStatus::Inactive,
                AgentStatus::Error => MemberStatus::Unhealthy,
            };
        }
        state.last_update = heartbeat.timestamp;

        info!("Sent heartbeat to cluster");
    }

    /// Check election timeout
    async fn check_election_timeout(election_state: Arc<RwLock<LeaderElectionState>>) {
        let mut state = election_state.write().await;

        if state.has_election_timed_out() {
            if let Some(election_id) = &state.current_election_id {
                warn!("Election {} timed out, resetting to idle", election_id);
                state.resign_leadership();
            }
        }
    }

    /// Clean up stale members
    async fn cleanup_stale_members(cluster_state: Arc<RwLock<ClusterState>>) {
        let now = current_timestamp();
        let session_timeout = 90_000; // 90 seconds in milliseconds

        let mut state = cluster_state.write().await;
        let mut to_remove = Vec::new();

        for (member_id, member) in &state.members {
            if now - member.last_heartbeat > session_timeout {
                to_remove.push(member_id.clone());
            }
        }

        for member_id in to_remove {
            state.members.remove(&member_id);
            warn!("Removed stale member: {}", member_id);
        }

        state.last_update = now;
    }

    /// Handle cluster message
    pub async fn handle_message(&self, message: ClusterMessage) -> Result<(), AgentError> {
        match message {
            ClusterMessage::Heartbeat(heartbeat) => {
                self.handle_heartbeat(heartbeat).await?;
            }
            ClusterMessage::MemberJoin(member) => {
                self.handle_member_join(member).await?;
            }
            ClusterMessage::MemberLeave(member_id) => {
                self.handle_member_leave(member_id).await?;
            }
            ClusterMessage::LeaderElection(election) => {
                self.handle_leader_election(election).await?;
            }
            ClusterMessage::TaskDistribution(distribution) => {
                self.handle_task_distribution(distribution).await?;
            }
            ClusterMessage::StateSync(sync) => {
                // Note: StateSyncMessage types are different between modules
                // This is a simplified implementation
            }
            _ => {
                // Handle other message types
                info!("Received unhandled message type");
            }
        }

        Ok(())
    }

    /// Handle heartbeat message
    async fn handle_heartbeat(&self, heartbeat: Heartbeat) -> Result<(), AgentError> {
        let mut state = self.cluster_state.write().await;

        if let Some(member) = state.members.get_mut(&heartbeat.agent_id) {
            member.last_heartbeat = heartbeat.timestamp;
            member.status = match heartbeat.status {
                AgentStatus::Running => MemberStatus::Active,
                AgentStatus::Starting => MemberStatus::Active,
                AgentStatus::Stopping => MemberStatus::Inactive,
                AgentStatus::Stopped => MemberStatus::Inactive,
                AgentStatus::Error => MemberStatus::Unhealthy,
            };
        }

        state.last_update = current_timestamp();
        Ok(())
    }

    /// Handle member join
    async fn handle_member_join(&self, member: TypesClusterMember) -> Result<(), AgentError> {
        let member_id = member.member_id.clone();
        let mut state = self.cluster_state.write().await;
        // Note: ClusterMember types are different between modules
        // This is a simplified implementation
        state.last_update = current_timestamp();

        info!("Member joined cluster: {}", member_id);
        Ok(())
    }

    /// Handle member leave
    async fn handle_member_leave(&self, member_id: String) -> Result<(), AgentError> {
        let mut state = self.cluster_state.write().await;
        state.members.remove(&member_id);
        state.last_update = current_timestamp();

        // If the leader left, start a new election
        if state.leader.as_ref() == Some(&member_id) {
            state.leader = None;
            info!("Leader left cluster, starting new election");
            self.start_leader_election().await?;
        }

        info!("Member left cluster: {}", member_id);
        Ok(())
    }

    /// Handle leader election
    async fn handle_leader_election(
        &self,
        election: TypesLeaderElectionMessage,
    ) -> Result<(), AgentError> {
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        // Handle election message
        info!("Received leader election message from: {}", election.candidate_id);

        // For now, just acknowledge the election
        // In a real implementation, this would involve voting logic
        Ok(())
    }

    /// Handle task distribution
    async fn handle_task_distribution(
        &self,
        distribution: TypesTaskDistributionMessage,
    ) -> Result<(), AgentError> {
        info!("Received task distribution: {}", distribution.task.task_id);

        // Check if we can handle this task type
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        let can_handle = agent_info
            .capabilities
            .task_types
            .contains(&distribution.task.task_type);

        if can_handle {
            // Process the task
            match distribution.task.task_type {
                crate::processing::tasks::TaskType::Ingestion => {
                    // Handle ingestion task
                    info!("Processing ingestion task: {}", distribution.task.task_id);
                }
                crate::processing::tasks::TaskType::Indexing => {
                    // Handle indexing task
                    info!("Processing indexing task: {}", distribution.task.task_id);
                }
                crate::processing::tasks::TaskType::Processing => {
                    // Handle processing task
                    info!("Processing data processing task: {}", distribution.task.task_id);
                }
                crate::processing::tasks::TaskType::Query => {
                    // Handle query task
                    info!("Processing query task: {}", distribution.task.task_id);
                }
                crate::processing::tasks::TaskType::Maintenance => {
                    // Handle maintenance task
                    info!("Processing maintenance task: {}", distribution.task.task_id);
                }
            }
        } else {
            warn!(
                "Cannot handle task type {:?} for task: {}",
                distribution.task.task_type, distribution.task.task_id
            );
        }

        Ok(())
    }
}
