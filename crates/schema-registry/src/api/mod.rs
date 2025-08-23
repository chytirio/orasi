//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! API module for the Schema Registry
//!
//! This module contains all the API-related functionality including endpoints,
//! middleware, request/response types, and server implementation.

pub mod common;
pub mod endpoints;
pub mod error;
pub mod middleware;
pub mod openapi;
pub mod requests;
pub mod responses;
pub mod server;
pub mod versioning;

// Re-export main types for convenience
pub use common::{build_query_string, extract_query_params, generate_request_id, *};
pub use error::ApiError;
pub use middleware::{AuthConfig, RateLimitConfig, SharedRateLimiter};
pub use openapi::generate_openapi_docs;
pub use requests::*;
pub use responses::*;
pub use server::SchemaRegistryApi;
pub use versioning::{ApiVersion, ApiVersionConfig, VersionedEndpoints};
