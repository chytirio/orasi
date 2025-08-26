//! Service discovery backend trait definition

use crate::{
    error::GatewayError,
    types::{ServiceInfo},
};
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for service discovery backends
#[async_trait]
pub trait ServiceDiscoveryBackend: Send + Sync {
    /// Initialize the backend
    async fn initialize(&mut self) -> Result<(), GatewayError>;

    /// Discover services
    async fn discover_services(&mut self) -> Result<HashMap<String, ServiceInfo>, GatewayError>;

    /// Register a service
    async fn register_service(
        &mut self,
        service_id: &str,
        service_info: &ServiceInfo,
    ) -> Result<(), GatewayError>;

    /// Deregister a service
    async fn deregister_service(&mut self, service_id: &str) -> Result<(), GatewayError>;

    /// Health check for the backend
    async fn health_check(&mut self) -> Result<bool, GatewayError>;
}
