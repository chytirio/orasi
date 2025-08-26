//! Transformation factory for creating transformation functions

use super::trait_def::TransformationFunction;
use super::fun::{UppercaseTransformation, LowercaseTransformation, TrimTransformation, ReplaceTransformation, SubstringTransformation};

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
