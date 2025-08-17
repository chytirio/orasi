//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Authentication middleware for HTTP requests

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use axum::extract::Request;
use std::sync::Arc;

use tracing::{debug, warn};

use crate::auth::{AuthCredentials, AuthManager, AuthMethod, AuthRequest};

/// Authentication middleware
pub struct AuthMiddleware;

impl AuthMiddleware {
    /// Create new authentication middleware
    pub fn new(_auth_manager: Arc<AuthManager>) -> Self {
        Self
    }

    /// Extract authentication from request
    pub async fn extract_auth(&self, headers: &HeaderMap) -> Option<AuthRequest> {
        // Try JWT token from Authorization header
        if let Some(auth_header) = headers.get("Authorization") {
            if let Ok(auth_value) = auth_header.to_str() {
                if auth_value.starts_with("Bearer ") {
                    let token = auth_value[7..].to_string();
                    return Some(AuthRequest {
                        method: AuthMethod::Jwt,
                        credentials: AuthCredentials::JwtToken { token },
                        metadata: std::collections::HashMap::new(),
                    });
                }
            }
        }

        // Try API key from X-API-Key header
        if let Some(api_key_header) = headers.get("X-API-Key") {
            if let Ok(api_key) = api_key_header.to_str() {
                return Some(AuthRequest {
                    method: AuthMethod::ApiKey,
                    credentials: AuthCredentials::ApiKey {
                        key: api_key.to_string(),
                    },
                    metadata: std::collections::HashMap::new(),
                });
            }
        }

        None
    }
}

/// Authentication extractor for Axum
pub struct AuthExtractor {
    /// User ID
    pub user_id: String,

    /// User roles
    pub roles: Vec<String>,

    /// User permissions
    pub permissions: Vec<String>,
}

impl AuthExtractor {
    /// Extract authentication from request
    pub async fn extract(
        headers: HeaderMap,
        auth_manager: Arc<AuthManager>,
    ) -> Result<Self, StatusCode> {
        let middleware = AuthMiddleware::new(auth_manager.clone());

        if let Some(auth_request) = middleware.extract_auth(&headers).await {
            match auth_manager.authenticate(auth_request).await {
                Ok(auth_response) => {
                    if let crate::auth::AuthStatus::Authenticated {
                        user_id,
                        roles,
                        permissions,
                    } = auth_response.status
                    {
                        Ok(Self {
                            user_id,
                            roles,
                            permissions,
                        })
                    } else {
                        Err(StatusCode::UNAUTHORIZED)
                    }
                }
                Err(_) => Err(StatusCode::UNAUTHORIZED),
            }
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Require authentication middleware
pub async fn require_auth(
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let middleware = AuthMiddleware::new(auth_manager.clone());

    if let Some(auth_request) = middleware.extract_auth(&headers).await {
        match auth_manager.authenticate(auth_request).await {
            Ok(_) => {
                debug!("Request authenticated successfully");
                Ok(next.run(request).await)
            }
            Err(_) => {
                warn!("Authentication failed");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    } else {
        warn!("No authentication provided");
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Require specific role middleware
pub async fn require_role(
    required_role: String,
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let middleware = AuthMiddleware::new(auth_manager.clone());

    if let Some(auth_request) = middleware.extract_auth(&headers).await {
        match auth_manager.authenticate(auth_request).await {
            Ok(auth_response) => {
                if let crate::auth::AuthStatus::Authenticated { roles, .. } = auth_response.status {
                    if roles.contains(&required_role) {
                        debug!("User has required role: {}", required_role);
                        Ok(next.run(request).await)
                    } else {
                        warn!("User does not have required role: {}", required_role);
                        Err(StatusCode::FORBIDDEN)
                    }
                } else {
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
            Err(_) => Err(StatusCode::UNAUTHORIZED),
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Require specific permission middleware
pub async fn require_permission(
    resource: String,
    action: String,
    State(auth_manager): State<Arc<AuthManager>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let middleware = AuthMiddleware::new(auth_manager.clone());

    if let Some(auth_request) = middleware.extract_auth(&headers).await {
        match auth_manager.authenticate(auth_request).await {
            Ok(auth_response) => {
                if let crate::auth::AuthStatus::Authenticated { user_id, .. } = auth_response.status
                {
                    match auth_manager.authorize(&user_id, &resource, &action).await {
                        Ok(true) => {
                            debug!("User authorized for {}:{}", resource, action);
                            Ok(next.run(request).await)
                        }
                        Ok(false) => {
                            warn!("User not authorized for {}:{}", resource, action);
                            Err(StatusCode::FORBIDDEN)
                        }
                        Err(_) => Err(StatusCode::FORBIDDEN),
                    }
                } else {
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
            Err(_) => Err(StatusCode::UNAUTHORIZED),
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthConfig;

    #[tokio::test]
    async fn test_auth_middleware_creation() {
        let config = AuthConfig::default();
        let auth_manager = AuthManager::new(config).await.unwrap();
        let _middleware = AuthMiddleware::new(Arc::new(auth_manager));
        assert!(true); // Just test that it can be created
    }
}
