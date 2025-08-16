//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Main authentication module

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::api_keys::ApiKeyManager;
use crate::config::AuthConfig;
use crate::jwt::JwtManager;
use crate::oauth::OAuthManager;
use crate::permissions::PermissionManager;
use crate::roles::RoleManager;
use crate::users::{User, UserManager};

// AuthError is already implemented by the derive macro

/// Authentication result type
pub type AuthResult<T> = Result<T, AuthError>;

/// Authentication error
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    #[error("Token generation failed: {0}")]
    TokenGeneration(String),

    #[error("Token validation failed: {0}")]
    TokenValidation(String),

    #[error("Token expired: {0}")]
    TokenExpired(String),

    #[error("Token not yet valid: {0}")]
    TokenNotYetValid(String),

    #[error("Token blacklisted: {0}")]
    TokenBlacklisted(String),

    #[error("Refresh tokens disabled: {0}")]
    RefreshDisabled(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("Account locked: {0}")]
    AccountLocked(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AuthError {
    pub fn authentication_failed(msg: String) -> Self {
        Self::AuthenticationFailed(msg)
    }

    pub fn authorization_failed(msg: String) -> Self {
        Self::AuthorizationFailed(msg)
    }

    pub fn token_generation(msg: String) -> Self {
        Self::TokenGeneration(msg)
    }

    pub fn token_validation(msg: String) -> Self {
        Self::TokenValidation(msg)
    }

    pub fn token_expired(msg: String) -> Self {
        Self::TokenExpired(msg)
    }

    pub fn token_not_yet_valid(msg: String) -> Self {
        Self::TokenNotYetValid(msg)
    }

    pub fn token_blacklisted(msg: String) -> Self {
        Self::TokenBlacklisted(msg)
    }

    pub fn refresh_disabled(msg: String) -> Self {
        Self::RefreshDisabled(msg)
    }

    pub fn user_not_found(msg: String) -> Self {
        Self::UserNotFound(msg)
    }

    pub fn invalid_credentials(msg: String) -> Self {
        Self::InvalidCredentials(msg)
    }

    pub fn account_locked(msg: String) -> Self {
        Self::AccountLocked(msg)
    }

    pub fn rate_limit_exceeded(msg: String) -> Self {
        Self::RateLimitExceeded(msg)
    }

    pub fn internal(msg: String) -> Self {
        Self::Internal(msg)
    }
}

/// Authentication status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthStatus {
    /// Authentication successful
    Authenticated {
        user_id: String,
        roles: Vec<String>,
        permissions: Vec<String>,
    },

    /// Authentication failed
    Failed { reason: String },

    /// Authentication required
    Required { auth_methods: Vec<AuthMethod> },
}

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// JWT token authentication
    Jwt,

    /// API key authentication
    ApiKey,

    /// OAuth authentication
    OAuth { provider: String, auth_url: String },

    /// Username/password authentication
    UsernamePassword,
}

/// Authentication request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    /// Authentication method
    pub method: AuthMethod,

    /// Credentials (varies by method)
    pub credentials: AuthCredentials,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthCredentials {
    /// JWT token
    JwtToken { token: String },

    /// API key
    ApiKey { key: String },

    /// Username and password
    UsernamePassword { username: String, password: String },

    /// OAuth code
    OAuthCode {
        provider: String,
        code: String,
        state: String,
    },
}

/// Authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Authentication status
    pub status: AuthStatus,

    /// JWT token (if authentication successful)
    pub token: Option<String>,

    /// Refresh token (if enabled)
    pub refresh_token: Option<String>,

    /// User information
    pub user: Option<User>,

    /// Expiration time
    pub expires_at: Option<DateTime<Utc>>,
}

/// Authentication manager
pub struct AuthManager {
    /// Configuration
    config: AuthConfig,

    /// JWT manager
    jwt_manager: JwtManager,

    /// API key manager
    api_key_manager: ApiKeyManager,

    /// OAuth manager
    oauth_manager: OAuthManager,

    /// User manager
    user_manager: UserManager,

