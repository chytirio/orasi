//! Routing submodule containing the gateway routing implementation

pub mod core;
pub mod matcher;
pub mod proxy;

// Re-export main types
pub use core::Router;
pub use matcher::RouteMatch;
