//! Static service discovery backend implementation

use crate::{
    error::GatewayError,
    types::{ServiceInfo},
};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info, warn};
use super::trait_def::ServiceDiscoveryBackend;

/// Static service discovery backend
pub struct StaticBackend {
    services: HashMap<String, ServiceInfo>,
}

impl StaticBackend {
    pub fn new(services: HashMap<String, ServiceInfo>) -> Self {
        Self { services }
    }

    pub fn from_config(services_config: Vec<(String, ServiceInfo)>) -> Self {
        let mut services = HashMap::new();
        for (id, info) in services_config {
            services.insert(id, info);
        }
        Self { services }
    }
}

#[async_trait]
impl ServiceDiscoveryBackend for StaticBackend {
    async fn initialize(&mut self) -> Result<(), GatewayError> {
        info!(
            "Initializing static service discovery backend with {} services",
            self.services.len()
        );
        Ok(())
    }

    async fn discover_services(&mut self) -> Result<HashMap<String, ServiceInfo>, GatewayError> {
        debug!("Returning {} static services", self.services.len());
        Ok(self.services.clone())
    }

    async fn register_service(
        &mut self,
        _service_id: &str,
        _service_info: &ServiceInfo,
    ) -> Result<(), GatewayError> {
        warn!("Cannot register services in static backend");
        Ok(())
    }

    async fn deregister_service(&mut self, _service_id: &str) -> Result<(), GatewayError> {
        warn!("Cannot deregister services in static backend");
        Ok(())
    }

    async fn health_check(&mut self) -> Result<bool, GatewayError> {
        Ok(true) // Static backend is always healthy
    }
}
