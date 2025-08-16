//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Permission model definitions

use serde::{Deserialize, Serialize};

/// Access level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccessLevel {
    None,
    Read,
    Write,
    Admin,
}

impl AccessLevel {
    /// Check if this access level includes read access
    pub fn can_read(&self) -> bool {
        matches!(self, AccessLevel::Read | AccessLevel::Write | AccessLevel::Admin)
    }

    /// Check if this access level includes write access
    pub fn can_write(&self) -> bool {
        matches!(self, AccessLevel::Write | AccessLevel::Admin)
    }

    /// Check if this access level includes admin access
    pub fn is_admin(&self) -> bool {
        matches!(self, AccessLevel::Admin)
    }

    /// Get the numeric value of the access level
    pub fn numeric_value(&self) -> u8 {
        match self {
            AccessLevel::None => 0,
            AccessLevel::Read => 1,
            AccessLevel::Write => 2,
            AccessLevel::Admin => 3,
        }
    }

    /// Check if this access level is at least as high as the given level
    pub fn is_at_least(&self, other: &AccessLevel) -> bool {
        self.numeric_value() >= other.numeric_value()
    }
}

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessLevel::None => write!(f, "None"),
            AccessLevel::Read => write!(f, "Read"),
            AccessLevel::Write => write!(f, "Write"),
            AccessLevel::Admin => write!(f, "Admin"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_level_capabilities() {
        assert!(!AccessLevel::None.can_read());
        assert!(AccessLevel::Read.can_read());
        assert!(AccessLevel::Write.can_read());
        assert!(AccessLevel::Admin.can_read());

        assert!(!AccessLevel::None.can_write());
        assert!(!AccessLevel::Read.can_write());
        assert!(AccessLevel::Write.can_write());
        assert!(AccessLevel::Admin.can_write());

        assert!(!AccessLevel::None.is_admin());
        assert!(!AccessLevel::Read.is_admin());
        assert!(!AccessLevel::Write.is_admin());
        assert!(AccessLevel::Admin.is_admin());
    }

    #[test]
    fn test_access_level_numeric_values() {
        assert_eq!(AccessLevel::None.numeric_value(), 0);
        assert_eq!(AccessLevel::Read.numeric_value(), 1);
        assert_eq!(AccessLevel::Write.numeric_value(), 2);
        assert_eq!(AccessLevel::Admin.numeric_value(), 3);
    }

    #[test]
    fn test_access_level_comparison() {
        assert!(AccessLevel::Admin.is_at_least(&AccessLevel::Read));
        assert!(AccessLevel::Write.is_at_least(&AccessLevel::Read));
        assert!(AccessLevel::Read.is_at_least(&AccessLevel::None));
        assert!(!AccessLevel::Read.is_at_least(&AccessLevel::Write));
        assert!(!AccessLevel::None.is_at_least(&AccessLevel::Read));
    }

    #[test]
    fn test_access_level_display() {
        assert_eq!(AccessLevel::None.to_string(), "None");
        assert_eq!(AccessLevel::Read.to_string(), "Read");
        assert_eq!(AccessLevel::Write.to_string(), "Write");
        assert_eq!(AccessLevel::Admin.to_string(), "Admin");
    }
}
