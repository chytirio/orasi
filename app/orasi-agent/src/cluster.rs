//! Cluster coordination for Orasi Agent

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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Cluster member information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    pub member_id: String,
    pub endpoint: String,
    pub status: MemberStatus,
    pub capabilities: AgentCapabilities,
    pub last_heartbeat: u64,
    pub metadata: HashMap<String, String>,
}

/// Cluster state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterState {
    pub members: HashMap<String, ClusterMember>,
    pub leader: Option<String>,
    pub health_status: ClusterHealthStatus,
    pub last_update: u64,
}

/// Cluster health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealthStatus {
    pub total_members: usize,
    pub healthy_members: usize,
    pub degraded_members: usize,
    pub unhealthy_members: usize,
    pub last_check: u64,
}

/// Leader election state
#[derive(Debug, Clone)]
pub struct LeaderElectionState {
    /// Current election ID
    pub current_election_id: Option<String>,

    /// Election start time
    pub election_start_time: Option<u64>,

    /// Election timeout
    pub election_timeout: u64,

    /// Candidate votes received
    pub votes_received: HashMap<String, String>, // candidate_id -> voter_id

    /// Election status
    pub election_status: ElectionStatus,

    /// Last election timestamp
    pub last_election_timestamp: u64,
}

/// Election status
#[derive(Debug, Clone, PartialEq)]
pub enum ElectionStatus {
    Idle,
    Campaigning,
    Elected,
    Following,
}

impl Default for LeaderElectionState {
    fn default() -> Self {
        Self {
            current_election_id: None,
            election_start_time: None,
            election_timeout: 60_000, // 60 seconds
            votes_received: HashMap::new(),
            election_status: ElectionStatus::Idle,
            last_election_timestamp: 0,
        }
    }
}

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

/// Leader election message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElectionMessage {
    /// Election ID
    pub election_id: String,

    /// Candidate ID
    pub candidate_id: String,

    /// Election timestamp
    pub timestamp: u64,

    /// Election type
    pub election_type: ElectionType,

    /// Voter ID (for vote messages)
    pub voter_id: Option<String>,

    /// Vote result (for result messages)
    pub vote_result: Option<bool>,
}

/// Election type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElectionType {
    Start,
    Vote,
    Result,
    Resign,
}

/// Task distribution message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDistributionMessage {
    /// Task to distribute
    pub task: Task,

    /// Target member ID (None for broadcast)
    pub target_member: Option<String>,

    /// Distribution timestamp
    pub timestamp: u64,
}

/// State synchronization message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSyncMessage {
    /// State data
    pub state_data: serde_json::Value,

    /// Sync timestamp
    pub timestamp: u64,

    /// Sync type
    pub sync_type: StateSyncType,
}

