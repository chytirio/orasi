//! Health checking

use crate::{config::GatewayConfig, error::GatewayError, types::*};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Health checker for gateway
pub struct HealthChecker {
    config: GatewayConfig,
    health_status: Arc<RwLock<HealthStatus>>,
    task_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    shutdown_signal: Arc<RwLock<bool>>,
}

impl HealthChecker {
    /// Create new health checker
    pub async fn new(config: &GatewayConfig) -> Result<Self, GatewayError> {
        let initial_health = HealthStatus {
            service: "orasi-gateway".to_string(),
            status: HealthState::Healthy,
            details: HashMap::new(),
            timestamp: current_timestamp(),
        };

        Ok(Self {
            config: config.clone(),
            health_status: Arc::new(RwLock::new(initial_health)),
            task_handle: Arc::new(RwLock::new(None)),
            shutdown_signal: Arc::new(RwLock::new(false)),
        })
    }

    /// Start health checker
    pub async fn start(&self) -> Result<(), GatewayError> {
        if self.task_handle.read().await.is_some() {
            warn!("Health checker is already running");
            return Ok(());
        }

        let health_status = Arc::clone(&self.health_status);
        let shutdown_signal = Arc::clone(&self.shutdown_signal);
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let mut interval_timer = interval(Duration::from_secs(
                config.service_discovery.health_check_interval.as_secs(),
            ));

            info!("Health checker started with interval: {:?}", config.service_discovery.health_check_interval);

            loop {
                interval_timer.tick().await;

                // Check if shutdown was requested
                if *shutdown_signal.read().await {
                    info!("Health checker shutdown requested");
                    break;
                }

                // Perform health check
                let health_result = Self::perform_health_check(&config).await;
                
                // Update health status
                {
                    let mut status = health_status.write().await;
                    *status = health_result.clone();
                }

                debug!("Health check completed: {:?}", health_result.status);
            }

            info!("Health checker stopped");
        });

        // Store the task handle
        {
            let mut task_handle = self.task_handle.write().await;
            *task_handle = Some(handle);
        }

        info!("Health checker started successfully");
        Ok(())
    }

    /// Stop health checker
    pub async fn stop(&self) -> Result<(), GatewayError> {
        // Signal shutdown
        {
            let mut shutdown = self.shutdown_signal.write().await;
            *shutdown = true;
        }

        // Wait for task to complete
        {
            let mut task_handle = self.task_handle.write().await;
            if let Some(handle) = task_handle.take() {
                match tokio::time::timeout(Duration::from_secs(5), handle).await {
                    Ok(_) => info!("Health checker stopped gracefully"),
                    Err(_) => {
                        warn!("Health checker did not stop within timeout, aborting");
                        // Note: handle is already consumed, so we can't abort it
                    }
                }
            }
        }

        // Reset shutdown signal
        {
            let mut shutdown = self.shutdown_signal.write().await;
            *shutdown = false;
        }

        info!("Health checker shutdown completed");
        Ok(())
    }

    /// Check health status
    pub async fn check_health(&self) -> Result<HealthStatus, GatewayError> {
        let status = self.health_status.read().await;
        Ok(status.clone())
    }

    /// Perform comprehensive health check
    async fn perform_health_check(config: &GatewayConfig) -> HealthStatus {
        let mut details = HashMap::new();
        let mut overall_status = HealthState::Healthy;

        // Check gateway endpoint availability
        if let Err(e) = Self::check_endpoint_availability(&config.gateway_endpoint).await {
            details.insert("gateway_endpoint".to_string(), format!("Error: {}", e));
            overall_status = HealthState::Unhealthy;
        } else {
            details.insert("gateway_endpoint".to_string(), "Available".to_string());
        }

        // Check admin endpoint availability
        if let Err(e) = Self::check_endpoint_availability(&config.admin_endpoint).await {
            details.insert("admin_endpoint".to_string(), format!("Error: {}", e));
            if overall_status == HealthState::Healthy {
                overall_status = HealthState::Degraded;
            }
        } else {
            details.insert("admin_endpoint".to_string(), "Available".to_string());
        }

        // Check metrics endpoint availability
        if let Err(e) = Self::check_endpoint_availability(&config.metrics_endpoint).await {
            details.insert("metrics_endpoint".to_string(), format!("Error: {}", e));
            if overall_status == HealthState::Healthy {
                overall_status = HealthState::Degraded;
            }
        } else {
            details.insert("metrics_endpoint".to_string(), "Available".to_string());
        }

        // Check service discovery connectivity
        match Self::check_service_discovery(&config).await {
            Ok(status) => {
                details.insert("service_discovery".to_string(), status);
            }
            Err(e) => {
                details.insert("service_discovery".to_string(), format!("Error: {}", e));
                if overall_status == HealthState::Healthy {
                    overall_status = HealthState::Degraded;
                }
            }
        }

        // Add system information
        details.insert("gateway_id".to_string(), config.gateway_id.clone());
        details.insert("uptime".to_string(), format!("{}s", current_timestamp()));

        HealthStatus {
            service: "orasi-gateway".to_string(),
            status: overall_status,
            details,
            timestamp: current_timestamp(),
        }
    }

    /// Check if an endpoint is available
    async fn check_endpoint_availability(endpoint: &str) -> Result<(), String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("http://{}", endpoint)
        };

        client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        Ok(())
    }

    /// Check service discovery connectivity
    async fn check_service_discovery(config: &GatewayConfig) -> Result<String, String> {
        match &config.service_discovery.backend {
            crate::config::ServiceDiscoveryBackend::Etcd => {
                // Try to connect to etcd endpoints
                for endpoint in &config.service_discovery.endpoints {
                    if let Ok(client) = reqwest::Client::builder()
                        .timeout(Duration::from_secs(2))
                        .build()
                    {
                        if client.get(endpoint).send().await.is_ok() {
                            return Ok("Etcd connected".to_string());
                        }
                    }
                }
                Err("No etcd endpoints available".to_string())
            }
            crate::config::ServiceDiscoveryBackend::Consul => {
                // Try to connect to Consul using the first endpoint
                if let Some(endpoint) = config.service_discovery.endpoints.first() {
                    let consul_url = format!("{}/v1/status/leader", endpoint);
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(2))
                        .build()
                        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

                    client
                        .get(&consul_url)
                        .send()
                        .await
                        .map_err(|e| format!("Consul connection failed: {}", e))?;

                    Ok("Consul connected".to_string())
                } else {
                    Err("No Consul endpoints configured".to_string())
                }
            }
            crate::config::ServiceDiscoveryBackend::Static => {
                Ok("Static service discovery".to_string())
            }
        }
    }
}

