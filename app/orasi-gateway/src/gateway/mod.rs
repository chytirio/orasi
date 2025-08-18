//! Gateway submodule containing the main gateway implementation

pub mod core;
pub mod health;
pub mod rate_limiter;
pub mod state;

// Re-export main types
pub use core::OrasiGateway;
pub use state::GatewayState;
