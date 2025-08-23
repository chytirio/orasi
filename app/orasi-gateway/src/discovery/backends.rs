//! Service discovery backend implementations

use crate::{
    config::ServiceDiscoveryBackend as ConfigBackend,
    error::GatewayError,
    types::{EndpointHealthStatus, ServiceEndpoint, ServiceHealthStatus, ServiceInfo},
};
use async_trait::async_trait;
use consul::Client as ConsulClient;
use etcd_client::{Client as EtcdClient, GetOptions, PutOptions};
use reqwest::Client as HttpClient;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use url;

/// Trait for service discovery backends
#[async_trait]
pub trait ServiceDiscoveryBackend: Send + Sync {
    /// Initialize the backend
    async fn initialize(&mut self) -> Result<(), GatewayError>;

    /// Discover services
    async fn discover_services(&mut self) -> Result<HashMap<String, ServiceInfo>, GatewayError>;

    /// Register a service
    async fn register_service(
        &mut self,
        service_id: &str,
        service_info: &ServiceInfo,
    ) -> Result<(), GatewayError>;

    /// Deregister a service
    async fn deregister_service(&mut self, service_id: &str) -> Result<(), GatewayError>;

    /// Health check for the backend
    async fn health_check(&mut self) -> Result<bool, GatewayError>;
}

/// Etcd service discovery backend
pub struct EtcdBackend {
    client: Option<EtcdClient>,
    endpoints: Vec<String>,
    prefix: String,
}

impl EtcdBackend {
    pub fn new(endpoints: Vec<String>, prefix: String) -> Self {
        Self {
            client: None,
            endpoints,
            prefix,
        }
    }

    async fn get_client(&mut self) -> Result<&mut EtcdClient, GatewayError> {
        if self.client.is_none() {
            let client = EtcdClient::connect(&self.endpoints, None)
                .await
                .map_err(|e| {
                    GatewayError::ServiceDiscovery(format!("Failed to connect to etcd: {}", e))
                })?;
            self.client = Some(client);
        }
        Ok(self.client.as_mut().unwrap())
    }
}

#[async_trait]
impl ServiceDiscoveryBackend for EtcdBackend {
    async fn initialize(&mut self) -> Result<(), GatewayError> {
        info!("Initializing etcd service discovery backend");
        self.get_client().await?;
        Ok(())
    }

    async fn discover_services(&mut self) -> Result<HashMap<String, ServiceInfo>, GatewayError> {
        let client = self.client.as_mut().ok_or_else(|| {
            GatewayError::ServiceDiscovery("Etcd client not initialized".to_string())
        })?;

        let key = format!("{}/services/", self.prefix);
        let response = client
            .get(key, Some(GetOptions::new().with_prefix()))
            .await
            .map_err(|e| {
                GatewayError::ServiceDiscovery(format!("Failed to get services from etcd: {}", e))
            })?;

        let mut services = HashMap::new();

        for kv in response.kvs() {
            let key = String::from_utf8_lossy(kv.key());
            let value = String::from_utf8_lossy(kv.value());

            if let Ok(service_info) = serde_json::from_str::<ServiceInfo>(&value) {
                let service_id = key.trim_start_matches(&format!("{}/services/", self.prefix));
                services.insert(service_id.to_string(), service_info);
            }
        }

        debug!("Discovered {} services from etcd", services.len());
        Ok(services)
    }

    async fn register_service(
        &mut self,
        service_id: &str,
        service_info: &ServiceInfo,
    ) -> Result<(), GatewayError> {
        let client = self.client.as_mut().ok_or_else(|| {
            GatewayError::ServiceDiscovery("Etcd client not initialized".to_string())
        })?;

        let key = format!("{}/services/{}", self.prefix, service_id);
        let value = serde_json::to_string(service_info).map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to serialize service info: {}", e))
        })?;

        client
            .put(key, value, Some(PutOptions::new()))
            .await
            .map_err(|e| {
                GatewayError::ServiceDiscovery(format!("Failed to register service in etcd: {}", e))
            })?;

        info!("Registered service {} in etcd", service_id);
        Ok(())
    }

    async fn deregister_service(&mut self, service_id: &str) -> Result<(), GatewayError> {
        let client = self.client.as_mut().ok_or_else(|| {
            GatewayError::ServiceDiscovery("Etcd client not initialized".to_string())
        })?;

        let key = format!("{}/services/{}", self.prefix, service_id);
        client.delete(key, None).await.map_err(|e| {
            GatewayError::ServiceDiscovery(format!("Failed to deregister service from etcd: {}", e))
        })?;

        info!("Deregistered service {} from etcd", service_id);
        Ok(())
    }

    async fn health_check(&mut self) -> Result<bool, GatewayError> {
        let client = self.client.as_mut().ok_or_else(|| {
            GatewayError::ServiceDiscovery("Etcd client not initialized".to_string())
        })?;

        // Simple health check by trying to get a non-existent key
        match client.get("health-check", None).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false), // Even if key doesn't exist, connection is working
        }
    }
}

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

impl ConsulBackend {
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

/// Factory for creating service discovery backends
pub struct BackendFactory;

impl BackendFactory {
    pub async fn create_backend(
        backend_type: &ConfigBackend,
        endpoints: &[String],
        prefix: Option<String>,
        consul_config: Option<&crate::config::ConsulConfig>,
    ) -> Result<Box<dyn ServiceDiscoveryBackend>, GatewayError> {
        match backend_type {
            ConfigBackend::Etcd => {
                let etcd_backend = EtcdBackend::new(
                    endpoints.to_vec(),
                    prefix.unwrap_or_else(|| "/orasi".to_string()),
                );
                Ok(Box::new(etcd_backend))
            }
            ConfigBackend::Consul => {
                let default_config = crate::config::ConsulConfig::default();
                let consul_config = consul_config.unwrap_or(&default_config);
                let consul_backend = ConsulBackend::new(
                    consul_config.datacenter.clone(),
                    consul_config.token.clone(),
                    endpoints.to_vec(),
                    consul_config.service_prefix.clone(),
                );
                Ok(Box::new(consul_backend))
            }
            ConfigBackend::Static => {
                let static_backend = StaticBackend::new(HashMap::new());
                Ok(Box::new(static_backend))
            }
        }
    }
}
