//! Types for service discovery

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Service registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistration {
    /// Service ID
    pub service_id: String,

    /// Service name
    pub service_name: String,

    /// Service endpoint
    pub endpoint: String,

    /// Service metadata
    pub metadata: HashMap<String, String>,

    /// Health check endpoint
    pub health_endpoint: Option<String>,

    /// Registration timestamp
    pub registered_at: u64,

    /// TTL in seconds
    pub ttl: Option<u64>,
}
