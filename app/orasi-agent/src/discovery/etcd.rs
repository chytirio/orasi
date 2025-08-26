//! etcd client for service discovery

use crate::error::AgentError;
use crate::discovery::types::ServiceRegistration;
use etcd_client::{
    Client, ConnectOptions, DeleteOptions, GetOptions, LeaseGrantOptions, PutOptions,
};
use std::time::Duration as StdDuration;
use tracing::{error, info};

/// etcd client wrapper
pub struct EtcdClient {
    client: Client,
    lease_id: Option<i64>,
}

impl EtcdClient {
    /// Create new etcd client
    pub async fn new(endpoints: Vec<String>) -> Result<Self, AgentError> {
        let connect_options = ConnectOptions::new()
            .with_timeout(StdDuration::from_secs(5))
            .with_keep_alive(StdDuration::from_secs(30), StdDuration::from_secs(10));

        let client = Client::connect(endpoints, Some(connect_options))
            .await
            .map_err(|e| {
                AgentError::ConnectionError(format!("Failed to connect to etcd: {}", e))
            })?;

        Ok(Self {
            client,
            lease_id: None,
        })
    }

    /// Register service with etcd
    pub async fn register_service(
        &mut self,
        registration: &ServiceRegistration,
    ) -> Result<(), AgentError> {
        let key = format!("/orasi/services/{}", registration.service_id);
        let value = serde_json::to_string(registration).map_err(|e| {
            AgentError::Serialization(format!("Failed to serialize registration: {}", e))
        })?;

        // Create lease for TTL
        let ttl = registration.ttl.unwrap_or(30) as i64;
        let lease_grant = self
            .client
            .lease_grant(ttl, Some(LeaseGrantOptions::new()))
            .await
            .map_err(|e| {
                AgentError::RegistrationError(format!("Failed to create etcd lease: {}", e))
            })?;

        let lease_id = lease_grant.id();

        // Put service with lease
        let put_options = PutOptions::new().with_lease(lease_id);
        self.client
            .put(key, value, Some(put_options))
            .await
            .map_err(|e| {
                AgentError::RegistrationError(format!("Failed to register service in etcd: {}", e))
            })?;

        self.lease_id = Some(lease_id);
        info!(
            "Service registered with etcd: {} (lease: {})",
            registration.service_id, lease_id
        );
        Ok(())
    }

    /// Deregister service from etcd
    pub async fn deregister_service(&mut self, service_id: &str) -> Result<(), AgentError> {
        let key = format!("/orasi/services/{}", service_id);

        self.client
            .delete(key, Some(DeleteOptions::new()))
            .await
            .map_err(|e| {
                AgentError::DeregistrationError(format!(
                    "Failed to deregister service from etcd: {}",
                    e
                ))
            })?;

        info!("Service deregistered from etcd: {}", service_id);
        Ok(())
    }

    /// Discover services from etcd
    pub async fn discover_services(&mut self) -> Result<Vec<ServiceRegistration>, AgentError> {
        let prefix = "/orasi/services/";
        let get_options = GetOptions::new().with_prefix();

        let response = self
            .client
            .get(prefix, Some(get_options))
            .await
            .map_err(|e| {
                AgentError::DiscoveryError(format!("Failed to discover services from etcd: {}", e))
            })?;

        let mut services = Vec::new();
        for kv in response.kvs() {
            if let Ok(registration) = serde_json::from_slice::<ServiceRegistration>(kv.value()) {
                services.push(registration);
            }
        }

        info!("Discovered {} services from etcd", services.len());
        Ok(services)
    }

    /// Keep lease alive
    pub async fn keep_alive(&mut self) -> Result<(), AgentError> {
        if let Some(lease_id) = self.lease_id {
            self.client.lease_keep_alive(lease_id).await.map_err(|e| {
                AgentError::ConnectionError(format!("Failed to keep etcd lease alive: {}", e))
            })?;
        }
        Ok(())
    }
}
