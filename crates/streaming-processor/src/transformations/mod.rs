//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data transformations for the bridge
//!
//! This module provides transformation functionality for streaming data
//! including various transformation functions and managers.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use std::collections::HashMap;

/// Transformation function trait
#[async_trait]
pub trait TransformationFunction: Send + Sync {
    /// Get function name
    fn name(&self) -> &str;

    /// Get function type
    fn function_type(&self) -> &str;

    /// Apply transformation function
    async fn apply(&self, value: String) -> BridgeResult<String>;

    /// Get function description
    fn description(&self) -> &str;
}

/// Uppercase transformation function
pub struct UppercaseTransformation;

#[async_trait]
impl TransformationFunction for UppercaseTransformation {
    fn name(&self) -> &str {
        "uppercase"
    }

    fn function_type(&self) -> &str {
        "string"
    }

    async fn apply(&self, value: String) -> BridgeResult<String> {
        Ok(value.to_uppercase())
    }

    fn description(&self) -> &str {
        "Convert string to uppercase"
    }
}

/// Lowercase transformation function
pub struct LowercaseTransformation;

#[async_trait]
impl TransformationFunction for LowercaseTransformation {
    fn name(&self) -> &str {
        "lowercase"
    }

    fn function_type(&self) -> &str {
        "string"
    }

    async fn apply(&self, value: String) -> BridgeResult<String> {
        Ok(value.to_lowercase())
    }

    fn description(&self) -> &str {
        "Convert string to lowercase"
    }
}

/// Trim transformation function
pub struct TrimTransformation;

#[async_trait]
impl TransformationFunction for TrimTransformation {
    fn name(&self) -> &str {
        "trim"
    }

    fn function_type(&self) -> &str {
        "string"
    }

    async fn apply(&self, value: String) -> BridgeResult<String> {
        Ok(value.trim().to_string())
    }

    fn description(&self) -> &str {
        "Trim whitespace from string"
    }
}

/// Replace transformation function
pub struct ReplaceTransformation {
    from: String,
    to: String,
}

impl ReplaceTransformation {
    /// Create new replace transformation
    pub fn new(from: String, to: String) -> Self {
        Self { from, to }
    }
}

#[async_trait]
impl TransformationFunction for ReplaceTransformation {
    fn name(&self) -> &str {
        "replace"
    }

    fn function_type(&self) -> &str {
        "string"
    }

    async fn apply(&self, value: String) -> BridgeResult<String> {
        Ok(value.replace(&self.from, &self.to))
    }

    fn description(&self) -> &str {
        "Replace substring in string"
    }
}

/// Substring transformation function
pub struct SubstringTransformation {
    start: usize,
    end: Option<usize>,
}

impl SubstringTransformation {
    /// Create new substring transformation
    pub fn new(start: usize, end: Option<usize>) -> Self {
        Self { start, end }
    }
}

#[async_trait]
impl TransformationFunction for SubstringTransformation {
    fn name(&self) -> &str {
        "substring"
    }

    fn function_type(&self) -> &str {
        "string"
    }

    async fn apply(&self, value: String) -> BridgeResult<String> {
        if self.start >= value.len() {
            return Ok(String::new());
        }

        let end = self.end.unwrap_or(value.len()).min(value.len());
        let start = self.start.min(end);

        Ok(value[start..end].to_string())
    }

    fn description(&self) -> &str {
        "Extract substring from string"
    }
}

/// Transformation manager for managing transformation functions
pub struct TransformationManager {
    functions: HashMap<String, Box<dyn TransformationFunction>>,
}

impl TransformationManager {
    /// Create new transformation manager
    pub fn new() -> Self {
        let mut manager = Self {
            functions: HashMap::new(),
        };

        // Register default transformation functions
        manager.register_function(Box::new(UppercaseTransformation));
        manager.register_function(Box::new(LowercaseTransformation));
        manager.register_function(Box::new(TrimTransformation));

        manager
    }

    /// Register transformation function
    pub fn register_function(&mut self, function: Box<dyn TransformationFunction>) {
        self.functions.insert(function.name().to_string(), function);
    }

    /// Get transformation function
    pub fn get_function(&self, name: &str) -> Option<&dyn TransformationFunction> {
        self.functions.get(name).map(|f| f.as_ref())
    }

    /// Get all function names
    pub fn get_function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    /// Apply transformation function
    pub async fn apply_function(&self, name: &str, value: String) -> BridgeResult<String> {
        if let Some(function) = self.get_function(name) {
            function.apply(value).await
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "Unknown transformation function: {}",
                name
            )))
        }
    }
}

/// Transformation factory for creating transformation functions
pub struct TransformationFactory;

impl TransformationFactory {
    /// Create transformation function by name
    pub fn create_function(name: &str) -> Option<Box<dyn TransformationFunction>> {
        match name {
            "uppercase" => Some(Box::new(UppercaseTransformation)),
            "lowercase" => Some(Box::new(LowercaseTransformation)),
            "trim" => Some(Box::new(TrimTransformation)),
            _ => None,
        }
    }

    /// Create replace transformation function
    pub fn create_replace_function(from: String, to: String) -> Box<dyn TransformationFunction> {
        Box::new(ReplaceTransformation::new(from, to))
    }

    /// Create substring transformation function
    pub fn create_substring_function(
        start: usize,
        end: Option<usize>,
    ) -> Box<dyn TransformationFunction> {
        Box::new(SubstringTransformation::new(start, end))
    }

    /// Get available function names
    pub fn get_available_functions() -> Vec<String> {
        vec![
            "uppercase".to_string(),
            "lowercase".to_string(),
            "trim".to_string(),
            "replace".to_string(),
            "substring".to_string(),
        ]
    }
}
