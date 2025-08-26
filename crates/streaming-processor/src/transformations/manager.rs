//! Transformation manager for managing transformation functions

use bridge_core::BridgeResult;
use std::collections::HashMap;

use super::trait_def::TransformationFunction;
use super::fun::{UppercaseTransformation, LowercaseTransformation, TrimTransformation};

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