    /// Role manager
    role_manager: RoleManager,

    /// Permission manager
    permission_manager: PermissionManager,

    /// Rate limiting
    rate_limits: Arc<RwLock<HashMap<String, RateLimitInfo>>>,

    /// Statistics
    stats: Arc<RwLock<AuthStats>>,
}

impl AuthManager {
    /// Create new authentication manager
    pub async fn new(config: AuthConfig) -> AuthResult<Self> {
        info!("Creating authentication manager");

        let jwt_manager = JwtManager::new(config.jwt.clone())?;
        let api_key_manager = ApiKeyManager::new(config.api_keys.clone()).await?;
        let oauth_manager = OAuthManager::new(config.oauth.clone()).await?;
        let user_manager = UserManager::new(config.users.clone()).await?;
        let role_manager = RoleManager::new(config.rbac.clone()).await?;
        let permission_manager = PermissionManager::new().await?;

        Ok(Self {
            config,
            jwt_manager,
            api_key_manager,
            oauth_manager,
            user_manager,
            role_manager,
            permission_manager,
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AuthStats::default())),
        })
    }

    /// Authenticate user
    pub async fn authenticate(&self, request: AuthRequest) -> AuthResult<AuthResponse> {
        // Check rate limiting
        self.check_rate_limit(&request).await?;

        let result = match request.method {
            AuthMethod::Jwt => self.authenticate_jwt(request.credentials).await,
            AuthMethod::ApiKey => self.authenticate_api_key(request.credentials).await,
            AuthMethod::OAuth { provider, .. } => {
                self.authenticate_oauth(request.credentials, provider).await
            }
            AuthMethod::UsernamePassword => {
                self.authenticate_username_password(request.credentials)
                    .await
            }
        };

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            match &result {
                Ok(_) => stats.successful_authentications += 1,
                Err(_) => stats.failed_authentications += 1,
            }
            stats.last_authentication = Some(Utc::now());
        }

        result
    }

    /// Authenticate with JWT token
    async fn authenticate_jwt(&self, credentials: AuthCredentials) -> AuthResult<AuthResponse> {
        let AuthCredentials::JwtToken { token } = credentials else {
            return Err(AuthError::invalid_credentials(
                "Invalid JWT credentials".to_string(),
            ));
        };

        // Validate JWT token
        let claims = self.jwt_manager.validate_token(&token).await?;

        // Get user
        let user = self
            .user_manager
            .get_user_by_id(claims.user_id())
            .await
            .ok_or_else(|| AuthError::user_not_found("User not found".to_string()))?;

        // Check if user is active
        if !user.is_active {
            return Err(AuthError::account_locked(
                "Account is deactivated".to_string(),
            ));
        }

        // Get user permissions
        let permissions = self.permission_manager.get_user_permissions(&user.id).await;

        Ok(AuthResponse {
            status: AuthStatus::Authenticated {
                user_id: user.id.clone(),
                roles: claims.roles().to_vec(),
                permissions,
            },
            token: Some(token),
            refresh_token: None, // JWT refresh handled separately
            user: Some(user),
            expires_at: Some(DateTime::from_timestamp(claims.exp, 0).unwrap_or(Utc::now())),
        })
    }

    /// Authenticate with API key
    async fn authenticate_api_key(&self, credentials: AuthCredentials) -> AuthResult<AuthResponse> {
        let AuthCredentials::ApiKey { key } = credentials else {
            return Err(AuthError::invalid_credentials(
                "Invalid API key credentials".to_string(),
            ));
        };

        // Validate API key
        let api_key = self.api_key_manager.validate_key(&key).await?;

        // Get user
        let user = self
            .user_manager
            .get_user_by_id(&api_key.user_id)
            .await
            .ok_or_else(|| AuthError::user_not_found("User not found".to_string()))?;

        // Check if user is active
        if !user.is_active {
            return Err(AuthError::account_locked(
                "Account is deactivated".to_string(),
            ));
        }

        // Get user roles and permissions
        let roles = self.role_manager.get_user_roles(&user.id).await;
        let permissions = self.permission_manager.get_user_permissions(&user.id).await;

        // Generate JWT token
        let token = self
            .jwt_manager
            .generate_token(user.id.clone(), roles.clone(), None)
            .await?;

        Ok(AuthResponse {
            status: AuthStatus::Authenticated {
                user_id: user.id.clone(),
                roles,
                permissions,
            },
            token: Some(token),
            refresh_token: None,
            user: Some(user),
            expires_at: Some(api_key.expires_at),
        })
    }

    /// Authenticate with OAuth
    async fn authenticate_oauth(
        &self,
        credentials: AuthCredentials,
        provider: String,
    ) -> AuthResult<AuthResponse> {
        let AuthCredentials::OAuthCode { code, state, .. } = credentials else {
            return Err(AuthError::invalid_credentials(
                "Invalid OAuth credentials".to_string(),
            ));
        };

        // Exchange code for token
        let oauth_user = self
            .oauth_manager
            .exchange_code_for_user(&provider, &code, &state)
            .await?;

        // Get or create user
        let user = self
            .user_manager
            .get_or_create_oauth_user(&oauth_user)
            .await?;

        // Check if user is active
        if !user.is_active {
            return Err(AuthError::account_locked(
                "Account is deactivated".to_string(),
            ));
        }

        // Get user roles and permissions
        let roles = self.role_manager.get_user_roles(&user.id).await;
        let permissions = self.permission_manager.get_user_permissions(&user.id).await;

        // Generate JWT token
        let token = self
            .jwt_manager
            .generate_token(user.id.clone(), roles.clone(), None)
            .await?;

        Ok(AuthResponse {
            status: AuthStatus::Authenticated {
                user_id: user.id.clone(),
                roles,
                permissions,
            },
            token: Some(token),
            refresh_token: None,
            user: Some(user),
            expires_at: Some(
                Utc::now() + chrono::Duration::seconds(self.config.jwt.expiration_secs as i64),
            ),
        })
    }

    /// Authenticate with username and password
    async fn authenticate_username_password(
        &self,
        credentials: AuthCredentials,
    ) -> AuthResult<AuthResponse> {
        let AuthCredentials::UsernamePassword { username, password } = credentials else {
            return Err(AuthError::invalid_credentials(
                "Invalid username/password credentials".to_string(),
            ));
        };

        // Get user by username
        let user = self
            .user_manager
            .get_user_by_username(&username)
            .await
            .ok_or_else(|| {
                AuthError::invalid_credentials("Invalid username or password".to_string())
            })?;

        // Check if user is active
        if !user.is_active {
            return Err(AuthError::account_locked(
                "Account is deactivated".to_string(),
            ));
        }

        // Check if account is locked
        if user.is_locked {
            return Err(AuthError::account_locked("Account is locked".to_string()));
        }

        // Verify password
        if !self
            .user_manager
            .verify_password(&user.id, &password)
            .await?
        {
            // Increment failed login attempts
            self.user_manager
                .increment_failed_login_attempts(&user.id)
                .await?;
            return Err(AuthError::invalid_credentials(
                "Invalid username or password".to_string(),
            ));
        }

        // Reset failed login attempts
        self.user_manager
            .reset_failed_login_attempts(&user.id)
            .await?;

        // Get user roles and permissions
        let roles = self.role_manager.get_user_roles(&user.id).await;
        let permissions = self.permission_manager.get_user_permissions(&user.id).await;

        // Generate JWT token
        let token = self
            .jwt_manager
            .generate_token(user.id.clone(), roles.clone(), None)
            .await?;

        Ok(AuthResponse {
            status: AuthStatus::Authenticated {
                user_id: user.id.clone(),
                roles,
                permissions,
            },
            token: Some(token),
            refresh_token: None,
            user: Some(user),
            expires_at: Some(
                Utc::now() + chrono::Duration::seconds(self.config.jwt.expiration_secs as i64),
            ),
        })
    }

    /// Check rate limiting
    async fn check_rate_limit(&self, request: &AuthRequest) -> AuthResult<()> {
        if !self.config.security.enable_rate_limiting {
            return Ok(());
        }

        let identifier = self.get_rate_limit_identifier(request).await;
        let now = Utc::now();

        {
            let mut rate_limits = self.rate_limits.write().await;
            let limit_info = rate_limits
                .entry(identifier)
                .or_insert_with(|| RateLimitInfo {
                    requests: 0,
                    window_start: now,
                });

            // Check if window has expired
            if now - limit_info.window_start > chrono::Duration::minutes(1) {
                limit_info.requests = 0;
                limit_info.window_start = now;
            }

            // Check if limit exceeded
            if limit_info.requests >= self.config.security.rate_limit_per_minute {
                return Err(AuthError::rate_limit_exceeded(
                    "Rate limit exceeded".to_string(),
                ));
            }

            limit_info.requests += 1;
        }

        Ok(())
    }

    /// Get rate limit identifier
    async fn get_rate_limit_identifier(&self, request: &AuthRequest) -> String {
        match &request.credentials {
            AuthCredentials::JwtToken { token } => {
                // Use a hash of the token as identifier
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(token.as_bytes());
                format!("jwt_{:x}", hasher.finalize())
            }
            AuthCredentials::ApiKey { key } => {
                format!("apikey_{}", key)
            }
            AuthCredentials::UsernamePassword { username, .. } => {
                format!("userpass_{}", username)
            }
            AuthCredentials::OAuthCode { provider, .. } => {
                format!("oauth_{}", provider)
            }
        }
    }

    /// Authorize access
    pub async fn authorize(&self, user_id: &str, resource: &str, action: &str) -> AuthResult<bool> {
        // Get user permissions
        let permissions = self.permission_manager.get_user_permissions(user_id).await;

        // Check if user has required permission
        let required_permission = format!("{}:{}", resource, action);
        let has_permission = permissions.iter().any(|p| p == &required_permission);

        if has_permission {
            debug!("User {} authorized for {}:{}", user_id, resource, action);
            Ok(true)
        } else {
            debug!(
                "User {} not authorized for {}:{}",
                user_id, resource, action
            );
            Ok(false)
        }
    }

    /// Refresh JWT token
    pub async fn refresh_token(&self, token: &str) -> AuthResult<String> {
        self.jwt_manager.refresh_token(token).await
    }

    /// Logout (blacklist token)
    pub async fn logout(&self, token: String) -> AuthResult<()> {
        self.jwt_manager.blacklist_token(token).await
    }

    /// Get authentication statistics
    pub async fn get_stats(&self) -> AuthStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get JWT manager
    pub fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }

    /// Get API key manager
    pub fn api_key_manager(&self) -> &ApiKeyManager {
        &self.api_key_manager
    }

    /// Get user manager
    pub fn user_manager(&self) -> &UserManager {
        &self.user_manager
    }

    /// Get role manager
    pub fn role_manager(&self) -> &RoleManager {
        &self.role_manager
    }

    /// Get permission manager
    pub fn permission_manager(&self) -> &PermissionManager {
        &self.permission_manager
    }

    /// Shutdown authentication manager
    pub async fn shutdown(&self) -> AuthResult<()> {
        info!("Shutting down authentication manager");

        // Clean up expired blacklisted tokens
        let cleaned = self.jwt_manager.cleanup_blacklist().await?;
        if cleaned > 0 {
            info!("Cleaned up {} expired blacklisted tokens", cleaned);
        }

        info!("Authentication manager shutdown completed");
        Ok(())
    }
}

