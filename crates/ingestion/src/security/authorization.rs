//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Authorization system for the ingestion platform
//!
//! This module provides comprehensive authorization capabilities including
//! role-based access control (RBAC), permission management, and audit logging.

use bridge_core::BridgeResult;
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Role-based access control (RBAC) system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacSystem {
    /// Roles and their permissions
    pub roles: HashMap<String, Role>,

    /// User role assignments
    pub user_roles: HashMap<String, HashSet<String>>,

    /// Resource permissions
    pub resource_permissions: HashMap<String, ResourcePermissions>,
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,

    /// Role description
    pub description: String,

    /// Permissions granted to this role
    pub permissions: HashSet<Permission>,

    /// Whether role is active
    pub active: bool,

    /// Role creation time
    pub created_at: DateTime<Utc>,

    /// Role last modified time
    pub modified_at: DateTime<Utc>,
}

/// Permission definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Permission {
    /// Resource being accessed
    pub resource: String,

    /// Action being performed
    pub action: Action,

    /// Conditions for permission
    pub conditions: Option<PermissionConditions>,
}

/// Action types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Action {
    /// Read access
    Read,

    /// Write access
    Write,

    /// Delete access
    Delete,

    /// Admin access
    Admin,

    /// Custom action
    Custom(String),
}

/// Permission conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PermissionConditions {
    /// Time-based restrictions
    pub time_restrictions: Option<TimeRestrictions>,

    /// IP-based restrictions
    pub ip_restrictions: Option<IpRestrictions>,

    /// Resource-based restrictions
    pub resource_restrictions: Option<ResourceRestrictions>,
}

/// Time-based restrictions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TimeRestrictions {
    /// Allowed time windows (UTC)
    pub allowed_windows: Vec<TimeWindow>,

    /// Days of week (0=Sunday, 6=Saturday)
    pub allowed_days: Option<Vec<u8>>,
}

/// Time window
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TimeWindow {
    /// Start time (HH:MM format)
    pub start: String,

    /// End time (HH:MM format)
    pub end: String,
}

/// IP-based restrictions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct IpRestrictions {
    /// Allowed IP addresses/CIDR blocks
    pub allowed_ips: Vec<String>,

    /// Blocked IP addresses/CIDR blocks
    pub blocked_ips: Vec<String>,
}

/// Resource-based restrictions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ResourceRestrictions {
    /// Allowed resource patterns
    pub allowed_patterns: Vec<String>,

    /// Blocked resource patterns
    pub blocked_patterns: Vec<String>,

    /// Maximum resource size
    pub max_size: Option<u64>,
}

/// Resource permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePermissions {
    /// Resource name
    pub resource: String,

    /// Default permissions
    pub default_permissions: HashSet<Action>,

    /// Role-specific permissions
    pub role_permissions: HashMap<String, HashSet<Action>>,

    /// Whether resource is public
    pub public: bool,
}

/// Authorization manager
pub struct AuthorizationManager {
    rbac: Arc<RwLock<RbacSystem>>,
    audit_log: Arc<RwLock<Vec<AuditLogEntry>>>,
    config: AuthorizationConfig,
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Enable authorization
    pub enabled: bool,

    /// Default role for new users
    pub default_role: String,

    /// Enable audit logging
    pub enable_audit_log: bool,

    /// Maximum audit log entries
    pub max_audit_entries: usize,

    /// Audit log retention days
    pub audit_retention_days: u32,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Entry ID
    pub id: Uuid,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// User ID
    pub user_id: String,

    /// Action performed
    pub action: String,

    /// Resource accessed
    pub resource: String,

    /// Result (success/failure)
    pub result: AuditResult,

    /// IP address
    pub ip_address: Option<String>,

    /// User agent
    pub user_agent: Option<String>,

    /// Additional details
    pub details: HashMap<String, String>,
}

/// Audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure(String),
}

