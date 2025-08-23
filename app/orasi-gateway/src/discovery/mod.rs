//! Service discovery submodule containing the gateway service discovery implementation

pub mod backends;
pub mod core;
pub mod health;
pub mod registry;

// Re-export main types
pub use backends::{
    BackendFactory, ConsulBackend, EtcdBackend, ServiceDiscoveryBackend, StaticBackend,
};
pub use core::ServiceDiscovery;
pub use health::{AdvancedHealthChecker, HealthCheckConfig, HealthChecker};
pub use registry::ServiceRegistry;
