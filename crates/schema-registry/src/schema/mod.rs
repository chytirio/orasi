//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Schema definitions for the Schema Registry
//!
//! This module provides schema structures and types for managing
//! telemetry data schemas in the registry.

pub mod cache;
pub mod core;
pub mod metadata;
pub mod resolution;
pub mod types;
pub mod version;

// Re-export main types for convenience
pub use core::Schema;
pub use metadata::{SchemaMetadata, SchemaSearchCriteria};
pub use types::{CompatibilityMode, SchemaFormat, SchemaType, SchemaVisibility};
pub use version::SchemaVersion;
