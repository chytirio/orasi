//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Data generators for creating realistic test data
//!
//! This module provides comprehensive generators for creating realistic
//! test data for the OpenTelemetry Data Lake Bridge, including telemetry
//! data, workflow patterns, and agent interactions.

pub mod agent;
pub mod repository;
pub mod telemetry;
pub mod utils;
pub mod workflow;

// Re-export main generator types
pub use agent::{AgentGenerator, AgentInteractionGenerator};
pub use repository::RepositoryGenerator;
pub use telemetry::{
    EventsGenerator, LogsGenerator, MetricsGenerator, TelemetryGenerator, TracesGenerator,
};
pub use utils::{CorrelationGenerator, DistributionGenerator, IdGenerator, TimeGenerator};
pub use workflow::{
    CoordinationGenerator, ImplementationGenerator, ResearchGenerator, WorkflowGenerator,
};

use crate::config::GeneratorConfig;
use crate::models::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Common trait for all data generators
pub trait DataGenerator {
    /// Generate data based on configuration
    async fn generate(&self, config: &GeneratorConfig) -> Result<Vec<serde_json::Value>>;

    /// Generate a single item
    async fn generate_one(&self, config: &GeneratorConfig) -> Result<serde_json::Value>;

    /// Validate generated data
    async fn validate(&self, data: &[serde_json::Value]) -> Result<bool>;
}

/// Main generator orchestrator that coordinates all generators
pub struct GeneratorOrchestrator {
    config: Arc<GeneratorConfig>,
    telemetry_generator: Arc<TelemetryGenerator>,
    workflow_generator: Arc<WorkflowGenerator>,
    agent_generator: Arc<AgentGenerator>,
    repository_generator: Arc<RepositoryGenerator>,
    correlation_generator: Arc<CorrelationGenerator>,
    time_generator: Arc<TimeGenerator>,
    id_generator: Arc<IdGenerator>,
}

impl GeneratorOrchestrator {
    /// Create a new generator orchestrator
    pub fn new(config: GeneratorConfig) -> Result<Self> {
        let config = Arc::new(config);

        let telemetry_generator = Arc::new(TelemetryGenerator::new(Arc::clone(&config))?);
        let workflow_generator = Arc::new(WorkflowGenerator::new(Arc::clone(&config))?);
        let agent_generator = Arc::new(AgentGenerator::new(Arc::clone(&config))?);
        let repository_generator = Arc::new(RepositoryGenerator::new(Arc::clone(&config))?);
        let correlation_generator = Arc::new(CorrelationGenerator::new(Arc::clone(&config))?);
        let time_generator = Arc::new(TimeGenerator::new(Arc::clone(&config))?);
        let id_generator = Arc::new(IdGenerator::new(Arc::clone(&config))?);

        Ok(Self {
            config,
            telemetry_generator,
            workflow_generator,
            agent_generator,
            repository_generator,
            correlation_generator,
            time_generator,
            id_generator,
        })
    }

    /// Generate comprehensive test data
    pub async fn generate_all(&self) -> Result<GeneratedData> {
        tracing::info!("Starting comprehensive test data generation");

        // Generate agents first as they're referenced by other generators
        let agents = self.agent_generator.generate_agents().await?;

        // Generate workflows that reference agents
        let workflows = self.workflow_generator.generate_workflows(&agents).await?;

        // Generate repository operations
        let repository_ops = self.repository_generator.generate_operations().await?;

        // Generate telemetry data that correlates with workflows
        let telemetry_data = self
            .telemetry_generator
            .generate_correlated(&workflows)
            .await?;

        Ok(GeneratedData {
            agents,
            workflows,
            repository_operations: repository_ops,
            telemetry: telemetry_data,
        })
    }

    /// Generate data for a specific scenario
    pub async fn generate_scenario(&self, scenario: &str) -> Result<GeneratedData> {
        tracing::info!("Generating data for scenario: {}", scenario);

        match scenario {
            "development" => self.generate_development_scenario().await,
            "failure" => self.generate_failure_scenario().await,
            "scale" => self.generate_scale_scenario().await,
            "evolution" => self.generate_evolution_scenario().await,
            _ => Err(anyhow::anyhow!("Unknown scenario: {}", scenario)),
        }
    }

