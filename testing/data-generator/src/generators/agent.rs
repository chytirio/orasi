//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Agent generators for creating realistic agent behavior patterns
//!
//! This module provides generators for creating agents and modeling their
//! interactions, capabilities, and behavior patterns.

use super::utils::{DistributionGenerator, IdGenerator, TimeGenerator};
use crate::config::GeneratorConfig;
use crate::models::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Main agent generator that creates agents and manages their interactions
pub struct AgentGenerator {
    config: Arc<GeneratorConfig>,
    id_generator: IdGenerator,
    time_generator: TimeGenerator,
    distribution_generator: DistributionGenerator,
}

impl AgentGenerator {
    /// Create a new agent generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let id_generator = IdGenerator::new(Arc::clone(&config))?;
        let time_generator = TimeGenerator::new(Arc::clone(&config))?;
        let distribution_generator = DistributionGenerator::new(Arc::clone(&config))?;

        Ok(Self {
            config,
            id_generator,
            time_generator,
            distribution_generator,
        })
    }

    /// Generate agents based on configuration
    pub async fn generate_agents(&self) -> Result<Vec<Agent>> {
        let mut agents = Vec::new();

        // Generate research agents
        for i in 0..2 {
            agents.push(self.generate_research_agent(i).await?);
        }

        // Generate code agents
        for i in 0..3 {
            agents.push(self.generate_code_agent(i).await?);
        }

        // Generate test agents
        for i in 0..2 {
            agents.push(self.generate_test_agent(i).await?);
        }

        // Generate review agents
        for i in 0..2 {
            agents.push(self.generate_review_agent(i).await?);
        }

        // Generate coordination agents
        for i in 0..1 {
            agents.push(self.generate_coordination_agent(i).await?);
        }

        Ok(agents)
    }

    /// Generate development scenario agents
    pub async fn generate_development_agents(&self) -> Result<Vec<Agent>> {
        let mut agents = Vec::new();

        // Focus on code and test agents for development
        for i in 0..3 {
            agents.push(self.generate_code_agent(i).await?);
        }

        for i in 0..2 {
            agents.push(self.generate_test_agent(i).await?);
        }

        // Add one review agent
        agents.push(self.generate_review_agent(0).await?);

        Ok(agents)
    }

    /// Generate scale scenario agents
    pub async fn generate_scale_agents(&self) -> Result<Vec<Agent>> {
        let mut agents = Vec::new();

        // Generate many agents for scale testing
        for i in 0..10 {
            agents.push(self.generate_code_agent(i).await?);
        }

        for i in 0..5 {
            agents.push(self.generate_test_agent(i).await?);
        }

        for i in 0..3 {
            agents.push(self.generate_coordination_agent(i).await?);
        }

        Ok(agents)
    }

    /// Generate a research agent
    async fn generate_research_agent(&self, index: usize) -> Result<Agent> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "agent_specialization".to_string(),
            "API research".to_string(),
        );
        metadata.insert(
            "research_focus".to_string(),
            "integration patterns".to_string(),
        );

        Ok(Agent {
            id: self.id_generator.generate_uuid().await,
            name: format!("Research Agent {}", index + 1),
            agent_type: AgentType::ResearchAgent,
            capabilities: vec![AgentCapability::Research, AgentCapability::Analysis],
            status: AgentStatus::Available,
            metadata,
            created_at: self.time_generator.generate_timestamp().await,
        })
    }

    /// Generate a code agent
    async fn generate_code_agent(&self, index: usize) -> Result<Agent> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "agent_specialization".to_string(),
            "code generation".to_string(),
        );
        metadata.insert("language_focus".to_string(), "rust".to_string());

        Ok(Agent {
            id: self.id_generator.generate_uuid().await,
            name: format!("Code Agent {}", index + 1),
            agent_type: AgentType::CodeAgent,
            capabilities: vec![AgentCapability::CodeGeneration, AgentCapability::CodeReview],
            status: AgentStatus::Available,
            metadata,
            created_at: self.time_generator.generate_timestamp().await,
        })
    }

    /// Generate a test agent
    async fn generate_test_agent(&self, index: usize) -> Result<Agent> {
        let mut metadata = HashMap::new();
        metadata.insert("agent_specialization".to_string(), "testing".to_string());
        metadata.insert("test_focus".to_string(), "integration tests".to_string());

        Ok(Agent {
            id: self.id_generator.generate_uuid().await,
            name: format!("Test Agent {}", index + 1),
            agent_type: AgentType::TestAgent,
            capabilities: vec![AgentCapability::Testing, AgentCapability::CodeReview],
            status: AgentStatus::Available,
            metadata,
            created_at: self.time_generator.generate_timestamp().await,
        })
    }

    /// Generate a review agent
    async fn generate_review_agent(&self, index: usize) -> Result<Agent> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "agent_specialization".to_string(),
            "code review".to_string(),
        );
        metadata.insert("review_focus".to_string(), "quality assurance".to_string());

        Ok(Agent {
            id: self.id_generator.generate_uuid().await,
            name: format!("Review Agent {}", index + 1),
            agent_type: AgentType::ReviewAgent,
            capabilities: vec![AgentCapability::CodeReview, AgentCapability::Testing],
            status: AgentStatus::Available,
            metadata,
            created_at: self.time_generator.generate_timestamp().await,
        })
    }

    /// Generate a coordination agent
    async fn generate_coordination_agent(&self, index: usize) -> Result<Agent> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "agent_specialization".to_string(),
            "coordination".to_string(),
        );
        metadata.insert(
            "coordination_focus".to_string(),
            "workflow management".to_string(),
        );

        Ok(Agent {
            id: self.id_generator.generate_uuid().await,
            name: format!("Coordination Agent {}", index + 1),
            agent_type: AgentType::CoordinationAgent,
            capabilities: vec![AgentCapability::Coordination, AgentCapability::Monitoring],
            status: AgentStatus::Available,
            metadata,
            created_at: self.time_generator.generate_timestamp().await,
        })
    }
}

