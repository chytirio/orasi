//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! API key authentication

use base64::Engine;
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use crate::config::ApiKeyConfig;

/// API key structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// API key ID
    pub id: String,

    /// User ID
    pub user_id: String,

    /// API key name/description
    pub name: String,

    /// API key hash
    pub key_hash: String,

    /// API key prefix (for identification)
    pub prefix: String,

    /// Creation time
    pub created_at: DateTime<Utc>,

    /// Expiration time
    pub expires_at: DateTime<Utc>,

    /// Last used time
    pub last_used_at: Option<DateTime<Utc>>,

    /// Whether the key is active
    pub is_active: bool,

    /// Permissions associated with this key
    pub permissions: Vec<String>,
}

impl ApiKey {
    /// Create new API key
    pub fn new(
        user_id: String,
        name: String,
        config: &ApiKeyConfig,
        permissions: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(config.expiration_secs as i64);

        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            name,
            key_hash: String::new(), // Will be set after generation
            prefix: config.prefix.clone(),
            created_at: now,
            expires_at,
            last_used_at: None,
            is_active: true,
            permissions,
        }
    }

    /// Check if key is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Update last used time
    pub fn update_last_used(&mut self) {
        self.last_used_at = Some(Utc::now());
    }
}

/// API key manager
#[derive(Debug)]
pub struct ApiKeyManager {
    /// API key configuration
    config: ApiKeyConfig,

    /// API keys storage (in production, this would be a database)
    keys: Arc<RwLock<HashMap<String, ApiKey>>>,

    /// Key hash to key ID mapping
    key_hash_to_id: Arc<RwLock<HashMap<String, String>>>,

    /// Statistics
    stats: Arc<RwLock<ApiKeyStats>>,
}

