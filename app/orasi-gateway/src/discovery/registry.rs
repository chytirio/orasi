//! Service registry implementation

use crate::{
    discovery::backends::ServiceDiscoveryBackend,
    discovery::health::{AdvancedHealthChecker, HealthCheckConfig, HealthChecker},
    error::GatewayError,
    types::ServiceInfo,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Service registry for managing discovered services
pub struct ServiceRegistry {
    pub services: HashMap<String, ServiceInfo>,
    pub backend: Option<Box<dyn ServiceDiscoveryBackend>>,
    pub health_checker: Option<AdvancedHealthChecker>,
    pub last_refresh: std::time::Instant,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            backend: None,
            health_checker: None,
            last_refresh: std::time::Instant::now(),
        }
    }

    /// Set the service discovery backend
    pub fn set_backend(&mut self, backend: Box<dyn ServiceDiscoveryBackend>) {
        self.backend = Some(backend);
    }

    /// Set the health checker
    pub fn set_health_checker(&mut self, health_checker: AdvancedHealthChecker) {
        self.health_checker = Some(health_checker);
    }

    /// Refresh services in the registry
    pub async fn refresh_services(&mut self) -> Result<(), GatewayError> {
        info!("Refreshing services in registry");

        if let Some(backend) = &mut self.backend {
            match backend.discover_services().await {
                Ok(discovered_services) => {
                    debug!(
                        "Discovered {} services from backend",
                        discovered_services.len()
                    );

                    // Update the registry with discovered services
                    for (service_id, service_info) in discovered_services {
                        self.services.insert(service_id, service_info);
                    }

                    // Perform health checks if health checker is available
                    if let Some(health_checker) = &mut self.health_checker {
                        for (service_id, service_info) in self.services.iter_mut() {
                            health_checker
                                .check_service_health(service_id, service_info)
                                .await;
                        }
                    }

                    self.last_refresh = std::time::Instant::now();
                    info!("Successfully refreshed {} services", self.services.len());
                }
                Err(e) => {
                    warn!("Failed to refresh services from backend: {}", e);
                    return Err(e);
                }
            }
        } else {
            warn!("No backend configured for service discovery");
        }

        Ok(())
    }

    /// Get all services from the registry
    pub fn get_services(&self) -> HashMap<String, ServiceInfo> {
        self.services.clone()
    }

    /// Add a service to the registry
    pub fn add_service(&mut self, service_id: String, service_info: ServiceInfo) {
        info!("Adding service {} to registry", service_id);
        self.services.insert(service_id, service_info);
    }

    /// Remove a service from the registry
    pub fn remove_service(&mut self, service_id: &str) {
        info!("Removing service {} from registry", service_id);
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

    /// Get services by health status
    pub fn get_services_by_health(
        &self,
        health_status: crate::types::ServiceHealthStatus,
    ) -> HashMap<String, ServiceInfo> {
        self.services
            .iter()
            .filter(|(_, service)| service.health_status == health_status)
            .map(|(id, service)| (id.clone(), service.clone()))
            .collect()
    }

    /// Get healthy services only
    pub fn get_healthy_services(&self) -> HashMap<String, ServiceInfo> {
        self.get_services_by_health(crate::types::ServiceHealthStatus::Healthy)
    }

    /// Get the last refresh time
    pub fn last_refresh_time(&self) -> std::time::Instant {
        self.last_refresh
    }

    /// Check if registry needs refresh based on interval
    pub fn needs_refresh(&self, refresh_interval: std::time::Duration) -> bool {
        self.last_refresh.elapsed() >= refresh_interval
    }
}
