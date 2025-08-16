//! Routing

use crate::{config::GatewayConfig, error::GatewayError, types::*};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Router for gateway
pub struct Router {
    config: GatewayConfig,
    state: Arc<RwLock<super::gateway::GatewayState>>,
}

impl Router {
    /// Create new router
    pub async fn new(
        config: &GatewayConfig,
        state: Arc<RwLock<super::gateway::GatewayState>>,
    ) -> Result<Self, GatewayError> {
        Ok(Self {
            config: config.clone(),
            state,
        })
    }

    /// Start router
    pub async fn start(&self) -> Result<(), GatewayError> {
        // TODO: Implement router startup
        Ok(())
    }

    /// Stop router
    pub async fn stop(&self) -> Result<(), GatewayError> {
        // TODO: Implement router shutdown
        Ok(())
    }

    /// Route request
    pub async fn route_request(&self, _request: RequestContext) -> Result<RouteMatch, GatewayError> {
        // TODO: Implement request routing
        Ok(RouteMatch {
            path: "".to_string(),
            method: "".to_string(),
            parameters: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        })
    }
}
