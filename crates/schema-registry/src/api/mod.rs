//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! HTTP API for the Schema Registry
//!
//! This module provides HTTP API endpoints for the schema registry.

pub mod endpoints;
pub mod error;
pub mod requests;
pub mod responses;
pub mod server;

// Re-export main types for convenience
pub use error::ApiError;
pub use requests::*;
pub use responses::*;
pub use server::SchemaRegistryApi;
