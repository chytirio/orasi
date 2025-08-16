//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! User configuration

use serde::{Deserialize, Serialize};

/// User management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Whether to enable user registration
    pub allow_registration: bool,

    /// Whether to require email verification
    pub require_email_verification: bool,

    /// Password minimum length
    pub min_password_length: usize,

    /// Password complexity requirements
    pub password_complexity: PasswordComplexity,

    /// Maximum login attempts
    pub max_login_attempts: usize,

    /// Account lockout duration in seconds
    pub lockout_duration_secs: u64,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            allow_registration: true,
            require_email_verification: false,
            min_password_length: 8,
            password_complexity: PasswordComplexity::default(),
            max_login_attempts: 5,
            lockout_duration_secs: 900, // 15 minutes
        }
    }
}

/// Password complexity requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordComplexity {
    /// Require uppercase letters
    pub require_uppercase: bool,

    /// Require lowercase letters
    pub require_lowercase: bool,

    /// Require numbers
    pub require_numbers: bool,

    /// Require special characters
    pub require_special: bool,
}

impl Default for PasswordComplexity {
    fn default() -> Self {
        Self {
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_config_default() {
        let config = UserConfig::default();
        assert!(config.allow_registration);
        assert_eq!(config.min_password_length, 8);
        assert_eq!(config.max_login_attempts, 5);
        assert_eq!(config.lockout_duration_secs, 900);
    }

    #[test]
    fn test_password_complexity_default() {
        let complexity = PasswordComplexity::default();
        assert!(complexity.require_uppercase);
        assert!(complexity.require_lowercase);
        assert!(complexity.require_numbers);
        assert!(!complexity.require_special);
    }
}
