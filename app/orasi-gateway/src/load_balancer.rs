//! Load balancer

use crate::{
    config::{GatewayConfig, LoadBalancingAlgorithm},
    error::GatewayError,
    types::*,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Load balancer for gateway
pub struct LoadBalancer {
    config: GatewayConfig,
    state: Arc<RwLock<super::gateway::GatewayState>>,
    endpoints: Arc<RwLock<HashMap<String, Vec<ServiceEndpoint>>>>,
    round_robin_counters: Arc<RwLock<HashMap<String, AtomicUsize>>>,
    connection_counts: Arc<RwLock<HashMap<String, usize>>>,
}

impl LoadBalancer {
    /// Create new load balancer
    pub async fn new(
        config: &GatewayConfig,
        state: Arc<RwLock<super::gateway::GatewayState>>,
    ) -> Result<Self, GatewayError> {
        let endpoints = Arc::new(RwLock::new(HashMap::new()));
        let round_robin_counters = Arc::new(RwLock::new(HashMap::new()));
        let connection_counts = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            config: config.clone(),
            state,
            endpoints,
            round_robin_counters,
            connection_counts,
        })
    }

    /// Start load balancer
    pub async fn start(&self) -> Result<(), GatewayError> {
        info!("Starting load balancer");

        // Initialize endpoints from service discovery
        self.refresh_endpoints().await?;

        info!("Load balancer started successfully");
        Ok(())
    }

    /// Stop load balancer
    pub async fn stop(&self) -> Result<(), GatewayError> {
        info!("Stopping load balancer");

        // Clear endpoints
        {
            let mut endpoints = self.endpoints.write().await;
            endpoints.clear();
        }

        info!("Load balancer stopped");
        Ok(())
    }

    /// Select endpoint without request context (for backward compatibility)
    pub async fn select_endpoint_simple(
        &self,
        service_name: &str,
    ) -> Result<ServiceEndpoint, GatewayError> {
        self.select_endpoint(service_name, None).await
    }

    /// Select endpoint
    pub async fn select_endpoint(
        &self,
        service_name: &str,
        request_context: Option<&RequestContext>,
    ) -> Result<ServiceEndpoint, GatewayError> {
        debug!("Selecting endpoint for service: {}", service_name);

        let endpoints = {
            let endpoints = self.endpoints.read().await;
            endpoints.get(service_name).cloned()
        };

        if let Some(endpoints) = endpoints {
            if endpoints.is_empty() {
                return Err(GatewayError::NoHealthyEndpoints(format!(
                    "No healthy endpoints for service: {}",
                    service_name
                )));
            }

            // Filter healthy endpoints
            let healthy_endpoints: Vec<ServiceEndpoint> = endpoints
                .into_iter()
                .filter(|ep| ep.health_status == EndpointHealthStatus::Healthy)
                .collect();

            if healthy_endpoints.is_empty() {
                return Err(GatewayError::NoHealthyEndpoints(format!(
                    "No healthy endpoints for service: {}",
                    service_name
                )));
            }

            // Select endpoint based on algorithm
            let selected_endpoint = match self.config.load_balancing.algorithm {
                LoadBalancingAlgorithm::RoundRobin => {
                    self.select_round_robin(service_name, &healthy_endpoints)
                        .await?
                }
                LoadBalancingAlgorithm::LeastConnections => {
                    self.select_least_connections(service_name, &healthy_endpoints)
                        .await?
                }
                LoadBalancingAlgorithm::WeightedRoundRobin => {
                    self.select_weighted_round_robin(service_name, &healthy_endpoints)
                        .await?
                }
                LoadBalancingAlgorithm::Random => self.select_random(&healthy_endpoints).await?,
                LoadBalancingAlgorithm::IpHash => {
                    self.select_ip_hash(service_name, &healthy_endpoints, request_context)
                        .await?
                }
            };

            // Increment connection count
            self.increment_connection_count(&selected_endpoint.url)
                .await;

            debug!("Selected endpoint: {}", selected_endpoint.url);
            Ok(selected_endpoint)
        } else {
            Err(GatewayError::ServiceNotFound(format!(
                "Service not found: {}",
                service_name
            )))
        }
    }

    /// Refresh endpoints from service discovery
    pub async fn refresh_endpoints(&self) -> Result<(), GatewayError> {
        info!("Refreshing endpoints from service discovery");

        // Get services from the gateway state
        let services = {
            let state = self.state.read().await;
            state.get_services()
        };

        // Convert ServiceInfo to ServiceEndpoint format
        let mut endpoints = HashMap::new();
        for (service_name, service_info) in &services {
            endpoints.insert(service_name.clone(), service_info.endpoints.clone());
        }

        // Update the endpoints
        {
            let mut current_endpoints = self.endpoints.write().await;
            current_endpoints.clear();
            current_endpoints.extend(endpoints);
        }

        info!(
            "Refreshed {} services with {} total endpoints",
            services.len(),
            services.values().map(|s| s.endpoints.len()).sum::<usize>()
        );
        Ok(())
    }

    /// Get endpoints for service
    pub async fn get_endpoints(&self, service_name: &str) -> Vec<ServiceEndpoint> {
        let endpoints = self.endpoints.read().await;
        endpoints.get(service_name).cloned().unwrap_or_default()
    }

    /// Update endpoint health status
    pub async fn update_endpoint_health(
        &self,
        service_name: &str,
        endpoint_url: &str,
        health_status: EndpointHealthStatus,
    ) -> Result<(), GatewayError> {
        {
            let mut endpoints = self.endpoints.write().await;
            if let Some(service_endpoints) = endpoints.get_mut(service_name) {
                for endpoint in service_endpoints {
                    if endpoint.url == endpoint_url {
                        endpoint.health_status = health_status.clone();
                        break;
                    }
                }
            }
        }

        debug!(
            "Updated endpoint health: {} -> {:?}",
            endpoint_url,
            health_status.clone()
        );
        Ok(())
    }

    /// Round-robin selection
    async fn select_round_robin(
        &self,
        service_name: &str,
        endpoints: &[ServiceEndpoint],
    ) -> Result<ServiceEndpoint, GatewayError> {
        let index = {
            let mut counters = self.round_robin_counters.write().await;
            let counter = counters
                .entry(service_name.to_string())
                .or_insert_with(|| AtomicUsize::new(0));
            counter.fetch_add(1, Ordering::Relaxed)
        };

        Ok(endpoints[index % endpoints.len()].clone())
    }

    /// Least connections selection
    async fn select_least_connections(
        &self,
        _service_name: &str,
        endpoints: &[ServiceEndpoint],
    ) -> Result<ServiceEndpoint, GatewayError> {
        let connection_counts = self.connection_counts.read().await;

        let mut min_connections = usize::MAX;
        let mut selected_endpoint = None;

        for endpoint in endpoints {
            let connections = connection_counts.get(&endpoint.url).unwrap_or(&0);
            if *connections < min_connections {
                min_connections = *connections;
                selected_endpoint = Some(endpoint.clone());
            }
        }

        selected_endpoint.ok_or_else(|| {
            GatewayError::LoadBalancingError(
                "No endpoints available for least connections selection".to_string(),
            )
        })
    }

    /// Weighted round-robin selection
    async fn select_weighted_round_robin(
        &self,
        service_name: &str,
        endpoints: &[ServiceEndpoint],
    ) -> Result<ServiceEndpoint, GatewayError> {
        let index = {
            let mut counters = self.round_robin_counters.write().await;
            let counter = counters
                .entry(service_name.to_string())
                .or_insert_with(|| AtomicUsize::new(0));
            counter.fetch_add(1, Ordering::Relaxed)
        };

        // Calculate weighted index
        let total_weight: u32 = endpoints.iter().map(|ep| ep.weight).sum();
        let weighted_index = (index % total_weight as usize) as u32;

        let mut current_weight: u32 = 0;
        for endpoint in endpoints {
            current_weight += endpoint.weight;
            if weighted_index < current_weight {
                return Ok(endpoint.clone());
            }
        }

        // Fallback to first endpoint
        Ok(endpoints[0].clone())
    }

    /// Random selection
    async fn select_random(
        &self,
        endpoints: &[ServiceEndpoint],
    ) -> Result<ServiceEndpoint, GatewayError> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..endpoints.len());
        Ok(endpoints[index].clone())
    }

    /// IP hash selection
    async fn select_ip_hash(
        &self,
        _service_name: &str,
        endpoints: &[ServiceEndpoint],
        request_context: Option<&RequestContext>,
    ) -> Result<ServiceEndpoint, GatewayError> {
        let client_ip = request_context.map(|ctx| &ctx.client_ip).ok_or_else(|| {
            GatewayError::LoadBalancingError(
                "Client IP not available for IP hash selection".to_string(),
            )
        })?;

        let hash = self.hash_ip(client_ip);
        let index = hash % endpoints.len();
        Ok(endpoints[index].clone())
    }

    /// Hash IP address
    fn hash_ip(&self, ip: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        ip.hash(&mut hasher);
        hasher.finish() as usize
    }

    /// Increment connection count
    async fn increment_connection_count(&self, endpoint_url: &str) {
        let mut connection_counts = self.connection_counts.write().await;
        *connection_counts
            .entry(endpoint_url.to_string())
            .or_insert(0) += 1;
    }

    /// Decrement connection count
    pub async fn decrement_connection_count(&self, endpoint_url: &str) {
        let mut connection_counts = self.connection_counts.write().await;
        if let Some(count) = connection_counts.get_mut(endpoint_url) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }
}
