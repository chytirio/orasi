//! Orasi Agent - Distributed data processing agent

pub mod agent;
pub mod cluster;
pub mod config;
pub mod discovery;
pub mod error;
pub mod health;
pub mod http;
pub mod metrics;
pub mod processing;
pub mod state;
pub mod types;

#[cfg(test)]
mod tests;

/// Agent version
pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Re-export main types for convenience
pub use agent::OrasiAgent;
pub use config::AgentConfig;
pub use error::AgentError;
pub use types::*;

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

/// Agent name
pub const AGENT_NAME: &str = "orasi-agent";

/// Default agent endpoint
pub const DEFAULT_AGENT_ENDPOINT: &str = "0.0.0.0:8082";

/// Default health check endpoint
pub const DEFAULT_HEALTH_ENDPOINT: &str = "0.0.0.0:8083";

/// Default metrics endpoint
pub const DEFAULT_METRICS_ENDPOINT: &str = "0.0.0.0:9092";

/// Default cluster coordination endpoint
pub const DEFAULT_CLUSTER_ENDPOINT: &str = "0.0.0.0:8084";

/// Default heartbeat interval in seconds
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Default task timeout in seconds
pub const DEFAULT_TASK_TIMEOUT_SECS: u64 = 300;

/// Initialize orasi agent
pub async fn init_agent(config: AgentConfig) -> AgentResult<OrasiAgent> {
    tracing::info!("Initializing Orasi agent v{}", AGENT_VERSION);

    let agent = OrasiAgent::new(config).await?;
    tracing::info!("Orasi agent initialization completed");

    Ok(agent)
}

/// Shutdown orasi agent
pub async fn shutdown_agent(agent: OrasiAgent) -> AgentResult<()> {
    tracing::info!("Shutting down Orasi agent");

    agent.shutdown().await?;
    tracing::info!("Orasi agent shutdown completed");

    Ok(())
}
