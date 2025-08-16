//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Session configuration

use serde::{Deserialize, Serialize};

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session timeout in seconds
    pub timeout_secs: u64,

    /// Whether to enable session refresh
    pub enable_refresh: bool,

    /// Session refresh interval in seconds
    pub refresh_interval_secs: u64,

    /// Maximum concurrent sessions per user
    pub max_concurrent_sessions: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout_secs: crate::DEFAULT_SESSION_TIMEOUT_SECS,
            enable_refresh: true,
            refresh_interval_secs: 300, // 5 minutes
            max_concurrent_sessions: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.timeout_secs, crate::DEFAULT_SESSION_TIMEOUT_SECS);
        assert!(config.enable_refresh);
        assert_eq!(config.refresh_interval_secs, 300);
        assert_eq!(config.max_concurrent_sessions, 5);
    }
}
