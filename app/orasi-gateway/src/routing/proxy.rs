//! HTTP proxy implementation

use crate::{
    config::GatewayConfig,
    error::GatewayError,
    gateway::state::GatewayState,
    types::{RequestContext, ResponseContext, ServiceEndpoint},
};
use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, HeaderValue, Request, Response, StatusCode},
    response::IntoResponse,
};
use hyper_tls::HttpsConnector;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// HTTP proxy for gateway
pub struct Proxy {
    config: GatewayConfig,
    state: Arc<RwLock<GatewayState>>,
    http_client: Client,
}

impl Proxy {
    /// Create new proxy
    pub async fn new(
        config: &GatewayConfig,
        state: Arc<RwLock<GatewayState>>,
    ) -> Result<Self, GatewayError> {
        // Create HTTP client with TLS support
        let http_client = Client::builder()
            .timeout(config.routing.request_timeout)
            .build()
            .expect("Failed to create HTTP client");

        Ok(Self {
            config: config.clone(),
            state,
            http_client,
        })
    }

    /// Start proxy
    pub async fn start(&self) -> Result<(), GatewayError> {
        info!("Starting HTTP proxy");
        info!("HTTP proxy started successfully");
        Ok(())
    }

    /// Stop proxy
    pub async fn stop(&self) -> Result<(), GatewayError> {
        info!("Stopping HTTP proxy");
        info!("HTTP proxy stopped");
        Ok(())
    }

    /// Proxy request to backend service
    pub async fn proxy_request(
        &self,
        request: Request<Body>,
        endpoint: &ServiceEndpoint,
    ) -> Result<Response<Body>, GatewayError> {
        debug!("Proxying request to: {}", endpoint.url);

        // Transform request
        let transformed_request = self.transform_request(request, endpoint).await?;

        // Forward request to backend
        let response = self.forward_request(transformed_request).await?;

        // Transform response
        let transformed_response = self.transform_response(response, endpoint).await?;

        debug!("Request proxied successfully");
        Ok(transformed_response)
    }

    /// Transform request for backend
    async fn transform_request(
        &self,
        request: Request<Body>,
        endpoint: &ServiceEndpoint,
    ) -> Result<Request<Body>, GatewayError> {
        // Build new URI
        let uri = self.build_backend_uri(request.uri(), endpoint).await?;

        // Extract headers before consuming the request
        let headers = request.headers().clone();

        // Create new request
        let mut transformed_request = Request::builder()
            .method(request.method().clone())
            .uri(uri)
            .body(request.into_body())
            .map_err(|e| GatewayError::Proxy(format!("Failed to build request: {}", e)))?;

        // Copy and transform headers
        self.transform_headers(&headers, transformed_request.headers_mut(), endpoint)
            .await?;

        Ok(transformed_request)
    }

    /// Transform response from backend
    async fn transform_response(
        &self,
        mut response: Response<Body>,
        _endpoint: &ServiceEndpoint,
    ) -> Result<Response<Body>, GatewayError> {
        // Add gateway headers
        response
            .headers_mut()
            .insert("X-Gateway-Proxy", HeaderValue::from_static("orasi-gateway"));

        // Add response time header
        // TODO: Add actual response time calculation
        response
            .headers_mut()
            .insert("X-Response-Time", HeaderValue::from_static("0ms"));

        Ok(response)
    }