/// Generator for agent interactions
pub struct AgentInteractionGenerator {
    config: Arc<GeneratorConfig>,
    id_generator: IdGenerator,
    time_generator: TimeGenerator,
    distribution_generator: DistributionGenerator,
}

impl AgentInteractionGenerator {
    /// Create a new agent interaction generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let id_generator = IdGenerator::new(Arc::clone(&config))?;
        let time_generator = TimeGenerator::new(Arc::clone(&config))?;
        let distribution_generator = DistributionGenerator::new(Arc::clone(&config))?;

        Ok(Self {
            config,
            id_generator,
            time_generator,
            distribution_generator,
        })
    }

    /// Generate interactions between agents
    pub async fn generate_interactions(&self, agents: &[Agent]) -> Result<Vec<AgentInteraction>> {
        let mut interactions = Vec::new();

        // Generate handoff interactions
        let handoffs = self.generate_handoff_interactions(agents).await?;
        interactions.extend(handoffs);

        // Generate collaboration interactions
        let collaborations = self.generate_collaboration_interactions(agents).await?;
        interactions.extend(collaborations);

        // Generate escalation interactions
        let escalations = self.generate_escalation_interactions(agents).await?;
        interactions.extend(escalations);

        Ok(interactions)
    }

    /// Generate handoff interactions
    async fn generate_handoff_interactions(
        &self,
        agents: &[Agent],
    ) -> Result<Vec<AgentInteraction>> {
        let mut interactions = Vec::new();

        // Generate handoffs between different agent types
        let research_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| matches!(a.agent_type, AgentType::ResearchAgent))
            .collect();

        let code_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| matches!(a.agent_type, AgentType::CodeAgent))
            .collect();

        let test_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| matches!(a.agent_type, AgentType::TestAgent))
            .collect();

        // Research to Code handoffs
        for research_agent in &research_agents {
            for code_agent in &code_agents {
                interactions.push(
                    self.generate_handoff_interaction(
                        research_agent,
                        code_agent,
                        "Research findings to implementation",
                    )
                    .await?,
                );
            }
        }

        // Code to Test handoffs
        for code_agent in &code_agents {
            for test_agent in &test_agents {
                interactions.push(
                    self.generate_handoff_interaction(
                        code_agent,
                        test_agent,
                        "Code implementation to testing",
                    )
                    .await?,
                );
            }
        }

        Ok(interactions)
    }

    /// Generate collaboration interactions
    async fn generate_collaboration_interactions(
        &self,
        agents: &[Agent],
    ) -> Result<Vec<AgentInteraction>> {
        let mut interactions = Vec::new();

        // Generate collaborations between similar agent types
        let code_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| matches!(a.agent_type, AgentType::CodeAgent))
            .collect();

        for i in 0..code_agents.len() {
            for j in (i + 1)..code_agents.len() {
                interactions.push(
                    self.generate_collaboration_interaction(
                        code_agents[i],
                        code_agents[j],
                        "Code review collaboration",
                    )
                    .await?,
                );
            }
        }

        Ok(interactions)
    }

    /// Generate escalation interactions
    async fn generate_escalation_interactions(
        &self,
        agents: &[Agent],
    ) -> Result<Vec<AgentInteraction>> {
        let mut interactions = Vec::new();

        let coordination_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| matches!(a.agent_type, AgentType::CoordinationAgent))
            .collect();

        let code_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| matches!(a.agent_type, AgentType::CodeAgent))
            .collect();

        // Escalate from code agents to coordination agents
        for code_agent in &code_agents {
            for coordination_agent in &coordination_agents {
                interactions.push(
                    self.generate_escalation_interaction(
                        code_agent,
                        coordination_agent,
                        "Complex implementation issue",
                    )
                    .await?,
                );
            }
        }

        Ok(interactions)
    }

    /// Generate a handoff interaction
    async fn generate_handoff_interaction(
        &self,
        source: &Agent,
        target: &Agent,
        reason: &str,
    ) -> Result<AgentInteraction> {
        let mut data = HashMap::new();
        data.insert(
            "handoff_reason".to_string(),
            serde_json::Value::String(reason.to_string()),
        );
        data.insert(
            "workflow_stage".to_string(),
            serde_json::Value::String("transition".to_string()),
        );

        let duration = chrono::Duration::minutes(2);
        let timestamp = self.time_generator.generate_timestamp().await;

        Ok(AgentInteraction {
            id: self.id_generator.generate_uuid().await,
            interaction_type: InteractionType::Handoff,
            initiator: source.id,
            target: target.id,
            data,
            timestamp,
            duration,
            status: InteractionStatus::Completed,
        })
    }

    /// Generate a collaboration interaction
    async fn generate_collaboration_interaction(
        &self,
        agent1: &Agent,
        agent2: &Agent,
        purpose: &str,
    ) -> Result<AgentInteraction> {
        let mut data = HashMap::new();
        data.insert(
            "collaboration_purpose".to_string(),
            serde_json::Value::String(purpose.to_string()),
        );
        data.insert(
            "collaboration_type".to_string(),
            serde_json::Value::String("peer_review".to_string()),
        );

        let duration = chrono::Duration::minutes(15);
        let timestamp = self.time_generator.generate_timestamp().await;

        Ok(AgentInteraction {
            id: self.id_generator.generate_uuid().await,
            interaction_type: InteractionType::Collaboration,
            initiator: agent1.id,
            target: agent2.id,
            data,
            timestamp,
            duration,
            status: InteractionStatus::Completed,
        })
    }

    /// Generate an escalation interaction
    async fn generate_escalation_interaction(
        &self,
        source: &Agent,
        target: &Agent,
        reason: &str,
    ) -> Result<AgentInteraction> {
        let mut data = HashMap::new();
        data.insert(
            "escalation_reason".to_string(),
            serde_json::Value::String(reason.to_string()),
        );
        data.insert(
            "escalation_level".to_string(),
            serde_json::Value::String("medium".to_string()),
        );

        let duration = chrono::Duration::minutes(5);
        let timestamp = self.time_generator.generate_timestamp().await;

        Ok(AgentInteraction {
            id: self.id_generator.generate_uuid().await,
            interaction_type: InteractionType::Escalation,
            initiator: source.id,
            target: target.id,
            data,
            timestamp,
            duration,
            status: InteractionStatus::Completed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GeneratorConfig;

    #[tokio::test]
    async fn test_agent_generator_creation() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = AgentGenerator::new(config);
        assert!(generator.is_ok());
    }

    #[tokio::test]
    async fn test_agent_generation() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = AgentGenerator::new(config).unwrap();

        let agents = generator.generate_agents().await.unwrap();
        assert!(!agents.is_empty());

        // Check that we have different agent types
        let agent_types: std::collections::HashSet<_> =
            agents.iter().map(|a| &a.agent_type).collect();
        assert!(agent_types.len() > 1);
    }

    #[tokio::test]
    async fn test_development_agents() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = AgentGenerator::new(config).unwrap();

        let agents = generator.generate_development_agents().await.unwrap();
        assert!(!agents.is_empty());

        // Should have more code and test agents
        let code_agents = agents
            .iter()
            .filter(|a| matches!(a.agent_type, AgentType::CodeAgent))
            .count();
        let test_agents = agents
            .iter()
            .filter(|a| matches!(a.agent_type, AgentType::TestAgent))
            .count();

        assert!(code_agents > 0);
        assert!(test_agents > 0);
    }

    #[tokio::test]
    async fn test_scale_agents() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = AgentGenerator::new(config).unwrap();

        let agents = generator.generate_scale_agents().await.unwrap();
        assert!(agents.len() > 10); // Should have many agents for scale testing
    }

    #[tokio::test]
    async fn test_agent_interaction_generator() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = AgentInteractionGenerator::new(config).unwrap();

        let agents = vec![
            Agent {
                id: Uuid::new_v4(),
                name: "Test Agent 1".to_string(),
                agent_type: AgentType::CodeAgent,
                capabilities: vec![AgentCapability::CodeGeneration],
                status: AgentStatus::Available,
                metadata: HashMap::new(),
                created_at: chrono::Utc::now(),
            },
            Agent {
                id: Uuid::new_v4(),
                name: "Test Agent 2".to_string(),
                agent_type: AgentType::TestAgent,
                capabilities: vec![AgentCapability::Testing],
                status: AgentStatus::Available,
                metadata: HashMap::new(),
                created_at: chrono::Utc::now(),
            },
        ];

        let interactions = generator.generate_interactions(&agents).await.unwrap();
        assert!(!interactions.is_empty());
    }
}
