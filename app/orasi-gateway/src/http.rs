//! HTTP server for Orasi Gateway

use crate::{
    config::GatewayConfig,
    error::GatewayError,
    gateway::rate_limiter::GatewayRateLimiter as RateLimiter,
    gateway::{GatewayState, OrasiGateway},
    load_balancer::LoadBalancer,
    routing::{proxy::Proxy, Router},
    types::*,
};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, Request, Response, StatusCode},
    middleware,
    response::{IntoResponse, Json},
    routing::{any, delete, get, post, put},
    Router as AxumRouter,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// HTTP server for gateway
pub struct HttpServer {
    config: GatewayConfig,
    gateway: Arc<RwLock<OrasiGateway>>,
    router: Arc<Router>,
    load_balancer: Arc<LoadBalancer>,
    proxy: Arc<Proxy>,
    rate_limiter: Arc<RateLimiter>,
}

impl HttpServer {
    /// Create new HTTP server
    pub fn new(
        config: GatewayConfig,
        gateway: Arc<RwLock<OrasiGateway>>,
        router: Arc<Router>,
        load_balancer: Arc<LoadBalancer>,
        proxy: Arc<Proxy>,
        rate_limiter: Arc<RateLimiter>,
    ) -> Self {
        Self {
            config,
            gateway,
            router,
            load_balancer,
            proxy,
            rate_limiter,
        }
    }

    /// Create router with all endpoints
    pub fn create_router(&self) -> AxumRouter {
        let app = AxumRouter::new()
            // Health and monitoring endpoints
            .route("/health", get(Self::health_check))
            .route("/health/live", get(Self::health_live))
            .route("/health/ready", get(Self::health_ready))
            .route("/metrics", get(Self::metrics))
            .route("/metrics/prometheus", get(Self::prometheus_metrics))
            // Gateway management endpoints
            .route("/gateway/info", get(Self::gateway_info))
            .route("/gateway/status", get(Self::gateway_status))
            .route("/gateway/routes", get(Self::list_routes))
            .route("/gateway/routes", post(Self::add_route))
            .route("/gateway/routes/:path", delete(Self::remove_route))
            // Load balancer endpoints
            .route("/loadbalancer/endpoints", get(Self::list_endpoints))
            .route(
                "/loadbalancer/endpoints/:service",
                get(Self::get_service_endpoints),
            )
            .route("/loadbalancer/health", get(Self::loadbalancer_health))
            // Rate limiter endpoints
            .route("/ratelimiter/stats", get(Self::rate_limiter_stats))
            .route("/ratelimiter/reset", post(Self::reset_rate_limits))
            // Proxy all other requests
            .fallback(Self::proxy_request)
            .with_state(Arc::new(self.clone()));

        // Add middleware
        app.layer(middleware::from_fn_with_state(
            Arc::new(self.clone()),
            Self::logging_middleware,
        ))
    }