    /// Forward request to backend
    async fn forward_request(
        &self,
        request: Request<Body>,
    ) -> Result<Response<Body>, GatewayError> {
        // Convert axum request to reqwest request
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();
        let body = request.into_body();

        // Convert body to bytes
        let body_bytes = to_bytes(body, 1024 * 1024) // 1MB limit
            .await
            .map_err(|e| GatewayError::Proxy(format!("Failed to read request body: {}", e)))?;

        // Build reqwest request
        let mut reqwest_request = self
            .http_client
            .request(method, uri.to_string())
            .headers(headers);

        // Add body if not empty
        if !body_bytes.is_empty() {
            reqwest_request = reqwest_request.body(body_bytes);
        }

        // Execute request
        match reqwest_request.send().await {
            Ok(response) => {
                // Convert reqwest response to axum response
                let status = response.status();
                let headers = response.headers().clone();
                let body_bytes = response.bytes().await.map_err(|e| {
                    GatewayError::Proxy(format!("Failed to read response body: {}", e))
                })?;

                let mut axum_response = Response::builder()
                    .status(status)
                    .body(Body::from(body_bytes))
                    .map_err(|e| GatewayError::Proxy(format!("Failed to build response: {}", e)))?;

                // Copy headers
                for (name, value) in headers {
                    if let Some(name) = name {
                        axum_response.headers_mut().insert(name, value);
                    }
                }

                Ok(axum_response)
            }
            Err(e) => {
                error!("Failed to forward request: {}", e);
                Err(GatewayError::Proxy(format!(
                    "Failed to forward request: {}",
                    e
                )))
            }
        }
    }

    /// Build backend URI
    async fn build_backend_uri(
        &self,
        original_uri: &axum::http::Uri,
        endpoint: &ServiceEndpoint,
    ) -> Result<axum::http::Uri, GatewayError> {
        let endpoint_uri = endpoint
            .url
            .parse::<axum::http::Uri>()
            .map_err(|e| GatewayError::Proxy(format!("Invalid endpoint URL: {}", e)))?;

        // Build new URI
        let mut uri_parts = endpoint_uri.into_parts();

        // Set path and query from original request
        uri_parts.path_and_query = original_uri.path_and_query().cloned();

        axum::http::Uri::from_parts(uri_parts)
            .map_err(|e| GatewayError::Proxy(format!("Failed to build URI: {}", e)))
    }

    /// Transform headers
    async fn transform_headers(
        &self,
        source_headers: &HeaderMap,
        target_headers: &mut HeaderMap,
        _endpoint: &ServiceEndpoint,
    ) -> Result<(), GatewayError> {
        // Copy relevant headers
        for (name, value) in source_headers {
            // Skip headers that should not be forwarded
            if self.should_skip_header(name) {
                continue;
            }

            target_headers.insert(name.clone(), value.clone());
        }

        // Add gateway-specific headers
        target_headers.insert("X-Forwarded-For", HeaderValue::from_static("orasi-gateway"));

        target_headers.insert("X-Forwarded-Proto", HeaderValue::from_static("http"));

        Ok(())
    }

    /// Check if header should be skipped
    fn should_skip_header(&self, name: &axum::http::HeaderName) -> bool {
        let skip_headers = ["host", "connection", "content-length", "transfer-encoding"];

        skip_headers.contains(&name.as_str().to_lowercase().as_str())
    }

    /// Create error response
    pub fn create_error_response(status: StatusCode, message: &str) -> Response<Body> {
        let body = serde_json::json!({
            "error": message,
            "status": status.as_u16(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Internal Server Error"))
                    .unwrap()
            })
    }

    /// Create timeout response
    pub fn create_timeout_response() -> Response<Body> {
        Self::create_error_response(StatusCode::GATEWAY_TIMEOUT, "Request timeout")
    }

    /// Create service unavailable response
    pub fn create_service_unavailable_response() -> Response<Body> {
        Self::create_error_response(StatusCode::SERVICE_UNAVAILABLE, "Service unavailable")
    }

    /// Create rate limit exceeded response
    pub fn create_rate_limit_response() -> Response<Body> {
        Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Content-Type", "application/json")
            .header("Retry-After", "60")
            .body(Body::from(
                serde_json::json!({
                    "error": "Rate limit exceeded",
                    "status": 429,
                    "retry_after": 60,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })
                .to_string(),
            ))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(Body::from("Rate limit exceeded"))
                    .unwrap()
            })
    }
}
