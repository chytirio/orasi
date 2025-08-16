//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! JWT management functionality

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use crate::config::JwtConfig;

use super::claims::JwtClaims;
use super::stats::JwtStats;

/// JWT manager
pub struct JwtManager {
    /// JWT configuration
    config: JwtConfig,

    /// Encoding key
    encoding_key: EncodingKey,

    /// Decoding key
    decoding_key: DecodingKey,

    /// Token blacklist (for logout)
    blacklist: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,

    /// Statistics
    stats: Arc<RwLock<JwtStats>>,
}

impl JwtManager {
    /// Create new JWT manager
    pub fn new(config: JwtConfig) -> crate::AuthResult<Self> {
        let encoding_key = match config.algorithm {
            crate::config::JwtAlgorithm::HS256
            | crate::config::JwtAlgorithm::HS384
            | crate::config::JwtAlgorithm::HS512 => {
                EncodingKey::from_secret(config.secret.as_ref())
            }
            crate::config::JwtAlgorithm::RS256
            | crate::config::JwtAlgorithm::RS384
            | crate::config::JwtAlgorithm::RS512 => {
                // For RS algorithms, we would need to load from files
                // For now, fall back to HS256
                warn!("RS algorithms not yet implemented, falling back to HS256");
                EncodingKey::from_secret(config.secret.as_ref())
            }
        };

        let decoding_key = match config.algorithm {
            crate::config::JwtAlgorithm::HS256
            | crate::config::JwtAlgorithm::HS384
            | crate::config::JwtAlgorithm::HS512 => {
                DecodingKey::from_secret(config.secret.as_ref())
            }
            crate::config::JwtAlgorithm::RS256
            | crate::config::JwtAlgorithm::RS384
            | crate::config::JwtAlgorithm::RS512 => {
                // For RS algorithms, we would need to load from files
                DecodingKey::from_secret(config.secret.as_ref())
            }
        };

        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            blacklist: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(JwtStats::default())),
        })
    }

    /// Generate JWT token
    pub async fn generate_token(
        &self,
        user_id: String,
        roles: Vec<String>,
        custom_claims: Option<HashMap<String, serde_json::Value>>,
    ) -> crate::AuthResult<String> {
        let claims = JwtClaims::new(user_id, roles, &self.config, custom_claims);

        let header = Header::default();
        let token = encode(&header, &claims, &self.encoding_key)
            .map_err(|e| crate::AuthError::token_generation(e.to_string()))?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.increment_tokens_generated();
        }

        info!("Generated JWT token for user: {}", claims.sub);
        Ok(token)
    }

    /// Validate JWT token
    pub async fn validate_token(&self, token: &str) -> crate::AuthResult<JwtClaims> {
        // Check if token is blacklisted
        {
            let blacklist = self.blacklist.read().await;
            if let Some(blacklisted_at) = blacklist.get(token) {
                return Err(crate::AuthError::token_blacklisted(format!(
                    "Token blacklisted at {}",
                    blacklisted_at
                )));
            }
        }

        // Decode and validate token
        let mut validation = Validation::default();
        validation.set_audience(&[&self.config.audience]);
        validation.set_issuer(&[&self.config.issuer]);
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| {
                // Update validation failure statistics
                tokio::spawn({
                    let stats = self.stats.clone();
                    async move {
                        let mut stats = stats.write().await;
                        stats.increment_validation_failures();
                    }
                });
                crate::AuthError::token_validation(e.to_string())
            })?;

        let claims = token_data.claims;

        // Check if token is expired
        if claims.is_expired() {
            return Err(crate::AuthError::token_expired(
                "Token has expired".to_string(),
            ));
        }

        // Check if token is not yet valid
        if claims.is_not_yet_valid() {
            return Err(crate::AuthError::token_not_yet_valid(
                "Token not yet valid".to_string(),
            ));
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.increment_tokens_validated();
        }

        debug!("Validated JWT token for user: {}", claims.sub);
        Ok(claims)
    }

    /// Blacklist token (for logout)
    pub async fn blacklist_token(&self, token: String) -> crate::AuthResult<()> {
        // First validate the token to get expiration
        let claims = self.validate_token(&token).await?;
        let expiration = chrono::DateTime::from_timestamp(claims.exp, 0).ok_or_else(|| {
            crate::AuthError::internal("Invalid expiration timestamp".to_string())
        })?;

        // Add to blacklist
        {
            let mut blacklist = self.blacklist.write().await;
            blacklist.insert(token, expiration);
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.increment_tokens_blacklisted();
        }

        info!("Blacklisted JWT token for user: {}", claims.sub);
        Ok(())
    }

    /// Clean up expired blacklisted tokens
    pub async fn cleanup_blacklist(&self) -> crate::AuthResult<usize> {
        let now = chrono::Utc::now();
        let mut removed_count = 0;

        {
            let mut blacklist = self.blacklist.write().await;
            blacklist.retain(|_, expiration| {
                if *expiration < now {
                    removed_count += 1;
                    false
                } else {
                    true
                }
            });
        }

        if removed_count > 0 {
            debug!("Cleaned up {} expired blacklisted tokens", removed_count);
        }

        Ok(removed_count)
    }

    /// Get JWT statistics
    pub async fn get_stats(&self) -> JwtStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Refresh token
    pub async fn refresh_token(&self, token: &str) -> crate::AuthResult<String> {
        if !self.config.enable_refresh_tokens {
            return Err(crate::AuthError::refresh_disabled(
                "Refresh tokens are disabled".to_string(),
            ));
        }

        // Validate current token
        let claims = self.validate_token(token).await?;

        // Generate new token with same claims but new expiration
        let new_claims = JwtClaims::new(
            claims.sub.clone(),
            claims.roles.clone(),
            &self.config,
            Some(claims.custom.clone()),
        );

        let header = Header::default();
        let new_token = encode(&header, &new_claims, &self.encoding_key)
            .map_err(|e| crate::AuthError::token_generation(e.to_string()))?;

        // Blacklist old token
        self.blacklist_token(token.to_string()).await?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.increment_tokens_refreshed();
        }

        info!("Refreshed JWT token for user: {}", claims.sub);
        Ok(new_token)
    }
}

