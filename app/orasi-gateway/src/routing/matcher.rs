//! Route matching implementation

use std::collections::HashMap;

/// Route match result
#[derive(Debug, Clone)]
pub struct RouteMatch {
    /// Matched path
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Route parameters
    pub parameters: HashMap<String, String>,
    /// Route metadata
    pub metadata: HashMap<String, String>,
}

impl RouteMatch {
    /// Create a new route match
    pub fn new(path: String, method: String) -> Self {
        Self {
            path,
            method,
            parameters: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a parameter to the route match
    pub fn with_parameter(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }

    /// Add metadata to the route match
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}
