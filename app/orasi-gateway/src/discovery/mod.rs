//! Service discovery submodule containing the gateway service discovery implementation

pub mod core;
pub mod registry;

// Re-export main types
pub use core::ServiceDiscovery;
pub use registry::ServiceRegistry;
