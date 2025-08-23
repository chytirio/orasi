//! Service health checking implementation

use crate::{
    error::GatewayError,
    types::{EndpointHealthStatus, ServiceEndpoint, ServiceHealthStatus, ServiceInfo},
};
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Health checker for service endpoints
pub struct HealthChecker {
    http_client: Client,
    timeout: Duration,
    health_check_path: String,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(timeout: Duration, health_check_path: String) -> Self {
        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            timeout,
            health_check_path,
        }
    }

    /// Check health of a single endpoint
    pub async fn check_endpoint_health(&self, endpoint: &ServiceEndpoint) -> EndpointHealthStatus {
        let health_url = format!("{}{}", endpoint.url, self.health_check_path);

        match timeout(self.timeout, self.http_client.get(&health_url).send()).await {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    debug!("Health check passed for endpoint: {}", endpoint.url);
                    EndpointHealthStatus::Healthy
                } else {
                    warn!(
                        "Health check failed for endpoint {}: status {}",
                        endpoint.url,
                        response.status()
                    );
                    EndpointHealthStatus::Unhealthy
                }
            }
            Ok(Err(e)) => {
                warn!("Health check error for endpoint {}: {}", endpoint.url, e);
                EndpointHealthStatus::Unhealthy
            }
            Err(_) => {
                warn!("Health check timeout for endpoint: {}", endpoint.url);
                EndpointHealthStatus::Unhealthy
            }
        }
    }

    /// Check health of all endpoints for a service
    pub async fn check_service_health(
        &self,
        service_info: &mut ServiceInfo,
    ) -> ServiceHealthStatus {
        let mut healthy_endpoints = 0;
        let total_endpoints = service_info.endpoints.len();

        for endpoint in &mut service_info.endpoints {
            let health_status = self.check_endpoint_health(endpoint).await;
            endpoint.health_status = health_status.clone();

            if health_status == EndpointHealthStatus::Healthy {
                healthy_endpoints += 1;
            }
        }

        // Determine overall service health based on endpoint health
        let health_ratio = if total_endpoints > 0 {
            healthy_endpoints as f64 / total_endpoints as f64
        } else {
            0.0
        };

        let service_health = if health_ratio >= 0.8 {
            ServiceHealthStatus::Healthy
        } else if health_ratio >= 0.5 {
            ServiceHealthStatus::Degraded
        } else {
            ServiceHealthStatus::Unhealthy
        };

        service_info.health_status = service_health.clone();

        info!(
            "Service {} health check: {}/{} endpoints healthy, status: {:?}",
            service_info.name, healthy_endpoints, total_endpoints, service_health
        );

        service_health
    }

    /// Check health of multiple services
    pub async fn check_services_health(
        &self,
        services: &mut HashMap<String, ServiceInfo>,
    ) -> HashMap<String, ServiceHealthStatus> {
        let mut results = HashMap::new();

        for (service_id, service_info) in services.iter_mut() {
            let health_status = self.check_service_health(service_info).await;
            results.insert(service_id.clone(), health_status);
        }

        results
    }
}

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    pub timeout: Duration,
    pub health_check_path: String,
    pub interval: Duration,
    pub retries: u32,
    pub success_threshold: u32,
    pub failure_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            health_check_path: "/health".to_string(),
            interval: Duration::from_secs(30),
            retries: 3,
            success_threshold: 2,
            failure_threshold: 3,
        }
    }
}

/// Advanced health checker with retry logic and thresholds
#[derive(Clone)]
pub struct AdvancedHealthChecker {
    config: HealthCheckConfig,
    http_client: Client,
    health_history: HashMap<String, Vec<bool>>,
}

impl AdvancedHealthChecker {
    /// Create a new advanced health checker
    pub fn new(config: HealthCheckConfig) -> Self {
        let http_client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            health_history: HashMap::new(),
        }
    }

    /// Check endpoint health with retry logic
    async fn check_endpoint_with_retries(&self, endpoint: &ServiceEndpoint) -> bool {
        let health_url = format!("{}{}", endpoint.url, self.config.health_check_path);
        let mut consecutive_failures = 0;

        for attempt in 0..self.config.retries {
            match timeout(
                self.config.timeout,
                self.http_client.get(&health_url).send(),
            )
            .await
            {
                Ok(Ok(response)) => {
                    if response.status().is_success() {
                        debug!(
                            "Health check passed for endpoint: {} (attempt {})",
                            endpoint.url,
                            attempt + 1
                        );
                        return true;
                    } else {
                        consecutive_failures += 1;
                        warn!(
                            "Health check failed for endpoint {}: status {} (attempt {})",
                            endpoint.url,
                            response.status(),
                            attempt + 1
                        );
                    }
                }
                Ok(Err(e)) => {
                    consecutive_failures += 1;
                    warn!(
                        "Health check error for endpoint {}: {} (attempt {})",
                        endpoint.url,
                        e,
                        attempt + 1
                    );
                }
                Err(_) => {
                    consecutive_failures += 1;
                    warn!(
                        "Health check timeout for endpoint: {} (attempt {})",
                        endpoint.url,
                        attempt + 1
                    );
                }
            }

            if consecutive_failures >= self.config.failure_threshold {
                break;
            }
        }

        false
    }

    /// Update health history for a service
    fn update_health_history(&mut self, service_id: &str, is_healthy: bool) {
        let history = self
            .health_history
            .entry(service_id.to_string())
            .or_insert_with(Vec::new);
        history.push(is_healthy);

        // Keep only recent history (last 10 checks)
        if history.len() > 10 {
            history.remove(0);
        }
    }

    /// Determine service health based on history and thresholds
    fn determine_service_health(&self, service_id: &str) -> ServiceHealthStatus {
        if let Some(history) = self.health_history.get(service_id) {
            if history.len() < self.config.success_threshold as usize {
                return ServiceHealthStatus::Unknown;
            }

            let recent_history = history
                .iter()
                .rev()
                .take(self.config.success_threshold as usize);
            let healthy_count = recent_history.filter(|&&healthy| healthy).count();

            if healthy_count >= self.config.success_threshold as usize {
                ServiceHealthStatus::Healthy
            } else if healthy_count > 0 {
                ServiceHealthStatus::Degraded
            } else {
                ServiceHealthStatus::Unhealthy
            }
        } else {
            ServiceHealthStatus::Unknown
        }
    }

    /// Check service health with advanced logic
    pub async fn check_service_health(
        &mut self,
        service_id: &str,
        service_info: &mut ServiceInfo,
    ) -> ServiceHealthStatus {
        let mut healthy_endpoints = 0;
        let total_endpoints = service_info.endpoints.len();

        for endpoint in &mut service_info.endpoints {
            let is_healthy = self.check_endpoint_with_retries(endpoint).await;
            endpoint.health_status = if is_healthy {
                EndpointHealthStatus::Healthy
            } else {
                EndpointHealthStatus::Unhealthy
            };

            if is_healthy {
                healthy_endpoints += 1;
            }
        }

        // Determine if service is overall healthy
        let service_healthy = if total_endpoints > 0 {
            healthy_endpoints as f64 / total_endpoints as f64 >= 0.5
        } else {
            false
        };

        // Update health history
        self.update_health_history(service_id, service_healthy);

        // Determine final health status
        let health_status = self.determine_service_health(service_id);
        service_info.health_status = health_status.clone();

        info!(
            "Advanced health check for service {}: {}/{} endpoints healthy, status: {:?}",
            service_info.name, healthy_endpoints, total_endpoints, health_status
        );

        health_status
    }
}
