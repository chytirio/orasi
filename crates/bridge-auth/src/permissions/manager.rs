//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Permission management functionality

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::model::AccessLevel;
use super::stats::PermissionStats;

/// Permission manager
#[derive(Debug)]
pub struct PermissionManager {
    /// User permissions cache
    user_permissions: Arc<RwLock<HashMap<String, Vec<String>>>>,

    /// Statistics
    stats: Arc<RwLock<PermissionStats>>,
}

impl PermissionManager {
    /// Create new permission manager
    pub async fn new() -> crate::AuthResult<Self> {
        Ok(Self {
            user_permissions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PermissionStats::default())),
        })
    }

    /// Get user permissions
    pub async fn get_user_permissions(&self, user_id: &str) -> Vec<String> {
        let permissions = self.user_permissions.read().await;
        permissions.get(user_id).cloned().unwrap_or_default()
    }

    /// Set user permissions
    pub async fn set_user_permissions(&self, user_id: &str, permissions: Vec<String>) {
        {
            let mut user_permissions = self.user_permissions.write().await;
            user_permissions.insert(user_id.to_string(), permissions);
        }

        info!("Set permissions for user: {}", user_id);
    }

    /// Add permission to user
    pub async fn add_user_permission(&self, user_id: &str, permission: String) {
        {
            let mut user_permissions = self.user_permissions.write().await;
            let permissions = user_permissions
                .entry(user_id.to_string())
                .or_insert_with(Vec::new);
            if !permissions.contains(&permission) {
                permissions.push(permission.clone());
            }
        }

        info!("Added permission {} to user: {}", permission, user_id);
    }

    /// Remove permission from user
    pub async fn remove_user_permission(&self, user_id: &str, permission: &str) {
        {
            let mut user_permissions = self.user_permissions.write().await;
            if let Some(permissions) = user_permissions.get_mut(user_id) {
                permissions.retain(|p| p != permission);
            }
        }

        info!("Removed permission {} from user: {}", permission, user_id);
    }

    /// Check if user has permission
    pub async fn user_has_permission(&self, user_id: &str, permission: &str) -> bool {
        let permissions = self.get_user_permissions(user_id).await;
        let has_permission = permissions.iter().any(|p| p == permission);

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.record_permission_check(has_permission);
        }

        has_permission
    }

    /// Check if user has any of the specified permissions
    pub async fn user_has_any_permission(&self, user_id: &str, permissions: &[String]) -> bool {
        let user_permissions = self.get_user_permissions(user_id).await;
        let has_any = permissions.iter().any(|p| user_permissions.contains(p));

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.record_permission_check(has_any);
        }

        has_any
    }

    /// Check if user has all of the specified permissions
    pub async fn user_has_all_permissions(&self, user_id: &str, permissions: &[String]) -> bool {
        let user_permissions = self.get_user_permissions(user_id).await;
        let has_all = permissions.iter().all(|p| user_permissions.contains(p));

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.record_permission_check(has_all);
        }

        has_all
    }

    /// Get user access level for a resource
    pub async fn get_user_access_level(&self, user_id: &str, resource: &str) -> AccessLevel {
        let permissions = self.get_user_permissions(user_id).await;

        // Check for admin permission first
        if permissions
            .iter()
            .any(|p| p == "admin:all" || p == &format!("{}:admin", resource))
        {
            return AccessLevel::Admin;
        }

        // Check for write permission
        if permissions
            .iter()
            .any(|p| p == &format!("{}:write", resource))
        {
            return AccessLevel::Write;
        }

        // Check for read permission
        if permissions
            .iter()
            .any(|p| p == &format!("{}:read", resource))
        {
            return AccessLevel::Read;
        }

        AccessLevel::None
    }

    /// Get permission statistics
    pub async fn get_stats(&self) -> PermissionStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_permission_manager_creation() {
        let result = PermissionManager::new().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_user_permissions() {
        let permission_manager = PermissionManager::new().await.unwrap();

        let user_id = "test-user";
        let permissions = vec!["read:metrics".to_string(), "write:logs".to_string()];

        // Set permissions
        permission_manager
            .set_user_permissions(user_id, permissions.clone())
            .await;

        // Get permissions
        let retrieved_permissions = permission_manager.get_user_permissions(user_id).await;
        assert_eq!(retrieved_permissions, permissions);

        // Check specific permission
        assert!(
            permission_manager
                .user_has_permission(user_id, "read:metrics")
                .await
        );
        assert!(
            !permission_manager
                .user_has_permission(user_id, "admin:all")
                .await
        );
    }

    #[tokio::test]
    async fn test_permission_management() {
        let permission_manager = PermissionManager::new().await.unwrap();
        let user_id = "test-user";

        // Add permission
        permission_manager
            .add_user_permission(user_id, "read:metrics".to_string())
            .await;

        assert!(
            permission_manager
                .user_has_permission(user_id, "read:metrics")
                .await
        );

        // Remove permission
        permission_manager
            .remove_user_permission(user_id, "read:metrics")
            .await;

        assert!(
            !permission_manager
                .user_has_permission(user_id, "read:metrics")
                .await
        );
    }

    #[tokio::test]
    async fn test_access_levels() {
        let permission_manager = PermissionManager::new().await.unwrap();
        let user_id = "test-user";

        // Test no access
        let access_level = permission_manager
            .get_user_access_level(user_id, "metrics")
            .await;
        assert_eq!(access_level, AccessLevel::None);

        // Test read access
        permission_manager
            .add_user_permission(user_id, "metrics:read".to_string())
            .await;
        let access_level = permission_manager
            .get_user_access_level(user_id, "metrics")
            .await;
        assert_eq!(access_level, AccessLevel::Read);

        // Test write access
        permission_manager
            .add_user_permission(user_id, "metrics:write".to_string())
            .await;
        let access_level = permission_manager
            .get_user_access_level(user_id, "metrics")
            .await;
        assert_eq!(access_level, AccessLevel::Write);

        // Test admin access
        permission_manager
            .add_user_permission(user_id, "metrics:admin".to_string())
            .await;
        let access_level = permission_manager
            .get_user_access_level(user_id, "metrics")
            .await;
        assert_eq!(access_level, AccessLevel::Admin);
    }

    #[tokio::test]
    async fn test_permission_statistics() {
        let permission_manager = PermissionManager::new().await.unwrap();
        let user_id = "test-user";

        // Check permissions to generate stats
        permission_manager
            .add_user_permission(user_id, "read:test".to_string())
            .await;

        permission_manager
            .user_has_permission(user_id, "read:test")
            .await;
        permission_manager
            .user_has_permission(user_id, "write:test")
            .await;

        let stats = permission_manager.get_stats().await;
        assert_eq!(stats.permission_checks, 2);
        assert_eq!(stats.permission_grants, 1);
        assert_eq!(stats.permission_denials, 1);
        assert!(stats.last_permission_check.is_some());
    }
}
