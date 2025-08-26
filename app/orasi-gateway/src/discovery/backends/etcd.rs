//! Etcd service discovery backend implementation

use crate::{
    error::GatewayError,
    types::{ServiceInfo},
};
use async_trait::async_trait;
use etcd_client::{Client as EtcdClient, GetOptions, PutOptions};
use serde_json;
use std::collections::HashMap;
use tracing::{debug, info};
use super::trait_def::ServiceDiscoveryBackend;

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
