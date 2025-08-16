//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Role-based access control configuration

use serde::{Deserialize, Serialize};

/// Role-based access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacConfig {
    /// Whether to enable RBAC
    pub enabled: bool,

    /// Default role for new users
    pub default_role: String,

    /// Admin role name
    pub admin_role: String,

    /// Whether to allow role inheritance
    pub allow_role_inheritance: bool,

    /// Maximum roles per user
    pub max_roles_per_user: usize,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_role: "user".to_string(),
            admin_role: "admin".to_string(),
            allow_role_inheritance: true,
            max_roles_per_user: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbac_config_default() {
        let config = RbacConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_role, "user");
        assert_eq!(config.admin_role, "admin");
        assert!(config.allow_role_inheritance);
        assert_eq!(config.max_roles_per_user, 5);
    }
}
