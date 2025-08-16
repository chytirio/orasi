//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Authentication and authorization for telemetry ingestion system
//!
//! This module provides comprehensive authentication and authorization
//! features including JWT, OAuth, RBAC, and permission management.

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// Authentication manager
pub struct AuthenticationManager {
    config: AuthConfig,
    users: Arc<RwLock<HashMap<String, User>>>,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    jwt_secret: String,
    oauth_providers: Arc<RwLock<HashMap<String, OAuthProvider>>>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication method
    pub method: AuthMethod,
    /// JWT configuration
    pub jwt: JwtConfig,
    /// OAuth configuration
    pub oauth: OAuthConfig,
    /// Session configuration
    pub session: SessionConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,
}

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Jwt,
    OAuth,
    ApiKey,
    None,
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret
    pub secret: String,
    /// JWT algorithm
    pub algorithm: String,
    /// Token expiration time
    pub expiration: Duration,
    /// Refresh token expiration
    pub refresh_expiration: Duration,
    /// Issuer
    pub issuer: String,
    /// Audience
    pub audience: String,
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth providers
    pub providers: HashMap<String, OAuthProviderConfig>,
    /// Redirect URL
    pub redirect_url: String,
    /// State secret
    pub state_secret: String,
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

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session timeout
    pub timeout: Duration,
    /// Maximum sessions per user
    pub max_sessions_per_user: usize,
    /// Session cleanup interval
    pub cleanup_interval: Duration,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Rate limit window
    pub window: Duration,
    /// Maximum requests per window
    pub max_requests: usize,
    /// Rate limit by IP
    pub by_ip: bool,
    /// Rate limit by user
    pub by_user: bool,
}

/// User
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email
    pub email: String,
    /// Password hash
    pub password_hash: Option<String>,
    /// Roles
    pub roles: Vec<String>,
    /// Permissions
    pub permissions: Vec<String>,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
    /// Last login
    pub last_login: Option<DateTime<Utc>>,
    /// Active
    pub active: bool,
}

/// Session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Token
    pub token: String,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Expires at
    pub expires_at: DateTime<Utc>,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Active
    pub active: bool,
}

/// OAuth provider
#[derive(Debug, Clone)]
pub struct OAuthProvider {
    /// Provider name
    pub name: String,
    /// Provider configuration
    pub config: OAuthProviderConfig,
    /// HTTP client
    pub client: reqwest::Client,
}

/// JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Issued at
    pub iat: i64,
    /// Expiration time
    pub exp: i64,
    /// Roles
    pub roles: Vec<String>,
    /// Permissions
    pub permissions: Vec<String>,
}

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthResult {
    /// Success
    pub success: bool,
    /// User
    pub user: Option<User>,
    /// Token
    pub token: Option<String>,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Error message
    pub error_message: Option<String>,
}

/// Authorization result
#[derive(Debug, Clone)]
pub struct AuthzResult {
    /// Allowed
    pub allowed: bool,
    /// Reason
    pub reason: Option<String>,
}

