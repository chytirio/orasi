//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! User management functionality

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::config::UserConfig;
use crate::oauth::OAuthUser;

use super::model::{User, UserRole};
use super::stats::UserStats;

/// User manager
#[derive(Debug)]
pub struct UserManager {
    /// User configuration
    config: UserConfig,

    /// Users storage (in production, this would be a database)
    users: Arc<RwLock<HashMap<String, User>>>,

    /// Username to user ID mapping
    username_to_id: Arc<RwLock<HashMap<String, String>>>,

    /// Email to user ID mapping
    email_to_id: Arc<RwLock<HashMap<String, String>>>,

    /// Statistics
    stats: Arc<RwLock<UserStats>>,
}

impl UserManager {
    /// Create new user manager
    pub async fn new(config: UserConfig) -> crate::AuthResult<Self> {
        Ok(Self {
            config,
            users: Arc::new(RwLock::new(HashMap::new())),
            username_to_id: Arc::new(RwLock::new(HashMap::new())),
            email_to_id: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(UserStats::default())),
        })
    }

    /// Create new user
    pub async fn create_user(
        &self,
        username: String,
        email: String,
        password: String,
        roles: Vec<UserRole>,
    ) -> crate::AuthResult<User> {
        // Validate password
        self.validate_password(&password)?;

        // Check if username already exists
        {
            let username_map = self.username_to_id.read().await;
            if username_map.contains_key(&username) {
                return Err(crate::AuthError::internal(
                    "Username already exists".to_string(),
                ));
            }
        }

        // Check if email already exists
        {
            let email_map = self.email_to_id.read().await;
            if email_map.contains_key(&email) {
                return Err(crate::AuthError::internal(
                    "Email already exists".to_string(),
                ));
            }
        }

        // Hash password
        let password_hash = self.hash_password(&password)?;

        // Create user
        let user = User::new(username.clone(), email.clone(), Some(password_hash), roles);

        // Store user
        {
            let mut users = self.users.write().await;
            users.insert(user.id.clone(), user.clone());
        }

        // Store mappings
        {
            let mut username_map = self.username_to_id.write().await;
            username_map.insert(username, user.id.clone());
        }

        {
            let mut email_map = self.email_to_id.write().await;
            email_map.insert(email, user.id.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.increment_users_created();
        }

        info!("Created user: {}", user.username);
        Ok(user)
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> Option<User> {
        let users = self.users.read().await;
        users.get(user_id).cloned()
    }

    /// Get user by username
    pub async fn get_user_by_username(&self, username: &str) -> Option<User> {
        let user_id = {
            let username_map = self.username_to_id.read().await;
            username_map.get(username).cloned()
        };

        if let Some(user_id) = user_id {
            self.get_user_by_id(&user_id).await
        } else {
            None
        }
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Option<User> {
        let user_id = {
            let email_map = self.email_to_id.read().await;
            email_map.get(email).cloned()
        };

        if let Some(user_id) = user_id {
            self.get_user_by_id(&user_id).await
        } else {
            None
        }
    }

    /// Get or create OAuth user
    pub async fn get_or_create_oauth_user(
        &self,
        oauth_user: &OAuthUser,
    ) -> crate::AuthResult<User> {
        // Try to find existing user by OAuth provider and ID
        let existing_user = {
            let users = self.users.read().await;
            users
                .values()
                .find(|user| {
                    user.oauth_provider.as_ref() == Some(&oauth_user.provider)
                        && user.oauth_provider_user_id.as_ref()
                            == Some(&oauth_user.provider_user_id)
                })
                .cloned()
        };

        if let Some(user) = existing_user {
            return Ok(user);
        }

        // Try to find by email
        if let Some(user) = self.get_user_by_email(&oauth_user.email).await {
            // Update user with OAuth information
            let mut updated_user = user.clone();
            updated_user.oauth_provider = Some(oauth_user.provider.clone());
            updated_user.oauth_provider_user_id = Some(oauth_user.provider_user_id.clone());

            // Update stored user
            {
                let mut users = self.users.write().await;
                users.insert(user.id.clone(), updated_user.clone());
            }

            return Ok(updated_user);
        }

        // Create new user
        let username = format!(
            "oauth_{}_{}",
            oauth_user.provider, oauth_user.provider_user_id
        );
        let roles = vec![UserRole::User];

        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: username.clone(),
            email: oauth_user.email.clone(),
            password_hash: None,
            roles,
            is_active: true,
            is_locked: false,
            failed_login_attempts: 0,
            lockout_until: None,
            created_at: chrono::Utc::now(),
            last_login_at: None,
            oauth_provider: Some(oauth_user.provider.clone()),
            oauth_provider_user_id: Some(oauth_user.provider_user_id.clone()),
        };

        // Store user
        {
            let mut users = self.users.write().await;
            users.insert(user.id.clone(), user.clone());
        }

        // Store mappings
        {
            let mut username_map = self.username_to_id.write().await;
            username_map.insert(username, user.id.clone());
        }

        {
            let mut email_map = self.email_to_id.write().await;
            email_map.insert(oauth_user.email.clone(), user.id.clone());
        }

        info!("Created OAuth user: {}", user.username);
        Ok(user)
    }

    /// Verify password
    pub async fn verify_password(&self, user_id: &str, password: &str) -> crate::AuthResult<bool> {
        let user = self
            .get_user_by_id(user_id)
            .await
            .ok_or_else(|| crate::AuthError::user_not_found("User not found".to_string()))?;

        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or_else(|| crate::AuthError::internal("User has no password hash".to_string()))?;

        Ok(self.verify_password_hash(password, password_hash))
    }

    /// Increment failed login attempts
    pub async fn increment_failed_login_attempts(&self, user_id: &str) -> crate::AuthResult<()> {
        let mut user = self
            .get_user_by_id(user_id)
            .await
            .ok_or_else(|| crate::AuthError::user_not_found("User not found".to_string()))?;

        user.failed_login_attempts += 1;

        // Check if account should be locked
        if user.failed_login_attempts >= self.config.max_login_attempts as u32 {
            user.is_locked = true;
            user.lockout_until = Some(
                chrono::Utc::now() + chrono::Duration::seconds(self.config.lockout_duration_secs as i64),
            );
        }

        // Update stored user
        {
            let mut users = self.users.write().await;
            users.insert(user_id.to_string(), user);
        }

        Ok(())
    }

    /// Reset failed login attempts
    pub async fn reset_failed_login_attempts(&self, user_id: &str) -> crate::AuthResult<()> {
        let mut user = self
            .get_user_by_id(user_id)
            .await
            .ok_or_else(|| crate::AuthError::user_not_found("User not found".to_string()))?;

        user.failed_login_attempts = 0;
        user.is_locked = false;
        user.lockout_until = None;
        user.last_login_at = Some(chrono::Utc::now());

        // Update stored user
        {
            let mut users = self.users.write().await;
            users.insert(user_id.to_string(), user);
        }

        Ok(())
    }

    /// Validate password
    fn validate_password(&self, password: &str) -> crate::AuthResult<()> {
        if password.len() < self.config.min_password_length {
            return Err(crate::AuthError::internal(format!(
                "Password must be at least {} characters long",
                self.config.min_password_length
            )));
        }

        let complexity = &self.config.password_complexity;

        if complexity.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(crate::AuthError::internal(
                "Password must contain uppercase letters".to_string(),
            ));
        }

        if complexity.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            return Err(crate::AuthError::internal(
                "Password must contain lowercase letters".to_string(),
            ));
        }

        if complexity.require_numbers && !password.chars().any(|c| c.is_numeric()) {
            return Err(crate::AuthError::internal(
                "Password must contain numbers".to_string(),
            ));
        }

        if complexity.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
            return Err(crate::AuthError::internal(
                "Password must contain special characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Hash password
    fn hash_password(&self, password: &str) -> crate::AuthResult<String> {
        // For now, use a simple hash since SaltString is not available
        // In a real implementation, you would use proper password hashing
        let password_hash = format!("hashed_{}", password);
        Ok(password_hash)
    }

    /// Verify password hash
    fn verify_password_hash(&self, password: &str, hash: &str) -> bool {
        // For now, use simple verification since proper hashing is not implemented
        // In a real implementation, you would use proper password verification
        hash == format!("hashed_{}", password)
    }

    /// Get user statistics
    pub async fn get_stats(&self) -> UserStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;

    #[tokio::test]
    async fn test_user_manager_creation() {
        let config = UserConfig::default();
        let result = UserManager::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_user_creation() {
        let config = UserConfig::default();
        let user_manager = UserManager::new(config).await.unwrap();

        let username = "testuser".to_string();
        let email = "test@example.com".to_string();
        let password = "TestPassword123".to_string();
        let roles = vec![UserRole::User];

        let user = user_manager
            .create_user(username.clone(), email.clone(), password, roles)
            .await
            .unwrap();
        assert_eq!(user.username, username);
        assert_eq!(user.email, email);
        assert!(user.password_hash.is_some());
    }
}