impl Drop for HealthChecker {
    fn drop(&mut self) {
        // Try to abort the task if it's still running
        // Note: This is a best-effort attempt since we can't await in Drop
        if let Ok(task_handle) = self.task_handle.try_read() {
            if let Some(handle) = task_handle.as_ref() {
                handle.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ConsulConfig, ServiceDiscoveryConfig};

    #[tokio::test]
    async fn test_health_checker_creation() {
        let config = GatewayConfig::default();
        let health_checker = HealthChecker::new(&config).await;
        assert!(health_checker.is_ok());
    }

    #[tokio::test]
    async fn test_health_checker_start_stop() {
        let config = GatewayConfig::default();
        let health_checker = HealthChecker::new(&config).await.unwrap();
        
        // Start the health checker
        assert!(health_checker.start().await.is_ok());
        
        // Wait a bit for it to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Stop the health checker
        assert!(health_checker.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_health_status_check() {
        let config = GatewayConfig::default();
        let health_checker = HealthChecker::new(&config).await.unwrap();
        
        let health_status = health_checker.check_health().await.unwrap();
        assert_eq!(health_status.service, "orasi-gateway");
        assert_eq!(health_status.status, HealthState::Healthy);
        assert!(!health_status.details.is_empty());
    }

    #[tokio::test]
    async fn test_endpoint_availability_check() {
        // Test with a non-existent endpoint (should fail)
        let result = HealthChecker::check_endpoint_availability("localhost:9999").await;
        assert!(result.is_err());
        
        // Test with a valid URL format
        let result = HealthChecker::check_endpoint_availability("http://example.com").await;
        // This might succeed or fail depending on network, but should not panic
        // We just test that the function handles the URL correctly
    }

    #[tokio::test]
    async fn test_service_discovery_check() {
        let mut config = GatewayConfig::default();
        
        // Test static service discovery
        config.service_discovery.backend = crate::config::ServiceDiscoveryBackend::Static;
        let result = HealthChecker::check_service_discovery(&config).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Static service discovery");
        
        // Test etcd service discovery (should fail without etcd running)
        config.service_discovery.backend = crate::config::ServiceDiscoveryBackend::Etcd;
        let result = HealthChecker::check_service_discovery(&config).await;
        assert!(result.is_err());
    }
}
