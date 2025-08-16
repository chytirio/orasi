//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Authentication and Authorization Example
//!
//! This example demonstrates the comprehensive authentication and authorization
//! capabilities of the OpenTelemetry Bridge, including JWT tokens, API keys,
//! OAuth, user management, role-based access control, and HTTP middleware.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use bridge_auth::{
    init_auth_system,
    middleware::{AuthExtractor, AuthMiddleware},
    shutdown_auth_system, AuthConfig, AuthCredentials, AuthManager, AuthMethod, AuthRequest,
    AuthStatus, User, UserRole,
};

/// Login request
#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

/// Login response
#[derive(Debug, Serialize)]
struct LoginResponse {
    token: String,
    user: User,
    expires_at: String,
}

/// API key request
#[derive(Debug, Deserialize)]
struct ApiKeyRequest {
    name: String,
    permissions: Vec<String>,
}

/// API key response
#[derive(Debug, Serialize)]
struct ApiKeyResponse {
    key: String,
    name: String,
    expires_at: String,
}

/// Protected resource response
#[derive(Debug, Serialize)]
struct ProtectedResourceResponse {
    message: String,
    user_id: String,
    roles: Vec<String>,
    permissions: Vec<String>,
}

/// Health check response
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    auth_stats: bridge_auth::AuthStats,
    jwt_stats: bridge_auth::JwtStats,
    api_key_stats: bridge_auth::ApiKeyStats,
}

/// Create authentication routes
fn create_auth_routes(auth_manager: Arc<AuthManager>) -> Router {
    Router::new()
        .route("/login", post(login_handler))
        .route("/logout", post(logout_handler))
        .route("/refresh", post(refresh_token_handler))
        .route("/api-keys", post(create_api_key_handler))
        .route("/api-keys", get(list_api_keys_handler))
        .route("/users", post(create_user_handler))
        .route("/users", get(list_users_handler))
        .route("/roles", post(assign_role_handler))
        .route("/protected", get(protected_resource_handler))
        .route("/admin", get(admin_resource_handler))
        .route("/metrics", get(metrics_resource_handler))
        .route("/health", get(health_check_handler))
        .with_state(auth_manager)
}

