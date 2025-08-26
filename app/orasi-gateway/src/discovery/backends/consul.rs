//! Consul service discovery backend implementation

use crate::{
    config::ServiceDiscoveryBackend as ConfigBackend,
    error::GatewayError,
    types::{EndpointHealthStatus, ServiceEndpoint, ServiceHealthStatus, ServiceInfo},
};
use async_trait::async_trait;
use consul::Client as ConsulClient;
use reqwest::Client as HttpClient;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use url;
use super::trait_def::ServiceDiscoveryBackend;

/// Consul service discovery backend
pub struct ConsulBackend {
    client: Option<ConsulClient>,
    http_client: HttpClient,
    datacenter: String,
    token: Option<String>,
    endpoints: Vec<String>,
    service_prefix: String,
}

impl ConsulBackend {
    pub fn new(
        datacenter: String,
        token: Option<String>,
        endpoints: Vec<String>,
        service_prefix: String,
    ) -> Self {
        Self {
            client: None,
            http_client: HttpClient::new(),
            datacenter,
            token,
            endpoints,
            service_prefix,
        }
    }

    async fn get_client(&mut self) -> Result<&ConsulClient, GatewayError> {
        if self.client.is_none() {
            // Create Consul client configuration
            let mut config = consul::Config::new().map_err(|e| {
                GatewayError::ServiceDiscovery(format!("Failed to create Consul config: {}", e))
            })?;

            // Set the address (use first endpoint)
            if let Some(endpoint) = self.endpoints.first() {
                config.address = endpoint.clone();
            }

            // Set the datacenter
            config.datacenter = Some(self.datacenter.clone());

            // Set the token if provided
            if let Some(ref token) = self.token {
                config.token = Some(token.clone());
            }

            // Create the client
            let client = ConsulClient::new(config);
            self.client = Some(client);
        }
        Ok(self.client.as_ref().unwrap())
    }