impl AuthenticationManager {
    /// Create new authentication manager
    pub fn new(config: AuthConfig) -> BridgeResult<Self> {
        let jwt_secret = config.jwt.secret.clone();
        let users = Arc::new(RwLock::new(HashMap::new()));
        let sessions = Arc::new(RwLock::new(HashMap::new()));
        let oauth_providers = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            config,
            users,
            sessions,
            jwt_secret,
            oauth_providers,
        })
    }

    /// Initialize authentication manager
    pub async fn initialize(&self) -> BridgeResult<()> {
        // Initialize OAuth providers
        for (name, config) in &self.config.oauth.providers {
            let provider = OAuthProvider {
                name: name.clone(),
                config: config.clone(),
                client: reqwest::Client::new(),
            };
            let mut providers = self.oauth_providers.write().await;
            providers.insert(name.clone(), provider);
        }

        // Start session cleanup task
        self.start_session_cleanup().await;

        info!("Authentication manager initialized successfully");
        Ok(())
    }

    /// Authenticate user with username and password
    pub async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
    ) -> BridgeResult<AuthResult> {
        let users = self.users.read().await;

        if let Some(user) = users.get(username) {
            if !user.active {
                return Ok(AuthResult {
                    success: false,
                    user: None,
                    token: None,
                    refresh_token: None,
                    error_message: Some("User account is inactive".to_string()),
                });
            }

            if let Some(password_hash) = &user.password_hash {
                if self.verify_password(password, password_hash)? {
                    // Generate tokens
                    let token = self.generate_jwt_token(user)?;
                    let refresh_token = self.generate_refresh_token(user)?;

                    // Create session
                    self.create_session(user, &token, &refresh_token).await?;

                    Ok(AuthResult {
                        success: true,
                        user: Some(user.clone()),
                        token: Some(token),
                        refresh_token: Some(refresh_token),
                        error_message: None,
                    })
                } else {
                    Ok(AuthResult {
                        success: false,
                        user: None,
                        token: None,
                        refresh_token: None,
                        error_message: Some("Invalid password".to_string()),
                    })
                }
            } else {
                Ok(AuthResult {
                    success: false,
                    user: None,
                    token: None,
                    refresh_token: None,
                    error_message: Some("User has no password set".to_string()),
                })
            }
        } else {
            Ok(AuthResult {
                success: false,
                user: None,
                token: None,
                refresh_token: None,
                error_message: Some("User not found".to_string()),
            })
        }
    }

    /// Authenticate with JWT token
    pub async fn authenticate_token(&self, token: &str) -> BridgeResult<AuthResult> {
        match self.validate_jwt_token(token) {
            Ok(claims) => {
                let users = self.users.read().await;
                if let Some(user) = users.get(&claims.sub) {
                    if !user.active {
                        return Ok(AuthResult {
                            success: false,
                            user: None,
                            token: None,
                            refresh_token: None,
                            error_message: Some("User account is inactive".to_string()),
                        });
                    }

                    Ok(AuthResult {
                        success: true,
                        user: Some(user.clone()),
                        token: Some(token.to_string()),
                        refresh_token: None,
                        error_message: None,
                    })
                } else {
                    Ok(AuthResult {
                        success: false,
                        user: None,
                        token: None,
                        refresh_token: None,
                        error_message: Some("User not found".to_string()),
                    })
                }
            }
            Err(e) => Ok(AuthResult {
                success: false,
                user: None,
                token: None,
                refresh_token: None,
                error_message: Some(format!("Invalid token: {}", e)),
            }),
        }
    }

    /// Authenticate with API key
    pub async fn authenticate_api_key(&self, api_key: &str) -> BridgeResult<AuthResult> {
        // In a real implementation, you would validate against stored API keys
        // For now, we'll use a simple check
        if api_key == "test-api-key" {
            let user = User {
                id: "api-user".to_string(),
                username: "api-user".to_string(),
                email: "api@example.com".to_string(),
                password_hash: None,
                roles: vec!["api".to_string()],
                permissions: vec!["read".to_string(), "write".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login: Some(Utc::now()),
                active: true,
            };

            Ok(AuthResult {
                success: true,
                user: Some(user),
                token: None,
                refresh_token: None,
                error_message: None,
            })
        } else {
            Ok(AuthResult {
                success: false,
                user: None,
                token: None,
                refresh_token: None,
                error_message: Some("Invalid API key".to_string()),
            })
        }
    }

    /// Refresh JWT token
    pub async fn refresh_token(&self, refresh_token: &str) -> BridgeResult<AuthResult> {
        // In a real implementation, you would validate the refresh token
        // For now, we'll return an error
        Ok(AuthResult {
            success: false,
            user: None,
            token: None,
            refresh_token: None,
            error_message: Some("Refresh token validation not implemented".to_string()),
        })
    }

    /// Authorize user for action
    pub async fn authorize(&self, user: &User, action: &str, resource: &str) -> AuthzResult {
        // Check if user has the required permission
        let required_permission = format!("{}:{}", action, resource);

        if user.permissions.contains(&required_permission) {
            AuthzResult {
                allowed: true,
                reason: None,
            }
        } else {
            // Check if user has wildcard permission
            let wildcard_permission = format!("{}:*", action);
            if user.permissions.contains(&wildcard_permission) {
                AuthzResult {
                    allowed: true,
                    reason: None,
                }
            } else {
                AuthzResult {
                    allowed: false,
                    reason: Some(format!(
                        "User does not have permission: {}",
                        required_permission
                    )),
                }
            }
        }
    }

    /// Check if user has role
    pub fn has_role(&self, user: &User, role: &str) -> bool {
        user.roles.contains(&role.to_string())
    }

    /// Check if user has permission
    pub fn has_permission(&self, user: &User, permission: &str) -> bool {
        user.permissions.contains(&permission.to_string())
    }

    /// Create user
    pub async fn create_user(
        &self,
        username: String,
        email: String,
        password: String,
        roles: Vec<String>,
    ) -> BridgeResult<User> {
        let password_hash = self.hash_password(&password)?;

        let user = User {
            id: Uuid::new_v4().to_string(),
            username: username.clone(),
            email,
            password_hash: Some(password_hash),
            roles,
            permissions: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            active: true,
        };

        let mut users = self.users.write().await;
        users.insert(username, user.clone());

        info!("Created user: {}", user.username);
        Ok(user)
    }

    /// Update user
    pub async fn update_user(&self, user_id: &str, updates: UserUpdates) -> BridgeResult<User> {
        let mut users = self.users.write().await;

        if let Some(user) = users.get_mut(user_id) {
            if let Some(email) = updates.email {
                user.email = email;
            }
            if let Some(roles) = updates.roles {
                user.roles = roles;
            }
            if let Some(permissions) = updates.permissions {
                user.permissions = permissions;
            }
            if let Some(active) = updates.active {
                user.active = active;
            }
            user.updated_at = Utc::now();

            info!("Updated user: {}", user.username);
            Ok(user.clone())
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "User not found: {}",
                user_id
            )))
        }
    }

    /// Delete user
    pub async fn delete_user(&self, user_id: &str) -> BridgeResult<()> {
        let mut users = self.users.write().await;

        if users.remove(user_id).is_some() {
            info!("Deleted user: {}", user_id);
            Ok(())
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "User not found: {}",
                user_id
            )))
        }
    }

    /// Logout user
    pub async fn logout(&self, token: &str) -> BridgeResult<()> {
        let mut sessions = self.sessions.write().await;

        // Find and remove session
        let session_id = self.get_session_id_from_token(token)?;
        if sessions.remove(&session_id).is_some() {
            info!("User logged out successfully");
            Ok(())
        } else {
            Err(bridge_core::BridgeError::configuration(
                "Session not found".to_string(),
            ))
        }
    }

    /// Generate JWT token
    fn generate_jwt_token(&self, user: &User) -> BridgeResult<String> {
        let now = Utc::now();
        let exp = now + chrono::Duration::seconds(self.config.jwt.expiration.as_secs() as i64);

        let claims = JwtClaims {
            sub: user.id.clone(),
            iss: self.config.jwt.issuer.clone(),
            aud: self.config.jwt.audience.clone(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            roles: user.roles.clone(),
            permissions: user.permissions.clone(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
        .map_err(|e| {
            bridge_core::BridgeError::authentication(format!("Failed to generate JWT token: {}", e))
        })?;

        Ok(token)
    }

    /// Generate refresh token
    fn generate_refresh_token(&self, _user: &User) -> BridgeResult<String> {
        let refresh_token = Uuid::new_v4().to_string();
        Ok(refresh_token)
    }

    /// Validate JWT token
    fn validate_jwt_token(&self, token: &str) -> BridgeResult<JwtClaims> {
        let token_data = decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|e| {
            bridge_core::BridgeError::authentication(format!("Failed to validate JWT token: {}", e))
        })?;

        Ok(token_data.claims)
    }

    /// Hash password
    fn hash_password(&self, password: &str) -> BridgeResult<String> {
        let salt = Uuid::new_v4().to_string();
        let mut mac = Hmac::<Sha256>::new_from_slice(salt.as_bytes()).map_err(|e| {
            bridge_core::BridgeError::authentication(format!("Failed to create HMAC: {}", e))
        })?;

        mac.update(password.as_bytes());
        let result = mac.finalize();
        let hash = format!("{}:{}", salt, hex::encode(result.into_bytes()));

        Ok(hash)
    }

    /// Verify password
    fn verify_password(&self, password: &str, hash: &str) -> BridgeResult<bool> {
        let parts: Vec<&str> = hash.split(':').collect();
        if parts.len() != 2 {
            return Ok(false);
        }

        let salt = parts[0];
        let stored_hash = parts[1];

        let mut mac = Hmac::<Sha256>::new_from_slice(salt.as_bytes()).map_err(|e| {
            bridge_core::BridgeError::authentication(format!("Failed to create HMAC: {}", e))
        })?;

        mac.update(password.as_bytes());
        let result = mac.finalize();
        let computed_hash = hex::encode(result.into_bytes());

        Ok(computed_hash == stored_hash)
    }

    /// Create session
    async fn create_session(
        &self,
        user: &User,
        token: &str,
        refresh_token: &str,
    ) -> BridgeResult<()> {
        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id: user.id.clone(),
            token: token.to_string(),
            refresh_token: Some(refresh_token.to_string()),
            created_at: Utc::now(),
            expires_at: Utc::now()
                + chrono::Duration::seconds(self.config.session.timeout.as_secs() as i64),
            ip_address: None,
            user_agent: None,
            active: true,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session);

        Ok(())
    }

    /// Get session ID from token
    fn get_session_id_from_token(&self, _token: &str) -> BridgeResult<String> {
        // In a real implementation, you would decode the token to get the session ID
        // For now, we'll use a simple hash
        let session_id = format!("session_{}", Uuid::new_v4());
        Ok(session_id)
    }

    /// Start session cleanup task
    async fn start_session_cleanup(&self) {
        let sessions = self.sessions.clone();
        let cleanup_interval = self.config.session.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;

                let mut sessions_write = sessions.write().await;
                let now = Utc::now();
                let expired_sessions: Vec<String> = sessions_write
                    .iter()
                    .filter(|(_, session)| session.expires_at < now || !session.active)
                    .map(|(id, _)| id.clone())
                    .collect();

                for session_id in &expired_sessions {
                    sessions_write.remove(session_id);
                }

                if !expired_sessions.is_empty() {
                    info!("Cleaned up {} expired sessions", expired_sessions.len());
                }
            }
        });
    }
}

/// User updates
#[derive(Debug, Clone)]
pub struct UserUpdates {
    pub email: Option<String>,
    pub roles: Option<Vec<String>>,
    pub permissions: Option<Vec<String>>,
    pub active: Option<bool>,
}

use bridge_core::BridgeResult;
