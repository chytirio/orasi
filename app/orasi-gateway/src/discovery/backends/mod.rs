//! Service discovery backend implementations

pub mod trait_def;
pub mod etcd;
pub mod consul;
pub mod static_backend;
pub mod factory;

// Re-export main types for convenience
pub use trait_def::ServiceDiscoveryBackend;
pub use etcd::EtcdBackend;
pub use consul::ConsulBackend;
pub use static_backend::StaticBackend;
pub use factory::BackendFactory;
