//! Core service discovery implementation

use crate::{
    config::GatewayConfig,
    error::GatewayError,
    discovery::registry::ServiceRegistry,
    types::ServiceInfo,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Service discovery for gateway
pub struct ServiceDiscovery {
    config: GatewayConfig,
    registry: Arc<RwLock<ServiceRegistry>>,
}

impl ServiceDiscovery {
    /// Create new service discovery
    pub async fn new(config: &GatewayConfig) -> Result<Self, GatewayError> {
        let registry = Arc::new(RwLock::new(ServiceRegistry::new()));
        
        Ok(Self {
            config: config.clone(),
            registry,
        })
    }

    /// Start service discovery
    pub async fn start(&self) -> Result<(), GatewayError> {
        // TODO: Implement service discovery startup
        Ok(())
    }

    /// Stop service discovery
    pub async fn stop(&self) -> Result<(), GatewayError> {
        // TODO: Implement service discovery shutdown
        Ok(())
    }

    /// Refresh services
    pub async fn refresh_services(&self) -> Result<HashMap<String, ServiceInfo>, GatewayError> {
        // TODO: Implement service refresh
        let mut registry = self.registry.write().await;
        registry.refresh_services().await?;
        Ok(registry.get_services())
    }
}
