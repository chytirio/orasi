//! API server implementation
//!
//! This module contains the API server implementation and router setup.

use crate::registry::SchemaRegistryManager;
use std::sync::Arc;

use super::endpoints::*;
use super::middleware::{AuthConfig, RateLimitConfig, RateLimiter, SharedRateLimiter};
use super::middleware::{CacheConfig, ResponseCache, SharedResponseCache};
use super::versioning::{ApiVersion, ApiVersionConfig, VersionedEndpoints};

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
#[allow(dead_code)]
pub struct SchemaRegistryApi {
    /// Registry manager
    registry: Arc<SchemaRegistryManager>,
    /// Rate limiter
    rate_limiter: SharedRateLimiter,
    /// Auth config
    auth_config: Arc<AuthConfig>,
    /// Versioned endpoints
    versioned_endpoints: VersionedEndpoints,
    /// API versioning config
    versioning_config: ApiVersionConfig,
    /// Router
    router: Router,
}

impl SchemaRegistryApi {
    /// Create a new API server
    pub fn new(registry: Arc<SchemaRegistryManager>) -> Self {
        let config = registry.get_config();

        // Create rate limiter from config
        let rate_limit_config = RateLimitConfig {
            max_requests: config.security.rate_limit_per_minute,
            window_seconds: 60,
            include_headers: true,
        };
        let rate_limiter = Arc::new(tokio::sync::RwLock::new(RateLimiter::new(
            rate_limit_config,
        )));

        // Create auth config from config
        let auth_config = Arc::new(AuthConfig {
            api_key_header: config.security.api_key_header.clone(),
            valid_api_keys: config.security.allowed_api_keys.clone(),
            required: config.security.enable_auth,
        });

        // Response cache with config from API settings
        let cache_config = CacheConfig {
            enabled: config.api.enable_cache,
            ttl_seconds: config.api.cache_ttl_seconds,
            max_entries: config.api.cache_max_entries,
        };
        let cache = Arc::new(tokio::sync::RwLock::new(ResponseCache::new(cache_config)));

        // Create versioned endpoints
        let versioned_endpoints = VersionedEndpoints::new();

        // Create versioning config from API config
        let versioning_config = ApiVersionConfig {
            supported_versions: config
                .api
                .versioning
                .supported_versions
                .iter()
                .filter_map(|v| v.parse::<ApiVersion>().ok())
                .collect(),
            default_version: config
                .api
                .versioning
                .default_version
                .parse::<ApiVersion>()
                .unwrap_or_else(|_| ApiVersion::v2_0_0()),
            allow_version_negotiation: config.api.versioning.allow_version_negotiation,
            deprecated_versions: config
                .api
                .versioning
                .deprecated_versions
                .iter()
                .filter_map(|(k, v)| k.parse::<ApiVersion>().ok().map(|ver| (ver, v.clone())))
                .collect(),
        };

        // Ensure base path starts with '/'
        let base_path = if config.api.base_path.starts_with('/') {
            config.api.base_path.clone()
        } else {
            format!("/{}", config.api.base_path)
        };

        let router = Self::create_router(
            registry.clone(),
            rate_limiter.clone(),
            auth_config.clone(),
            cache.clone(),
            &base_path,
            &versioned_endpoints,
            &versioning_config,
        );

        Self {
            registry,
            rate_limiter,
            auth_config,
            versioned_endpoints,
            versioning_config,
            router,
        }
    }

    /// Create the router with all endpoints
    fn create_router(
        registry: Arc<SchemaRegistryManager>,
        rate_limiter: SharedRateLimiter,
        auth_config: Arc<AuthConfig>,
        cache: SharedResponseCache,
        base_path: &str,
        versioned_endpoints: &VersionedEndpoints,
        versioning_config: &ApiVersionConfig,
    ) -> Router {
        // Create a combined state that includes all our state types
        let combined_state = (registry, rate_limiter, auth_config, cache);

        // Create versioned routes
        let mut versioned_routes = Router::new();

        // Add routes for each supported version
        for version in &versioning_config.supported_versions {
            if let Some(endpoints) = versioned_endpoints.get_endpoints(version) {
                let version_path = format!("/{}", version.to_path());
                let version_routes = Self::create_version_routes(combined_state.clone(), endpoints);
                versioned_routes = versioned_routes.nest(&version_path, version_routes);
            }
        }

        // Add legacy routes (without version prefix) for backward compatibility
        if versioning_config.allow_version_negotiation {
            let legacy_routes = Self::create_legacy_routes(
                combined_state.clone(),
                versioned_endpoints,
                versioning_config,
            );
            versioned_routes = versioned_routes.merge(legacy_routes);
        }

        // Add common middleware
        let api_routes = versioned_routes
            .layer(axum::middleware::from_fn(
                super::middleware::request_id_middleware,
            ))
            .layer(axum::middleware::from_fn(
                super::middleware::cors_middleware,
            ))
            .layer(axum::middleware::from_fn(
                super::middleware::security_headers_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                combined_state.clone(),
                super::middleware::version_negotiation_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                combined_state.clone(),
                super::middleware::version_compatibility_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                combined_state.clone(),
                super::middleware::rate_limit_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                combined_state.clone(),
                super::middleware::auth_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                combined_state.clone(),
                super::middleware::authorization_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                combined_state.clone(),
                super::middleware::response_cache_middleware,
            ));

        // Mount API routes under the base path
        Router::new().nest(base_path, api_routes)
    }

