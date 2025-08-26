//! Processing configuration types

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Batch size
    pub batch_size: usize,

    /// Buffer size
    pub buffer_size: usize,

    /// Processing timeout
    pub timeout: Duration,

    /// Enable parallel processing
    pub enable_parallel: bool,

    /// Number of parallel workers
    pub num_workers: usize,

    /// Enable backpressure
    pub enable_backpressure: bool,

    /// Backpressure threshold (percentage)
    pub backpressure_threshold: u8,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            buffer_size: 10000,
            timeout: Duration::from_secs(30),
            enable_parallel: false,
            num_workers: 1,
            enable_backpressure: true,
            backpressure_threshold: 80,
        }
    }
}

impl ProcessingConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.batch_size == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Batch size must be greater than 0".to_string(),
            ));
        }

        if self.buffer_size == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Buffer size must be greater than 0".to_string(),
            ));
        }

        if self.num_workers == 0 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Number of workers must be greater than 0".to_string(),
            ));
        }

        if self.backpressure_threshold > 100 {
            return Err(bridge_core::error::BridgeError::configuration(
                "Backpressure threshold must be between 0 and 100".to_string(),
            ));
        }

        Ok(())
    }
}
