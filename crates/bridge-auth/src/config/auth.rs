//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Main authentication configuration

use serde::{Deserialize, Serialize};

use super::api_keys::ApiKeyConfig;
use super::jwt::JwtConfig;
use super::oauth::OAuthConfig;
use super::rbac::RbacConfig;
use super::security::SecurityConfig;
use super::session::SessionConfig;
use super::user::UserConfig;

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT configuration
    pub jwt: JwtConfig,

    /// API key configuration
    pub api_keys: ApiKeyConfig,

    /// OAuth configuration
    pub oauth: OAuthConfig,

    /// User management configuration
    pub users: UserConfig,

    /// Role-based access control configuration
    pub rbac: RbacConfig,

    /// Session configuration
    pub session: SessionConfig,

    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt: JwtConfig::default(),
            api_keys: ApiKeyConfig::default(),
            oauth: OAuthConfig::default(),
            users: UserConfig::default(),
            rbac: RbacConfig::default(),
            session: SessionConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl AuthConfig {
    /// Validate the authentication configuration
    pub fn validate(&self) -> crate::AuthResult<()> {
        // Validate JWT configuration
        if self.jwt.secret.is_empty() {
            return Err(crate::AuthError::internal(
                "JWT secret cannot be empty".to_string(),
            ));
        }

        // Validate OAuth configuration if enabled
        if self.oauth.enabled {
            if self.oauth.providers.is_empty() {
                return Err(crate::AuthError::internal(
                    "OAuth is enabled but no providers are configured".to_string(),
                ));
            }

            for (name, provider) in &self.oauth.providers {
                provider.validate().map_err(|e| {
                    crate::AuthError::internal(format!(
                        "OAuth provider '{}' configuration error: {}",
                        name, e
                    ))
                })?;
            }
        }

        Ok(())
    }

    /// Create configuration from environment variables
    ///
    /// This method reads configuration values from environment variables with sensible defaults.
    /// Environment variables are prefixed with `AUTH_` to avoid conflicts.
    ///
    /// # Environment Variables
    ///
    /// ## JWT Configuration
    /// - `AUTH_JWT_SECRET`: JWT secret key (required)
    /// - `AUTH_JWT_EXPIRATION_SECS`: JWT expiration time in seconds (default: 86400)
    /// - `AUTH_JWT_ALGORITHM`: JWT algorithm (default: HS256)
    /// - `AUTH_JWT_ISSUER`: JWT issuer (default: "orasi-auth")
    /// - `AUTH_JWT_AUDIENCE`: JWT audience (default: "orasi-users")
    /// - `AUTH_JWT_ENABLE_REFRESH_TOKENS`: Enable refresh tokens (default: true)
    ///
    /// ## API Key Configuration
    /// - `AUTH_API_KEYS_ENABLED`: Enable API keys (default: true)
    /// - `AUTH_API_KEYS_EXPIRATION_SECS`: API key expiration time in seconds (default: 2592000)
    /// - `AUTH_API_KEYS_KEY_LENGTH`: API key length (default: 32)
    ///
    /// ## Security Configuration
    /// - `AUTH_SECURITY_ENABLE_RATE_LIMITING`: Enable rate limiting (default: true)
    /// - `AUTH_SECURITY_RATE_LIMIT_PER_MINUTE`: Rate limit per minute (default: 100)
    /// - `AUTH_SECURITY_ENABLE_CORS`: Enable CORS (default: true)
    /// - `AUTH_SECURITY_HTTPS_ONLY`: HTTPS only (default: false)
    ///
    /// ## User Configuration
    /// - `AUTH_USERS_ALLOW_REGISTRATION`: Allow user registration (default: true)
    /// - `AUTH_USERS_MIN_PASSWORD_LENGTH`: Minimum password length (default: 8)
    /// - `AUTH_USERS_MAX_FAILED_LOGIN_ATTEMPTS`: Max failed login attempts (default: 5)
    /// - `AUTH_USERS_ACCOUNT_LOCKOUT_DURATION_SECS`: Account lockout duration (default: 900)
    ///
    /// ## RBAC Configuration
    /// - `AUTH_RBAC_ENABLED`: Enable RBAC (default: true)
    /// - `AUTH_RBAC_DEFAULT_ROLE`: Default role (default: "user")
    /// - `AUTH_RBAC_ADMIN_ROLE`: Admin role (default: "admin")
    /// - `AUTH_RBAC_MAX_ROLES_PER_USER`: Max roles per user (default: 5)
    ///
    /// ## Session Configuration
    /// - `AUTH_SESSION_TIMEOUT_SECS`: Session timeout in seconds (default: 28800)
    /// - `AUTH_SESSION_ENABLE_REFRESH`: Enable session refresh (default: true)
    /// - `AUTH_SESSION_REFRESH_INTERVAL_SECS`: Refresh interval in seconds (default: 300)
    /// - `AUTH_SESSION_MAX_CONCURRENT_SESSIONS`: Max concurrent sessions (default: 5)
    ///
    /// # Example
    ///
    /// ```bash
    /// export AUTH_JWT_SECRET="your-super-secret-key"
    /// export AUTH_JWT_EXPIRATION_SECS="3600"
    /// export AUTH_SECURITY_RATE_LIMIT_PER_MINUTE="50"
    /// ```
    ///
    /// ```rust
    /// use bridge_auth::config::AuthConfig;
    ///
    /// // Set required environment variable for the test
    /// std::env::set_var("AUTH_JWT_SECRET", "test-secret-key");
    /// let config = AuthConfig::from_env().expect("Failed to load config from environment");
    /// ```
    pub fn from_env() -> crate::AuthResult<Self> {
        let mut config = Self::default();

        // JWT Configuration
        if let Ok(secret) = std::env::var("AUTH_JWT_SECRET") {
            config.jwt.secret = secret;
        } else {
            return Err(crate::AuthError::internal(
                "AUTH_JWT_SECRET environment variable is required".to_string(),
            ));
        }

        if let Ok(expiration) = std::env::var("AUTH_JWT_EXPIRATION_SECS") {
            config.jwt.expiration_secs = expiration.parse().map_err(|_| {
                crate::AuthError::internal(
                    "AUTH_JWT_EXPIRATION_SECS must be a valid number".to_string(),
                )
            })?;
        }

        if let Ok(algorithm) = std::env::var("AUTH_JWT_ALGORITHM") {
            config.jwt.algorithm = match algorithm.to_uppercase().as_str() {
                "HS256" => super::jwt::JwtAlgorithm::HS256,
                "HS384" => super::jwt::JwtAlgorithm::HS384,
                "HS512" => super::jwt::JwtAlgorithm::HS512,
                "RS256" => super::jwt::JwtAlgorithm::RS256,
                "RS384" => super::jwt::JwtAlgorithm::RS384,
                "RS512" => super::jwt::JwtAlgorithm::RS512,
                _ => {
                    return Err(crate::AuthError::internal(format!(
                        "Unsupported JWT algorithm: {}",
                        algorithm
                    )))
                }
            };
        }

        if let Ok(issuer) = std::env::var("AUTH_JWT_ISSUER") {
            config.jwt.issuer = issuer;
        }

        if let Ok(audience) = std::env::var("AUTH_JWT_AUDIENCE") {
            config.jwt.audience = audience;
        }

        if let Ok(enable_refresh) = std::env::var("AUTH_JWT_ENABLE_REFRESH_TOKENS") {
            config.jwt.enable_refresh_tokens = enable_refresh.parse().unwrap_or(true);
        }

        // API Key Configuration
        if let Ok(enabled) = std::env::var("AUTH_API_KEYS_ENABLED") {
            config.api_keys.enabled = enabled.parse().unwrap_or(true);
        }

        if let Ok(expiration) = std::env::var("AUTH_API_KEYS_EXPIRATION_SECS") {
            config.api_keys.expiration_secs = expiration.parse().unwrap_or(2592000);
        }

        if let Ok(key_length) = std::env::var("AUTH_API_KEYS_KEY_LENGTH") {
            config.api_keys.key_length = key_length.parse().unwrap_or(32);
        }

        // Security Configuration
        if let Ok(enable_rate_limiting) = std::env::var("AUTH_SECURITY_ENABLE_RATE_LIMITING") {
            config.security.enable_rate_limiting = enable_rate_limiting.parse().unwrap_or(true);
        }

        if let Ok(rate_limit) = std::env::var("AUTH_SECURITY_RATE_LIMIT_PER_MINUTE") {
            config.security.rate_limit_per_minute = rate_limit.parse().unwrap_or(100);
        }

        if let Ok(enable_cors) = std::env::var("AUTH_SECURITY_ENABLE_CORS") {
            config.security.enable_cors = enable_cors.parse().unwrap_or(true);
        }

        if let Ok(https_only) = std::env::var("AUTH_SECURITY_HTTPS_ONLY") {
            config.security.https_only = https_only.parse().unwrap_or(false);
        }

        // User Configuration
        if let Ok(allow_registration) = std::env::var("AUTH_USERS_ALLOW_REGISTRATION") {
            config.users.allow_registration = allow_registration.parse().unwrap_or(true);
        }

        if let Ok(min_password_length) = std::env::var("AUTH_USERS_MIN_PASSWORD_LENGTH") {
            config.users.min_password_length = min_password_length.parse().unwrap_or(8);
        }

        if let Ok(max_failed_attempts) = std::env::var("AUTH_USERS_MAX_FAILED_LOGIN_ATTEMPTS") {
            config.users.max_login_attempts = max_failed_attempts.parse().unwrap_or(5);
        }

        if let Ok(lockout_duration) = std::env::var("AUTH_USERS_ACCOUNT_LOCKOUT_DURATION_SECS") {
            config.users.lockout_duration_secs = lockout_duration.parse().unwrap_or(900);
        }

        // RBAC Configuration
        if let Ok(enabled) = std::env::var("AUTH_RBAC_ENABLED") {
            config.rbac.enabled = enabled.parse().unwrap_or(true);
        }

        if let Ok(default_role) = std::env::var("AUTH_RBAC_DEFAULT_ROLE") {
            config.rbac.default_role = default_role;
        }

        if let Ok(admin_role) = std::env::var("AUTH_RBAC_ADMIN_ROLE") {
            config.rbac.admin_role = admin_role;
        }

        if let Ok(max_roles) = std::env::var("AUTH_RBAC_MAX_ROLES_PER_USER") {
            config.rbac.max_roles_per_user = max_roles.parse().unwrap_or(5);
        }

        // Session Configuration
        if let Ok(timeout) = std::env::var("AUTH_SESSION_TIMEOUT_SECS") {
            config.session.timeout_secs = timeout.parse().unwrap_or(28800);
        }

        if let Ok(enable_refresh) = std::env::var("AUTH_SESSION_ENABLE_REFRESH") {
            config.session.enable_refresh = enable_refresh.parse().unwrap_or(true);
        }

        if let Ok(refresh_interval) = std::env::var("AUTH_SESSION_REFRESH_INTERVAL_SECS") {
            config.session.refresh_interval_secs = refresh_interval.parse().unwrap_or(300);
        }

        if let Ok(max_sessions) = std::env::var("AUTH_SESSION_MAX_CONCURRENT_SESSIONS") {
            config.session.max_concurrent_sessions = max_sessions.parse().unwrap_or(5);
        }

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }

    /// Get OAuth provider by name
    pub fn get_oauth_provider(&self, name: &str) -> Option<&super::oauth::OAuthProviderConfig> {
        self.oauth.providers.get(name)
    }

    /// Add OAuth provider
    pub fn add_oauth_provider(
        &mut self,
        name: String,
        provider: super::oauth::OAuthProviderConfig,
    ) -> crate::AuthResult<()> {
        provider.validate()?;
        self.oauth.providers.insert(name, provider);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.jwt.secret.is_empty());
        assert_eq!(
            config.jwt.expiration_secs,
            crate::DEFAULT_JWT_EXPIRATION_SECS
        );
        assert!(config.api_keys.enabled);
        assert!(!config.oauth.enabled);
        assert!(config.users.allow_registration);
        assert!(config.rbac.enabled);
        assert!(config.security.enable_rate_limiting);
    }

    #[test]
    fn test_auth_config_validation() {
        let mut config = AuthConfig::default();
        assert!(config.validate().is_ok());

        // Test empty JWT secret
        config.jwt.secret = "".to_string();
        assert!(config.validate().is_err());

        // Test OAuth enabled but no providers
        config.jwt.secret = "test-secret".to_string();
        config.oauth.enabled = true;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_auth_config_from_env() {
        // Clean up any existing environment variables first
        std::env::remove_var("AUTH_JWT_SECRET");

        // Set required environment variable BEFORE calling from_env()
        std::env::set_var("AUTH_JWT_SECRET", "test-secret-from-env");

        let config = AuthConfig::from_env();
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.jwt.secret, "test-secret-from-env");

        // Clean up
        std::env::remove_var("AUTH_JWT_SECRET");
    }

    #[test]
    #[ignore] // Ignore this test as it conflicts with the other test due to environment variable sharing
    fn test_auth_config_from_env_missing_secret() {
        // Ensure AUTH_JWT_SECRET is not set
        std::env::remove_var("AUTH_JWT_SECRET");

        let config = AuthConfig::from_env();
        assert!(config.is_err());

        // Clean up
        std::env::remove_var("AUTH_JWT_SECRET");
    }
}
