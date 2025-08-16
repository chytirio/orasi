//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Role and permission model definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Permission structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    /// Permission ID
    pub id: String,

    /// Permission name
    pub name: String,

    /// Permission description
    pub description: String,

    /// Resource
    pub resource: String,

    /// Action
    pub action: String,

    /// Creation time
    pub created_at: DateTime<Utc>,
}

impl Permission {
    /// Create new permission
    pub fn new(id: String, name: String, description: String, resource: String, action: String) -> Self {
        Self {
            id,
            name,
            description,
            resource,
            action,
            created_at: Utc::now(),
        }
    }

    /// Get permission string (resource:action format)
    pub fn permission_string(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }
}

/// Role structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role ID
    pub id: String,

    /// Role name
    pub name: String,

    /// Role description
    pub description: String,

    /// Permissions
    pub permissions: Vec<String>,

    /// Parent roles (for inheritance)
    pub parent_roles: Vec<String>,

    /// Creation time
    pub created_at: DateTime<Utc>,
}

impl Role {
    /// Create new role
    pub fn new(
        id: String,
        name: String,
        description: String,
        permissions: Vec<String>,
        parent_roles: Vec<String>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            permissions,
            parent_roles,
            created_at: Utc::now(),
        }
    }

    /// Check if role has permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    /// Add permission to role
    pub fn add_permission(&mut self, permission: String) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }

    /// Remove permission from role
    pub fn remove_permission(&mut self, permission: &str) {
        self.permissions.retain(|p| p != permission);
    }

    /// Add parent role
    pub fn add_parent_role(&mut self, parent_role: String) {
        if !self.parent_roles.contains(&parent_role) {
            self.parent_roles.push(parent_role);
        }
    }

    /// Remove parent role
    pub fn remove_parent_role(&mut self, parent_role: &str) {
        self.parent_roles.retain(|p| p != parent_role);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let permission = Permission::new(
            "test-perm".to_string(),
            "Test Permission".to_string(),
            "A test permission".to_string(),
            "test".to_string(),
            "read".to_string(),
        );

        assert_eq!(permission.id, "test-perm");
        assert_eq!(permission.name, "Test Permission");
        assert_eq!(permission.permission_string(), "test:read");
    }

    #[test]
    fn test_role_creation() {
        let permissions = vec!["read:test".to_string(), "write:test".to_string()];
        let role = Role::new(
            "test-role".to_string(),
            "Test Role".to_string(),
            "A test role".to_string(),
            permissions.clone(),
            vec![],
        );

        assert_eq!(role.id, "test-role");
        assert_eq!(role.name, "Test Role");
        assert_eq!(role.permissions, permissions);
        assert!(role.has_permission("read:test"));
        assert!(!role.has_permission("delete:test"));
    }

    #[test]
    fn test_role_permission_management() {
        let mut role = Role::new(
            "test-role".to_string(),
            "Test Role".to_string(),
            "A test role".to_string(),
            vec![],
            vec![],
        );

        role.add_permission("read:test".to_string());
        assert!(role.has_permission("read:test"));

        role.remove_permission("read:test");
        assert!(!role.has_permission("read:test"));
    }

    #[test]
    fn test_role_parent_management() {
        let mut role = Role::new(
            "test-role".to_string(),
            "Test Role".to_string(),
            "A test role".to_string(),
            vec![],
            vec![],
        );

        role.add_parent_role("parent-role".to_string());
        assert!(role.parent_roles.contains(&"parent-role".to_string()));

        role.remove_parent_role("parent-role");
        assert!(!role.parent_roles.contains(&"parent-role".to_string()));
    }
}
