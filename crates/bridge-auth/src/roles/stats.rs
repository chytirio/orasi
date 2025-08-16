//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Role statistics

use chrono::{DateTime, Utc};

/// Role statistics
#[derive(Debug, Clone, Default)]
pub struct RoleStats {
    /// Number of roles created
    pub roles_created: u64,

    /// Number of role assignments
    pub role_assignments: u64,

    /// Number of role removals
    pub role_removals: u64,

    /// Last role created
    pub last_role_created: Option<DateTime<Utc>>,

    /// Last role assignment
    pub last_role_assignment: Option<DateTime<Utc>>,
}

impl RoleStats {
    /// Create new role statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment roles created count
    pub fn increment_roles_created(&mut self) {
        self.roles_created += 1;
        self.last_role_created = Some(Utc::now());
    }

    /// Increment role assignments count
    pub fn increment_role_assignments(&mut self) {
        self.role_assignments += 1;
        self.last_role_assignment = Some(Utc::now());
    }

    /// Increment role removals count
    pub fn increment_role_removals(&mut self) {
        self.role_removals += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_stats_creation() {
        let stats = RoleStats::new();
        assert_eq!(stats.roles_created, 0);
        assert_eq!(stats.role_assignments, 0);
        assert_eq!(stats.role_removals, 0);
        assert!(stats.last_role_created.is_none());
        assert!(stats.last_role_assignment.is_none());
    }

    #[test]
    fn test_role_stats_increment() {
        let mut stats = RoleStats::new();
        
        stats.increment_roles_created();
        assert_eq!(stats.roles_created, 1);
        assert!(stats.last_role_created.is_some());
        
        stats.increment_role_assignments();
        assert_eq!(stats.role_assignments, 1);
        assert!(stats.last_role_assignment.is_some());
        
        stats.increment_role_removals();
        assert_eq!(stats.role_removals, 1);
    }
}