    /// Get the base URL for Consul API calls
    fn get_base_url(&self) -> Result<String, GatewayError> {
        let endpoint = self.endpoints.first().ok_or_else(|| {
            GatewayError::ServiceDiscovery("No Consul endpoints configured".to_string())
        })?;

        // Ensure the endpoint has a scheme
        if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            Ok(endpoint.clone())
        } else {
            Ok(format!("http://{}", endpoint))
        }
    }

    /// Make an authenticated request to Consul API
    async fn make_consul_request(&self, path: &str) -> Result<reqwest::Response, GatewayError> {
        let base_url = self.get_base_url()?;
        let url = format!("{}/v1{}", base_url, path);

        let mut request = self.http_client.get(&url);

        // Add token if provided
        if let Some(ref token) = self.token {
            request = request.header("X-Consul-Token", token);
        }

        request.send().await.map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to make Consul API request: {}", e))
        })
    }

    /// Make an authenticated POST request to Consul API
    async fn make_consul_post_request(
        &self,
        path: &str,
        body: Option<String>,
    ) -> Result<reqwest::Response, GatewayError> {
        let base_url = self.get_base_url()?;
        let url = format!("{}/v1{}", base_url, path);

        let mut request = self.http_client.post(&url);

        // Add token if provided
        if let Some(ref token) = self.token {
            request = request.header("X-Consul-Token", token);
        }

        // Add body if provided
        if let Some(body_content) = body {
            request = request
                .header("Content-Type", "application/json")
                .body(body_content);
        }

        request.send().await.map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to make Consul API POST request: {}", e))
        })
    }

    /// Make an authenticated DELETE request to Consul API
    async fn make_consul_delete_request(
        &self,
        path: &str,
    ) -> Result<reqwest::Response, GatewayError> {
        let base_url = self.get_base_url()?;
        let url = format!("{}/v1{}", base_url, path);

        let mut request = self.http_client.delete(&url);

        // Add token if provided
        if let Some(ref token) = self.token {
            request = request.header("X-Consul-Token", token);
        }

        request.send().await.map_err(|e| {
            GatewayError::ServiceDiscovery(format!(
                "Failed to make Consul API DELETE request: {}",
                e
            ))
        })
    }

    /// Convert service info to Consul service registration format
    fn service_info_to_consul_format(
        &self,
        service_id: &str,
        service_info: &ServiceInfo,
    ) -> Result<serde_json::Value, GatewayError> {
        let mut checks = Vec::new();

        // Add health checks for each endpoint
        for (i, endpoint) in service_info.endpoints.iter().enumerate() {
            let check = serde_json::json!({
                "id": format!("{}-endpoint-{}", service_id, i),
                "name": format!("{} endpoint health check", service_info.name),
                "http": endpoint.url,
                "interval": "10s",
                "timeout": "5s"
            });
            checks.push(check);
        }

        let service = serde_json::json!({
            "id": service_id,
            "name": service_info.name,
            "tags": vec![self.service_prefix.clone()],
            "port": self.extract_port_from_url(&service_info.endpoints.first().map(|e| &e.url).unwrap_or(&"".to_string())),
            "address": self.extract_host_from_url(&service_info.endpoints.first().map(|e| &e.url).unwrap_or(&"".to_string())),
            "checks": checks,
            "meta": service_info.metadata
        });

        Ok(service)
    }

    /// Extract port from URL
    fn extract_port_from_url(&self, url_str: &str) -> Option<u16> {
        if let Ok(url) = url::Url::parse(url_str) {
            url.port()
        } else {
            None
        }
    }

    /// Extract host from URL
    fn extract_host_from_url(&self, url_str: &str) -> Option<String> {
        if let Ok(url) = url::Url::parse(url_str) {
            url.host_str().map(|h| h.to_string())
        } else {
            None
        }
    }

    /// Convert service info to JSON for storage
    fn service_info_to_json(&self, service_info: &ServiceInfo) -> Result<String, GatewayError> {
        serde_json::to_string(service_info).map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to serialize service info: {}", e))
        })
    }

    /// Convert JSON back to service info
    fn json_to_service_info(&self, json: &str) -> Result<ServiceInfo, GatewayError> {
        serde_json::from_str(json).map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to deserialize service info: {}", e))
        })
    }

    /// Get detailed service information from Consul
    async fn get_service_details(&self, service_name: &str) -> Result<ServiceInfo, GatewayError> {
        let response = self
            .make_consul_request(&format!("/catalog/service/{}", service_name))
            .await?;

        if !response.status().is_success() {
            return Err(GatewayError::ServiceDiscovery(format!(
                "Failed to get service details from Consul: HTTP {}",
                response.status()
            )));
        }

        let service_data: Vec<serde_json::Value> = response.json().await.map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to parse service details: {}", e))
        })?;

        if service_data.is_empty() {
            return Err(GatewayError::ServiceDiscovery(format!(
                "No service data found for {}",
                service_name
            )));
        }

        let service = &service_data[0];

        // Extract service information
        let name = service["ServiceName"]
            .as_str()
            .unwrap_or(service_name)
            .to_string();
        let mut endpoints = Vec::new();

        // Create endpoint from service address and port
        if let (Some(address), Some(port)) = (
            service["ServiceAddress"].as_str(),
            service["ServicePort"].as_u64(),
        ) {
            let url = if port == 80 {
                format!("http://{}", address)
            } else {
                format!("http://{}:{}", address, port)
            };

            let endpoint = ServiceEndpoint {
                url,
                weight: 1,
                health_status: EndpointHealthStatus::Healthy,
                metadata: HashMap::new(),
            };
            endpoints.push(endpoint);
        }

        // Extract metadata
        let mut metadata = HashMap::new();
        if let Some(meta) = service["ServiceMeta"].as_object() {
            for (key, value) in meta {
                if let Some(value_str) = value.as_str() {
                    metadata.insert(key.clone(), value_str.to_string());
                }
            }
        }

        // Determine health status based on service checks
        let health_status = self.get_service_health_status(service_name).await?;

        Ok(ServiceInfo {
            name,
            endpoints,
            health_status,
            metadata,
        })
    }

    /// Get service health status from Consul
    async fn get_service_health_status(
        &self,
        service_name: &str,
    ) -> Result<ServiceHealthStatus, GatewayError> {
        let response = self
            .make_consul_request(&format!("/health/service/{}", service_name))
            .await?;

        if !response.status().is_success() {
            return Ok(ServiceHealthStatus::Unknown);
        }

        let health_data: Vec<serde_json::Value> = response.json().await.map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to parse health data: {}", e))
        })?;

        if health_data.is_empty() {
            return Ok(ServiceHealthStatus::Unknown);
        }

        // Check if any checks are failing
        let mut has_failing = false;
        let mut has_passing = false;

        for check in health_data {
            if let Some(checks) = check["Checks"].as_array() {
                for check_data in checks {
                    if let Some(status) = check_data["Status"].as_str() {
                        match status {
                            "passing" => has_passing = true,
                            "critical" | "warning" => has_failing = true,
                            _ => {}
                        }
                    }
                }
            }
        }

        if has_failing {
            Ok(ServiceHealthStatus::Unhealthy)
        } else if has_passing {
            Ok(ServiceHealthStatus::Healthy)
        } else {
            Ok(ServiceHealthStatus::Unknown)
        }
    }
}