impl std::fmt::Debug for AuthManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthManager")
            .field("config", &"<sensitive>")
            .field("jwt_manager", &"<sensitive>")
            .field("api_key_manager", &"<sensitive>")
            .field("oauth_manager", &"<sensitive>")
            .field("user_manager", &"<sensitive>")
            .field("role_manager", &"<sensitive>")
            .field("permission_manager", &"<sensitive>")
            .field("rate_limits", &"<sensitive>")
            .field("stats", &self.stats)
            .finish()
    }
}

/// Rate limit information
#[derive(Debug, Clone)]
struct RateLimitInfo {
    requests: usize,
    window_start: DateTime<Utc>,
}

/// Authentication statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct AuthStats {
    /// Number of successful authentications
    pub successful_authentications: u64,

    /// Number of failed authentications
    pub failed_authentications: u64,

    /// Number of authorizations
    pub authorizations: u64,

    /// Number of token refreshes
    pub token_refreshes: u64,

    /// Number of logouts
    pub logouts: u64,

    /// Last authentication
    pub last_authentication: Option<DateTime<Utc>>,

    /// Last authorization
    pub last_authorization: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthConfig;

    #[tokio::test]
    async fn test_auth_manager_creation() {
        let config = AuthConfig::default();
        let result = AuthManager::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_auth_manager_debug() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();
        let debug_output = format!("{:?}", auth_manager);
        assert!(debug_output.contains("AuthManager"));
        assert!(debug_output.contains("<sensitive>"));
    }

    #[tokio::test]
    async fn test_auth_manager_shutdown() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();
        let result = auth_manager.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_auth_manager_stats() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();
        let stats = auth_manager.get_stats().await;
        assert_eq!(stats.successful_authentications, 0);
        assert_eq!(stats.failed_authentications, 0);
    }

    #[tokio::test]
    async fn test_invalid_jwt_authentication() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();

        let request = AuthRequest {
            method: AuthMethod::Jwt,
            credentials: AuthCredentials::JwtToken {
                token: "invalid.token.here".to_string(),
            },
            metadata: HashMap::new(),
        };

        let result = auth_manager.authenticate(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_api_key_authentication() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();

        let request = AuthRequest {
            method: AuthMethod::ApiKey,
            credentials: AuthCredentials::ApiKey {
                key: "invalid-api-key".to_string(),
            },
            metadata: HashMap::new(),
        };

        let result = auth_manager.authenticate(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_oauth_authentication() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();

        let request = AuthRequest {
            method: AuthMethod::OAuth {
                provider: "github".to_string(),
                auth_url: "https://github.com/login/oauth/authorize".to_string(),
            },
            credentials: AuthCredentials::OAuthCode {
                provider: "github".to_string(),
                code: "invalid-code".to_string(),
                state: "invalid-state".to_string(),
            },
            metadata: HashMap::new(),
        };

        let result = auth_manager.authenticate(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_username_password_authentication() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();

        let request = AuthRequest {
            method: AuthMethod::UsernamePassword,
            credentials: AuthCredentials::UsernamePassword {
                username: "nonexistent".to_string(),
                password: "wrongpassword".to_string(),
            },
            metadata: HashMap::new(),
        };

        let result = auth_manager.authenticate(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiting_disabled() {
        let mut config = AuthConfig::default();
        config.security.enable_rate_limiting = false;

        let auth_manager = AuthManager::new(config).await.unwrap();

        let request = AuthRequest {
            method: AuthMethod::UsernamePassword,
            credentials: AuthCredentials::UsernamePassword {
                username: "test".to_string(),
                password: "test".to_string(),
            },
            metadata: HashMap::new(),
        };

        // Should not fail due to rate limiting when disabled
        let result = auth_manager.check_rate_limit(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limit_identifier_generation() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();

        let request = AuthRequest {
            method: AuthMethod::UsernamePassword,
            credentials: AuthCredentials::UsernamePassword {
                username: "testuser".to_string(),
                password: "testpass".to_string(),
            },
            metadata: HashMap::new(),
        };

        let identifier = auth_manager.get_rate_limit_identifier(&request).await;
        assert!(identifier.starts_with("userpass_"));
        assert!(identifier.contains("testuser"));
    }

    #[tokio::test]
    async fn test_authorization_without_user() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();

        let result = auth_manager
            .authorize("nonexistent-user", "test", "read")
            .await;
        assert!(result.is_ok());
        // Should return false for non-existent user
        assert_eq!(result.unwrap(), false);
    }

    #[tokio::test]
    async fn test_jwt_token_refresh() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();

        let result = auth_manager.refresh_token("invalid-token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_logout_invalid_token() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();

        let result = auth_manager.logout("invalid-token".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_auth_manager_getters() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();

        // Test that all getters return references to the managers
        assert!(std::ptr::eq(
            auth_manager.jwt_manager(),
            &auth_manager.jwt_manager
        ));
        assert!(std::ptr::eq(
            auth_manager.api_key_manager(),
            &auth_manager.api_key_manager
        ));
        assert!(std::ptr::eq(
            auth_manager.user_manager(),
            &auth_manager.user_manager
        ));
        assert!(std::ptr::eq(
            auth_manager.role_manager(),
            &auth_manager.role_manager
        ));
        assert!(std::ptr::eq(
            auth_manager.permission_manager(),
            &auth_manager.permission_manager
        ));
    }

    #[tokio::test]
    async fn test_auth_error_creation() {
        let auth_error = AuthError::authentication_failed("Test error".to_string());
        assert!(matches!(auth_error, AuthError::AuthenticationFailed(_)));

        let auth_error = AuthError::authorization_failed("Test error".to_string());
        assert!(matches!(auth_error, AuthError::AuthorizationFailed(_)));

        let auth_error = AuthError::token_generation("Test error".to_string());
        assert!(matches!(auth_error, AuthError::TokenGeneration(_)));

        let auth_error = AuthError::token_validation("Test error".to_string());
        assert!(matches!(auth_error, AuthError::TokenValidation(_)));

        let auth_error = AuthError::token_expired("Test error".to_string());
        assert!(matches!(auth_error, AuthError::TokenExpired(_)));

        let auth_error = AuthError::token_not_yet_valid("Test error".to_string());
        assert!(matches!(auth_error, AuthError::TokenNotYetValid(_)));

        let auth_error = AuthError::token_blacklisted("Test error".to_string());
        assert!(matches!(auth_error, AuthError::TokenBlacklisted(_)));

        let auth_error = AuthError::refresh_disabled("Test error".to_string());
        assert!(matches!(auth_error, AuthError::RefreshDisabled(_)));

        let auth_error = AuthError::user_not_found("Test error".to_string());
        assert!(matches!(auth_error, AuthError::UserNotFound(_)));

        let auth_error = AuthError::invalid_credentials("Test error".to_string());
        assert!(matches!(auth_error, AuthError::InvalidCredentials(_)));

        let auth_error = AuthError::account_locked("Test error".to_string());
        assert!(matches!(auth_error, AuthError::AccountLocked(_)));

        let auth_error = AuthError::rate_limit_exceeded("Test error".to_string());
        assert!(matches!(auth_error, AuthError::RateLimitExceeded(_)));

        let auth_error = AuthError::internal("Test error".to_string());
        assert!(matches!(auth_error, AuthError::Internal(_)));
    }

    #[tokio::test]
    async fn test_auth_response_creation() {
        let response = AuthResponse {
            status: AuthStatus::Authenticated {
                user_id: "test-user".to_string(),
                roles: vec!["user".to_string()],
                permissions: vec!["read".to_string()],
            },
            token: Some("test-token".to_string()),
            refresh_token: None,
            user: None,
            expires_at: Some(Utc::now()),
        };

        assert!(matches!(response.status, AuthStatus::Authenticated { .. }));
        assert_eq!(response.token, Some("test-token".to_string()));
        assert_eq!(response.refresh_token, None);
    }

    #[tokio::test]
    async fn test_auth_stats_default() {
        let stats = AuthStats::default();
        assert_eq!(stats.successful_authentications, 0);
        assert_eq!(stats.failed_authentications, 0);
        assert_eq!(stats.authorizations, 0);
        assert_eq!(stats.token_refreshes, 0);
        assert_eq!(stats.logouts, 0);
        assert_eq!(stats.last_authentication, None);
        assert_eq!(stats.last_authorization, None);
    }
}
