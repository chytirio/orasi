//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Authentication and Authorization for the OpenTelemetry Bridge
//!
//! This crate provides comprehensive authentication and authorization capabilities
//! including JWT tokens, API keys, OAuth2 integration, and role-based access control.
//!
//! # Features
//!
//! - **JWT Authentication**: Secure token-based authentication with configurable algorithms
//! - **API Key Management**: Generate and manage API keys with fine-grained permissions
//! - **OAuth2 Integration**: Support for multiple OAuth providers (GitHub, Google, etc.)
//! - **Role-Based Access Control (RBAC)**: Flexible role and permission management
//! - **User Management**: Complete user lifecycle management with password hashing
//! - **Rate Limiting**: Built-in rate limiting to prevent abuse
//! - **Session Management**: Token blacklisting and refresh token support
//! - **HTTP Middleware**: Ready-to-use middleware for Axum web frameworks
//!
//! # Quick Start
//!
//! ```rust
//! use auth::{AuthConfig, AuthManager, AuthMethod, AuthCredentials, AuthRequest};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize authentication system
//!     let config = AuthConfig::default();
//!     let auth_manager = AuthManager::new(config).await?;
//!
//!     // Authenticate a user
//!     let request = AuthRequest {
//!         method: AuthMethod::UsernamePassword,
//!         credentials: AuthCredentials::UsernamePassword {
//!             username: "user@example.com".to_string(),
//!             password: "password123".to_string(),
//!         },
//!         metadata: HashMap::new(),
//!     };
//!
//!     match auth_manager.authenticate(request).await {
//!         Ok(response) => {
//!             println!("Authentication successful: {:?}", response.status);
//!         }
//!         Err(e) => {
//!             println!("Authentication failed: {}", e);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Configuration
//!
//! The authentication system is highly configurable through the `AuthConfig` struct:
//!
//! ```rust
//! use auth::AuthConfig;
//!
//! let mut config = AuthConfig::default();
//!
//! // Configure JWT settings
//! config.jwt.secret = "your-super-secret-key".to_string();
//! config.jwt.expiration_secs = 3600; // 1 hour
//! config.jwt.enable_refresh_tokens = true;
//!
//! // Configure API keys
//! config.api_keys.enabled = true;
//! config.api_keys.expiration_secs = 30 * 24 * 3600; // 30 days
//!
//! // Configure security settings
//! config.security.enable_rate_limiting = true;
//! config.security.rate_limit_per_minute = 100;
//! ```
//!
//! # HTTP Middleware Integration
//!
//! The crate provides ready-to-use middleware for Axum web frameworks:
//!
//! ```rust
//! use auth::middleware::{AuthMiddleware, AuthExtractor};
//! use axum::{extract::State, http::HeaderMap, Json};
//! use std::sync::Arc;
//!
//! async fn protected_route(
//!     State(auth_manager): State<Arc<AuthManager>>,
//!     headers: HeaderMap,
//! ) -> Json<serde_json::Value> {
//!     let auth_extractor = AuthExtractor::extract(headers, auth_manager)
//!         .await
//!         .expect("Authentication required");
//!
//!     Json(serde_json::json!({
//!         "message": "Access granted",
//!         "user_id": auth_extractor.user_id,
//!         "roles": auth_extractor.roles,
//!     }))
//! }
//! ```
//!
//! # Security Best Practices
//!
//! ## JWT Security
//!
//! - Use strong, randomly generated secrets (at least 256 bits)
//! - Set appropriate token expiration times
//! - Enable token blacklisting for logout functionality
//! - Use HTTPS in production to prevent token interception
//! - Consider using RS256/RS384/RS512 algorithms for better security
//!
//! ## API Key Security
//!
//! - Generate cryptographically secure random keys
//! - Set appropriate expiration times
//! - Implement key rotation policies
//! - Monitor key usage for suspicious activity
//! - Use HTTPS to transmit API keys
//!
//! ## Password Security
//!
//! - Enforce strong password policies
//! - Use Argon2 for password hashing (default)
//! - Implement account lockout after failed attempts
//! - Enable two-factor authentication when possible
//!
//! ## Rate Limiting
//!
//! - Enable rate limiting to prevent brute force attacks
//! - Configure appropriate limits based on your use case
//! - Monitor rate limit violations for security threats
//! - Consider implementing progressive delays
//!
//! # Error Handling
//!
//! The crate provides comprehensive error types for different failure scenarios:
//!
//! ```rust
//! use auth::AuthError;
//!
//! match auth_result {
//!     Ok(response) => {
//!         // Handle successful authentication
//!     }
//!     Err(AuthError::AuthenticationFailed(msg)) => {
//!         // Handle authentication failure
//!     }
//!     Err(AuthError::AuthorizationFailed(msg)) => {
//!         // Handle authorization failure
//!     }
//!     Err(AuthError::TokenExpired(msg)) => {
//!         // Handle expired token
//!     }
//!     Err(AuthError::RateLimitExceeded(msg)) => {
//!         // Handle rate limit exceeded
//!     }
//!     Err(AuthError::Internal(msg)) => {
//!         // Handle internal errors
//!     }
//! }
//! ```
//!
//! # Monitoring and Observability
//!
//! The crate provides built-in statistics and metrics:
//!
//! ```rust
//! // Get authentication statistics
//! let auth_stats = auth_manager.get_stats().await;
//! println!("Successful authentications: {}", auth_stats.successful_authentications);
//! println!("Failed authentications: {}", auth_stats.failed_authentications);
//!
//! // Get JWT statistics
//! let jwt_stats = auth_manager.jwt_manager().get_stats().await;
//! println!("Tokens generated: {}", jwt_stats.tokens_generated);
//! println!("Tokens validated: {}", jwt_stats.tokens_validated);
//!
//! // Get API key statistics
//! let api_key_stats = auth_manager.api_key_manager().get_stats().await;
//! println!("Active API keys: {}", api_key_stats.active_keys);
//! ```
//!
//! # Testing
//!
//! The crate includes comprehensive unit tests and provides test utilities:
//!
//! ```rust
//! #[cfg(test)]
//! mod tests {
//!     use super::*;
//!
//!     #[tokio::test]
//!     async fn test_authentication() {
//!         let config = AuthConfig::default();
//!         let auth_manager = AuthManager::new(config).await.unwrap();
//!
//!         // Test authentication logic
//!         // ...
//!     }
//! }

