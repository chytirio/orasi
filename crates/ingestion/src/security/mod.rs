//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Security and reliability modules for telemetry ingestion
//!
//! This module provides comprehensive security and reliability features
//! including authentication, authorization, encryption, and circuit breakers.

pub mod auth;
pub mod authorization;
pub mod circuit_breaker;
pub mod encryption;
pub mod tls;

pub use auth::{
    AuthConfig, AuthResult as InternalAuthResult, AuthenticationManager, User as LocalUser,
};
pub use authorization::{
    Action, AuditLogEntry, AuditResult, AuthorizationConfig, AuthorizationContext,
    AuthorizationManager, IpRestrictions, Permission, PermissionConditions, RbacSystem,
    ResourcePermissions, ResourceRestrictions, Role, TimeRestrictions, TimeWindow,
};
pub use bridge_auth::{AuthMethod, JwtClaims, User};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerState};
pub use encryption::{EncryptionAlgorithm, EncryptionManager};
pub use tls::{TlsConfig, TlsManager};

/// Security manager for comprehensive security features
pub struct SecurityManager {
    auth_manager: AuthenticationManager,
    authorization_manager: AuthorizationManager,
    encryption_manager: EncryptionManager,
    circuit_breaker: CircuitBreaker,
    tls_manager: TlsManager,
}

impl SecurityManager {
    /// Create new security manager
    pub async fn new(
        auth_config: AuthConfig,
        authorization_config: AuthorizationConfig,
        tls_config: Option<TlsConfig>,
    ) -> BridgeResult<Self> {
        let auth_manager = AuthenticationManager::new(auth_config).map_err(|e| {
            bridge_core::BridgeError::authentication(format!(
                "Failed to create auth manager: {}",
                e
            ))
        })?;
        let authorization_manager = AuthorizationManager::new(authorization_config);
        let encryption_manager = EncryptionManager::new(EncryptionAlgorithm::Aes256Gcm)?;
        let circuit_breaker = CircuitBreaker::new(5, std::time::Duration::from_secs(60));

        // Create TLS manager with default config if none provided
        let tls_config = tls_config.unwrap_or_else(|| TlsConfig {
            cert_file: std::path::PathBuf::from(""),
            key_file: std::path::PathBuf::from(""),
            ca_file: None,
            verify_client: false,
        });
        let tls_manager = TlsManager::new(tls_config)?;

        Ok(Self {
            auth_manager,
            authorization_manager,
            encryption_manager,
            circuit_breaker,
            tls_manager,
        })
    }

    /// Initialize security manager
    pub async fn initialize(&self) -> BridgeResult<()> {
        info!("Security manager initialized successfully");
        Ok(())
    }

    /// Get authentication manager
    pub fn auth_manager(&self) -> &AuthenticationManager {
        &self.auth_manager
    }

    /// Get encryption manager
    pub fn encryption_manager(&self) -> &EncryptionManager {
        &self.encryption_manager
    }

