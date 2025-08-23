//! Core routing implementation

use crate::{
    config::GatewayConfig,
    error::GatewayError,
    gateway::state::GatewayState,
    routing::matcher::RouteMatch,
    types::{RequestContext, Route, RouteRule, RouteRuleCondition, ServiceEndpoint},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Router for gateway
pub struct Router {
    config: GatewayConfig,
    state: Arc<RwLock<GatewayState>>,
    routes: Arc<RwLock<HashMap<String, Route>>>,
}

impl Router {
    /// Create new router
    pub async fn new(
        config: &GatewayConfig,
        state: Arc<RwLock<GatewayState>>,
    ) -> Result<Self, GatewayError> {
        let routes = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            config: config.clone(),
            state,
            routes,
        })
    }

    /// Start router
    pub async fn start(&self) -> Result<(), GatewayError> {
        info!("Starting router");

        // Load initial routes from configuration
        self.load_routes_from_config().await?;

        info!("Router started successfully");
        Ok(())
    }

    /// Stop router
    pub async fn stop(&self) -> Result<(), GatewayError> {
        info!("Stopping router");

        // Clear routes
        {
            let mut routes = self.routes.write().await;
            routes.clear();
        }

        info!("Router stopped");
        Ok(())
    }

    /// Route request
    pub async fn route_request(&self, request: RequestContext) -> Result<RouteMatch, GatewayError> {
        debug!("Routing request: {} {}", request.method, request.path);

        // Find matching route
        let route = self.find_matching_route(&request).await?;

        if let Some(route) = route {
            // Create route match
            let route_match = RouteMatch {
                path: route.path.clone(),
                method: request.method.clone(),
                parameters: self.extract_parameters(&request.path, &route.path).await,
                metadata: route.metadata.clone(),
            };

            debug!("Route matched: {} -> {}", request.path, route.service_name);
            Ok(route_match)
        } else {
            warn!("No route found for: {} {}", request.method, request.path);
            Err(GatewayError::RouteNotFound(format!(
                "No route found for {} {}",
                request.method, request.path
            )))
        }
    }

    /// Add route
    pub async fn add_route(&self, route: Route) -> Result<(), GatewayError> {
        let route_key = self.generate_route_key(&route.path, &route.method);
        let method = route.method.clone();
        let path = route.path.clone();

        {
            let mut routes = self.routes.write().await;
            routes.insert(route_key, route);
        }

        info!("Added route: {} {}", method, path);
        Ok(())
    }

    /// Remove route
    pub async fn remove_route(&self, path: &str, method: &str) -> Result<(), GatewayError> {
        let route_key = self.generate_route_key(path, method);

        {
            let mut routes = self.routes.write().await;
            routes.remove(&route_key);
        }

        info!("Removed route: {} {}", method, path);
        Ok(())
    }

    /// Get all routes
    pub async fn get_routes(&self) -> Vec<Route> {
        let routes = self.routes.read().await;
        routes.values().cloned().collect()
    }

    /// Find matching route
    async fn find_matching_route(
        &self,
        request: &RequestContext,
    ) -> Result<Option<Route>, GatewayError> {
        let routes = self.routes.read().await;

        // Sort routes by priority (highest first)
        let mut sorted_routes: Vec<_> = routes.values().collect();
        sorted_routes.sort_by(|a, b| b.priority.cmp(&a.priority));

        for route in sorted_routes {
            if self.matches_route(request, route).await? {
                return Ok(Some(route.clone()));
            }
        }

        Ok(None)
    }

    /// Check if request matches route
    async fn matches_route(
        &self,
        request: &RequestContext,
        route: &Route,
    ) -> Result<bool, GatewayError> {
        // Check method
        if !self.matches_method(&request.method, &route.method) {
            return Ok(false);
        }

        // Check path
        if !self.matches_path(&request.path, &route.path) {
            return Ok(false);
        }

        // Check rules
        for rule in &route.rules {
            if !self.matches_rule(request, rule).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check method matching
    fn matches_method(&self, request_method: &str, route_method: &str) -> bool {
        route_method == "*" || route_method.to_uppercase() == request_method.to_uppercase()
    }

    /// Check path matching
    fn matches_path(&self, request_path: &str, route_path: &str) -> bool {
        if route_path == "*" {
            return true;
        }

        // Simple path matching (can be enhanced with regex)
        if route_path.contains("*") {
            // Wildcard matching
            let route_parts: Vec<&str> = route_path.split('/').collect();
            let request_parts: Vec<&str> = request_path.split('/').collect();

            if route_parts.len() > request_parts.len() {
                return false;
            }

            for (i, route_part) in route_parts.iter().enumerate() {
                if *route_part == "*" {
                    continue;
                }
                if i >= request_parts.len() || *route_part != request_parts[i] {
                    return false;
                }
            }
            true
        } else {
            // Exact matching
            route_path == request_path
        }
    }

    /// Check rule matching
    async fn matches_rule(
        &self,
        request: &RequestContext,
        rule: &RouteRuleCondition,
    ) -> Result<bool, GatewayError> {
        match rule {
            RouteRuleCondition::Header { name, value } => {
                let header_value = request.headers.get(name);
                Ok(header_value.map(|v| v == value).unwrap_or(false))
            }
            RouteRuleCondition::QueryParam { name, value } => {
                let param_value = request.query_params.get(name);
                Ok(param_value.map(|v| v == value).unwrap_or(false))
            }
            RouteRuleCondition::PathParam { name, value } => {
                // Extract path parameters and check
                let params = self.extract_parameters(&request.path, &name).await;
                Ok(params.get(name).map(|v| v == value).unwrap_or(false))
            }
        }
    }

    /// Extract path parameters
    async fn extract_parameters(
        &self,
        request_path: &str,
        route_path: &str,
    ) -> HashMap<String, String> {
        let mut params = HashMap::new();

        let route_parts: Vec<&str> = route_path.split('/').collect();
        let request_parts: Vec<&str> = request_path.split('/').collect();

        for (i, route_part) in route_parts.iter().enumerate() {
            if route_part.starts_with(':') && i < request_parts.len() {
                let param_name = &route_part[1..]; // Remove ':'
                params.insert(param_name.to_string(), request_parts[i].to_string());
            }
        }

        params
    }

    /// Generate route key
    fn generate_route_key(&self, path: &str, method: &str) -> String {
        format!("{}:{}", method.to_uppercase(), path)
    }

    /// Load routes from configuration
    async fn load_routes_from_config(&self) -> Result<(), GatewayError> {
        // Load routes from configuration
        for route_config in &self.config.routing.routes {
            // Create a route for each method in the route rule
            for method in &route_config.methods {
                let route = Route {
                    path: route_config.path.clone(),
                    method: method.clone(),
                    service_name: route_config.service.clone(),
                    priority: route_config.priority,
                    rules: Vec::new(), // RouteRule doesn't have nested rules
                    metadata: route_config.metadata.clone(),
                };

                self.add_route(route).await?;
            }
        }

        Ok(())
    }
}
