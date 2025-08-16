//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! User model definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    User,
    ReadOnly,
    Custom(String),
}

/// User structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: String,

    /// Username
    pub username: String,

    /// Email
    pub email: String,

    /// Password hash
    pub password_hash: Option<String>,

    /// User roles
    pub roles: Vec<UserRole>,

    /// Whether the user is active
    pub is_active: bool,

    /// Whether the account is locked
    pub is_locked: bool,

    /// Failed login attempts
    pub failed_login_attempts: u32,

    /// Account lockout until
    pub lockout_until: Option<DateTime<Utc>>,

    /// Creation time
    pub created_at: DateTime<Utc>,

    /// Last login time
    pub last_login_at: Option<DateTime<Utc>>,

    /// OAuth provider (if applicable)
    pub oauth_provider: Option<String>,

    /// OAuth provider user ID (if applicable)
    pub oauth_provider_user_id: Option<String>,
}

impl User {
    /// Create new user
    pub fn new(
        username: String,
        email: String,
        password_hash: Option<String>,
        roles: Vec<UserRole>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            username,
            email,
            password_hash,
            roles,
            is_active: true,
            is_locked: false,
            failed_login_attempts: 0,
            lockout_until: None,
            created_at: Utc::now(),
            last_login_at: None,
            oauth_provider: None,
            oauth_provider_user_id: None,
        }
    }

    /// Check if user has role
    pub fn has_role(&self, role: &UserRole) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        self.has_role(&UserRole::Admin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let username = "testuser".to_string();
        let email = "test@example.com".to_string();
        let roles = vec![UserRole::User];

        let user = User::new(username.clone(), email.clone(), None, roles.clone());
        
        assert_eq!(user.username, username);
        assert_eq!(user.email, email);
        assert_eq!(user.roles, roles);
        assert!(user.is_active);
        assert!(!user.is_locked);
        assert_eq!(user.failed_login_attempts, 0);
    }

    #[test]
    fn test_user_role_checking() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            None,
            vec![UserRole::User, UserRole::Admin],
        );

        assert!(user.has_role(&UserRole::User));
        assert!(user.has_role(&UserRole::Admin));
        assert!(!user.has_role(&UserRole::ReadOnly));
        assert!(user.is_admin());
    }
}
