//! Rate limiting configuration types

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Requests per second
    pub requests_per_second: u32,

    /// Burst size
    pub burst_size: u32,

    /// Rate limit by IP
    pub limit_by_ip: bool,

    /// Rate limit by user
    pub limit_by_user: bool,
}

impl RateLimitingConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.requests_per_second == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Requests per second must be greater than 0".to_string(),
            ));
        }

        if self.burst_size == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Burst size must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}