impl ApiKeyManager {
    /// Create new API key manager
    pub async fn new(config: ApiKeyConfig) -> crate::AuthResult<Self> {
        Ok(Self {
            config,
            keys: Arc::new(RwLock::new(HashMap::new())),
            key_hash_to_id: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ApiKeyStats::default())),
        })
    }

    /// Generate new API key
    pub async fn generate_key(
        &self,
        user_id: String,
        name: String,
        permissions: Vec<String>,
    ) -> crate::AuthResult<(ApiKey, String)> {
        // Check if user has reached maximum keys limit
        let user_keys = self.get_user_keys(&user_id).await;
        if user_keys.len() >= self.config.max_keys_per_user {
            return Err(crate::AuthError::internal(format!(
                "User has reached maximum API keys limit ({})",
                self.config.max_keys_per_user
            )));
        }

        // Generate API key
        let mut api_key = ApiKey::new(user_id, name, &self.config, permissions);
        let raw_key = self.generate_raw_key();

        // Hash the key
        let key_hash = self.hash_key(&raw_key);
        api_key.key_hash = key_hash.clone();

        // Store the key
        {
            let mut keys = self.keys.write().await;
            keys.insert(api_key.id.clone(), api_key.clone());
        }

        // Store hash mapping
        {
            let mut hash_map = self.key_hash_to_id.write().await;
            hash_map.insert(key_hash, api_key.id.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.keys_generated += 1;
            stats.last_key_generated = Some(Utc::now());
        }

        info!("Generated API key for user: {}", api_key.user_id);
        Ok((api_key, raw_key))
    }

    /// Validate API key
    pub async fn validate_key(&self, key: &str) -> crate::AuthResult<ApiKey> {
        // Hash the provided key
        let key_hash = self.hash_key(key);

        // Find key by hash
        let key_id = {
            let hash_map = self.key_hash_to_id.read().await;
            hash_map.get(&key_hash).cloned()
        };

        let key_id = key_id
            .ok_or_else(|| crate::AuthError::invalid_credentials("Invalid API key".to_string()))?;

        // Get the key
        let api_key = {
            let keys = self.keys.read().await;
            keys.get(&key_id).cloned()
        };

        let mut api_key = api_key.ok_or_else(|| {
            crate::AuthError::invalid_credentials("API key not found".to_string())
        })?;

        // Check if key is active
        if !api_key.is_active {
            return Err(crate::AuthError::invalid_credentials(
                "API key is deactivated".to_string(),
            ));
        }

        // Check if key is expired
        if api_key.is_expired() {
            return Err(crate::AuthError::invalid_credentials(
                "API key has expired".to_string(),
            ));
        }

        // Update last used time
        api_key.update_last_used();

        // Update stored key
        {
            let mut keys = self.keys.write().await;
            keys.insert(key_id, api_key.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.keys_validated += 1;
            stats.last_key_validated = Some(Utc::now());
        }

        debug!("Validated API key: {}", api_key.id);
        Ok(api_key)
    }

    /// Revoke API key
    pub async fn revoke_key(&self, key_id: &str) -> crate::AuthResult<()> {
        let api_key = {
            let mut keys = self.keys.write().await;
            keys.get_mut(key_id).cloned()
        };

        let mut api_key =
            api_key.ok_or_else(|| crate::AuthError::internal("API key not found".to_string()))?;

        api_key.is_active = false;

        // Update stored key
        {
            let mut keys = self.keys.write().await;
            keys.insert(key_id.to_string(), api_key.clone());
        }

        // Remove from hash mapping
        {
            let mut hash_map = self.key_hash_to_id.write().await;
            hash_map.remove(&api_key.key_hash);
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.keys_revoked += 1;
            stats.last_key_revoked = Some(Utc::now());
        }

        info!("Revoked API key: {}", key_id);
        Ok(())
    }

    /// Get user's API keys
    pub async fn get_user_keys(&self, user_id: &str) -> Vec<ApiKey> {
        let keys = self.keys.read().await;
        keys.values()
            .filter(|key| key.user_id == user_id)
            .cloned()
            .collect()
    }

    /// Get API key by ID
    pub async fn get_key_by_id(&self, key_id: &str) -> Option<ApiKey> {
        let keys = self.keys.read().await;
        keys.get(key_id).cloned()
    }

    /// Clean up expired keys
    pub async fn cleanup_expired_keys(&self) -> crate::AuthResult<usize> {
        let mut removed_count = 0;
        let mut expired_keys = Vec::new();

        // Find expired keys
        {
            let keys = self.keys.read().await;
            for (key_id, key) in keys.iter() {
                if key.is_expired() {
                    expired_keys.push(key_id.clone());
                }
            }
        }

        // Remove expired keys
        for key_id in expired_keys {
            self.revoke_key(&key_id).await?;
            removed_count += 1;
        }

        if removed_count > 0 {
            debug!("Cleaned up {} expired API keys", removed_count);
        }

        Ok(removed_count)
    }

    /// Get API key statistics
    pub async fn get_stats(&self) -> ApiKeyStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Generate raw API key
    fn generate_raw_key(&self) -> String {
        let mut rng = rand::thread_rng();
        let key_length = self.config.key_length;

        // Generate random bytes
        let bytes: Vec<u8> = (0..key_length).map(|_| rng.gen()).collect();

        // Encode as base64
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
    }

    /// Hash API key
    fn hash_key(&self, key: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// API key statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct ApiKeyStats {
    /// Number of keys generated
    pub keys_generated: u64,

    /// Number of keys validated
    pub keys_validated: u64,

    /// Number of keys revoked
    pub keys_revoked: u64,

    /// Number of validation failures
    pub validation_failures: u64,

    /// Last key generated
    pub last_key_generated: Option<DateTime<Utc>>,

    /// Last key validated
    pub last_key_validated: Option<DateTime<Utc>>,

    /// Last key revoked
    pub last_key_revoked: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiKeyConfig;

    #[tokio::test]
    async fn test_api_key_manager_creation() {
        let config = ApiKeyConfig::default();
        let result = ApiKeyManager::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_key_generation_and_validation() {
        let config = ApiKeyConfig::default();
        let api_key_manager = ApiKeyManager::new(config).await.unwrap();

        let user_id = "test-user".to_string();
        let name = "Test API Key".to_string();
        let permissions = vec!["read".to_string(), "write".to_string()];

        // Generate API key
        let (api_key, raw_key) = api_key_manager
            .generate_key(user_id.clone(), name.clone(), permissions.clone())
            .await
            .unwrap();
        assert_eq!(api_key.user_id, user_id);
        assert_eq!(api_key.name, name);
        assert_eq!(api_key.permissions, permissions);
        assert!(!raw_key.is_empty());

        // Validate API key
        let validated_key = api_key_manager.validate_key(&raw_key).await.unwrap();
        assert_eq!(validated_key.id, api_key.id);
        assert!(validated_key.last_used_at.is_some());
    }

    #[tokio::test]
    async fn test_api_key_revocation() {
        let config = ApiKeyConfig::default();
        let api_key_manager = ApiKeyManager::new(config).await.unwrap();

        let user_id = "test-user".to_string();
        let name = "Test API Key".to_string();
        let permissions = vec!["read".to_string()];

        // Generate API key
        let (api_key, raw_key) = api_key_manager
            .generate_key(user_id, name, permissions)
            .await
            .unwrap();

        // Revoke API key
        api_key_manager.revoke_key(&api_key.id).await.unwrap();

        // Try to validate revoked key
        let result = api_key_manager.validate_key(&raw_key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_api_key_max_limit() {
        let mut config = ApiKeyConfig::default();
        config.max_keys_per_user = 2;
        let api_key_manager = ApiKeyManager::new(config).await.unwrap();

        let user_id = "test-user".to_string();
        let name = "Test API Key".to_string();
        let permissions = vec!["read".to_string()];

        // Generate first key
        api_key_manager
            .generate_key(user_id.clone(), format!("{} 1", name), permissions.clone())
            .await
            .unwrap();

        // Generate second key
        api_key_manager
            .generate_key(user_id.clone(), format!("{} 2", name), permissions.clone())
            .await
            .unwrap();

        // Try to generate third key (should fail)
        let result = api_key_manager
            .generate_key(user_id, format!("{} 3", name), permissions)
            .await;
        assert!(result.is_err());
    }
}
