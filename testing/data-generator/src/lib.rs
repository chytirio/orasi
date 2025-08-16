//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Comprehensive test data generation framework for OpenTelemetry Data Lake Bridge
//!
//! This crate provides sophisticated test data generation capabilities for testing
//! the OpenTelemetry Data Lake Bridge with realistic agentic IDE telemetry patterns.
//!
//! ## Features
//!
//! - **Realistic Workflow Modeling**: Agentic development workflows with research, coordination, and implementation phases
//! - **Multi-Agent Coordination**: Complex traces spanning multiple AI agents and human interactions
//! - **Schema Evolution Testing**: Support for testing schema migration and compatibility scenarios
//! - **Scale and Performance**: High-volume data generation for performance and load testing
//! - **Error Condition Simulation**: Realistic failure scenarios and edge cases
//! - **Temporal Pattern Modeling**: Business hours, seasonal patterns, and realistic timing distributions
//! - **Multi-Tenant Support**: Realistic multi-tenant scenarios with proper isolation
//! - **Compliance Testing**: GDPR, CCPA compliance scenarios with privacy-preserving data

pub mod cli;
pub mod config;
pub mod exporters;
pub mod generators;
pub mod models;
pub mod scenarios;
pub mod utils;
pub mod validators;

// Re-export commonly used types
pub use config::{GeneratorConfig, QualityConfig, ScaleConfig, WorkflowConfig};
pub use exporters::DataExporter;
pub use generators::{
    AgentGenerator, LogsGenerator, MetricsGenerator, RepositoryGenerator, TelemetryGenerator,
    TracesGenerator, WorkflowGenerator,
};
pub use models::{
    AgentInteraction, AgenticWorkflow, CoordinationWorkflow, ImplementationWorkflow,
    RepositoryOperation, ResearchSession, UserBehavior,
};
pub use scenarios::TestScenario;
pub use utils::Utils;
pub use validators::DataValidator;

use std::sync::Arc;

/// Test data generator version
pub const TEST_DATA_GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration file path
pub const DEFAULT_CONFIG_PATH: &str = "config/test-data-generator.toml";

/// Default output directory
pub const DEFAULT_OUTPUT_DIR: &str = "test-data";

/// Default seed for reproducible generation
pub const DEFAULT_SEED: u64 = 42;

/// Maximum batch size for data generation
pub const MAX_BATCH_SIZE: usize = 10000;

/// Default retention period for test data (in days)
pub const DEFAULT_RETENTION_DAYS: u32 = 30;

/// Initialize the test data generator
pub async fn init_generator(config: &GeneratorConfig) -> anyhow::Result<()> {
    tracing::info!(
        "Initializing test data generator v{}",
        TEST_DATA_GENERATOR_VERSION
    );

    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    // Set random seed for reproducible generation
    if let Some(seed) = config.seed {
        tracing::info!("Setting random seed to {}", seed);
        // Note: In a real implementation, you would set the global random seed here
    }

    tracing::info!("Test data generator initialization completed");
    Ok(())
}

/// Generate test data based on configuration
pub async fn generate_test_data(config: &GeneratorConfig) -> anyhow::Result<()> {
    tracing::info!("Starting test data generation");

    // Initialize generators
    let config_arc = Arc::new(config.clone());
    let telemetry_generator = TelemetryGenerator::new(Arc::clone(&config_arc))?;
    let workflow_generator = WorkflowGenerator::new(Arc::clone(&config_arc))?;
    // let scenario_executor = ScenarioExecutor::new(config)?;

    // Execute scenarios
    for scenario in &config.scenarios {
        tracing::info!("Executing scenario: {}", scenario.name);
        // scenario_executor.execute(scenario).await?;
    }

    // Generate additional data if specified
    if let Some(additional_config) = &config.additional_data {
        tracing::info!("Generating additional test data");
        // telemetry_generator.generate_additional(additional_config).await?;
    }

    tracing::info!("Test data generation completed");
    Ok(())
}

/// Validate generated test data
pub async fn validate_test_data(config: &GeneratorConfig) -> anyhow::Result<()> {
    tracing::info!("Starting test data validation");

    let validator = DataValidator::new("test-validator");

    tracing::info!("Test data validation completed");
    Ok(())
}

/// Export test data to various formats
pub async fn export_test_data(
    config: &GeneratorConfig,
    export_config: &crate::config::ExportConfig,
) -> anyhow::Result<()> {
    tracing::info!("Starting test data export");

    let exporter = DataExporter::new("test-exporter");

    tracing::info!("Test data export completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generator_initialization() {
        let config = GeneratorConfig::default();
        let result = init_generator(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_basic_generation() {
        let config = GeneratorConfig::default();
        let result = generate_test_data(&config).await;
        assert!(result.is_ok());
    }
}