    /// Health check endpoint
    async fn health_check(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let gateway = server.gateway.read().await;

        let status = gateway.get_status().await;
        let response = json!({
            "status": status,
            "gateway_id": server.config.gateway_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        (StatusCode::OK, Json(response))
    }

    /// Liveness probe endpoint
    async fn health_live(State(_server): State<Arc<Self>>) -> impl IntoResponse {
        let response = json!({
            "status": "alive",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        (StatusCode::OK, Json(response))
    }

    /// Readiness probe endpoint
    async fn health_ready(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let gateway = server.gateway.read().await;
        let status = gateway.get_status().await;

        let is_ready = matches!(status, GatewayStatus::Running);
        let status_code = if is_ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };

        let response = json!({
            "status": if is_ready { "ready" } else { "not_ready" },
            "gateway_status": status,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        (status_code, Json(response))
    }

    /// Metrics endpoint (JSON format)
    async fn metrics(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let gateway = server.gateway.read().await;

        let metrics = gateway.get_metrics().await;
        let response = json!({
            "metrics": metrics,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        (StatusCode::OK, Json(response))
    }

    /// Prometheus metrics endpoint
    async fn prometheus_metrics(State(_server): State<Arc<Self>>) -> impl IntoResponse {
        // TODO: Implement Prometheus metrics export
        let metrics = "# HELP orasi_gateway_up Gateway is running\n";
        let metrics = format!("{}# TYPE orasi_gateway_up gauge\n", metrics);
        let metrics = format!("{}orasi_gateway_up 1\n", metrics);

        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            "text/plain; version=0.0.4; charset=utf-8".parse().unwrap(),
        );

        (StatusCode::OK, headers, metrics)
    }

    /// Gateway information endpoint
    async fn gateway_info(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let gateway = server.gateway.read().await;

        let info = gateway.get_gateway_info().await;
        let response = json!({
            "gateway_info": info,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        (StatusCode::OK, Json(response))
    }

    /// Gateway status endpoint
    async fn gateway_status(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let gateway = server.gateway.read().await;
        let status = gateway.get_status().await;

        let response = json!({
            "status": status,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        (StatusCode::OK, Json(response))
    }

    /// List routes endpoint
    async fn list_routes(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let routes = server.router.get_routes().await;

        let response = json!({
            "routes": routes,
            "total_routes": routes.len(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        (StatusCode::OK, Json(response))
    }

    /// Add route endpoint
    async fn add_route(
        State(server): State<Arc<Self>>,
        Json(route): Json<Route>,
    ) -> impl IntoResponse {
        match server.router.add_route(route.clone()).await {
            Ok(_) => {
                let response = json!({
                    "message": "Route added successfully",
                    "route": route,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                (StatusCode::CREATED, Json(response))
            }
            Err(e) => {
                error!("Failed to add route: {}", e);
                let response = json!({
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                (StatusCode::BAD_REQUEST, Json(response))
            }
        }
    }

    /// Remove route endpoint
    async fn remove_route(
        State(server): State<Arc<Self>>,
        Path(path): Path<String>,
        Query(params): Query<HashMap<String, String>>,
    ) -> impl IntoResponse {
        let default_method = "GET".to_string();
        let method = params.get("method").unwrap_or(&default_method);

        match server.router.remove_route(&path, method).await {
            Ok(_) => {
                let response = json!({
                    "message": "Route removed successfully",
                    "path": path,
                    "method": method,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                (StatusCode::OK, Json(response))
            }
            Err(e) => {
                error!("Failed to remove route: {}", e);
                let response = json!({
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                (StatusCode::BAD_REQUEST, Json(response))
            }
        }
    }

    /// List endpoints endpoint
    async fn list_endpoints(State(server): State<Arc<Self>>) -> impl IntoResponse {
        // TODO: Implement endpoint listing
        let response = json!({
            "endpoints": [],
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        (StatusCode::OK, Json(response))
    }

    /// Get service endpoints endpoint
    async fn get_service_endpoints(
        State(server): State<Arc<Self>>,
        Path(service_name): Path<String>,
    ) -> impl IntoResponse {
        let endpoints = server.load_balancer.get_endpoints(&service_name).await;

        let response = json!({
            "service": service_name,
            "endpoints": endpoints,
            "total_endpoints": endpoints.len(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        (StatusCode::OK, Json(response))
    }

    /// Load balancer health endpoint
    async fn loadbalancer_health(State(_server): State<Arc<Self>>) -> impl IntoResponse {
        let response = json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        (StatusCode::OK, Json(response))
    }

    /// Rate limiter stats endpoint
    async fn rate_limiter_stats(State(server): State<Arc<Self>>) -> impl IntoResponse {
        let stats = server.rate_limiter.get_stats().await;

        let response = json!({
            "rate_limit_stats": stats,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        (StatusCode::OK, Json(response))
    }

    /// Reset rate limits endpoint
    async fn reset_rate_limits(State(server): State<Arc<Self>>) -> impl IntoResponse {
        match server.rate_limiter.reset_counters().await {
            Ok(_) => {
                let response = json!({
                    "message": "Rate limit counters reset successfully",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                (StatusCode::OK, Json(response))
            }
            Err(e) => {
                error!("Failed to reset rate limits: {}", e);
                let response = json!({
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
    }

    /// Proxy request to backend service
    async fn proxy_request(
        State(server): State<Arc<Self>>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        debug!("Proxying request: {} {}", request.method(), request.uri());

        // Extract client ID (for rate limiting)
        let client_id = Self::extract_client_id(&request);

        // Check rate limit
        if !server
            .rate_limiter
            .check_rate_limit(&client_id, request.uri().path())
            .await
            .unwrap_or(false)
        {
            return Proxy::create_rate_limit_response();
        }

        // Create request context (clone necessary parts first)
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();
        let request_context = Self::create_request_context_from_parts(method, uri, headers).await;

        // Route request
        match server.router.route_request(request_context.clone()).await {
            Ok(route_match) => {
                // Select endpoint
                // Extract service name from route match metadata or use default
                let service_name = route_match
                    .metadata
                    .get("service_name")
                    .cloned()
                    .unwrap_or_else(|| "default".to_string());
                match server
                    .load_balancer
                    .select_endpoint(&service_name, Some(&request_context))
                    .await
                {
                    Ok(endpoint) => {
                        // Proxy request
                        match server.proxy.proxy_request(request, &endpoint).await {
                            Ok(response) => response,
                            Err(e) => {
                                error!("Proxy error: {}", e);
                                Proxy::create_service_unavailable_response()
                            }
                        }
                    }
                    Err(e) => {
                        error!("Load balancer error: {}", e);
                        Proxy::create_service_unavailable_response()
                    }
                }
            }
            Err(e) => {
                error!("Routing error: {}", e);
                Proxy::create_error_response(StatusCode::NOT_FOUND, "Route not found")
            }
        }
    }

    /// Logging middleware
    async fn logging_middleware(
        State(server): State<Arc<Self>>,
        request: Request<Body>,
        next: middleware::Next,
    ) -> Response<Body> {
        let start = std::time::Instant::now();
        let method = request.method().clone();
        let uri = request.uri().clone();

        let response = next.run(request).await;

        let latency = start.elapsed();
        info!(
            "{} {} {} {}ms",
            method,
            uri,
            response.status(),
            latency.as_millis()
        );

        response
    }

    /// Extract client ID from request
    fn extract_client_id(request: &Request<Body>) -> String {
        // Try to extract from headers
        if let Some(client_id) = request.headers().get("X-Client-ID") {
            return client_id.to_str().unwrap_or("unknown").to_string();
        }

        // Try to extract from query parameters
        if let Some(query) = request.uri().query() {
            for param in query.split('&') {
                if param.starts_with("client_id=") {
                    return param.split('=').nth(1).unwrap_or("unknown").to_string();
                }
            }
        }

        // Fallback to IP address
        "unknown".to_string()
    }

    /// Create request context from HTTP request
    async fn create_request_context(request: Request<Body>) -> RequestContext {
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();
        Self::create_request_context_from_parts(method, uri, headers).await
    }

    /// Create request context from request parts
    async fn create_request_context_from_parts(
        method: axum::http::Method,
        uri: axum::http::Uri,
        headers: HeaderMap,
    ) -> RequestContext {
        let mut header_map = HashMap::new();
        for (name, value) in &headers {
            header_map.insert(name.to_string(), value.to_str().unwrap_or("").to_string());
        }

        let mut query_params = HashMap::new();
        if let Some(query) = uri.query() {
            for param in query.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    query_params.insert(key.to_string(), value.to_string());
                }
            }
        }

        // Extract client IP from various headers in order of preference
        let client_ip = Self::extract_client_ip_from_headers(&headers);

        RequestContext {
            request_id: uuid::Uuid::new_v4().to_string(),
            client_ip,
            user_agent: headers
                .get("User-Agent")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            method: method.to_string(),
            path: uri.path().to_string(),
            headers: header_map,
            query_params,
            body: None, // TODO: Extract body if needed
            metadata: HashMap::new(),
        }
    }

    /// Extract client IP from headers
    fn extract_client_ip_from_headers(headers: &HeaderMap) -> String {
        // Try X-Forwarded-For header (most common for proxied requests)
        if let Some(forwarded_for) = headers.get("X-Forwarded-For") {
            if let Ok(forwarded_for_str) = forwarded_for.to_str() {
                // X-Forwarded-For can contain multiple IPs, take the first one
                if let Some(first_ip) = forwarded_for_str.split(',').next() {
                    let trimmed_ip = first_ip.trim();
                    if Self::is_valid_ip(trimmed_ip) {
                        return trimmed_ip.to_string();
                    }
                }
            }
        }

        // Try X-Real-IP header
        if let Some(real_ip) = headers.get("X-Real-IP") {
            if let Ok(real_ip_str) = real_ip.to_str() {
                if Self::is_valid_ip(real_ip_str) {
                    return real_ip_str.to_string();
                }
            }
        }

        // Try X-Client-IP header
        if let Some(client_ip) = headers.get("X-Client-IP") {
            if let Ok(client_ip_str) = client_ip.to_str() {
                if Self::is_valid_ip(client_ip_str) {
                    return client_ip_str.to_string();
                }
            }
        }

        // Try CF-Connecting-IP header (Cloudflare)
        if let Some(cf_ip) = headers.get("CF-Connecting-IP") {
            if let Ok(cf_ip_str) = cf_ip.to_str() {
                if Self::is_valid_ip(cf_ip_str) {
                    return cf_ip_str.to_string();
                }
            }
        }

        // Fallback to localhost
        "127.0.0.1".to_string()
    }

    /// Validate IP address format
    fn is_valid_ip(ip: &str) -> bool {
        // Basic IP validation - check if it looks like an IPv4 or IPv6 address
        if ip.contains('.') {
            // IPv4 validation
            let parts: Vec<&str> = ip.split('.').collect();
            if parts.len() == 4 {
                return parts.iter().all(|part| part.parse::<u8>().is_ok());
            }
        } else if ip.contains(':') {
            // Basic IPv6 validation - just check if it contains colons
            return ip.matches(':').count() >= 2;
        }
        false
    }
}

impl Clone for HttpServer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            gateway: self.gateway.clone(),
            router: self.router.clone(),
            load_balancer: self.load_balancer.clone(),
            proxy: self.proxy.clone(),
            rate_limiter: self.rate_limiter.clone(),
        }
    }
}
