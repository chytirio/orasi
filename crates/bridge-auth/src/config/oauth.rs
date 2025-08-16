//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OAuth configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Whether to enable OAuth authentication
    pub enabled: bool,

    /// OAuth providers
    pub providers: HashMap<String, OAuthProviderConfig>,

    /// OAuth callback URL
    pub callback_url: String,

    /// OAuth state timeout in seconds
    pub state_timeout_secs: u64,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            providers: HashMap::new(),
            callback_url: "http://localhost:8080/auth/callback".to_string(),
            state_timeout_secs: 300, // 5 minutes
        }
    }
}

/// OAuth provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderConfig {
    /// Client ID
    pub client_id: String,

    /// Client secret
    pub client_secret: String,

    /// Authorization URL
    pub auth_url: String,

    /// Token URL
    pub token_url: String,

    /// User info URL
    pub user_info_url: String,

    /// Scopes
    pub scopes: Vec<String>,
}

impl OAuthProviderConfig {
    /// Create a new OAuth provider configuration
    pub fn new(
        client_id: String,
        client_secret: String,
        auth_url: String,
        token_url: String,
        user_info_url: String,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            auth_url,
            token_url,
            user_info_url,
            scopes,
        }
    }

    /// Validate the OAuth provider configuration
    pub fn validate(&self) -> crate::AuthResult<()> {
        if self.client_id.is_empty() {
            return Err(crate::AuthError::internal(
                "OAuth client ID cannot be empty".to_string(),
            ));
        }
        if self.client_secret.is_empty() {
            return Err(crate::AuthError::internal(
                "OAuth client secret cannot be empty".to_string(),
            ));
        }
        if self.auth_url.is_empty() {
            return Err(crate::AuthError::internal(
                "OAuth authorization URL cannot be empty".to_string(),
            ));
        }
        if self.token_url.is_empty() {
            return Err(crate::AuthError::internal(
                "OAuth token URL cannot be empty".to_string(),
            ));
        }
        if self.user_info_url.is_empty() {
            return Err(crate::AuthError::internal(
                "OAuth user info URL cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Bridge API OAuth configuration (type alias for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeApiOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub user_info_url: Option<String>,
}

/// Trait for converting between different OAuth configuration formats
pub trait OAuthConfigConverter {
    /// Convert to bridge-api OAuth configuration format
    fn to_bridge_api_format(&self) -> BridgeApiOAuthConfig;

    /// Convert from bridge-api OAuth configuration format
    fn from_bridge_api_format(config: &BridgeApiOAuthConfig) -> Self;
}

impl OAuthConfigConverter for OAuthConfig {
    fn to_bridge_api_format(&self) -> BridgeApiOAuthConfig {
        // For now, we'll use the first provider or create a default one
        let provider =
            self.providers
                .values()
                .next()
                .cloned()
                .unwrap_or_else(|| OAuthProviderConfig {
                    client_id: "".to_string(),
                    client_secret: "".to_string(),
                    auth_url: "".to_string(),
                    token_url: "".to_string(),
                    user_info_url: "".to_string(),
                    scopes: vec![],
                });

        BridgeApiOAuthConfig {
            client_id: provider.client_id,
            client_secret: provider.client_secret,
            authorization_url: provider.auth_url,
            token_url: provider.token_url,
            user_info_url: Some(provider.user_info_url),
        }
    }

    fn from_bridge_api_format(config: &BridgeApiOAuthConfig) -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "default".to_string(),
            OAuthProviderConfig {
                client_id: config.client_id.clone(),
                client_secret: config.client_secret.clone(),
                auth_url: config.authorization_url.clone(),
                token_url: config.token_url.clone(),
                user_info_url: config.user_info_url.clone().unwrap_or_default(),
                scopes: vec![
                    "openid".to_string(),
                    "profile".to_string(),
                    "email".to_string(),
                ],
            },
        );

        Self {
            enabled: !config.client_id.is_empty(),
            providers,
            callback_url: "http://localhost:8080/auth/callback".to_string(),
            state_timeout_secs: 300,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_config_default() {
        let config = OAuthConfig::default();
        assert!(!config.enabled);
        assert!(config.providers.is_empty());
    }

    #[test]
    fn test_oauth_provider_config_validation() {
        let provider = OAuthProviderConfig::new(
            "client_id".to_string(),
            "client_secret".to_string(),
            "auth_url".to_string(),
            "token_url".to_string(),
            "user_info_url".to_string(),
            vec!["openid".to_string()],
        );
        assert!(provider.validate().is_ok());

        let invalid_provider = OAuthProviderConfig::new(
            "".to_string(),
            "client_secret".to_string(),
            "auth_url".to_string(),
            "token_url".to_string(),
            "user_info_url".to_string(),
            vec![],
        );
        assert!(invalid_provider.validate().is_err());
    }
}
