//! Service registry implementation

use crate::{
    error::GatewayError,
    types::ServiceInfo,
};
use std::collections::HashMap;

/// Service registry for managing discovered services
pub struct ServiceRegistry {
    services: HashMap<String, ServiceInfo>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Refresh services in the registry
    pub async fn refresh_services(&mut self) -> Result<(), GatewayError> {
        // TODO: Implement service refresh logic
        // This would typically involve:
        // 1. Querying service discovery backends (Consul, etcd, etc.)
        // 2. Updating the local service cache
        // 3. Handling service health checks
        Ok(())
    }

    /// Get all services from the registry
    pub fn get_services(&self) -> HashMap<String, ServiceInfo> {
        self.services.clone()
    }

    /// Add a service to the registry
    pub fn add_service(&mut self, service_id: String, service_info: ServiceInfo) {
        self.services.insert(service_id, service_info);
    }

    /// Remove a service from the registry
    pub fn remove_service(&mut self, service_id: &str) {
        self.services.remove(service_id);
    }

    /// Get a specific service by ID
    pub fn get_service(&self, service_id: &str) -> Option<&ServiceInfo> {
        self.services.get(service_id)
    }

    /// Check if a service exists in the registry
    pub fn has_service(&self, service_id: &str) -> bool {
        self.services.contains_key(service_id)
    }
}