impl AuthorizationManager {
    /// Create new authorization manager
    pub fn new(config: AuthorizationConfig) -> Self {
        let rbac = RbacSystem {
            roles: HashMap::new(),
            user_roles: HashMap::new(),
            resource_permissions: HashMap::new(),
        };

        Self {
            rbac: Arc::new(RwLock::new(rbac)),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Initialize authorization manager
    pub async fn initialize(&self) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Create default roles
        self.create_default_roles().await?;

        info!("Authorization manager initialized");
        Ok(())
    }

    /// Create default roles
    async fn create_default_roles(&self) -> BridgeResult<()> {
        let mut rbac = self.rbac.write().await;

        // Admin role
        let admin_role = Role {
            name: "admin".to_string(),
            description: "Administrator with full access".to_string(),
            permissions: HashSet::from([Permission {
                resource: "*".to_string(),
                action: Action::Admin,
                conditions: None,
            }]),
            active: true,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        rbac.roles.insert("admin".to_string(), admin_role);

        // User role
        let user_role = Role {
            name: "user".to_string(),
            description: "Standard user with read/write access".to_string(),
            permissions: HashSet::from([
                Permission {
                    resource: "ingestion".to_string(),
                    action: Action::Read,
                    conditions: None,
                },
                Permission {
                    resource: "ingestion".to_string(),
                    action: Action::Write,
                    conditions: None,
                },
            ]),
            active: true,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        rbac.roles.insert("user".to_string(), user_role);

        // Read-only role
        let read_only_role = Role {
            name: "readonly".to_string(),
            description: "Read-only access".to_string(),
            permissions: HashSet::from([Permission {
                resource: "ingestion".to_string(),
                action: Action::Read,
                conditions: None,
            }]),
            active: true,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        rbac.roles.insert("readonly".to_string(), read_only_role);

        Ok(())
    }

    /// Check if user has permission
    pub async fn check_permission(
        &self,
        user_id: &str,
        resource: &str,
        action: &Action,
        context: Option<AuthorizationContext>,
    ) -> BridgeResult<bool> {
        if !self.config.enabled {
            return Ok(true);
        }

        let rbac = self.rbac.read().await;

        // Get user roles
        let user_roles = rbac.user_roles.get(user_id).cloned().unwrap_or_default();

        // Check each role for permission
        for role_name in user_roles {
            if let Some(role) = rbac.roles.get(&role_name) {
                if !role.active {
                    continue;
                }

                for permission in &role.permissions {
                    if self
                        .matches_permission(permission, resource, action, &context)
                        .await?
                    {
                        self.log_audit_entry(
                            user_id,
                            &format!("{:?}", action),
                            resource,
                            AuditResult::Success,
                            context.as_ref(),
                        )
                        .await;
                        return Ok(true);
                    }
                }
            }
        }

        self.log_audit_entry(
            user_id,
            &format!("{:?}", action),
            resource,
            AuditResult::Failure("Permission denied".to_string()),
            context.as_ref(),
        )
        .await;

        Ok(false)
    }

    /// Check if permission matches
    async fn matches_permission(
        &self,
        permission: &Permission,
        resource: &str,
        action: &Action,
        context: &Option<AuthorizationContext>,
    ) -> BridgeResult<bool> {
        // Check resource match
        if permission.resource != "*" && permission.resource != resource {
            return Ok(false);
        }

        // Check action match
        if &permission.action != action {
            return Ok(false);
        }

        // Check conditions
        if let Some(conditions) = &permission.conditions {
            if let Some(context) = context {
                if let Some(time_restrictions) = &conditions.time_restrictions {
                    if !self.check_time_restrictions(time_restrictions).await? {
                        return Ok(false);
                    }
                }

                if let Some(ip_restrictions) = &conditions.ip_restrictions {
                    if let Some(ip) = &context.ip_address {
                        if !self.check_ip_restrictions(ip_restrictions, ip).await? {
                            return Ok(false);
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    /// Check time restrictions
    async fn check_time_restrictions(&self, restrictions: &TimeRestrictions) -> BridgeResult<bool> {
        let now = Utc::now();
        let current_time = now.format("%H:%M").to_string();
        let current_day = now.weekday().num_days_from_sunday();

        // Check day restrictions
        if let Some(allowed_days) = &restrictions.allowed_days {
            if !allowed_days.contains(&(current_day as u8)) {
                return Ok(false);
            }
        }

        // Check time window restrictions
        for window in &restrictions.allowed_windows {
            if current_time >= window.start && current_time <= window.end {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check IP restrictions
    async fn check_ip_restrictions(
        &self,
        restrictions: &IpRestrictions,
        ip: &str,
    ) -> BridgeResult<bool> {
        // Check blocked IPs first
        for blocked_ip in &restrictions.blocked_ips {
            if self.ip_matches(ip, blocked_ip).await? {
                return Ok(false);
            }
        }

        // Check allowed IPs
        for allowed_ip in &restrictions.allowed_ips {
            if self.ip_matches(ip, allowed_ip).await? {
                return Ok(true);
            }
        }

        // If no allowed IPs specified, deny by default
        Ok(restrictions.allowed_ips.is_empty())
    }

    /// Check if IP matches pattern
    async fn ip_matches(&self, ip: &str, pattern: &str) -> BridgeResult<bool> {
        // Simple CIDR matching - in production, use a proper IP matching library
        if pattern.contains('/') {
            // CIDR notation
            Ok(ip.starts_with(&pattern[..pattern.find('/').unwrap()]))
        } else {
            // Exact match
            Ok(ip == pattern)
        }
    }

    /// Add role to user
    pub async fn add_user_role(&self, user_id: &str, role_name: &str) -> BridgeResult<()> {
        let mut rbac = self.rbac.write().await;

        // Verify role exists
        if !rbac.roles.contains_key(role_name) {
            return Err(bridge_core::BridgeError::authentication(format!(
                "Role '{}' does not exist",
                role_name
            )));
        }

        // Add role to user
        let user_roles = rbac
            .user_roles
            .entry(user_id.to_string())
            .or_insert_with(HashSet::new);
        user_roles.insert(role_name.to_string());

        info!("Added role '{}' to user '{}'", role_name, user_id);
        Ok(())
    }

    /// Remove role from user
    pub async fn remove_user_role(&self, user_id: &str, role_name: &str) -> BridgeResult<()> {
        let mut rbac = self.rbac.write().await;

        if let Some(user_roles) = rbac.user_roles.get_mut(user_id) {
            user_roles.remove(role_name);
            info!("Removed role '{}' from user '{}'", role_name, user_id);
        }

        Ok(())
    }

    /// Create new role
    pub async fn create_role(&self, role: Role) -> BridgeResult<()> {
        let mut rbac = self.rbac.write().await;
        let role_name = role.name.clone();

        if rbac.roles.contains_key(&role_name) {
            return Err(bridge_core::BridgeError::authentication(format!(
                "Role '{}' already exists",
                role_name
            )));
        }

        rbac.roles.insert(role_name.clone(), role);
        info!("Created role '{}'", role_name);
        Ok(())
    }

    /// Update role
    pub async fn update_role(&self, role_name: &str, role: Role) -> BridgeResult<()> {
        let mut rbac = self.rbac.write().await;

        if !rbac.roles.contains_key(role_name) {
            return Err(bridge_core::BridgeError::authentication(format!(
                "Role '{}' does not exist",
                role_name
            )));
        }

        rbac.roles.insert(role_name.to_string(), role);
        info!("Updated role '{}'", role_name);
        Ok(())
    }

    /// Delete role
    pub async fn delete_role(&self, role_name: &str) -> BridgeResult<()> {
        let mut rbac = self.rbac.write().await;

        // Remove role from all users
        for user_roles in rbac.user_roles.values_mut() {
            user_roles.remove(role_name);
        }

        // Remove role
        rbac.roles.remove(role_name);
        info!("Deleted role '{}'", role_name);
        Ok(())
    }

    /// Get user roles
    pub async fn get_user_roles(&self, user_id: &str) -> BridgeResult<HashSet<String>> {
        let rbac = self.rbac.read().await;
        Ok(rbac.user_roles.get(user_id).cloned().unwrap_or_default())
    }

    /// Get role permissions
    pub async fn get_role_permissions(&self, role_name: &str) -> BridgeResult<HashSet<Permission>> {
        let rbac = self.rbac.read().await;

        if let Some(role) = rbac.roles.get(role_name) {
            Ok(role.permissions.clone())
        } else {
            Err(bridge_core::BridgeError::authentication(format!(
                "Role '{}' does not exist",
                role_name
            )))
        }
    }

    /// Log audit entry
    async fn log_audit_entry(
        &self,
        user_id: &str,
        action: &str,
        resource: &str,
        result: AuditResult,
        context: Option<&AuthorizationContext>,
    ) {
        if !self.config.enable_audit_log {
            return;
        }

        let entry = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            user_id: user_id.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            result,
            ip_address: context.and_then(|c| c.ip_address.clone()),
            user_agent: context.and_then(|c| c.user_agent.clone()),
            details: context.map(|c| c.details.clone()).unwrap_or_default(),
        };

        let mut audit_log = self.audit_log.write().await;
        audit_log.push(entry);

        // Trim audit log if it exceeds maximum size
        if audit_log.len() > self.config.max_audit_entries {
            audit_log.remove(0);
        }
    }

    /// Get audit log entries
    pub async fn get_audit_log(&self, limit: Option<usize>) -> BridgeResult<Vec<AuditLogEntry>> {
        let audit_log = self.audit_log.read().await;
        let entries: Vec<AuditLogEntry> = audit_log.iter().cloned().collect();

        if let Some(limit) = limit {
            Ok(entries.into_iter().take(limit).collect())
        } else {
            Ok(entries)
        }
    }

    /// Clean up old audit log entries
    pub async fn cleanup_audit_log(&self) -> BridgeResult<()> {
        let retention_duration = chrono::Duration::days(self.config.audit_retention_days as i64);
        let cutoff_time = Utc::now() - retention_duration;

        let mut audit_log = self.audit_log.write().await;
        audit_log.retain(|entry| entry.timestamp >= cutoff_time);

        info!("Cleaned up audit log, retained {} entries", audit_log.len());
        Ok(())
    }
}

/// Authorization context
#[derive(Debug, Clone)]
pub struct AuthorizationContext {
    /// IP address
    pub ip_address: Option<String>,

    /// User agent
    pub user_agent: Option<String>,

    /// Additional context details
    pub details: HashMap<String, String>,
}

impl AuthorizationContext {
    /// Create new authorization context
    pub fn new() -> Self {
        Self {
            ip_address: None,
            user_agent: None,
            details: HashMap::new(),
        }
    }

    /// Set IP address
    pub fn with_ip_address(mut self, ip_address: String) -> Self {
        self.ip_address = Some(ip_address);
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Add detail
    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }
}

impl Default for AuthorizationContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authorization_manager_creation() {
        let config = AuthorizationConfig {
            enabled: true,
            default_role: "user".to_string(),
            enable_audit_log: true,
            max_audit_entries: 1000,
            audit_retention_days: 30,
        };

        let manager = AuthorizationManager::new(config);
        manager.initialize().await.unwrap();

        // Test permission check
        let has_permission = manager
            .check_permission("test_user", "ingestion", &Action::Read, None)
            .await
            .unwrap();
        assert!(!has_permission); // User has no roles assigned
    }

    #[tokio::test]
    async fn test_role_management() {
        let config = AuthorizationConfig {
            enabled: true,
            default_role: "user".to_string(),
            enable_audit_log: true,
            max_audit_entries: 1000,
            audit_retention_days: 30,
        };

        let manager = AuthorizationManager::new(config);
        manager.initialize().await.unwrap();

        // Add role to user
        manager.add_user_role("test_user", "user").await.unwrap();

        // Check permission
        let has_permission = manager
            .check_permission("test_user", "ingestion", &Action::Read, None)
            .await
            .unwrap();
        assert!(has_permission);

        // Remove role
        manager.remove_user_role("test_user", "user").await.unwrap();

        // Check permission again
        let has_permission = manager
            .check_permission("test_user", "ingestion", &Action::Read, None)
            .await
            .unwrap();
        assert!(!has_permission);
    }
}
