//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Middleware for the Schema Registry API
//!
//! This module provides middleware for authentication, rate limiting, response
//! caching, and other cross-cutting concerns.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::{self, Body},
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use bytes::Bytes;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::error::ApiError;
use super::versioning::{ApiVersion, ApiVersionConfig};
use crate::registry::SchemaRegistryManager;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Whether to include rate limit headers
    pub include_headers: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_seconds: 60,
            include_headers: true,
        }
    }
}

/// Rate limiter state
#[derive(Debug)]
pub struct RateLimiter {
    requests: HashMap<String, Vec<Instant>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            requests: HashMap::new(),
            config,
        }
    }

    fn is_allowed(&mut self, key: &str) -> bool {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(self.config.window_seconds);

        // Get or create request history for this key
        let requests = self
            .requests
            .entry(key.to_string())
            .or_insert_with(Vec::new);

        // Remove old requests outside the window
        requests.retain(|&time| time > window_start);

        // Check if we're under the limit
        if requests.len() < self.config.max_requests as usize {
            requests.push(now);
            true
        } else {
            false
        }
    }

    fn get_remaining(&self, key: &str) -> u32 {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(self.config.window_seconds);

        if let Some(requests) = self.requests.get(key) {
            let recent_requests = requests.iter().filter(|&&time| time > window_start).count();
            self.config
                .max_requests
                .saturating_sub(recent_requests as u32)
        } else {
            self.config.max_requests
        }
    }

    fn get_reset_time(&self) -> u64 {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(self.config.window_seconds);
        let reset_time = window_start + Duration::from_secs(self.config.window_seconds);
        reset_time.duration_since(now).as_secs()
    }
}

/// Shared rate limiter state
pub type SharedRateLimiter = Arc<RwLock<RateLimiter>>;

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// API key header name
    pub api_key_header: String,
    /// Valid API keys (in production, this would be stored securely)
    pub valid_api_keys: Vec<String>,
    /// Whether authentication is required
    pub required: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_key_header: "X-API-Key".to_string(),
            valid_api_keys: vec!["test-api-key".to_string()],
            required: false,
        }
    }
}

/// Extract client identifier from request
pub fn extract_client_id(headers: &HeaderMap) -> String {
    // Try to get from X-Forwarded-For header first
    if let Some(forwarded_for) = headers.get("X-Forwarded-For") {
        if let Ok(ip) = forwarded_for.to_str() {
            return ip.split(',').next().unwrap_or("unknown").trim().to_string();
        }
    }

    // Fall back to X-Real-IP
    if let Some(real_ip) = headers.get("X-Real-IP") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.to_string();
        }
    }

    // Default to unknown
    "unknown".to_string()
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State((_, rate_limiter, _, _)): State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let client_id = extract_client_id(&headers);
    let mut limiter = rate_limiter.write().await;

    if !limiter.is_allowed(&client_id) {
        let mut response = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body(Body::from(
                serde_json::json!({
                    "error": "Rate limit exceeded",
                    "message": "Too many requests, please try again later"
                })
                .to_string(),
            ))
            .unwrap();

        // Add rate limit headers if configured
        if limiter.config.include_headers {
            let headers = response.headers_mut();
            headers.insert(
                "X-RateLimit-Limit",
                limiter.config.max_requests.to_string().parse().unwrap(),
            );
            headers.insert("X-RateLimit-Remaining", "0".parse().unwrap());
            headers.insert(
                "X-RateLimit-Reset",
                limiter.get_reset_time().to_string().parse().unwrap(),
            );
        }

        return Ok(response);
    }

    // Add rate limit headers to successful responses
    let mut response = next.run(request).await;

    if limiter.config.include_headers {
        let headers = response.headers_mut();
        headers.insert(
            "X-RateLimit-Limit",
            limiter.config.max_requests.to_string().parse().unwrap(),
        );
        headers.insert(
            "X-RateLimit-Remaining",
            limiter
                .get_remaining(&client_id)
                .to_string()
                .parse()
                .unwrap(),
        );
        headers.insert(
            "X-RateLimit-Reset",
            limiter.get_reset_time().to_string().parse().unwrap(),
        );
    }

    Ok(response)
}

/// Authentication middleware
pub async fn auth_middleware(
    State((_, _, auth_config, _)): State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Skip authentication if not required
    if !auth_config.required {
        return Ok(next.run(request).await);
    }

    // Extract API key from headers
    let api_key = headers
        .get(&auth_config.api_key_header)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            ApiError::unauthorized(&format!(
                "Missing API key header: {}",
                auth_config.api_key_header
            ))
        })?;

    // Validate API key
    if !auth_config.valid_api_keys.contains(&api_key.to_string()) {
        return Err(ApiError::unauthorized("Invalid API key"));
    }

    Ok(next.run(request).await)
}

/// Request ID middleware
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    // Generate request ID
    let request_id = Uuid::new_v4().to_string();

    // Add request ID to request extensions
    request.extensions_mut().insert(request_id.clone());

    let mut response = next.run(request).await;

    // Add request ID to response headers
    response
        .headers_mut()
        .insert("X-Request-ID", request_id.parse().unwrap());

    response
}

