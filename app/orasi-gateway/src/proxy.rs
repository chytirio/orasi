//! Proxy

use crate::{config::GatewayConfig, error::GatewayError, types::*};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Proxy for gateway
pub struct Proxy {
    config: GatewayConfig,
    state: Arc<RwLock<super::gateway::GatewayState>>,
}

impl Proxy {
    /// Create new proxy
    pub async fn new(
        config: &GatewayConfig,
        state: Arc<RwLock<super::gateway::GatewayState>>,
    ) -> Result<Self, GatewayError> {
        Ok(Self {
            config: config.clone(),
            state,
        })
    }

    /// Start proxy
    pub async fn start(&self) -> Result<(), GatewayError> {
        // TODO: Implement proxy startup
        Ok(())
    }

    /// Stop proxy
    pub async fn stop(&self) -> Result<(), GatewayError> {
        // TODO: Implement proxy shutdown
        Ok(())
    }

    /// Forward request
    pub async fn forward_request(&self, _request: RequestContext, _endpoint: ServiceEndpoint) -> Result<ResponseContext, GatewayError> {
        // TODO: Implement request forwarding
        Ok(ResponseContext {
            status_code: 200,
            headers: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        })
    }
}