/// Login handler
async fn login_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    Json(login_req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    info!("Login attempt for user: {}", login_req.username);

    let auth_request = AuthRequest {
        method: AuthMethod::UsernamePassword,
        credentials: AuthCredentials::UsernamePassword {
            username: login_req.username,
            password: login_req.password,
        },
        metadata: HashMap::new(),
    };

    match auth_manager.authenticate(auth_request).await {
        Ok(auth_response) => {
            if let AuthStatus::Authenticated {
                user_id,
                roles,
                permissions: _,
            } = auth_response.status
            {
                if let Some(token) = auth_response.token {
                    if let Some(user) = auth_response.user {
                        if let Some(expires_at) = auth_response.expires_at {
                            info!("Login successful for user: {}", user_id);

                            // Assign default role if user has no roles
                            if roles.is_empty() {
                                auth_manager
                                    .role_manager()
                                    .assign_role_to_user(&user_id, "user")
                                    .await
                                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                            }

                            return Ok(Json(LoginResponse {
                                token,
                                user,
                                expires_at: expires_at.to_rfc3339(),
                            }));
                        }
                    }
                }
            }
            Err(StatusCode::UNAUTHORIZED)
        }
        Err(e) => {
            warn!("Login failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Logout handler
async fn logout_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let middleware = AuthMiddleware::new(auth_manager.clone());

    if let Some(auth_request) = middleware.extract_auth(&headers).await {
        if let AuthCredentials::JwtToken { token } = auth_request.credentials {
            match auth_manager.logout(token).await {
                Ok(_) => {
                    info!("Logout successful");
                    Ok(Json(serde_json::json!({
                        "message": "Logout successful"
                    })))
                }
                Err(e) => {
                    warn!("Logout failed: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Refresh token handler
async fn refresh_token_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let middleware = AuthMiddleware::new(auth_manager.clone());

    if let Some(auth_request) = middleware.extract_auth(&headers).await {
        if let AuthCredentials::JwtToken { token } = auth_request.credentials {
            match auth_manager.refresh_token(&token).await {
                Ok(new_token) => {
                    info!("Token refreshed successfully");
                    Ok(Json(serde_json::json!({
                        "token": new_token,
                        "message": "Token refreshed successfully"
                    })))
                }
                Err(e) => {
                    warn!("Token refresh failed: {}", e);
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Create API key handler
async fn create_api_key_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
    Json(api_key_req): Json<ApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, StatusCode> {
    // Extract user from token
    let auth_extractor = AuthExtractor::extract(headers, auth_manager.clone())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_id = auth_extractor.user_id.clone();
    match auth_manager
        .api_key_manager()
        .generate_key(user_id.clone(), api_key_req.name, api_key_req.permissions)
        .await
    {
        Ok((api_key, raw_key)) => {
            info!("API key created for user: {}", user_id);
            Ok(Json(ApiKeyResponse {
                key: raw_key,
                name: api_key.name,
                expires_at: api_key.expires_at.to_rfc3339(),
            }))
        }
        Err(e) => {
            warn!("Failed to create API key: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List API keys handler
async fn list_api_keys_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth_extractor = AuthExtractor::extract(headers, auth_manager.clone())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let api_keys = auth_manager
        .api_key_manager()
        .get_user_keys(&auth_extractor.user_id)
        .await;

    Ok(Json(serde_json::json!({
        "api_keys": api_keys
    })))
}

/// Create user handler (admin only)
async fn create_user_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
    Json(user_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth_extractor = AuthExtractor::extract(headers, auth_manager.clone())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check if user has admin role
    if !auth_extractor.roles.contains(&"admin".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Extract user data with proper error handling
    let username = user_data["username"]
        .as_str()
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    
    let email = user_data["email"]
        .as_str()
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    
    let password = user_data["password"]
        .as_str()
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    
    let roles = user_data["roles"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| match s {
                    "admin" => UserRole::Admin,
                    "user" => UserRole::User,
                    "readonly" => UserRole::ReadOnly,
                    _ => UserRole::User,
                })
                .collect()
        })
        .unwrap_or_else(|| vec![UserRole::User]);

    match auth_manager
        .user_manager()
        .create_user(username, email, password, roles)
        .await
    {
        Ok(user) => {
            info!("User created by admin: {}", auth_extractor.user_id);
            Ok(Json(serde_json::json!({
                "user": user,
                "message": "User created successfully"
            })))
        }
        Err(e) => {
            warn!("Failed to create user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List users handler (admin only)
async fn list_users_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth_extractor = AuthExtractor::extract(headers, auth_manager.clone())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check if user has admin role
    if !auth_extractor.roles.contains(&"admin".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // In a real implementation, you would get all users from the database
    // For now, return a mock response
    Ok(Json(serde_json::json!({
        "users": [],
        "message": "Users list (mock data)"
    })))
}

/// Assign role handler (admin only)
async fn assign_role_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
    Json(role_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth_extractor = AuthExtractor::extract(headers, auth_manager.clone())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check if user has admin role
    if !auth_extractor.roles.contains(&"admin".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let user_id = role_data["user_id"]
        .as_str()
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    
    let role_id = role_data["role_id"]
        .as_str()
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();

    match auth_manager
        .role_manager()
        .assign_role_to_user(&user_id, &role_id)
        .await
    {
        Ok(_) => {
            info!(
                "Role {} assigned to user {} by admin: {}",
                role_id, user_id, auth_extractor.user_id
            );
            Ok(Json(serde_json::json!({
                "message": "Role assigned successfully"
            })))
        }
        Err(e) => {
            warn!("Failed to assign role: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Protected resource handler
async fn protected_resource_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
) -> Result<Json<ProtectedResourceResponse>, StatusCode> {
    let auth_extractor = AuthExtractor::extract(headers, auth_manager.clone())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(Json(ProtectedResourceResponse {
        message: "Access granted to protected resource".to_string(),
        user_id: auth_extractor.user_id,
        roles: auth_extractor.roles,
        permissions: auth_extractor.permissions,
    }))
}

/// Admin resource handler
async fn admin_resource_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth_extractor = AuthExtractor::extract(headers, auth_manager.clone())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check if user has admin role
    if !auth_extractor.roles.contains(&"admin".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(serde_json::json!({
        "message": "Access granted to admin resource",
        "user_id": auth_extractor.user_id,
        "roles": auth_extractor.roles,
        "permissions": auth_extractor.permissions,
    })))
}

/// Metrics resource handler (requires metrics:read permission)
async fn metrics_resource_handler(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth_extractor = AuthExtractor::extract(headers, auth_manager.clone())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check if user has metrics:read permission
    match auth_manager
        .authorize(&auth_extractor.user_id, "metrics", "read")
        .await
    {
        Ok(true) => Ok(Json(serde_json::json!({
            "message": "Access granted to metrics resource",
            "metrics": {
                "cpu_usage": 45.2,
                "memory_usage": 67.8,
                "active_connections": 1234,
            }
        }))),
        Ok(false) => Err(StatusCode::FORBIDDEN),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Health check handler
async fn health_check_handler(
    State(auth_manager): State<Arc<AuthManager>>,
) -> Json<HealthResponse> {
    let auth_stats = auth_manager.get_stats().await;
    let jwt_stats = auth_manager.jwt_manager().get_stats().await;
    let api_key_stats = auth_manager.api_key_manager().get_stats().await;

    Json(HealthResponse {
        status: "healthy".to_string(),
        auth_stats,
        jwt_stats,
        api_key_stats,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Authentication and Authorization Example");

    // Create authentication configuration
    let mut config = AuthConfig::default();

    // Configure JWT
    config.jwt.secret = "your-super-secret-jwt-key-change-in-production".to_string();
    config.jwt.expiration_secs = 3600; // 1 hour
    config.jwt.enable_refresh_tokens = true;

    // Configure API keys
    config.api_keys.enabled = true;
    config.api_keys.expiration_secs = 30 * 24 * 3600; // 30 days

    // Configure OAuth
    config.oauth.enabled = false; // Disabled for this example

    // Configure users
    config.users.allow_registration = true;
    config.users.min_password_length = 8;

    // Configure RBAC
    config.rbac.enabled = true;
    config.rbac.default_role = "user".to_string();
    config.rbac.admin_role = "admin".to_string();

    // Configure security
    config.security.enable_rate_limiting = true;
    config.security.rate_limit_per_minute = 100;
    config.security.enable_cors = true;

    // Initialize authentication system
    let auth_manager = init_auth_system(config).await.map_err(|e| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        ))
    })?;
    let auth_manager = Arc::new(auth_manager);

    info!("Authentication system initialized");

    // Create some test users
    create_test_users(&auth_manager).await?;

    // Create HTTP server
    let app = Router::new()
        .nest("/auth", create_auth_routes(auth_manager.clone()))
        .route("/", get(|| async { "OpenTelemetry Bridge Auth Example" }));

    info!("Starting HTTP server on http://localhost:8080");
    info!("Available endpoints:");
    info!("  POST /auth/login - Login with username/password");
    info!("  POST /auth/logout - Logout (requires JWT token)");
    info!("  POST /auth/refresh - Refresh JWT token");
    info!("  POST /auth/api-keys - Create API key (requires JWT token)");
    info!("  GET  /auth/api-keys - List API keys (requires JWT token)");
    info!("  POST /auth/users - Create user (admin only)");
    info!("  GET  /auth/users - List users (admin only)");
    info!("  POST /auth/roles - Assign role (admin only)");
    info!("  GET  /auth/protected - Protected resource (requires JWT token)");
    info!("  GET  /auth/admin - Admin resource (requires admin role)");
    info!("  GET  /auth/metrics - Metrics resource (requires metrics:read permission)");
    info!("  GET  /auth/health - Health check");

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, app).await?;

    // Shutdown authentication system
    let auth_manager = Arc::try_unwrap(auth_manager).map_err(|_| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to unwrap Arc",
        ))
    })?;
    shutdown_auth_system(auth_manager).await.map_err(|e| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        ))
    })?;

    info!("Authentication example completed");
    Ok(())
}

/// Create test users
async fn create_test_users(
    auth_manager: &Arc<AuthManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Creating test users...");

    // Create admin user
    let admin_user = auth_manager
        .user_manager()
        .create_user(
            "admin".to_string(),
            "admin@example.com".to_string(),
            "AdminPass123!".to_string(),
            vec![UserRole::Admin],
        )
        .await?;

    // Assign admin role
    auth_manager
        .role_manager()
        .assign_role_to_user(&admin_user.id, "admin")
        .await?;

    // Set admin permissions
    auth_manager
        .permission_manager()
        .set_user_permissions(
            &admin_user.id,
            vec![
                "metrics:read".to_string(),
                "metrics:write".to_string(),
                "traces:read".to_string(),
                "traces:write".to_string(),
                "logs:read".to_string(),
                "logs:write".to_string(),
                "users:read".to_string(),
                "users:write".to_string(),
                "roles:read".to_string(),
                "roles:write".to_string(),
            ],
        )
        .await;

    // Create regular user
    let regular_user = auth_manager
        .user_manager()
        .create_user(
            "user".to_string(),
            "user@example.com".to_string(),
            "UserPass123!".to_string(),
            vec![UserRole::User],
        )
        .await?;

    // Assign user role
    auth_manager
        .role_manager()
        .assign_role_to_user(&regular_user.id, "user")
        .await?;

    // Set user permissions
    auth_manager
        .permission_manager()
        .set_user_permissions(
            &regular_user.id,
            vec![
                "metrics:read".to_string(),
                "traces:read".to_string(),
                "logs:read".to_string(),
            ],
        )
        .await;

    // Create read-only user
    let readonly_user = auth_manager
        .user_manager()
        .create_user(
            "readonly".to_string(),
            "readonly@example.com".to_string(),
            "ReadOnlyPass123!".to_string(),
            vec![UserRole::ReadOnly],
        )
        .await?;

    // Assign read-only role
    auth_manager
        .role_manager()
        .assign_role_to_user(&readonly_user.id, "readonly")
        .await?;

    // Set read-only permissions
    auth_manager
        .permission_manager()
        .set_user_permissions(&readonly_user.id, vec!["metrics:read".to_string()])
        .await;

    info!("Test users created:");
    info!("  Admin: admin@example.com / AdminPass123!");
    info!("  User: user@example.com / UserPass123!");
    info!("  ReadOnly: readonly@example.com / ReadOnlyPass123!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_example_creation() {
        let config = AuthConfig::default();
        let auth_manager = init_auth_system(config).await.unwrap();
        let auth_manager = Arc::new(auth_manager);

        // Test that we can create test users
        let result = create_test_users(&auth_manager).await;
        assert!(result.is_ok());

        // Test shutdown
        let result = shutdown_auth_system(Arc::try_unwrap(auth_manager).unwrap()).await;
        assert!(result.is_ok());
    }
}
