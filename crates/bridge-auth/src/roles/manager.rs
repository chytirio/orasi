//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Role management functionality

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::config::RbacConfig;

use super::model::Role;
use super::stats::RoleStats;

/// Role manager
#[derive(Debug)]
pub struct RoleManager {
    /// RBAC configuration
    config: RbacConfig,

    /// Roles storage
    roles: Arc<RwLock<HashMap<String, Role>>>,

    /// User role assignments
    user_roles: Arc<RwLock<HashMap<String, Vec<String>>>>,

    /// Statistics
    stats: Arc<RwLock<RoleStats>>,
}

impl RoleManager {
    /// Create new role manager
    pub async fn new(config: RbacConfig) -> crate::AuthResult<Self> {
        let role_manager = Self {
            config,
            roles: Arc::new(RwLock::new(HashMap::new())),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RoleStats::default())),
        };

        // Initialize default roles
        role_manager.initialize_default_roles().await;

        Ok(role_manager)
    }

    /// Initialize default roles
    async fn initialize_default_roles(&self) {
        // Create admin role
        let admin_role = Role::new(
            "admin".to_string(),
            "Admin".to_string(),
            "Administrator role with full access".to_string(),
            vec![
                "metrics:read".to_string(),
                "metrics:write".to_string(),
                "traces:read".to_string(),
                "traces:write".to_string(),
                "logs:read".to_string(),
                "logs:write".to_string(),
                "users:read".to_string(),
                "users:write".to_string(),
                "roles:read".to_string(),
                "roles:write".to_string(),
            ],
            Vec::new(),
        );

        // Create user role
        let user_role = Role::new(
            "user".to_string(),
            "User".to_string(),
            "Standard user role".to_string(),
            vec![
                "metrics:read".to_string(),
                "traces:read".to_string(),
                "logs:read".to_string(),
            ],
            Vec::new(),
        );

        // Create read-only role
        let readonly_role = Role::new(
            "readonly".to_string(),
            "Read Only".to_string(),
            "Read-only access".to_string(),
            vec![
                "metrics:read".to_string(),
                "traces:read".to_string(),
                "logs:read".to_string(),
            ],
            Vec::new(),
        );

        // Store roles
        {
            let mut roles = self.roles.write().await;
            roles.insert(admin_role.id.clone(), admin_role);
            roles.insert(user_role.id.clone(), user_role);
            roles.insert(readonly_role.id.clone(), readonly_role);
        }

        info!("Initialized default roles");
    }

    /// Get user roles
    pub async fn get_user_roles(&self, user_id: &str) -> Vec<String> {
        let user_roles = self.user_roles.read().await;
        user_roles.get(user_id).cloned().unwrap_or_default()
    }

    /// Assign role to user
    pub async fn assign_role_to_user(&self, user_id: &str, role_id: &str) -> crate::AuthResult<()> {
        // Check if role exists
        {
            let roles = self.roles.read().await;
            if !roles.contains_key(role_id) {
                return Err(crate::AuthError::internal("Role not found".to_string()));
            }
        }

        // Check if user has reached maximum roles limit
        let current_roles = self.get_user_roles(user_id).await;
        if current_roles.len() >= self.config.max_roles_per_user {
            return Err(crate::AuthError::internal(format!(
                "User has reached maximum roles limit ({})",
                self.config.max_roles_per_user
            )));
        }

        // Assign role
        {
            let mut user_roles = self.user_roles.write().await;
            let roles = user_roles
                .entry(user_id.to_string())
                .or_insert_with(Vec::new);
            if !roles.contains(&role_id.to_string()) {
                roles.push(role_id.to_string());
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.increment_role_assignments();
        }

        info!("Assigned role {} to user {}", role_id, user_id);
        Ok(())
    }

    /// Remove role from user
    pub async fn remove_role_from_user(
        &self,
        user_id: &str,
        role_id: &str,
    ) -> crate::AuthResult<()> {
        {
            let mut user_roles = self.user_roles.write().await;
            if let Some(roles) = user_roles.get_mut(user_id) {
                roles.retain(|r| r != role_id);
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.increment_role_removals();
        }

        info!("Removed role {} from user {}", role_id, user_id);
        Ok(())
    }

    /// Get role by ID
    pub async fn get_role_by_id(&self, role_id: &str) -> Option<Role> {
        let roles = self.roles.read().await;
        roles.get(role_id).cloned()
    }

    /// Create new role
    pub async fn create_role(
        &self,
        name: String,
        description: String,
        permissions: Vec<String>,
        parent_roles: Vec<String>,
    ) -> crate::AuthResult<Role> {
        let role = Role::new(
            Uuid::new_v4().to_string(),
            name,
            description,
            permissions,
            parent_roles,
        );

        {
            let mut roles = self.roles.write().await;
            roles.insert(role.id.clone(), role.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.increment_roles_created();
        }

        info!("Created role: {}", role.name);
        Ok(role)
    }

    /// Get role statistics
    pub async fn get_stats(&self) -> RoleStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RbacConfig;

    #[tokio::test]
    async fn test_role_manager_creation() {
        let config = RbacConfig::default();
        let result = RoleManager::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_role_assignment() {
        let config = RbacConfig::default();
        let role_manager = RoleManager::new(config).await.unwrap();

        let user_id = "test-user";
        let role_id = "user";

        // Assign role
        role_manager
            .assign_role_to_user(user_id, role_id)
            .await
            .unwrap();

        // Get user roles
        let roles = role_manager.get_user_roles(user_id).await;
        assert!(roles.contains(&role_id.to_string()));
    }
}
