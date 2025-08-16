//! API server implementation
//!
//! This module contains the API server implementation and router setup.

use crate::registry::SchemaRegistryManager;
use std::sync::Arc;

use super::endpoints::*;

#[cfg(feature = "http")]
use axum::{
    routing::{delete, get, post},
    Router,
};

#[cfg(not(feature = "http"))]
type Router = ();

#[cfg(not(feature = "http"))]
fn get<T>(_handler: T) -> () {
    ()
}

#[cfg(not(feature = "http"))]
fn post<T>(_handler: T) -> () {
    ()
}

#[cfg(not(feature = "http"))]
fn delete<T>(_handler: T) -> () {
    ()
}

/// API server for the schema registry
pub struct SchemaRegistryApi {
    /// Registry manager
    registry: Arc<SchemaRegistryManager>,

    /// Router
    router: Router,
}

impl SchemaRegistryApi {
    /// Create a new API server
    pub fn new(registry: Arc<SchemaRegistryManager>) -> Self {
        let router = Self::create_router(registry.clone());

        Self { registry, router }
    }

    /// Create the router with all endpoints
    fn create_router(registry: Arc<SchemaRegistryManager>) -> Router {
        Router::new()
            .route("/health", get(health_check))
            .route("/schemas", post(register_schema))
            .route("/schemas", get(list_schemas))
            .route("/schemas/:fingerprint", get(get_schema))
            .route("/schemas/:fingerprint", delete(delete_schema))
            .route("/schemas/:fingerprint/validate", post(validate_data))
            .route("/schemas/validate", post(validate_schema))
            .route("/schemas/evolve", post(evolve_schema))
            .route("/metrics", get(get_metrics))
            .route("/stats", get(get_stats))
            .with_state(registry)
    }

    /// Create the Axum app
    pub fn create_app(&self) -> Router {
        self.router.clone()
    }

    /// Get the router
    pub fn router(&self) -> Router {
        self.router.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_creation() {
        let registry = Arc::new(SchemaRegistryManager::default());
        let api = SchemaRegistryApi::new(registry);

        // Test that router was created
        let _router = api.router();
    }
}
