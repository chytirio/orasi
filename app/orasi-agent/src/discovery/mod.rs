//! Service discovery for Orasi Agent

pub mod types;
pub mod etcd;
pub mod consul;
pub mod service_discovery;

pub use types::ServiceRegistration;
pub use service_discovery::ServiceDiscovery;
pub use etcd::EtcdClient;
pub use consul::ConsulClientWrapper;

/// Get current timestamp in milliseconds
pub(crate) fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
