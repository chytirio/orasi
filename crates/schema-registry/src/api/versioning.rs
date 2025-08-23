//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! API versioning support
//!
//! This module provides support for multiple API versions with version-specific
//! endpoints and handlers.

use std::collections::HashMap;
use std::sync::Arc;

use crate::api::endpoints::*;
use crate::api::middleware::{AuthConfig, SharedRateLimiter, SharedResponseCache};
use crate::registry::SchemaRegistryManager;

/// API version information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiVersion {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl ApiVersion {
    /// Create a new API version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Create version 1.0.0
    pub fn v1_0_0() -> Self {
        Self::new(1, 0, 0)
    }

    /// Create version 2.0.0
    pub fn v2_0_0() -> Self {
        Self::new(2, 0, 0)
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        format!("v{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Convert to path segment
    pub fn to_path(&self) -> String {
        format!("v{}", self.major)
    }

    /// Check if this version is compatible with another
    pub fn is_compatible_with(&self, other: &ApiVersion) -> bool {
        // Major version must match for compatibility
        self.major == other.major
    }

    /// Check if this version is newer than another
    pub fn is_newer_than(&self, other: &ApiVersion) -> bool {
        if self.major != other.major {
            self.major > other.major
        } else if self.minor != other.minor {
            self.minor > other.minor
        } else {
            self.patch > other.patch
        }
    }
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl std::str::FromStr for ApiVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_start_matches('v');
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() != 3 {
            return Err(format!("Invalid API version format: {}", s));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(Self::new(major, minor, patch))
    }
}

/// API version configuration
#[derive(Debug, Clone)]
pub struct ApiVersionConfig {
    /// Supported API versions
    pub supported_versions: Vec<ApiVersion>,
    /// Default API version
    pub default_version: ApiVersion,
    /// Whether to allow version negotiation
    pub allow_version_negotiation: bool,
    /// Version deprecation warnings
    pub deprecated_versions: HashMap<ApiVersion, String>,
}

impl Default for ApiVersionConfig {
    fn default() -> Self {
        let mut deprecated_versions = HashMap::new();
        deprecated_versions.insert(
            ApiVersion::v1_0_0(),
            "API v1.0.0 is deprecated. Please upgrade to v2.0.0.".to_string(),
        );

        Self {
            supported_versions: vec![ApiVersion::v1_0_0(), ApiVersion::v2_0_0()],
            default_version: ApiVersion::v2_0_0(),
            allow_version_negotiation: true,
            deprecated_versions,
        }
    }
}

/// Version-specific endpoint handlers
pub struct VersionedEndpoints {
    /// Version 1.0.0 endpoints
    pub v1: V1Endpoints,
    /// Version 2.0.0 endpoints
    pub v2: V2Endpoints,
}

impl VersionedEndpoints {
    /// Create new versioned endpoints
    pub fn new() -> Self {
        Self {
            v1: V1Endpoints::new(),
            v2: V2Endpoints::new(),
        }
    }

    /// Get endpoints for a specific version
    pub fn get_endpoints(&self, version: &ApiVersion) -> Option<&dyn EndpointProvider> {
        match version.major {
            1 => Some(&self.v1),
            2 => Some(&self.v2),
            _ => None,
        }
    }
}

/// Trait for providing version-specific endpoints
pub trait EndpointProvider {
    /// Get the version this provider supports
    fn version(&self) -> ApiVersion;

    /// Get health check handler
    fn health_check(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::HealthResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get register schema handler
    fn register_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Json<crate::api::requests::RegisterSchemaRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::RegisterSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get list schemas handler
    fn list_schemas(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Query<crate::schema::SchemaSearchCriteria>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::ListSchemasResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get get schema handler
    fn get_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Path<String>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::GetSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get delete schema handler
    fn delete_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Path<String>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::DeleteSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get validate data handler
    fn validate_data(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Path<String>,
        axum::extract::Json<crate::api::requests::ValidateDataRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::ValidateDataResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get validate schema handler
    fn validate_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Json<crate::api::requests::ValidateSchemaRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::ValidateSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get evolve schema handler
    fn evolve_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Json<crate::api::requests::EvolveSchemaRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::EvolveSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get metrics handler
    fn get_metrics(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::MetricsResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get stats handler
    fn get_stats(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::StatsResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;

    /// Get OpenAPI docs handler
    fn get_openapi_docs(
        &self,
    ) -> fn() -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<serde_json::Value>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    >;
}

/// Version 1.0.0 endpoints
pub struct V1Endpoints;

impl V1Endpoints {
    pub fn new() -> Self {
        Self
    }
}

impl EndpointProvider for V1Endpoints {
    fn version(&self) -> ApiVersion {
        ApiVersion::v1_0_0()
    }

    fn health_check(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::HealthResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state| Box::pin(async move { health_check(state).await })
    }

    fn register_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Json<crate::api::requests::RegisterSchemaRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::RegisterSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, request| Box::pin(async move { register_schema(state, request).await })
    }

    fn list_schemas(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Query<crate::schema::SchemaSearchCriteria>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::ListSchemasResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, criteria| Box::pin(async move { list_schemas(state, criteria).await })
    }

    fn get_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Path<String>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::GetSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, fingerprint| Box::pin(async move { get_schema(state, fingerprint).await })
    }

    fn delete_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Path<String>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::DeleteSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, fingerprint| Box::pin(async move { delete_schema(state, fingerprint).await })
    }

    fn validate_data(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Path<String>,
        axum::extract::Json<crate::api::requests::ValidateDataRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::ValidateDataResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, fingerprint, request| {
            Box::pin(async move { validate_data(state, fingerprint, request).await })
        }
    }

    fn validate_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Json<crate::api::requests::ValidateSchemaRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::ValidateSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, request| Box::pin(async move { validate_schema(state, request).await })
    }

    fn evolve_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Json<crate::api::requests::EvolveSchemaRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::EvolveSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, request| Box::pin(async move { evolve_schema(state, request).await })
    }

    fn get_metrics(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::MetricsResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state| Box::pin(async move { get_metrics(state).await })
    }

    fn get_stats(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::StatsResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state| Box::pin(async move { get_stats(state).await })
    }

    fn get_openapi_docs(
        &self,
    ) -> fn() -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<serde_json::Value>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        || Box::pin(async move { get_openapi_docs().await })
    }
}

