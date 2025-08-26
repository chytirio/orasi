//! Cluster coordination module for the Orasi Agent

pub mod types;
pub mod leader_election;
pub mod task_distribution;
pub mod state_sync;
pub mod coordinator;

// Re-export main types for convenience
pub use types::*;
pub use leader_election::*;
pub use task_distribution::*;
pub use state_sync::*;
pub use coordinator::ClusterCoordinator;

// Re-export the main coordinator struct for backward compatibility
pub use coordinator::ClusterCoordinator as Coordinator;