/// CORS middleware
pub async fn cors_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    // Add CORS headers
    let headers = response.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap(),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization, X-API-Key".parse().unwrap(),
    );
    headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());

    response
}

/// Security headers middleware
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    // Add security headers
    let headers = response.headers_mut();
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );

    response
}

// -------------------------------
// Response caching
// -------------------------------

/// Response cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 30,
            max_entries: 1024,
        }
    }
}

#[derive(Clone, Debug)]
struct CachedResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
    expires_at: Instant,
}

#[derive(Debug)]
pub struct ResponseCache {
    map: HashMap<String, CachedResponse>,
    config: CacheConfig,
}

impl ResponseCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            map: HashMap::new(),
            config,
        }
    }

    fn get(&self, key: &str) -> Option<CachedResponse> {
        if !self.config.enabled {
            return None;
        }
        self.map.get(key).and_then(|c| {
            if Instant::now() < c.expires_at {
                Some(c.clone())
            } else {
                None
            }
        })
    }

    pub async fn insert(&mut self, key: String, response: Response) {
        if !self.config.enabled {
            return;
        }

        // Evict a random entry if we're over capacity
        if self.map.len() >= self.config.max_entries {
            if let Some(first_key) = self.map.keys().next().cloned() {
                self.map.remove(&first_key);
            }
        }

        let expires_at = Instant::now() + Duration::from_secs(self.config.ttl_seconds);

        let (parts, body) = response.into_parts();
        let body_bytes = body::to_bytes(body, usize::MAX)
            .await
            .unwrap_or_else(|_| Bytes::new());
        let cached = CachedResponse {
            status: parts.status,
            headers: parts.headers.clone(),
            body: body_bytes,
            expires_at,
        };
        self.map.insert(key, cached);
    }
}

pub type SharedResponseCache = Arc<RwLock<ResponseCache>>;

/// Response caching middleware (GET requests only)
pub async fn response_cache_middleware(
    State((_, _, _, cache)): State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if request.method() != axum::http::Method::GET {
        return Ok(next.run(request).await);
    }

    let key = request.uri().to_string();
    {
        let cache_guard = cache.read().await;
        if let Some(cached) = cache_guard.get(&key) {
            let mut response = Response::builder()
                .status(cached.status)
                .body(Body::from(cached.body.clone()))
                .unwrap();
            *response.headers_mut() = cached.headers.clone();
            response
                .headers_mut()
                .insert("X-Cache", "HIT".parse().unwrap());
            return Ok(response);
        }
    }

    // Not in cache; proceed and then cache
    let response = next.run(request).await;
    {
        let mut cache_guard = cache.write().await;
        cache_guard.insert(key.clone(), response).await;
    }

    // Return from cache for consistent code path
    let cache_guard = cache.read().await;
    if let Some(cached) = cache_guard.get(&key) {
        let mut response = Response::builder()
            .status(cached.status)
            .body(Body::from(cached.body.clone()))
            .unwrap();
        *response.headers_mut() = cached.headers.clone();
        response
            .headers_mut()
            .insert("X-Cache", "MISS".parse().unwrap());
        Ok(response)
    } else {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(""))
            .unwrap())
    }
}

/// Version negotiation middleware
pub async fn version_negotiation_middleware(
    State((registry, _, _, _)): axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let config = registry.get_config();
    let versioning_config = &config.api.versioning;

    // Extract requested version from headers or path
    let requested_version = extract_requested_version(&headers, &request);

    // Check if version is supported
    if let Some(version) = requested_version {
        let supported_versions: Vec<String> = versioning_config
            .supported_versions
            .iter()
            .map(|v| v.to_string())
            .collect();

        if !supported_versions.contains(&version.to_string()) {
            return Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "error": "Unsupported API version",
                        "message": format!("API version {} is not supported", version),
                        "supported_versions": supported_versions,
                        "default_version": versioning_config.default_version
                    }))
                    .unwrap(),
                ))
                .unwrap();
        }

        // Check if version is deprecated
        let deprecated_versions: HashMap<String, String> = versioning_config
            .deprecated_versions
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();

        if let Some(deprecation_warning) = deprecated_versions.get(&version.to_string()) {
            // Add deprecation warning header
            let mut response = next.run(request).await;
            response.headers_mut().insert(
                "X-API-Deprecation-Warning",
                deprecation_warning.parse().unwrap(),
            );
            return response;
        }
    }

    next.run(request).await
}

/// Extract requested API version from headers or path
fn extract_requested_version(headers: &HeaderMap, request: &Request) -> Option<ApiVersion> {
    // Try to extract from Accept header
    if let Some(accept_header) = headers.get("Accept") {
        if let Ok(accept_str) = accept_header.to_str() {
            if let Some(version) = extract_version_from_accept_header(accept_str) {
                return Some(version);
            }
        }
    }

    // Try to extract from custom API version header
    if let Some(version_header) = headers.get("X-API-Version") {
        if let Ok(version_str) = version_header.to_str() {
            if let Ok(version) = version_str.parse::<ApiVersion>() {
                return Some(version);
            }
        }
    }

    // Try to extract from path
    if let Some(uri) = request.uri().path().split('/').nth(2) {
        if let Ok(version) = format!("{}.0.0", uri.trim_start_matches('v')).parse::<ApiVersion>() {
            return Some(version);
        }
    }

    None
}

