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
use std::str::FromStr;
use hyper_tls::HttpsConnector;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use chrono;

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

        // Start timing the entire request
        let total_start_time = std::time::Instant::now();

        // Time the request transformation phase
        let transform_start = std::time::Instant::now();
        let transformed_request = self.transform_request(request, endpoint).await?;
        let transform_time = transform_start.elapsed();

        // Time the backend forwarding phase
        let forward_start = std::time::Instant::now();
        let response = self.forward_request(transformed_request).await?;
        let forward_time = forward_start.elapsed();

        // Calculate total response time
        let total_response_time = total_start_time.elapsed();

        // Transform response with timing information
        let transformed_response = self.transform_response(response, endpoint, total_response_time).await?;

        // Log detailed timing breakdown
        debug!(
            "Request proxied successfully - Total: {:?}, Transform: {:?}, Backend: {:?}",
            total_response_time, transform_time, forward_time
        );

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
        endpoint: &ServiceEndpoint,
        response_time: Duration,
    ) -> Result<Response<Body>, GatewayError> {
        // Add gateway headers
        response
            .headers_mut()
            .insert("X-Gateway-Proxy", HeaderValue::from_static("orasi-gateway"));

        // Add response time header with actual calculation
        let response_time_ms = response_time.as_millis();
        let response_time_header = format!("{}ms", response_time_ms);
        response
            .headers_mut()
            .insert("X-Response-Time", HeaderValue::from_str(&response_time_header)
                .map_err(|e| GatewayError::Proxy(format!("Failed to create response time header: {}", e)))?);

        // Add detailed timing information
        let response_time_micros = response_time.as_micros();
        response
            .headers_mut()
            .insert("X-Response-Time-Micros", HeaderValue::from_str(&response_time_micros.to_string())
                .map_err(|e| GatewayError::Proxy(format!("Failed to create microsecond header: {}", e)))?);

        // Add response time in seconds for easier parsing
        let response_time_secs = response_time.as_secs_f64();
        response
            .headers_mut()
            .insert("X-Response-Time-Seconds", HeaderValue::from_str(&format!("{:.6}", response_time_secs))
                .map_err(|e| GatewayError::Proxy(format!("Failed to create seconds header: {}", e)))?);

        // Add endpoint information for debugging
        response
            .headers_mut()
            .insert("X-Gateway-Endpoint", HeaderValue::from_str(&endpoint.url)
                .map_err(|e| GatewayError::Proxy(format!("Failed to create endpoint header: {}", e)))?);

        // Add gateway timestamp
        let timestamp = chrono::Utc::now().to_rfc3339();
        response
            .headers_mut()
            .insert("X-Gateway-Timestamp", HeaderValue::from_str(&timestamp)
                .map_err(|e| GatewayError::Proxy(format!("Failed to create timestamp header: {}", e)))?);

        // Log performance metrics
        if response_time > Duration::from_secs(1) {
            warn!("Slow response time for {}: {:?}", endpoint.url, response_time);
        } else if response_time > Duration::from_millis(500) {
            info!("Moderate response time for {}: {:?}", endpoint.url, response_time);
        } else {
            debug!("Fast response time for {}: {:?}", endpoint.url, response_time);
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Uri};
    use std::time::Duration;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_response_time_calculation() {
        // Create a mock config and state
        let config = GatewayConfig::default();
        let state = Arc::new(RwLock::new(GatewayState::new(&config)));
        
        // Create proxy instance
        let proxy = Proxy::new(&config, state).await.unwrap();
        
        // Create a mock endpoint
        let endpoint = ServiceEndpoint {
            url: "http://localhost:8080".to_string(),
            weight: 1,
            health_status: crate::types::EndpointHealthStatus::Healthy,
            metadata: HashMap::new(),
        };
        
        // Create a mock request
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://localhost:8080/test")
            .body(Body::empty())
            .unwrap();
        
        // Test that the proxy_request function includes timing
        // Note: This will fail due to network issues, but we can verify the timing logic
        let result = proxy.proxy_request(request, &endpoint).await;
        
        // The request should fail due to network, but we can verify the timing headers
        // would be added if the request succeeded
        assert!(result.is_err()); // Expected to fail due to network
        
        // Test the transform_response function directly
        let mock_response = Response::builder()
            .status(200)
            .body(Body::empty())
            .unwrap();
        
        let response_time = Duration::from_millis(150);
        let transformed_response = proxy.transform_response(mock_response, &endpoint, response_time).await;
        
        assert!(transformed_response.is_ok());
        
        let response = transformed_response.unwrap();
        let headers = response.headers();
        
        // Verify response time headers are present
        assert!(headers.contains_key("X-Response-Time"));
        assert!(headers.contains_key("X-Response-Time-Micros"));
        assert!(headers.contains_key("X-Response-Time-Seconds"));
        assert!(headers.contains_key("X-Gateway-Proxy"));
        assert!(headers.contains_key("X-Gateway-Endpoint"));
        assert!(headers.contains_key("X-Gateway-Timestamp"));
        
        // Verify response time values
        let response_time_header = headers.get("X-Response-Time").unwrap();
        assert_eq!(response_time_header.to_str().unwrap(), "150ms");
        
        let response_time_micros = headers.get("X-Response-Time-Micros").unwrap();
        assert_eq!(response_time_micros.to_str().unwrap(), "150000");
        
        let response_time_secs = headers.get("X-Response-Time-Seconds").unwrap();
        assert_eq!(response_time_secs.to_str().unwrap(), "0.150000");
    }
    
    #[test]
    fn test_response_time_formatting() {
        // Test various response time durations
        let test_cases = vec![
            (Duration::from_millis(1), "1ms", "1000", "0.001000"),
            (Duration::from_millis(100), "100ms", "100000", "0.100000"),
            (Duration::from_millis(1500), "1500ms", "1500000", "1.500000"),
            (Duration::from_micros(500), "0ms", "500", "0.000500"),
        ];
        
        for (duration, expected_ms, expected_micros, expected_secs) in test_cases {
            let ms = duration.as_millis();
            let micros = duration.as_micros();
            let secs = duration.as_secs_f64();
            
            assert_eq!(format!("{}ms", ms), expected_ms);
            assert_eq!(format!("{}", micros), expected_micros);
            assert_eq!(format!("{:.6}", secs), expected_secs);
        }
    }
}
