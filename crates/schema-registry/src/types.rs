//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Common types for the Schema Registry
//!
//! This module provides common types and utilities used throughout
//! the schema registry.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Component schema for OpenTelemetry components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSchema {
    /// Component name
    pub name: String,

    /// Component version
    pub version: String,

    /// Component type
    pub component_type: String,

    /// Schema content
    pub schema: serde_json::Value,

    /// Schema format
    pub format: String,

    /// Component description
    pub description: Option<String>,

    /// Component tags
    pub tags: Vec<String>,
}
