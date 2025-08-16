//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! API key configuration

use serde::{Deserialize, Serialize};

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// Whether to enable API key authentication
    pub enabled: bool,

    /// API key expiration time in seconds
    pub expiration_secs: u64,

    /// API key prefix
    pub prefix: String,

    /// API key length
    pub key_length: usize,

    /// Maximum API keys per user
    pub max_keys_per_user: usize,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            expiration_secs: crate::DEFAULT_API_KEY_EXPIRATION_SECS,
            prefix: "otb_".to_string(),
            key_length: 32,
            max_keys_per_user: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_config_default() {
        let config = ApiKeyConfig::default();
        assert!(config.enabled);
        assert_eq!(
            config.expiration_secs,
            crate::DEFAULT_API_KEY_EXPIRATION_SECS
        );
        assert_eq!(config.key_length, 32);
    }
}