#[async_trait]
impl ServiceDiscoveryBackend for ConsulBackend {
    async fn initialize(&mut self) -> Result<(), GatewayError> {
        info!("Initializing consul service discovery backend");

        // Test connection by making a health check request
        let response = self.make_consul_request("/agent/self").await?;

        if !response.status().is_success() {
            return Err(GatewayError::ServiceDiscovery(format!(
                "Failed to connect to Consul: HTTP {}",
                response.status()
            )));
        }

        info!("Successfully connected to Consul");
        Ok(())
    }

    async fn discover_services(&mut self) -> Result<HashMap<String, ServiceInfo>, GatewayError> {
        debug!(
            "Discovering services from Consul with prefix: {}",
            self.service_prefix
        );

        // Get all services
        let response = self.make_consul_request("/catalog/services").await?;

        if !response.status().is_success() {
            return Err(GatewayError::ServiceDiscovery(format!(
                "Failed to get services from Consul: HTTP {}",
                response.status()
            )));
        }

        let services_data: serde_json::Value = response.json().await.map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to parse Consul response: {}", e))
        })?;

        let mut discovered_services = HashMap::new();

        // Iterate through services and filter by prefix
        if let Some(services) = services_data.as_object() {
            for (service_name, _) in services {
                if service_name.starts_with(&self.service_prefix) {
                    // Get detailed service information
                    match self.get_service_details(service_name).await {
                        Ok(service_info) => {
                            discovered_services.insert(service_name.clone(), service_info);
                        }
                        Err(e) => {
                            warn!("Failed to get details for service {}: {}", service_name, e);
                        }
                    }
                }
            }
        }

        debug!(
            "Discovered {} services from Consul",
            discovered_services.len()
        );
        Ok(discovered_services)
    }

    async fn register_service(
        &mut self,
        service_id: &str,
        service_info: &ServiceInfo,
    ) -> Result<(), GatewayError> {
        debug!("Registering service {} in Consul", service_id);

        // Convert service info to Consul format
        let consul_service = self.service_info_to_consul_format(service_id, service_info)?;
        let body = serde_json::to_string(&consul_service).map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to serialize service for Consul: {}", e))
        })?;

        // Register the service
        let response = self
            .make_consul_post_request("/agent/service/register", Some(body))
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(GatewayError::ServiceDiscovery(format!(
                "Failed to register service in Consul: HTTP {} - {}",
                status, error_text
            )));
        }

        info!("Successfully registered service {} in Consul", service_id);
        Ok(())
    }

    async fn deregister_service(&mut self, service_id: &str) -> Result<(), GatewayError> {
        debug!("Deregistering service {} from Consul", service_id);

        // Deregister the service
        let response = self
            .make_consul_delete_request(&format!("/agent/service/deregister/{}", service_id))
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(GatewayError::ServiceDiscovery(format!(
                "Failed to deregister service from Consul: HTTP {} - {}",
                status, error_text
            )));
        }

        info!(
            "Successfully deregistered service {} from Consul",
            service_id
        );
        Ok(())
    }

    async fn health_check(&mut self) -> Result<bool, GatewayError> {
        debug!("Performing health check on Consul backend");

        // Check Consul agent health
        let response = self.make_consul_request("/agent/self").await?;

        if response.status().is_success() {
            debug!("Consul health check passed");
            Ok(true)
        } else {
            debug!("Consul health check failed: HTTP {}", response.status());
            Ok(false)
        }
    }
}