impl std::fmt::Debug for JwtManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtManager")
            .field("config", &"<sensitive>")
            .field("encoding_key", &"<sensitive>")
            .field("decoding_key", &"<sensitive>")
            .field("blacklist", &"<sensitive>")
            .field("stats", &self.stats)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::JwtConfig;

    #[tokio::test]
    async fn test_jwt_manager_creation() {
        let config = JwtConfig::default();
        let result = JwtManager::new(config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_jwt_manager_debug() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();
        let debug_output = format!("{:?}", jwt_manager);
        assert!(debug_output.contains("JwtManager"));
        assert!(debug_output.contains("<sensitive>"));
    }

    #[tokio::test]
    async fn test_jwt_token_generation_and_validation() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();

        let user_id = "test-user".to_string();
        let roles = vec!["user".to_string()];

        // Generate token
        let token = jwt_manager
            .generate_token(user_id.clone(), roles.clone(), None)
            .await
            .unwrap();
        assert!(!token.is_empty());

        // Validate token
        let claims = jwt_manager.validate_token(&token).await.unwrap();
        assert_eq!(claims.user_id(), user_id);
        assert_eq!(claims.roles(), roles.as_slice());
    }

    #[tokio::test]
    async fn test_jwt_token_with_custom_claims() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();

        let user_id = "test-user".to_string();
        let roles = vec!["user".to_string()];
        let mut custom_claims = HashMap::new();
        custom_claims.insert("custom_field".to_string(), serde_json::json!("custom_value"));

        // Generate token with custom claims
        let token = jwt_manager
            .generate_token(user_id.clone(), roles.clone(), Some(custom_claims.clone()))
            .await
            .unwrap();

        // Validate token
        let claims = jwt_manager.validate_token(&token).await.unwrap();
        assert_eq!(claims.user_id(), user_id);
        assert_eq!(claims.roles(), roles.as_slice());
        assert_eq!(claims.get_custom_claim("custom_field"), Some(&serde_json::json!("custom_value")));
    }

    #[tokio::test]
    async fn test_jwt_token_blacklisting() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();

        let user_id = "test-user".to_string();
        let roles = vec!["user".to_string()];

        // Generate token
        let token = jwt_manager
            .generate_token(user_id, roles, None)
            .await
            .unwrap();

        // Blacklist token
        jwt_manager.blacklist_token(token.clone()).await.unwrap();

        // Try to validate blacklisted token
        let result = jwt_manager.validate_token(&token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_jwt_token_refresh() {
        let mut config = JwtConfig::default();
        config.enable_refresh_tokens = true;
        let jwt_manager = JwtManager::new(config).unwrap();

        let user_id = "test-user".to_string();
        let roles = vec!["user".to_string()];

        // Generate token
        let token = jwt_manager
            .generate_token(user_id.clone(), roles.clone(), None)
            .await
            .unwrap();

        // Refresh token
        let new_token = jwt_manager.refresh_token(&token).await.unwrap();
        assert_ne!(token, new_token);

        // Old token should be blacklisted
        let result = jwt_manager.validate_token(&token).await;
        assert!(result.is_err());

        // New token should be valid
        let claims = jwt_manager.validate_token(&new_token).await.unwrap();
        assert_eq!(claims.user_id(), user_id);
    }

    #[tokio::test]
    async fn test_jwt_token_refresh_disabled() {
        let mut config = JwtConfig::default();
        config.enable_refresh_tokens = false;
        let jwt_manager = JwtManager::new(config).unwrap();

        let user_id = "test-user".to_string();
        let roles = vec!["user".to_string()];

        // Generate token
        let token = jwt_manager
            .generate_token(user_id, roles, None)
            .await
            .unwrap();

        // Try to refresh token when disabled
        let result = jwt_manager.refresh_token(&token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_jwt_token_validation_failures() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();

        // Test invalid token format
        let result = jwt_manager.validate_token("invalid.token.format").await;
        assert!(result.is_err());

        // Test empty token
        let result = jwt_manager.validate_token("").await;
        assert!(result.is_err());

        // Test malformed token
        let result = jwt_manager.validate_token("not.a.valid.jwt.token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_jwt_stats() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();

        let stats = jwt_manager.get_stats().await;
        assert_eq!(stats.tokens_generated, 0);
        assert_eq!(stats.tokens_validated, 0);
        assert_eq!(stats.tokens_blacklisted, 0);
        assert_eq!(stats.tokens_refreshed, 0);
        assert_eq!(stats.validation_failures, 0);
    }

    #[tokio::test]
    async fn test_jwt_blacklist_cleanup() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();

        // Initially no cleanup should be needed
        let cleaned = jwt_manager.cleanup_blacklist().await.unwrap();
        assert_eq!(cleaned, 0);
    }

    #[tokio::test]
    async fn test_jwt_manager_with_different_algorithms() {
        // Test with HS256 (default)
        let mut config = JwtConfig::default();
        config.algorithm = crate::config::JwtAlgorithm::HS256;
        let result = JwtManager::new(config);
        assert!(result.is_ok());

        // Test with HS384
        let mut config = JwtConfig::default();
        config.algorithm = crate::config::JwtAlgorithm::HS384;
        let result = JwtManager::new(config);
        assert!(result.is_ok());

        // Test with HS512
        let mut config = JwtConfig::default();
        config.algorithm = crate::config::JwtAlgorithm::HS512;
        let result = JwtManager::new(config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_jwt_token_expiration() {
        let mut config = JwtConfig::default();
        config.expiration_secs = 1; // Very short expiration for testing
        let jwt_manager = JwtManager::new(config).unwrap();

        let user_id = "test-user".to_string();
        let roles = vec!["user".to_string()];

        // Generate token
        let token = jwt_manager
            .generate_token(user_id, roles, None)
            .await
            .unwrap();

        // Token should be valid immediately
        let result = jwt_manager.validate_token(&token).await;
        assert!(result.is_ok());

        // Wait for token to expire
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Token should now be expired
        let result = jwt_manager.validate_token(&token).await;
        assert!(result.is_err());
    }
}