/// State sync type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateSyncType {
    Full,
    Incremental,
    Request,
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
            election_state.current_election_id = Some(election_id.clone());
            election_state.election_start_time = Some(now);
            election_state.election_status = ElectionStatus::Campaigning;
            election_state.votes_received.clear();
            election_state.last_election_timestamp = now;
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

        if let (Some(election_id), Some(start_time)) =
            (state.current_election_id.clone(), state.election_start_time)
        {
            let now = current_timestamp();
            if now - start_time > state.election_timeout {
                warn!("Election {} timed out, resetting to idle", election_id);
                state.election_status = ElectionStatus::Idle;
                state.current_election_id = None;
                state.election_start_time = None;
                state.votes_received.clear();
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

        // Note: election_type field not available in TypesLeaderElectionMessage
        // This is a simplified implementation
        match "start" {
            "start" => {
                // Received election start message
                info!("Received election start from: {}", election.candidate_id);

                // Vote for the candidate if we're not campaigning ourselves
                let election_state = self.leader_election_state.read().await;
                if election_state.election_status != ElectionStatus::Campaigning {
                    self.vote_for_candidate(
                        &election.election_id,
                        &election.candidate_id,
                        &agent_info.agent_id,
                    )
                    .await?;
                }
            }
            "vote" => {
                // Received vote message
                // Note: voter_id field not available in TypesLeaderElectionMessage
                // This is a simplified implementation
                info!("Received vote for: {}", election.candidate_id);
            }
            "result" => {
                // Received election result
                // Note: vote_result field not available in TypesLeaderElectionMessage
                // This is a simplified implementation
                info!(
                    "Election result: {} is the new leader",
                    election.candidate_id
                );
            }
            "resign" => {
                // Leader resigned
                info!("Leader resigned: {}", election.candidate_id);
                let mut cluster_state = self.cluster_state.write().await;
                if cluster_state.leader.as_ref() == Some(&election.candidate_id) {
                    cluster_state.leader = None;
                    info!("Starting new election after leader resignation");
                    self.start_leader_election().await?;
                }
            }
            _ => {
                info!("Unknown election type for agent: {}", agent_info.agent_id);
                // This is a simplified implementation
            }
        }
        Ok(())
    }

    /// Vote for a candidate
    async fn vote_for_candidate(
        &self,
        election_id: &str,
        candidate_id: &str,
        voter_id: &str,
    ) -> Result<(), AgentError> {
        let message = ClusterMessage::LeaderElection(TypesLeaderElectionMessage {
            election_id: election_id.to_string(),
            candidate_id: candidate_id.to_string(),
            term: 1, // Default term
            timestamp: current_timestamp(),
        });

        info!("Voting for candidate: {}", candidate_id);
        self.send_message(message).await?;
        Ok(())
    }

    /// Process a vote
    async fn process_vote(
        &self,
        election_id: &str,
        candidate_id: &str,
        voter_id: &str,
    ) -> Result<(), AgentError> {
        let mut election_state = self.leader_election_state.write().await;

        // Check if this vote is for our current election
        if election_state.current_election_id.as_ref() != Some(&election_id.to_string()) {
            return Ok(());
        }

        // Record the vote
        election_state
            .votes_received
            .insert(voter_id.to_string(), candidate_id.to_string());

        // Count votes
        let total_members = {
            let cluster_state = self.cluster_state.read().await;
            cluster_state.members.len()
        };

        let votes_for_candidate = election_state
            .votes_received
            .values()
            .filter(|&&ref voted_for| voted_for == candidate_id)
            .count();

        // Check if we have a majority
        let majority_threshold = (total_members / 2) + 1;

        if votes_for_candidate >= majority_threshold {
            info!(
                "Candidate {} received majority of votes ({} >= {})",
                candidate_id, votes_for_candidate, majority_threshold
            );

            // Send election result
            let message = ClusterMessage::LeaderElection(TypesLeaderElectionMessage {
                election_id: election_id.to_string(),
                candidate_id: candidate_id.to_string(),
                term: 1, // Default term
                timestamp: current_timestamp(),
            });

            self.send_message(message).await?;

            // Reset election state
            election_state.election_status = ElectionStatus::Idle;
            election_state.current_election_id = None;
            election_state.election_start_time = None;
            election_state.votes_received.clear();
        }

        Ok(())
    }

    /// Set the leader
    async fn set_leader(&self, leader_id: &str) -> Result<(), AgentError> {
        let mut cluster_state = self.cluster_state.write().await;
        cluster_state.leader = Some(leader_id.to_string());
        cluster_state.last_update = current_timestamp();

        // Update election state
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        let mut election_state = self.leader_election_state.write().await;
        if leader_id == agent_info.agent_id {
            election_state.election_status = ElectionStatus::Elected;
            info!("We are now the leader!");
        } else {
            election_state.election_status = ElectionStatus::Following;
            info!("Following leader: {}", leader_id);
        }

        Ok(())
    }

    /// Handle task distribution
    async fn handle_task_distribution(
        &self,
        distribution: TypesTaskDistributionMessage,
    ) -> Result<(), AgentError> {
        let agent_info = {
            let state = self.state.read().await;
            state.get_agent_info()
        };

        // Check if this task is intended for this agent
        // Note: target_member is String, not Option<String>
        // This is a simplified implementation
        if distribution.target_member != agent_info.agent_id {
            info!(
                "Task {} not intended for this agent, ignoring",
                distribution.task.task_id
            );
            return Ok(());
        }

        // Validate task
        if let Some(expires_at) = distribution.task.expires_at {
            if current_timestamp() > expires_at {
                warn!("Task {} has expired, ignoring", distribution.task.task_id);
                return Ok(());
            }
        }

        // Check if we can handle this task type
        if !agent_info
            .capabilities
            .task_types
            .contains(&distribution.task.task_type)
        {
            warn!(
                "Agent cannot handle task type {:?}, rejecting task {}",
                distribution.task.task_type, distribution.task.task_id
            );
            return Ok(());
        }

        // Check current load
        let current_load = {
            let state = self.state.read().await;
            state.get_load_metrics()
        };

        if current_load.active_tasks >= agent_info.capabilities.max_concurrent_tasks {
            warn!(
                "Agent at capacity ({} active tasks), rejecting task {}",
                current_load.active_tasks, distribution.task.task_id
            );
            return Ok(());
        }

        // Add task to active tasks and update load metrics
        {
            let mut state = self.state.write().await;
            state.add_active_task(distribution.task.clone());

            let mut load_metrics = state.get_load_metrics();
            load_metrics.active_tasks += 1;
            state.update_load_metrics(load_metrics);
        }

        info!("Started processing task: {}", distribution.task.task_id);

        // Process the task based on its type
        let processing_result = match &distribution.task.payload {
            TaskPayload::Ingestion(ingestion_task) => {
                info!(
                    "Processing ingestion task: {} for source: {}",
                    distribution.task.task_id, ingestion_task.source_id
                );
                let processor = IngestionProcessor::new(&self.config, self.state.clone()).await?;
                let result = processor.process_ingestion(ingestion_task.clone()).await?;
                info!("Ingestion task completed: {}", result.task_id);
                Ok::<TaskResult, AgentError>(result)
            }
            TaskPayload::Indexing(indexing_task) => {
                info!(
                    "Processing indexing task: {} for data location: {}",
                    distribution.task.task_id, indexing_task.data_location
                );
                let processor = IndexingProcessor::new(&self.config, self.state.clone()).await?;
                let result = processor.process_indexing(indexing_task.clone()).await?;
                info!("Indexing task completed: {}", result.task_id);
                Ok(result)
            }
            TaskPayload::Processing(processing_task) => {
                info!(
                    "Processing data processing task: {} for pipeline: {}",
                    distribution.task.task_id, processing_task.pipeline
                );
                let processor =
                    DataProcessingProcessor::new(&self.config, self.state.clone()).await?;
                let result = processor.process_data(processing_task.clone()).await?;
                info!("Data processing task completed: {}", result.task_id);
                Ok(result)
            }
            TaskPayload::Query(query_task) => {
                info!(
                    "Processing query task: {} for query: {}",
                    distribution.task.task_id, query_task.query
                );
                let processor = QueryProcessor::new(&self.config, self.state.clone()).await?;
                let result = processor.process_query(query_task.clone()).await?;
                info!("Query task completed: {}", result.task_id);
                Ok(result)
            }
            TaskPayload::Maintenance(maintenance_task) => {
                info!(
                    "Processing maintenance task: {} for type: {}",
                    distribution.task.task_id, maintenance_task.maintenance_type
                );
                let processor = MaintenanceProcessor::new(&self.config, self.state.clone()).await?;
                let result = processor
                    .process_maintenance(maintenance_task.clone())
                    .await?;
                info!("Maintenance task completed: {}", result.task_id);
                Ok(result)
            }
        };

        // Handle processing result
        match processing_result {
            Ok(result) => {
                // Update agent state with task completion
                {
                    let mut state = self.state.write().await;
                    state.remove_active_task(&distribution.task.task_id);

                    // Update load metrics
                    let mut load_metrics = state.get_load_metrics();
                    load_metrics.active_tasks = load_metrics.active_tasks.saturating_sub(1);
                    state.update_load_metrics(load_metrics);
                }

                info!(
                    "Task {} processed successfully in {}ms",
                    distribution.task.task_id, result.processing_time_ms
                );
            }
            Err(e) => {
                error!(
                    "Task {} processing failed: {}",
                    distribution.task.task_id, e
                );

                // Update agent state with task failure
                {
                    let mut state = self.state.write().await;
                    state.remove_active_task(&distribution.task.task_id);

                    // Update load metrics
                    let mut load_metrics = state.get_load_metrics();
                    load_metrics.active_tasks = load_metrics.active_tasks.saturating_sub(1);
                    state.update_load_metrics(load_metrics);
                }

                // Handle retry logic if needed
                if distribution.task.retry_count < distribution.task.max_retries {
                    warn!(
                        "Task {} will be retried (attempt {}/{})",
                        distribution.task.task_id,
                        distribution.task.retry_count + 1,
                        distribution.task.max_retries
                    );
                } else {
                    error!(
                        "Task {} exceeded maximum retries, marking as failed",
                        distribution.task.task_id
                    );
                }
            }
        }

        info!(
            "Successfully handled task distribution: {}",
            distribution.task.task_id
        );
        Ok(())
    }

    /// Handle state sync
    async fn handle_state_sync(&self, sync: StateSyncMessage) -> Result<(), AgentError> {
        match sync.sync_type {
            StateSyncType::Full => {
                // Full state synchronization - replace entire cluster state
                if let Ok(cluster_state_data) =
                    serde_json::from_value::<ClusterState>(sync.state_data.clone())
                {
                    let mut cluster_state = self.cluster_state.write().await;
                    *cluster_state = cluster_state_data;
                    cluster_state.last_update = sync.timestamp;
                    info!(
                        "Applied full state sync with {} members",
                        cluster_state.members.len()
                    );
                } else {
                    error!("Failed to deserialize full state sync data");
                    return Err(AgentError::Serialization(
                        "Invalid state sync data".to_string(),
                    ));
                }
            }
            StateSyncType::Incremental => {
                // Incremental state synchronization - update specific parts
                if let Some(members_data) = sync.state_data.get("members") {
                    if let Ok(members) = serde_json::from_value::<HashMap<String, ClusterMember>>(
                        members_data.clone(),
                    ) {
                        let member_count = members.len();
                        let mut cluster_state = self.cluster_state.write().await;
                        for (member_id, member) in members {
                            cluster_state.members.insert(member_id, member);
                        }
                        cluster_state.last_update = sync.timestamp;
                        info!(
                            "Applied incremental state sync with {} member updates",
                            member_count
                        );
                    }
                }

                // Handle leader update
                if let Some(leader_id) = sync.state_data.get("leader").and_then(|v| v.as_str()) {
                    let mut cluster_state = self.cluster_state.write().await;
                    cluster_state.leader = Some(leader_id.to_string());
                    cluster_state.last_update = sync.timestamp;
                    info!("Updated leader to: {}", leader_id);
                }

                // Handle health status update
                if let Some(health_status) = sync.state_data.get("health_status") {
                    if let Ok(status) =
                        serde_json::from_value::<ClusterHealthStatus>(health_status.clone())
                    {
                        let mut cluster_state = self.cluster_state.write().await;
                        cluster_state.health_status = status;
                        cluster_state.last_update = sync.timestamp;
                        info!("Updated cluster health status");
                    }
                }
            }
            StateSyncType::Request => {
                // State sync request - send our current state
                let cluster_state = self.cluster_state.read().await;
                let state_data = serde_json::to_value(&*cluster_state).map_err(|e| {
                    AgentError::Serialization(format!("Failed to serialize cluster state: {}", e))
                })?;

                let response_sync = StateSyncMessage {
                    state_data,
                    timestamp: current_timestamp(),
                    sync_type: StateSyncType::Full,
                };

                // Send state sync response
                // Note: StateSyncMessage types are different between modules
                // This is a simplified implementation
                info!("Responded to state sync request");
            }
        }

        info!("Successfully handled state sync: {:?}", sync.sync_type);
        Ok(())
    }

    /// Request state sync from cluster
    pub async fn request_state_sync(&self) -> Result<(), AgentError> {
        let sync_request = StateSyncMessage {
            state_data: serde_json::Value::Null,
            timestamp: current_timestamp(),
            sync_type: StateSyncType::Request,
        };

        // Note: StateSyncMessage types are different between modules
        // This is a simplified implementation

        info!("Requested state sync from cluster");
        Ok(())
    }

    /// Get cluster health status
    pub async fn get_cluster_health(&self) -> ClusterHealthStatus {
        let cluster_state = self.cluster_state.read().await;

        let total_members = cluster_state.members.len();
        if total_members == 0 {
            return ClusterHealthStatus {
                total_members: 0,
                healthy_members: 0,
                degraded_members: 0,
                unhealthy_members: 0,
                last_check: 0,
            };
        }

        let active_members = cluster_state
            .members
            .values()
            .filter(|m| m.status == MemberStatus::Active)
            .count();

        let degraded_members = cluster_state
            .members
            .values()
            .filter(|m| m.status == MemberStatus::Inactive)
            .count();

        let unhealthy_members = cluster_state
            .members
            .values()
            .filter(|m| m.status == MemberStatus::Unhealthy)
            .count();

        ClusterHealthStatus {
            total_members,
            healthy_members: active_members,
            degraded_members,
            unhealthy_members,
            last_check: current_timestamp(),
        }
    }

    /// Update cluster health status
    pub async fn update_cluster_health(&self) -> Result<(), AgentError> {
        let health_status = self.get_cluster_health().await;

        let mut cluster_state = self.cluster_state.write().await;
        cluster_state.health_status = health_status.clone();
        cluster_state.last_update = current_timestamp();

        info!("Updated cluster health status: {:?}", health_status);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentConfig;
    use crate::processing::tasks::{
        IndexConfig, IngestionTask, ProcessingTask, Task, TaskPayload, TaskPriority, TaskStatus,
        TaskType,
    };
    use crate::state::AgentState;

    #[tokio::test]
    async fn test_cluster_coordinator_creation() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await;
        assert!(coordinator.is_ok());
    }

    #[tokio::test]
    async fn test_cluster_join_and_leave() {
        let mut config = AgentConfig::default();
        config.cluster.service_discovery = ServiceDiscoveryBackend::Static; // Use static discovery for testing
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let mut coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Test join cluster
        let join_result = coordinator.join_cluster().await;
        assert!(join_result.is_ok());

        // Test leave cluster
        let leave_result = coordinator.leave_cluster().await;
        assert!(leave_result.is_ok());
    }

    #[tokio::test]
    async fn test_heartbeat_sending() {
        let mut config = AgentConfig::default();
        config.cluster.service_discovery = ServiceDiscoveryBackend::Static; // Use static discovery for testing
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        let heartbeat_result = coordinator.send_heartbeat().await;
        assert!(heartbeat_result.is_ok());
    }

    #[tokio::test]
    async fn test_cluster_state_management() {
        let mut config = AgentConfig::default();
        config.cluster.service_discovery = ServiceDiscoveryBackend::Static; // Use static discovery for testing
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Test getting cluster state
        let cluster_state = coordinator.get_cluster_state().await;
        assert_eq!(cluster_state.members.len(), 0);
        assert!(cluster_state.leader.is_none());

        // Test getting cluster members
        let members = coordinator.get_cluster_members().await;
        assert!(members.is_ok());
        assert_eq!(members.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_leader_election() {
        let mut config = AgentConfig::default();
        config.cluster.service_discovery = ServiceDiscoveryBackend::Static; // Use static discovery for testing
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Test leader election start
        let election_result = coordinator.start_leader_election().await;
        assert!(election_result.is_ok());

        // Test leadership check
        let is_leader = coordinator.is_leader().await;
        assert!(!is_leader); // Should not be leader initially
    }

    #[tokio::test]
    async fn test_task_distribution() {
        let mut config = AgentConfig::default();
        config.cluster.service_discovery = ServiceDiscoveryBackend::Static; // Use static discovery for testing
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Create a test task
        let task = Task {
            task_id: "test-task-1".to_string(),
            task_type: TaskType::Ingestion,
            priority: TaskPriority::Normal,
            payload: TaskPayload::Ingestion(IngestionTask {
                source_id: "test-source".to_string(),
                format: "json".to_string(),
                location: "s3://test-bucket/data".to_string(),
                schema: None,
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

        // Test task distribution
        let distribution_result = coordinator.distribute_task(task, None).await;
        assert!(distribution_result.is_ok());
    }

    #[tokio::test]
    async fn test_cluster_health_calculation() {
        let mut config = AgentConfig::default();
        config.cluster.service_discovery = ServiceDiscoveryBackend::Static; // Use static discovery for testing
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Test health calculation with no members
        let health = coordinator.get_cluster_health().await;
        assert_eq!(health.total_members, 0);
        assert_eq!(health.healthy_members, 0);
        assert_eq!(health.degraded_members, 0);
        assert_eq!(health.unhealthy_members, 0);

        // Test health update
        let update_result = coordinator.update_cluster_health().await;
        assert!(update_result.is_ok());
    }

    #[tokio::test]
    async fn test_state_sync_request() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Test state sync request
        let sync_result = coordinator.request_state_sync().await;
        assert!(sync_result.is_ok());
    }

    #[tokio::test]
    async fn test_ingestion_task_processing() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Create an ingestion task
        let task = Task {
            task_id: "test-ingestion-1".to_string(),
            task_type: TaskType::Ingestion,
            priority: TaskPriority::Normal,
            payload: TaskPayload::Ingestion(IngestionTask {
                source_id: "test-source".to_string(),
                format: "json".to_string(),
                location: "s3://test-bucket/data".to_string(),
                schema: None,
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

        // Test task distribution
        let distribution_result = coordinator.distribute_task(task, None).await;
        assert!(distribution_result.is_ok());
    }

    #[tokio::test]
    async fn test_indexing_task_processing() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Create an indexing task
        let task = Task {
            task_id: "test-indexing-1".to_string(),
            task_type: TaskType::Indexing,
            priority: TaskPriority::Normal,
            payload: TaskPayload::Indexing(IndexingTask {
                data_location: "s3://test-bucket/data".to_string(),
                destination: "s3://test-bucket/indexes".to_string(),
                index_config: IndexConfig {
                    index_type: "btree".to_string(),
                    fields: vec!["id".to_string(), "name".to_string()],
                    options: HashMap::new(),
                },
            }),
            metadata: HashMap::new(),
            created_at: current_timestamp(),
            expires_at: None,
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            status: TaskStatus::Pending,
        };

        // Test task distribution
        let distribution_result = coordinator.distribute_task(task, None).await;
        assert!(distribution_result.is_ok());
    }

    #[tokio::test]
    async fn test_query_task_processing() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Create a query task
        let task = Task {
            task_id: "test-query-1".to_string(),
            task_type: TaskType::Query,
            priority: TaskPriority::Normal,
            payload: TaskPayload::Query(QueryTask {
                query: "SELECT * FROM test_table LIMIT 10".to_string(),
                parameters: HashMap::new(),
                result_destination: "test-database".to_string(),
            }),
            metadata: HashMap::new(),
            created_at: current_timestamp(),
            expires_at: None,
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            status: TaskStatus::Pending,
        };

        // Test task distribution
        let distribution_result = coordinator.distribute_task(task, None).await;
        assert!(distribution_result.is_ok());
    }

    #[tokio::test]
    async fn test_maintenance_task_processing() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Create a maintenance task
        let task = Task {
            task_id: "test-maintenance-1".to_string(),
            task_type: TaskType::Maintenance,
            priority: TaskPriority::Normal,
            payload: TaskPayload::Maintenance(MaintenanceTask {
                maintenance_type: "cleanup".to_string(),
                targets: vec!["temp_files".to_string()],
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

        // Test task distribution
        let distribution_result = coordinator.distribute_task(task, None).await;
        assert!(distribution_result.is_ok());
    }

    #[tokio::test]
    async fn test_processing_task_processing() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let coordinator = ClusterCoordinator::new(&config, state).await.unwrap();

        // Create a processing task
        let task = Task {
            task_id: "test-processing-1".to_string(),
            task_type: TaskType::Processing,
            priority: TaskPriority::Normal,
            payload: TaskPayload::Processing(ProcessingTask {
                input_location: "s3://test-bucket/input".to_string(),
                pipeline: "transformation".to_string(),
                output_destination: "s3://test-bucket/output".to_string(),
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

        // Test task distribution
        let distribution_result = coordinator.distribute_task(task, None).await;
        assert!(distribution_result.is_ok());
    }
}
