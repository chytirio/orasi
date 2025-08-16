//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Permission statistics

use chrono::{DateTime, Utc};

/// Permission statistics
#[derive(Debug, Clone, Default)]
pub struct PermissionStats {
    /// Number of permission checks
    pub permission_checks: u64,

    /// Number of permission grants
    pub permission_grants: u64,

    /// Number of permission denials
    pub permission_denials: u64,

    /// Last permission check
    pub last_permission_check: Option<DateTime<Utc>>,
}

impl PermissionStats {
    /// Create new permission statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment permission checks count
    pub fn increment_permission_checks(&mut self) {
        self.permission_checks += 1;
        self.last_permission_check = Some(Utc::now());
    }

    /// Increment permission grants count
    pub fn increment_permission_grants(&mut self) {
        self.permission_grants += 1;
    }

    /// Increment permission denials count
    pub fn increment_permission_denials(&mut self) {
        self.permission_denials += 1;
    }

    /// Record a permission check result
    pub fn record_permission_check(&mut self, granted: bool) {
        self.increment_permission_checks();
        if granted {
            self.increment_permission_grants();
        } else {
            self.increment_permission_denials();
        }
    }

    /// Get permission grant rate
    pub fn grant_rate(&self) -> f64 {
        if self.permission_checks == 0 {
            0.0
        } else {
            self.permission_grants as f64 / self.permission_checks as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_stats_creation() {
        let stats = PermissionStats::new();
        assert_eq!(stats.permission_checks, 0);
        assert_eq!(stats.permission_grants, 0);
        assert_eq!(stats.permission_denials, 0);
        assert!(stats.last_permission_check.is_none());
    }

    #[test]
    fn test_permission_stats_increment() {
        let mut stats = PermissionStats::new();
        
        stats.increment_permission_checks();
        assert_eq!(stats.permission_checks, 1);
        assert!(stats.last_permission_check.is_some());
        
        stats.increment_permission_grants();
        assert_eq!(stats.permission_grants, 1);
        
        stats.increment_permission_denials();
        assert_eq!(stats.permission_denials, 1);
    }

    #[test]
    fn test_permission_stats_record_check() {
        let mut stats = PermissionStats::new();
        
        stats.record_permission_check(true);
        assert_eq!(stats.permission_checks, 1);
        assert_eq!(stats.permission_grants, 1);
        assert_eq!(stats.permission_denials, 0);
        
        stats.record_permission_check(false);
        assert_eq!(stats.permission_checks, 2);
        assert_eq!(stats.permission_grants, 1);
        assert_eq!(stats.permission_denials, 1);
    }

    #[test]
    fn test_permission_stats_grant_rate() {
        let mut stats = PermissionStats::new();
        
        // No checks yet
        assert_eq!(stats.grant_rate(), 0.0);
        
        // All grants
        stats.record_permission_check(true);
        stats.record_permission_check(true);
        assert_eq!(stats.grant_rate(), 1.0);
        
        // Mixed results
        stats.record_permission_check(false);
        assert_eq!(stats.grant_rate(), 2.0 / 3.0);
        
        // All denials
        stats.record_permission_check(false);
        stats.record_permission_check(false);
        assert_eq!(stats.grant_rate(), 2.0 / 5.0);
    }
}