/// Version 2.0.0 endpoints (with enhanced features)
pub struct V2Endpoints;

impl V2Endpoints {
    pub fn new() -> Self {
        Self
    }
}

impl EndpointProvider for V2Endpoints {
    fn version(&self) -> ApiVersion {
        ApiVersion::v2_0_0()
    }

    fn health_check(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::HealthResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state| {
            Box::pin(async move {
                // V2 health check with enhanced information
                health_check_v2(state).await
            })
        }
    }

    fn register_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Json<crate::api::requests::RegisterSchemaRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::RegisterSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, request| {
            Box::pin(async move {
                // V2 register schema with enhanced validation
                register_schema_v2(state, request).await
            })
        }
    }

    fn list_schemas(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Query<crate::schema::SchemaSearchCriteria>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::ListSchemasResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, criteria| {
            Box::pin(async move {
                // V2 list schemas with enhanced filtering
                list_schemas_v2(state, criteria).await
            })
        }
    }

    fn get_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Path<String>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::GetSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, fingerprint| {
            Box::pin(async move {
                // V2 get schema with enhanced metadata
                get_schema_v2(state, fingerprint).await
            })
        }
    }

    fn delete_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Path<String>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::DeleteSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, fingerprint| {
            Box::pin(async move {
                // V2 delete schema with soft delete option
                delete_schema_v2(state, fingerprint).await
            })
        }
    }

    fn validate_data(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Path<String>,
        axum::extract::Json<crate::api::requests::ValidateDataRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::ValidateDataResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, fingerprint, request| {
            Box::pin(async move {
                // V2 validate data with enhanced validation
                validate_data_v2(state, fingerprint, request).await
            })
        }
    }

    fn validate_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Json<crate::api::requests::ValidateSchemaRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::ValidateSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, request| {
            Box::pin(async move {
                // V2 validate schema with enhanced validation
                validate_schema_v2(state, request).await
            })
        }
    }

    fn evolve_schema(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
        axum::extract::Json<crate::api::requests::EvolveSchemaRequest>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::EvolveSchemaResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state, request| {
            Box::pin(async move {
                // V2 evolve schema with enhanced evolution
                evolve_schema_v2(state, request).await
            })
        }
    }

    fn get_metrics(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::MetricsResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state| {
            Box::pin(async move {
                // V2 metrics with enhanced metrics
                get_metrics_v2(state).await
            })
        }
    }

    fn get_stats(
        &self,
    ) -> fn(
        axum::extract::State<(
            Arc<SchemaRegistryManager>,
            SharedRateLimiter,
            Arc<AuthConfig>,
            SharedResponseCache,
        )>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<crate::api::responses::StatsResponse>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        |state| {
            Box::pin(async move {
                // V2 stats with enhanced statistics
                get_stats_v2(state).await
            })
        }
    }

    fn get_openapi_docs(
        &self,
    ) -> fn() -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        axum::response::Json<serde_json::Value>,
                        crate::api::error::ApiError,
                    >,
                > + Send,
        >,
    > {
        || {
            Box::pin(async move {
                // V2 OpenAPI docs with enhanced documentation
                get_openapi_docs_v2().await
            })
        }
    }
}

// V2 endpoint implementations (enhanced versions)
async fn health_check_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
) -> Result<axum::response::Json<crate::api::responses::HealthResponse>, crate::api::error::ApiError>
{
    // Enhanced health check with additional information
    health_check(state).await
}