    /// Get circuit breaker
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }

    /// Encrypt data
    pub async fn encrypt_data(&mut self, data: &[u8]) -> BridgeResult<Vec<u8>> {
        self.encryption_manager.encrypt(data)
    }

    /// Decrypt data
    pub async fn decrypt_data(&self, encrypted_data: &[u8]) -> BridgeResult<Vec<u8>> {
        self.encryption_manager.decrypt(encrypted_data)
    }

    /// Execute with circuit breaker
    pub async fn execute_with_circuit_breaker<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + 'static + std::convert::From<std::io::Error>,
    {
        if !self.circuit_breaker.can_execute() {
            return Err(
                std::io::Error::new(std::io::ErrorKind::Other, "Circuit breaker is open").into(),
            );
        }

        match operation() {
            Ok(result) => {
                self.circuit_breaker.record_success();
                Ok(result)
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                Err(e)
            }
        }
    }

    /// Authenticate a user
    pub async fn authenticate(
        &self,
        request: bridge_auth::AuthRequest,
    ) -> BridgeResult<bridge_auth::AuthResponse> {
        // Convert the bridge_auth::AuthRequest to our internal authentication
        let auth_result = match &request.method {
            bridge_auth::AuthMethod::UsernamePassword => {
                if let bridge_auth::AuthCredentials::UsernamePassword { username, password } =
                    &request.credentials
                {
                    // Use the internal authentication manager
                    match self
                        .auth_manager
                        .authenticate_user(username, password)
                        .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            return Err(bridge_core::BridgeError::authentication(format!(
                                "Authentication failed: {}",
                                e
                            )))
                        }
                    }
                } else {
                    return Err(bridge_core::BridgeError::authentication(
                        "Invalid credentials for username/password authentication".to_string(),
                    ));
                }
            }
            bridge_auth::AuthMethod::Jwt => {
                if let bridge_auth::AuthCredentials::JwtToken { token } = &request.credentials {
                    // Use the internal authentication manager for JWT validation
                    match self.auth_manager.authenticate_token(token).await {
                        Ok(result) => result,
                        Err(e) => {
                            return Err(bridge_core::BridgeError::authentication(format!(
                                "JWT authentication failed: {}",
                                e
                            )))
                        }
                    }
                } else {
                    return Err(bridge_core::BridgeError::authentication(
                        "Invalid credentials for JWT authentication".to_string(),
                    ));
                }
            }
            bridge_auth::AuthMethod::ApiKey => {
                if let bridge_auth::AuthCredentials::ApiKey { key } = &request.credentials {
                    // Use the internal authentication manager for API key validation
                    match self.auth_manager.authenticate_api_key(key).await {
                        Ok(result) => result,
                        Err(e) => {
                            return Err(bridge_core::BridgeError::authentication(format!(
                                "API key authentication failed: {}",
                                e
                            )))
                        }
                    }
                } else {
                    return Err(bridge_core::BridgeError::authentication(
                        "Invalid credentials for API key authentication".to_string(),
                    ));
                }
            }
            bridge_auth::AuthMethod::OAuth { provider, .. } => {
                // OAuth authentication is not implemented in the internal auth manager
                return Err(bridge_core::BridgeError::authentication(
                    "OAuth authentication not implemented in internal auth manager".to_string(),
                ));
            }
        };

        // Convert internal AuthResult to bridge_auth::AuthResponse
        let response = self
            .convert_auth_result_to_response(auth_result, &request)
            .await;
        Ok(response)
    }

    /// Convert internal AuthResult to bridge_auth::AuthResponse
    async fn convert_auth_result_to_response(
        &self,
        auth_result: InternalAuthResult,
        request: &bridge_auth::AuthRequest,
    ) -> bridge_auth::AuthResponse {
        if auth_result.success {
            // Convert internal User to bridge_auth::User if authentication was successful
            let bridge_user = if let Some(internal_user) = auth_result.user {
                Some(bridge_auth::User {
                    id: internal_user.id,
                    username: internal_user.username,
                    email: internal_user.email,
                    password_hash: internal_user.password_hash,
                    roles: internal_user
                        .roles
                        .iter()
                        .map(|role| {
                            // Convert string role to UserRole enum
                            match role.as_str() {
                                "admin" => bridge_auth::UserRole::Admin,
                                "user" => bridge_auth::UserRole::User,
                                "readonly" => bridge_auth::UserRole::ReadOnly,
                                _ => bridge_auth::UserRole::Custom(role.clone()),
                            }
                        })
                        .collect(),
                    is_active: internal_user.active,
                    is_locked: false, // Internal auth manager doesn't track lockouts
                    failed_login_attempts: 0, // Internal auth manager doesn't track failed attempts
                    lockout_until: None,
                    created_at: internal_user.created_at,
                    last_login_at: internal_user.last_login,
                    oauth_provider: None,
                    oauth_provider_user_id: None,
                })
            } else {
                None
            };

            // Calculate expiration time based on token type
            let expires_at = match request.method {
                bridge_auth::AuthMethod::Jwt => {
                    // JWT tokens typically expire in 1 hour
                    Some(chrono::Utc::now() + chrono::Duration::hours(1))
                }
                bridge_auth::AuthMethod::ApiKey => {
                    // API keys typically expire in 30 days
                    Some(chrono::Utc::now() + chrono::Duration::days(30))
                }
                _ => None,
            };

            bridge_auth::AuthResponse {
                status: bridge_auth::AuthStatus::Authenticated {
                    user_id: bridge_user
                        .as_ref()
                        .map(|u| u.id.clone())
                        .unwrap_or_default(),
                    roles: bridge_user
                        .as_ref()
                        .map(|u| u.roles.iter().map(|r| format!("{:?}", r)).collect())
                        .unwrap_or_default(),
                    permissions: bridge_user.as_ref().map(|u| vec![]).unwrap_or_default(), // Internal user doesn't have permissions in the bridge_auth format
                },
                token: auth_result.token,
                refresh_token: auth_result.refresh_token,
                user: bridge_user,
                expires_at,
            }
        } else {
            bridge_auth::AuthResponse {
                status: bridge_auth::AuthStatus::Failed {
                    reason: auth_result
                        .error_message
                        .unwrap_or_else(|| "Authentication failed".to_string()),
                },
                token: None,
                refresh_token: None,
                user: None,
                expires_at: None,
            }
        }
    }

    /// Authorize an action
    pub async fn check_permission(
        &self,
        user_id: &str,
        resource: &str,
        action: &authorization::Action,
        context: Option<AuthorizationContext>,
    ) -> BridgeResult<bool> {
        self.authorization_manager
            .check_permission(user_id, resource, action, context)
            .await
    }

    /// Check if operation can be executed (circuit breaker)
    pub fn can_execute_operation(&self) -> bool {
        self.circuit_breaker.can_execute()
    }

    /// Record operation success
    pub fn record_operation_success(&self) {
        self.circuit_breaker.record_success();
    }

    /// Record operation failure
    pub fn record_operation_failure(&self) {
        self.circuit_breaker.record_failure();
    }

    /// Check if TLS is enabled
    pub fn is_tls_enabled(&self) -> bool {
        self.tls_manager.is_enabled()
    }

    /// Validate TLS configuration
    pub fn validate_tls_config(&self) -> BridgeResult<()> {
        self.tls_manager.validate_config()
    }

    /// Initialize TLS
    pub async fn initialize_tls(&mut self) -> BridgeResult<()> {
        self.tls_manager.initialize().await
    }

    /// Get audit logs
    pub async fn get_audit_logs(&self) -> BridgeResult<Vec<AuditLogEntry>> {
        self.authorization_manager.get_audit_log(None).await
    }
}

use bridge_core::BridgeResult;
use tracing::info;