    /// Generate development scenario data
    async fn generate_development_scenario(&self) -> Result<GeneratedData> {
        let agents = self.agent_generator.generate_development_agents().await?;
        let workflows = self
            .workflow_generator
            .generate_development_workflows(&agents)
            .await?;
        let repository_ops = self
            .repository_generator
            .generate_development_operations()
            .await?;
        let telemetry = self
            .telemetry_generator
            .generate_development_telemetry(&workflows)
            .await?;

        Ok(GeneratedData {
            agents,
            workflows,
            repository_operations: repository_ops,
            telemetry,
        })
    }

    /// Generate failure scenario data
    async fn generate_failure_scenario(&self) -> Result<GeneratedData> {
        let agents = self.agent_generator.generate_agents().await?;
        let workflows = self
            .workflow_generator
            .generate_failure_workflows(&agents)
            .await?;
        let repository_ops = self
            .repository_generator
            .generate_failure_operations()
            .await?;
        let telemetry = self
            .telemetry_generator
            .generate_failure_telemetry(&workflows)
            .await?;

        Ok(GeneratedData {
            agents,
            workflows,
            repository_operations: repository_ops,
            telemetry,
        })
    }

    /// Generate scale scenario data
    async fn generate_scale_scenario(&self) -> Result<GeneratedData> {
        let agents = self.agent_generator.generate_scale_agents().await?;
        let workflows = self
            .workflow_generator
            .generate_scale_workflows(&agents)
            .await?;
        let repository_ops = self
            .repository_generator
            .generate_scale_operations()
            .await?;
        let telemetry = self
            .telemetry_generator
            .generate_scale_telemetry(&workflows)
            .await?;

        Ok(GeneratedData {
            agents,
            workflows,
            repository_operations: repository_ops,
            telemetry,
        })
    }

    /// Generate evolution scenario data
    async fn generate_evolution_scenario(&self) -> Result<GeneratedData> {
        let agents = self.agent_generator.generate_agents().await?;
        let workflows = self
            .workflow_generator
            .generate_evolution_workflows(&agents)
            .await?;
        let repository_ops = self
            .repository_generator
            .generate_evolution_operations()
            .await?;
        let telemetry = self
            .telemetry_generator
            .generate_evolution_telemetry(&workflows)
            .await?;

        Ok(GeneratedData {
            agents,
            workflows,
            repository_operations: repository_ops,
            telemetry,
        })
    }
}

/// Container for all generated data
#[derive(Debug, Clone)]
pub struct GeneratedData {
    pub agents: Vec<Agent>,
    pub workflows: Vec<AgenticWorkflow>,
    pub repository_operations: Vec<RepositoryOperation>,
    pub telemetry: TelemetryData,
}

/// Container for telemetry data
#[derive(Debug, Clone)]
pub struct TelemetryData {
    pub metrics: Vec<serde_json::Value>,
    pub traces: Vec<serde_json::Value>,
    pub logs: Vec<serde_json::Value>,
    pub events: Vec<serde_json::Value>,
}

impl TelemetryData {
    /// Create empty telemetry data
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
            traces: Vec::new(),
            logs: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Get total count of all telemetry items
    pub fn total_count(&self) -> usize {
        self.metrics.len() + self.traces.len() + self.logs.len() + self.events.len()
    }

    /// Merge with another telemetry data set
    pub fn merge(&mut self, other: TelemetryData) {
        self.metrics.extend(other.metrics);
        self.traces.extend(other.traces);
        self.logs.extend(other.logs);
        self.events.extend(other.events);
    }
}

impl Default for TelemetryData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GeneratorConfig;

    #[tokio::test]
    async fn test_generator_orchestrator_creation() {
        let config = GeneratorConfig::default();
        let orchestrator = GeneratorOrchestrator::new(config);
        assert!(orchestrator.is_ok());
    }

    #[tokio::test]
    async fn test_telemetry_data_operations() {
        let mut telemetry = TelemetryData::new();
        assert_eq!(telemetry.total_count(), 0);

        telemetry
            .metrics
            .push(serde_json::json!({"test": "metric"}));
        assert_eq!(telemetry.total_count(), 1);
    }
}
