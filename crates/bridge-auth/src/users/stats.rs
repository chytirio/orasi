//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! User statistics

use chrono::{DateTime, Utc};

/// User statistics
#[derive(Debug, Clone, Default)]
pub struct UserStats {
    /// Number of users created
    pub users_created: u64,

    /// Number of password verifications
    pub password_verifications: u64,

    /// Number of failed login attempts
    pub failed_login_attempts: u64,

    /// Last user created
    pub last_user_created: Option<DateTime<Utc>>,

    /// Last password verification
    pub last_password_verification: Option<DateTime<Utc>>,
}

impl UserStats {
    /// Create new user statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment users created count
    pub fn increment_users_created(&mut self) {
        self.users_created += 1;
        self.last_user_created = Some(Utc::now());
    }

    /// Increment password verifications count
    pub fn increment_password_verifications(&mut self) {
        self.password_verifications += 1;
        self.last_password_verification = Some(Utc::now());
    }

    /// Increment failed login attempts count
    pub fn increment_failed_login_attempts(&mut self) {
        self.failed_login_attempts += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_stats_creation() {
        let stats = UserStats::new();
        assert_eq!(stats.users_created, 0);
        assert_eq!(stats.password_verifications, 0);
        assert_eq!(stats.failed_login_attempts, 0);
        assert!(stats.last_user_created.is_none());
        assert!(stats.last_password_verification.is_none());
    }

    #[test]
    fn test_user_stats_increment() {
        let mut stats = UserStats::new();

        stats.increment_users_created();
        assert_eq!(stats.users_created, 1);
        assert!(stats.last_user_created.is_some());

        stats.increment_password_verifications();
        assert_eq!(stats.password_verifications, 1);
        assert!(stats.last_password_verification.is_some());

        stats.increment_failed_login_attempts();
        assert_eq!(stats.failed_login_attempts, 1);
    }
}
