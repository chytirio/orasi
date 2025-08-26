//! Main service discovery implementation

use crate::config::{AgentConfig, ServiceDiscoveryBackend};
use crate::error::AgentError;
use crate::types::AgentInfo;
use crate::discovery::types::ServiceRegistration;
use crate::discovery::etcd::EtcdClient;
use crate::discovery::consul::ConsulClientWrapper;
use crate::discovery::current_timestamp;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{error, info};

/// Service discovery for agent registration and discovery
pub struct ServiceDiscovery {
    config: AgentConfig,
    backend: ServiceDiscoveryBackend,
    registered_services: Arc<RwLock<HashMap<String, ServiceRegistration>>>,
    discovered_services: Arc<RwLock<HashMap<String, ServiceRegistration>>>,
    etcd_client: Option<EtcdClient>,
    consul_client: Option<ConsulClientWrapper>,
    running: bool,
}

impl ServiceDiscovery {
    /// Create new service discovery
    pub async fn new(config: &AgentConfig) -> Result<Self, AgentError> {
        let backend = config.cluster.service_discovery.clone();

        let mut etcd_client = None;
        let mut consul_client = None;

        // Initialize backend clients
        match backend {
            ServiceDiscoveryBackend::Etcd => {
                let endpoints = config.cluster.etcd_endpoints.clone();
                etcd_client = Some(EtcdClient::new(endpoints).await?);
            }
            ServiceDiscoveryBackend::Consul => {
                let consul_url = config.cluster.consul_url.clone();
                consul_client = Some(ConsulClientWrapper::new(&consul_url).await?);
            }
            ServiceDiscoveryBackend::Static => {
                // No client needed for static discovery
            }
        }

        Ok(Self {
            config: config.clone(),
            backend,
            registered_services: Arc::new(RwLock::new(HashMap::new())),
            discovered_services: Arc::new(RwLock::new(HashMap::new())),
            etcd_client,
            consul_client,
            running: false,
        })
    }