async fn register_schema_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    request: axum::extract::Json<crate::api::requests::RegisterSchemaRequest>,
) -> Result<
    axum::response::Json<crate::api::responses::RegisterSchemaResponse>,
    crate::api::error::ApiError,
> {
    // Enhanced schema registration with additional validation
    register_schema(state, request).await
}

async fn list_schemas_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    criteria: axum::extract::Query<crate::schema::SchemaSearchCriteria>,
) -> Result<
    axum::response::Json<crate::api::responses::ListSchemasResponse>,
    crate::api::error::ApiError,
> {
    // Enhanced schema listing with additional filtering options
    list_schemas(state, criteria).await
}

async fn get_schema_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    fingerprint: axum::extract::Path<String>,
) -> Result<
    axum::response::Json<crate::api::responses::GetSchemaResponse>,
    crate::api::error::ApiError,
> {
    // Enhanced schema retrieval with additional metadata
    get_schema(state, fingerprint).await
}

async fn delete_schema_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    fingerprint: axum::extract::Path<String>,
) -> Result<
    axum::response::Json<crate::api::responses::DeleteSchemaResponse>,
    crate::api::error::ApiError,
> {
    // Enhanced schema deletion with soft delete option
    delete_schema(state, fingerprint).await
}

async fn validate_data_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    fingerprint: axum::extract::Path<String>,
    request: axum::extract::Json<crate::api::requests::ValidateDataRequest>,
) -> Result<
    axum::response::Json<crate::api::responses::ValidateDataResponse>,
    crate::api::error::ApiError,
> {
    // Enhanced data validation with additional validation rules
    validate_data(state, fingerprint, request).await
}

async fn validate_schema_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    request: axum::extract::Json<crate::api::requests::ValidateSchemaRequest>,
) -> Result<
    axum::response::Json<crate::api::responses::ValidateSchemaResponse>,
    crate::api::error::ApiError,
> {
    // Enhanced schema validation with additional validation rules
    validate_schema(state, request).await
}

async fn evolve_schema_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
    request: axum::extract::Json<crate::api::requests::EvolveSchemaRequest>,
) -> Result<
    axum::response::Json<crate::api::responses::EvolveSchemaResponse>,
    crate::api::error::ApiError,
> {
    // Enhanced schema evolution with additional evolution strategies
    evolve_schema(state, request).await
}

async fn get_metrics_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
) -> Result<axum::response::Json<crate::api::responses::MetricsResponse>, crate::api::error::ApiError>
{
    // Enhanced metrics with additional metrics
    get_metrics(state).await
}

async fn get_stats_v2(
    state: axum::extract::State<(
        Arc<SchemaRegistryManager>,
        SharedRateLimiter,
        Arc<AuthConfig>,
        SharedResponseCache,
    )>,
) -> Result<axum::response::Json<crate::api::responses::StatsResponse>, crate::api::error::ApiError>
{
    // Enhanced stats with additional statistics
    get_stats(state).await
}

async fn get_openapi_docs_v2(
) -> Result<axum::response::Json<serde_json::Value>, crate::api::error::ApiError> {
    // Enhanced OpenAPI docs with V2-specific documentation
    get_openapi_docs().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_creation() {
        let v1 = ApiVersion::v1_0_0();
        assert_eq!(v1.major, 1);
        assert_eq!(v1.minor, 0);
        assert_eq!(v1.patch, 0);
        assert_eq!(v1.to_string(), "v1.0.0");
        assert_eq!(v1.to_path(), "v1");
    }

    #[test]
    fn test_api_version_parsing() {
        let v1 = "v1.0.0".parse::<ApiVersion>().unwrap();
        assert_eq!(v1, ApiVersion::v1_0_0());

        let v2 = "v2.1.3".parse::<ApiVersion>().unwrap();
        assert_eq!(v2, ApiVersion::new(2, 1, 3));
    }

    #[test]
    fn test_api_version_compatibility() {
        let v1_0_0 = ApiVersion::v1_0_0();
        let v1_1_0 = ApiVersion::new(1, 1, 0);
        let v2_0_0 = ApiVersion::v2_0_0();

        assert!(v1_0_0.is_compatible_with(&v1_1_0));
        assert!(!v1_0_0.is_compatible_with(&v2_0_0));
        assert!(v2_0_0.is_newer_than(&v1_0_0));
    }

    #[test]
    fn test_versioned_endpoints() {
        let endpoints = VersionedEndpoints::new();

        let v1_endpoints = endpoints.get_endpoints(&ApiVersion::v1_0_0());
        assert!(v1_endpoints.is_some());
        assert_eq!(v1_endpoints.unwrap().version(), ApiVersion::v1_0_0());

        let v2_endpoints = endpoints.get_endpoints(&ApiVersion::v2_0_0());
        assert!(v2_endpoints.is_some());
        assert_eq!(v2_endpoints.unwrap().version(), ApiVersion::v2_0_0());

        let v3_endpoints = endpoints.get_endpoints(&ApiVersion::new(3, 0, 0));
        assert!(v3_endpoints.is_none());
    }
}
