//! Connection configuration types

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection timeout
    pub timeout: Duration,

    /// Maximum retries
    pub max_retries: u32,

    /// Retry delay
    pub retry_delay: Duration,

    /// Keep-alive configuration
    pub keep_alive: Option<KeepAliveConfig>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            keep_alive: Some(KeepAliveConfig::default()),
        }
    }
}

impl ConnectionConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.max_retries == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Maximum retries must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Keep-alive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepAliveConfig {
    /// Keep-alive interval
    pub interval: Duration,

    /// Keep-alive timeout
    pub timeout: Duration,
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
        }
    }
}