    /// Start service discovery
    pub async fn start(&mut self) -> Result<(), AgentError> {
        if self.running {
            return Ok(());
        }

        info!(
            "Starting service discovery with backend: {:?}",
            self.backend
        );
        self.running = true;

        // Start service discovery loop
        let discovered_services = self.discovered_services.clone();
        let backend = self.backend.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            Self::service_discovery_loop(discovered_services, backend, config).await;
        });

        // Start lease keep-alive for etcd
        // Note: tokio::spawn requires 'static lifetime
        // This is a simplified implementation
        if let Some(ref etcd_client) = self.etcd_client {
            info!("Etcd client available for keep-alive (not started due to lifetime constraints)");
        }

        info!("Service discovery started successfully");
        Ok(())
    }

    /// Stop service discovery
    pub async fn stop(&mut self) -> Result<(), AgentError> {
        if !self.running {
            return Ok(());
        }

        info!("Stopping service discovery");
        self.running = false;

        // Deregister all services
        self.deregister_all_services().await?;

        info!("Service discovery stopped");
        Ok(())
    }

    /// Register agent with service discovery
    pub async fn register_agent(&mut self, agent_info: AgentInfo) -> Result<(), AgentError> {
        let registration = ServiceRegistration {
            service_id: agent_info.agent_id.clone(),
            service_name: "orasi-agent".to_string(),
            endpoint: agent_info.endpoint.clone(),
            metadata: agent_info.metadata.clone(),
            health_endpoint: Some(format!("{}/health", agent_info.endpoint)),
            registered_at: current_timestamp(),
            ttl: Some(30), // 30 seconds TTL
        };

        match self.backend {
            ServiceDiscoveryBackend::Etcd => {
                if let Some(ref mut etcd_client) = self.etcd_client {
                    etcd_client.register_service(&registration).await?;
                }
            }
            ServiceDiscoveryBackend::Consul => {
                if let Some(ref consul_client) = self.consul_client {
                    consul_client.register_service(&registration).await?;
                }
            }
            ServiceDiscoveryBackend::Static => {
                self.register_static(&registration).await?;
            }
        }

        // Add to registered services
        {
            let mut services = self.registered_services.write().await;
            services.insert(agent_info.agent_id.clone(), registration);
        }

        info!(
            "Agent registered with service discovery: {}",
            agent_info.agent_id
        );
        Ok(())
    }

    /// Deregister agent from service discovery
    pub async fn deregister_agent(&mut self, agent_id: &str) -> Result<(), AgentError> {
        match self.backend {
            ServiceDiscoveryBackend::Etcd => {
                if let Some(ref mut etcd_client) = self.etcd_client {
                    etcd_client.deregister_service(agent_id).await?;
                }
            }
            ServiceDiscoveryBackend::Consul => {
                if let Some(ref consul_client) = self.consul_client {
                    consul_client.deregister_service(agent_id).await?;
                }
            }
            ServiceDiscoveryBackend::Static => {
                self.deregister_static(agent_id).await?;
            }
        }

        // Remove from registered services
        {
            let mut services = self.registered_services.write().await;
            services.remove(agent_id);
        }

        info!("Agent deregistered from service discovery: {}", agent_id);
        Ok(())
    }

    /// Discover services
    pub async fn discover_services(&mut self) -> Result<Vec<ServiceRegistration>, AgentError> {
        let services = match self.backend {
            ServiceDiscoveryBackend::Etcd => {
                if let Some(ref mut etcd_client) = self.etcd_client {
                    etcd_client.discover_services().await?
                } else {
                    vec![]
                }
            }
            ServiceDiscoveryBackend::Consul => {
                if let Some(ref consul_client) = self.consul_client {
                    consul_client.discover_services().await?
                } else {
                    vec![]
                }
            }
            ServiceDiscoveryBackend::Static => self.discover_static().await?,
        };

        // Update discovered services
        {
            let mut discovered = self.discovered_services.write().await;
            discovered.clear();
            for service in &services {
                discovered.insert(service.service_id.clone(), service.clone());
            }
        }

        Ok(services)
    }

    /// Get discovered services
    pub async fn get_discovered_services(&self) -> Vec<ServiceRegistration> {
        let services = self.discovered_services.read().await;
        services.values().cloned().collect()
    }

    /// Service discovery loop
    async fn service_discovery_loop(
        discovered_services: Arc<RwLock<HashMap<String, ServiceRegistration>>>,
        backend: ServiceDiscoveryBackend,
        config: AgentConfig,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            // Discover services
            let services = match backend {
                ServiceDiscoveryBackend::Etcd => Self::discover_from_etcd_static(&config).await,
                ServiceDiscoveryBackend::Consul => Self::discover_from_consul_static(&config).await,
                ServiceDiscoveryBackend::Static => Self::discover_static_static(&config).await,
            };

            match services {
                Ok(services) => {
                    let mut discovered = discovered_services.write().await;
                    discovered.clear();
                    for service in services {
                        discovered.insert(service.service_id.clone(), service);
                    }
                }
                Err(e) => {
                    error!("Failed to discover services: {}", e);
                }
            }
        }
    }

    /// Register static service
    async fn register_static(&self, registration: &ServiceRegistration) -> Result<(), AgentError> {
        info!("Registering static service: {}", registration.service_id);
        Ok(())
    }

    /// Deregister static service
    async fn deregister_static(&self, service_id: &str) -> Result<(), AgentError> {
        info!("Deregistering static service: {}", service_id);
        Ok(())
    }

    /// Discover static services
    async fn discover_static(&self) -> Result<Vec<ServiceRegistration>, AgentError> {
        // Return static services from configuration
        let static_services = self.config.cluster.static_services.clone();
        let mut services = Vec::new();

        for static_service in static_services {
            let registration = ServiceRegistration {
                service_id: static_service.id,
                service_name: static_service.name,
                endpoint: static_service.endpoint,
                metadata: static_service.metadata,
                health_endpoint: static_service.health_endpoint,
                registered_at: current_timestamp(),
                ttl: None,
            };
            services.push(registration);
        }

        info!("Discovered {} static services", services.len());
        Ok(services)
    }

    /// Deregister all services
    async fn deregister_all_services(&mut self) -> Result<(), AgentError> {
        let services = {
            let registered = self.registered_services.read().await;
            registered.keys().cloned().collect::<Vec<_>>()
        };

        for service_id in services {
            self.deregister_agent(&service_id).await?;
        }

        Ok(())
    }

    /// Static etcd discovery
    async fn discover_from_etcd_static(
        config: &AgentConfig,
    ) -> Result<Vec<ServiceRegistration>, AgentError> {
        let endpoints = config.cluster.etcd_endpoints.clone();
        let mut etcd_client = EtcdClient::new(endpoints).await?;
        etcd_client.discover_services().await
    }

    /// Static Consul discovery
    async fn discover_from_consul_static(
        config: &AgentConfig,
    ) -> Result<Vec<ServiceRegistration>, AgentError> {
        let consul_url = config.cluster.consul_url.clone();
        let consul_client = ConsulClientWrapper::new(&consul_url).await?;
        consul_client.discover_services().await
    }

    /// Static service discovery
    async fn discover_static_static(
        config: &AgentConfig,
    ) -> Result<Vec<ServiceRegistration>, AgentError> {
        // Return static services from configuration
        let static_services = config.cluster.static_services.clone();
        let mut services = Vec::new();

        for static_service in static_services {
            let registration = ServiceRegistration {
                service_id: static_service.id,
                service_name: static_service.name,
                endpoint: static_service.endpoint,
                metadata: static_service.metadata,
                health_endpoint: static_service.health_endpoint,
                registered_at: current_timestamp(),
                ttl: None,
            };
            services.push(registration);
        }

        Ok(services)
    }
}