pub mod api_keys;
pub mod auth;
pub mod config;
pub mod jwt;
pub mod middleware;
pub mod oauth;
pub mod permissions;
pub mod roles;
pub mod users;

// Re-export commonly used types
pub use api_keys::{ApiKey, ApiKeyManager, ApiKeyStats};
pub use auth::{
    AuthCredentials, AuthError, AuthManager, AuthMethod, AuthRequest, AuthResponse, AuthResult,
    AuthStats, AuthStatus,
};
pub use config::AuthConfig;
pub use jwt::{JwtClaims, JwtManager, JwtStats};
pub use middleware::{AuthExtractor, AuthMiddleware};
pub use oauth::{OAuthManager, OAuthProvider, OAuthStats};
pub use permissions::{AccessLevel, PermissionManager, PermissionStats};
pub use roles::{Permission, Role, RoleManager, RoleStats};
pub use users::{User, UserManager, UserRole, UserStats};

/// Authentication and authorization version
pub const AUTH_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default JWT expiration time in seconds (24 hours)
pub const DEFAULT_JWT_EXPIRATION_SECS: u64 = 24 * 60 * 60;

/// Default API key expiration time in seconds (30 days)
pub const DEFAULT_API_KEY_EXPIRATION_SECS: u64 = 30 * 24 * 60 * 60;

/// Default password hash rounds for Argon2
pub const DEFAULT_PASSWORD_HASH_ROUNDS: u32 = 10;

/// Default session timeout in seconds (8 hours)
pub const DEFAULT_SESSION_TIMEOUT_SECS: u64 = 8 * 60 * 60;

/// Initialize authentication system
///
/// This function creates and initializes a new `AuthManager` with the provided configuration.
/// It sets up all the necessary components including JWT, API key, OAuth, user, role, and
/// permission managers.
///
/// # Arguments
///
/// * `config` - The authentication configuration
///
/// # Returns
///
/// Returns a `Result` containing the initialized `AuthManager` or an error if initialization fails.
///
/// # Example
///
/// ```rust
/// use auth::{init_auth_system, AuthConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = AuthConfig::default();
///     let auth_manager = init_auth_system(config).await?;
///     
///     // Use auth_manager for authentication operations
///     Ok(())
/// }
/// ```
pub async fn init_auth_system(
    config: AuthConfig,
) -> Result<AuthManager, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Initializing authentication system v{}", AUTH_VERSION);

    let auth_manager = AuthManager::new(config).await?;

    tracing::info!("Authentication system initialization completed");
    Ok(auth_manager)
}

/// Shutdown authentication system
///
/// This function gracefully shuts down the authentication system, performing any
/// necessary cleanup operations such as cleaning up expired tokens and closing
/// database connections.
///
/// # Arguments
///
/// * `auth_manager` - The authentication manager to shutdown
///
/// # Returns
///
/// Returns a `Result` indicating success or failure of the shutdown process.
///
/// # Example
///
/// ```rust
/// use auth::{init_auth_system, shutdown_auth_system, AuthConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = AuthConfig::default();
///     let auth_manager = init_auth_system(config).await?;
///     
///     // Perform authentication operations
///     
///     // Shutdown when done
///     shutdown_auth_system(auth_manager).await?;
///     Ok(())
/// }
/// ```
pub async fn shutdown_auth_system(
    auth_manager: AuthManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Shutting down authentication system");

    auth_manager.shutdown().await?;

    tracing::info!("Authentication system shutdown completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_system_initialization() {
        let config = AuthConfig::default();
        let result = init_auth_system(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_auth_system_shutdown() {
        let config = AuthConfig::default();
        let auth_manager = init_auth_system(config).await.unwrap();
        let result = shutdown_auth_system(auth_manager).await;
        assert!(result.is_ok());
    }
}
