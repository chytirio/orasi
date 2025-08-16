//! Schema resolution and reference handling
//!
//! This module contains functionality for resolving schema references and imports.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::core::Schema;

/// Resolved schema with all references and imports resolved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSchema {
    /// Original schema
    pub schema: Schema,

    /// Resolved content with all references expanded
    pub resolved_content: String,

    /// List of dependencies
    pub dependencies: Vec<String>,

    /// Resolved semantic conventions
    pub semantic_conventions: HashMap<String, serde_json::Value>,
}

impl ResolvedSchema {
    /// Create a new resolved schema
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            resolved_content: String::new(),
            dependencies: Vec::new(),
            semantic_conventions: HashMap::new(),
        }
    }

    /// Set resolved content
    pub fn with_resolved_content(mut self, content: String) -> Self {
        self.resolved_content = content;
        self
    }

    /// Add dependency
    pub fn add_dependency(&mut self, dependency: String) {
        if !self.dependencies.contains(&dependency) {
            self.dependencies.push(dependency);
        }
    }

    /// Add semantic convention
    pub fn add_semantic_convention(&mut self, key: String, value: serde_json::Value) {
        self.semantic_conventions.insert(key, value);
    }
}