    /// Create routes for a specific API version
    fn create_version_routes(
        state: (
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        ),
        _endpoints: &dyn super::versioning::EndpointProvider,
    ) -> Router {
        // For now, use the standard endpoints for all versions
        // In the future, this can be enhanced to use version-specific endpoints
        Router::new()
            .route("/health", get(health_check))
            .route("/schemas", post(register_schema))
            .route("/schemas", get(list_schemas))
            .route("/schemas/{fingerprint}", get(get_schema))
            .route("/schemas/{fingerprint}", delete(delete_schema))
            .route("/schemas/{fingerprint}/validate", post(validate_data))
            .route("/schemas/validate", post(validate_schema))
            .route("/schemas/evolve", post(evolve_schema))
            .route("/metrics", get(get_metrics))
            .route("/stats", get(get_stats))
            .route("/docs", get(get_openapi_docs))
            .with_state(state)
    }

    /// Create legacy routes for backward compatibility
    fn create_legacy_routes(
        state: (
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        ),
        _versioned_endpoints: &VersionedEndpoints,
        _versioning_config: &ApiVersionConfig,
    ) -> Router {
        // Use default version endpoints for legacy routes
        Router::new()
            .route("/health", get(health_check))
            .route("/schemas", post(register_schema))
            .route("/schemas", get(list_schemas))
            .route("/schemas/{fingerprint}", get(get_schema))
            .route("/schemas/{fingerprint}", delete(delete_schema))
            .route("/schemas/{fingerprint}/validate", post(validate_data))
            .route("/schemas/validate", post(validate_schema))
            .route("/schemas/evolve", post(evolve_schema))
            .route("/metrics", get(get_metrics))
            .route("/stats", get(get_stats))
            .route("/docs", get(get_openapi_docs))
            .with_state(state)
    }

    /// Create the Axum app
    pub fn create_app(&self) -> Router {
        self.router.clone()
    }

    /// Get the router
    pub fn router(&self) -> Router {
        self.router.clone()
    }

    /// Get supported API versions
    pub fn get_supported_versions(&self) -> &[ApiVersion] {
        &self.versioning_config.supported_versions
    }

    /// Get default API version
    pub fn get_default_version(&self) -> &ApiVersion {
        &self.versioning_config.default_version
    }

    /// Check if a version is deprecated
    pub fn is_version_deprecated(&self, version: &ApiVersion) -> Option<&String> {
        self.versioning_config.deprecated_versions.get(version)
    }

    /// Get version compatibility information
    pub fn get_version_compatibility(&self, version: &ApiVersion) -> Vec<ApiVersion> {
        self.versioning_config
            .supported_versions
            .iter()
            .filter(|v| v.is_compatible_with(version))
            .cloned()
            .collect()
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

        // Test version information
        let supported_versions = api.get_supported_versions();
        assert!(!supported_versions.is_empty());

        let default_version = api.get_default_version();
        assert!(supported_versions.contains(default_version));
    }

    #[test]
    fn test_version_deprecation() {
        let registry = Arc::new(SchemaRegistryManager::default());
        let api = SchemaRegistryApi::new(registry);

        let v1 = ApiVersion::v1_0_0();
        let deprecation_warning = api.is_version_deprecated(&v1);
        assert!(deprecation_warning.is_some());

        let v2 = ApiVersion::v2_0_0();
        let deprecation_warning = api.is_version_deprecated(&v2);
        assert!(deprecation_warning.is_none());
    }

    #[test]
    fn test_version_compatibility() {
        let registry = Arc::new(SchemaRegistryManager::default());
        let api = SchemaRegistryApi::new(registry);

        let v1 = ApiVersion::v1_0_0();
        let compatible_versions = api.get_version_compatibility(&v1);
        assert!(!compatible_versions.is_empty());
        assert!(compatible_versions
            .iter()
            .all(|v| v.is_compatible_with(&v1)));
    }

    #[test]
    fn test_version_path_generation() {
        let v1 = ApiVersion::v1_0_0();
        let v2 = ApiVersion::v2_0_0();

        assert_eq!(v1.to_path(), "v1");
        assert_eq!(v2.to_path(), "v2");
        assert_eq!(v1.to_string(), "v1.0.0");
        assert_eq!(v2.to_string(), "v2.0.0");
    }
}
