//! Agent module containing the main Orasi Agent implementation

pub mod load_balancer;
pub mod lifecycle;

// Re-export main types for convenience
pub use load_balancer::{LoadBalancer, LoadBalancingStrategy, LoadBalancerStats};
pub use lifecycle::OrasiAgent;

// Re-export the main agent struct for backward compatibility
pub use lifecycle::OrasiAgent as Agent;
