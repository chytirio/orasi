//! Service discovery backend factory implementation

use crate::{
    config::ServiceDiscoveryBackend as ConfigBackend,
    error::GatewayError,
};
use super::trait_def::ServiceDiscoveryBackend;
use super::etcd::EtcdBackend;
use super::consul::ConsulBackend;
use super::static_backend::StaticBackend;
use std::collections::HashMap;

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
