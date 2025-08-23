//! Core service discovery implementation

use crate::{
    config::GatewayConfig,
    discovery::{
        backends::{BackendFactory, ServiceDiscoveryBackend},
        health::{AdvancedHealthChecker, HealthCheckConfig},
        registry::ServiceRegistry,
    },
    error::GatewayError,
    types::ServiceInfo,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Service discovery for gateway
pub struct ServiceDiscovery {
    config: GatewayConfig,
    registry: Arc<RwLock<ServiceRegistry>>,
    refresh_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    health_check_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    running: Arc<RwLock<bool>>,
}

impl ServiceDiscovery {
    /// Create new service discovery
    pub async fn new(config: &GatewayConfig) -> Result<Self, GatewayError> {
        info!("Initializing service discovery");

        let mut registry = ServiceRegistry::new();

        // Create and configure the backend
        let mut backend = BackendFactory::create_backend(
            &config.service_discovery.backend,
            &config.service_discovery.endpoints,
            Some("/orasi".to_string()),
            Some(&config.service_discovery.consul),
        )
        .await?;

        // Initialize the backend
        backend.initialize().await?;

        // Set the backend in the registry
        registry.set_backend(backend);

        // Create and configure health checker
        let health_config = HealthCheckConfig {
            timeout: Duration::from_secs(5),
            health_check_path: "/health".to_string(),
            interval: config.service_discovery.health_check_interval,
            retries: 3,
            success_threshold: 2,
            failure_threshold: 3,
        };

        let health_checker = AdvancedHealthChecker::new(health_config);
        registry.set_health_checker(health_checker);

        let registry = Arc::new(RwLock::new(registry));

        Ok(Self {
            config: config.clone(),
            registry,
            refresh_handle: Arc::new(RwLock::new(None)),
            health_check_handle: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start service discovery
    pub async fn start(&self) -> Result<(), GatewayError> {
        info!("Starting service discovery");

        let mut running = self.running.write().await;
        if *running {
            warn!("Service discovery is already running");
            return Ok(());
        }
        *running = true;
        drop(running);

        // Start the refresh loop
        let registry = Arc::clone(&self.registry);
        let refresh_interval = self.config.service_discovery.refresh_interval;
        let refresh_handle = tokio::spawn(async move {
            Self::refresh_loop(registry, refresh_interval).await;
        });

        // Start the health check loop
        let registry = Arc::clone(&self.registry);
        let health_check_interval = self.config.service_discovery.health_check_interval;
        let health_check_handle = tokio::spawn(async move {
            Self::health_check_loop(registry, health_check_interval).await;
        });

        // Store the handles
        {
            let mut refresh_handle_guard = self.refresh_handle.write().await;
            *refresh_handle_guard = Some(refresh_handle);
        }
        {
            let mut health_check_handle_guard = self.health_check_handle.write().await;
            *health_check_handle_guard = Some(health_check_handle);
        }

        info!("Service discovery started successfully");
        Ok(())
    }

    /// Stop service discovery
    pub async fn stop(&self) -> Result<(), GatewayError> {
        info!("Stopping service discovery");

        let mut running = self.running.write().await;
        if !*running {
            warn!("Service discovery is not running");
            return Ok(());
        }
        *running = false;
        drop(running);

        // Cancel the background tasks
        {
            let refresh_handle_guard = self.refresh_handle.read().await;
            if let Some(handle) = &*refresh_handle_guard {
                handle.abort();
            }
        }

        {
            let health_check_handle_guard = self.health_check_handle.read().await;
            if let Some(handle) = &*health_check_handle_guard {
                handle.abort();
            }
        }

        info!("Service discovery stopped successfully");
        Ok(())
    }

    /// Refresh services
    pub async fn refresh_services(&self) -> Result<HashMap<String, ServiceInfo>, GatewayError> {
        debug!("Manual service refresh requested");
        let mut registry = self.registry.write().await;
        registry.refresh_services().await?;
        Ok(registry.get_services())
    }

    /// Get all services
    pub async fn get_services(&self) -> HashMap<String, ServiceInfo> {
        let registry = self.registry.read().await;
        registry.get_services()
    }

    /// Get healthy services only
    pub async fn get_healthy_services(&self) -> HashMap<String, ServiceInfo> {
        let registry = self.registry.read().await;
        registry.get_healthy_services()
    }

    /// Get a specific service
    pub async fn get_service(&self, service_id: &str) -> Option<ServiceInfo> {
        let registry = self.registry.read().await;
        registry.get_service(service_id).cloned()
    }

    /// Register a service
    pub async fn register_service(
        &self,
        service_id: &str,
        service_info: &ServiceInfo,
    ) -> Result<(), GatewayError> {
        info!("Registering service: {}", service_id);

        // Add to local registry
        let mut registry = self.registry.write().await;
        registry.add_service(service_id.to_string(), service_info.clone());

        // Register with backend if available
        if let Some(backend) = &mut registry.backend {
            if let Err(e) = backend.register_service(service_id, service_info).await {
                warn!("Failed to register service with backend: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Deregister a service
    pub async fn deregister_service(&self, service_id: &str) -> Result<(), GatewayError> {
        info!("Deregistering service: {}", service_id);

        // Remove from local registry
        let mut registry = self.registry.write().await;
        registry.remove_service(service_id);

        // Deregister from backend if available
        if let Some(backend) = &mut registry.backend {
            if let Err(e) = backend.deregister_service(service_id).await {
                warn!("Failed to deregister service from backend: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Background refresh loop
    async fn refresh_loop(registry: Arc<RwLock<ServiceRegistry>>, interval: Duration) {
        let mut interval_timer = tokio::time::interval(interval);

        loop {
            interval_timer.tick().await;

            let mut registry = registry.write().await;
            if let Err(e) = registry.refresh_services().await {
                error!("Failed to refresh services: {}", e);
            }
        }
    }

    /// Background health check loop
    async fn health_check_loop(registry: Arc<RwLock<ServiceRegistry>>, interval: Duration) {
        let mut interval_timer = tokio::time::interval(interval);

        loop {
            interval_timer.tick().await;

            // First, get the services and health checker
            let (services, health_checker) = {
                let mut registry = registry.write().await;
                let services: Vec<(String, ServiceInfo)> = registry.services.drain().collect();
                let health_checker = registry.health_checker.clone();
                (services, health_checker)
            };

            // Then, perform health checks
            if let Some(mut health_checker) = health_checker {
                let mut updated_services = Vec::new();
                for (service_id, mut service_info) in services {
                    health_checker
                        .check_service_health(&service_id, &mut service_info)
                        .await;
                    updated_services.push((service_id, service_info));
                }

                // Finally, update the registry
                {
                    let mut registry = registry.write().await;
                    for (service_id, service_info) in updated_services {
                        registry.services.insert(service_id, service_info);
                    }
                }
            }
        }
    }

    /// Check if service discovery is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

impl Clone for ServiceDiscovery {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            registry: Arc::clone(&self.registry),
            refresh_handle: Arc::new(RwLock::new(None)), // Handles can't be cloned
            health_check_handle: Arc::new(RwLock::new(None)),
            running: Arc::clone(&self.running),
        }
    }
}