/// Extract API version from Accept header
fn extract_version_from_accept_header(accept: &str) -> Option<ApiVersion> {
    // Parse Accept header for version information
    // Example: "application/json; version=2.0"
    for part in accept.split(',') {
        let part = part.trim();
        if part.contains("version=") {
            if let Some(version_str) = part.split("version=").nth(1) {
                let version_str = version_str.split(';').next().unwrap_or(version_str);
                if let Ok(version) = format!("{}.0.0", version_str).parse::<ApiVersion>() {
                    return Some(version);
                }
            }
        }
    }
    None
}

/// Version compatibility middleware
pub async fn version_compatibility_middleware(
    State((registry, _, _, _)): axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let config = registry.get_config();
    let versioning_config = &config.api.versioning;

    // Extract requested version
    if let Some(requested_version) = extract_requested_version(&headers, &request) {
        // Check compatibility with default version
        let default_version = versioning_config
            .default_version
            .parse::<ApiVersion>()
            .unwrap_or_else(|_| ApiVersion::v2_0_0());

        if !requested_version.is_compatible_with(&default_version) {
            // Add compatibility warning header
            let mut response = next.run(request).await;
            response.headers_mut().insert(
                "X-API-Compatibility-Warning",
                format!(
                    "API version {} may not be fully compatible with the latest version",
                    requested_version
                )
                .parse()
                .unwrap(),
            );
            return response;
        }
    }

    next.run(request).await
}

/// Derive resource and action from request
fn derive_permission_from_request(req: &Request) -> (String, String) {
    let path = req.uri().path();
    let method = req.method().as_str().to_lowercase();

    // Normalize resource from path segments after base prefix, using first segment as resource
    // e.g., /api/v1/schemas/... -> resource "schemas"
    let resource = path
        .split('/')
        .filter(|s| !s.is_empty())
        .nth(2) // skip base (api) and version (v1)
        .unwrap_or("root")
        .to_string();

    let action = match method.as_str() {
        "get" => "read",
        "head" => "read",
        "options" => "read",
        "post" => {
            if path.ends_with("/validate") || path.contains("/validate") {
                "validate"
            } else {
                "write"
            }
        }
        "put" => "write",
        "patch" => "write",
        "delete" => "delete",
        _ => "read",
    }
    .to_string();

    (resource, action)
}

/// Authorization middleware (API key based permissions)
pub async fn authorization_middleware(
    State((registry, _, auth_config, _)): axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let config = registry.get_config();
    if !config.security.enable_authorization {
        return Ok(next.run(request).await);
    }

    // Extract API key
    let header_name = config.security.api_key_header.clone();
    let api_key = headers
        .get(&header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::unauthorized("Missing API key"))?;

    // Quick allow if no permissions configured for the key and auth is disabled
    let permissions_map = &config.security.api_key_permissions;
    let key_permissions = permissions_map.get(&api_key).cloned().unwrap_or_default();

    let (resource, action) = derive_permission_from_request(&request);
    let required = format!("{}:{}", resource, action);

    // Allow read of /health and /metrics by default
    if resource == "health" || resource == "metrics" || resource == "docs" {
        return Ok(next.run(request).await);
    }

    if key_permissions
        .iter()
        .any(|p| p == &required || p == "*:*" || *p == format!("{}:*", resource))
    {
        Ok(next.run(request).await)
    } else {
        Err(ApiError::bad_request(&format!(
            "Insufficient permissions: missing {}",
            required
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_client_id() {
        let mut headers = HeaderMap::new();

        // Test X-Forwarded-For
        headers.insert(
            "X-Forwarded-For",
            HeaderValue::from_static("192.168.1.1, 10.0.0.1"),
        );
        assert_eq!(extract_client_id(&headers), "192.168.1.1");

        // Test X-Real-IP
        headers.remove("X-Forwarded-For");
        headers.insert("X-Real-IP", HeaderValue::from_static("192.168.1.2"));
        assert_eq!(extract_client_id(&headers), "192.168.1.2");

        // Test fallback
        headers.clear();
        assert_eq!(extract_client_id(&headers), "unknown");
    }

    #[test]
    fn test_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_seconds: 1,
            include_headers: false,
        };

        let mut limiter = RateLimiter::new(config);

        // First two requests should be allowed
        assert!(limiter.is_allowed("client1"));
        assert!(limiter.is_allowed("client1"));

        // Third request should be blocked
        assert!(!limiter.is_allowed("client1"));

        // Different client should be allowed
        assert!(limiter.is_allowed("client2"));
    }

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert_eq!(config.api_key_header, "X-API-Key");
        assert!(!config.required);
        assert!(config.valid_api_keys.contains(&"test-api-key".to_string()));
    }
}
