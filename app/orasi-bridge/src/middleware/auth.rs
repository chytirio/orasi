//! Authentication middleware

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use chrono;
use tracing::{debug, warn};
use uuid::Uuid;

use super::utils::RequestContext;
use crate::{config::BridgeAPIConfig, error::ApiError, rest::AppState};

// Add auth crate imports
use bridge_auth::config::OAuthConfigConverter;

/// Authentication middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if !state.config.auth.enabled {
        return next.run(request).await;
    }

    let headers = request.headers().clone();
    match state.config.auth.auth_type {
        crate::config::AuthType::ApiKey => {
            match auth_api_key(&state.config, &headers, request, next).await {
                Ok(response) => response,
                Err(_) => Response::builder()
                    .status(401)
                    .body(axum::body::Body::from("Unauthorized"))
                    .unwrap(),
            }
        }
        crate::config::AuthType::Jwt => {
            match auth_jwt(&state.config, &headers, request, next).await {
                Ok(response) => response,
                Err(_) => Response::builder()
                    .status(401)
                    .body(axum::body::Body::from("Unauthorized"))
                    .unwrap(),
            }
        }
        crate::config::AuthType::OAuth => {
            match auth_oauth(&state.config, &headers, request, next).await {
                Ok(response) => response,
                Err(_) => Response::builder()
                    .status(401)
                    .body(axum::body::Body::from("Unauthorized"))
                    .unwrap(),
            }
        }
        crate::config::AuthType::None => next.run(request).await,
    }
}

/// API key authentication
async fn auth_api_key(
    config: &BridgeAPIConfig,
    headers: &HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let api_key_header = config.auth.api_key.header_name.as_str();

    // Extract API key from headers
    let api_key = headers
        .get(api_key_header)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            ApiError::Unauthorized(format!("Missing API key header: {}", api_key_header))
        })?;

    // Validate API key
    if !config
        .auth
        .api_key
        .valid_keys
        .contains(&api_key.to_string())
    {
        return Err(ApiError::Unauthorized("Invalid API key".to_string()));
    }

    // Validate API key format if regex is provided
    if let Some(ref validation_regex) = config.auth.api_key.validation_regex {
        let regex = regex::Regex::new(validation_regex)
            .map_err(|e| ApiError::Internal(format!("Invalid API key validation regex: {}", e)))?;

        if !regex.is_match(api_key) {
            return Err(ApiError::Unauthorized(
                "API key format is invalid".to_string(),
            ));
        }
    }

    // Add user context to request extensions
    let mut request = request;
    let context = RequestContext {
        request_id: Uuid::new_v4().to_string(),
        start_time: std::time::Instant::now(),
        user_id: Some(format!("api_key:{}", api_key)),
        metadata: std::collections::HashMap::new(),
    };
    request.extensions_mut().insert(context);

    Ok(next.run(request).await)
}

/// JWT authentication
async fn auth_jwt(
    config: &BridgeAPIConfig,
    headers: &HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract JWT token from Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_string()))?;

    // Check if it's a Bearer token
    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::Unauthorized(
            "Invalid Authorization header format".to_string(),
        ));
    }

    let token = &auth_header[7..]; // Remove "Bearer " prefix

    // Decode and validate JWT token
    let token_data = jsonwebtoken::decode::<serde_json::Value>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(config.auth.jwt.secret_key.as_ref()),
        &jsonwebtoken::Validation::default(),
    )
    .map_err(|e| ApiError::Unauthorized(format!("Invalid JWT token: {}", e)))?;

    // Check expiration
    if let Some(exp) = token_data.claims.get("exp") {
        if let Some(exp_timestamp) = exp.as_u64() {
            let current_timestamp = chrono::Utc::now().timestamp() as u64;
            if current_timestamp > exp_timestamp {
                return Err(ApiError::Unauthorized("JWT token has expired".to_string()));
            }
        }
    }

    // Check issuer if configured
    if let Some(ref issuer) = config.auth.jwt.issuer {
        if let Some(token_issuer) = token_data.claims.get("iss") {
            if token_issuer.as_str() != Some(issuer) {
                return Err(ApiError::Unauthorized("Invalid JWT issuer".to_string()));
            }
        }
    }

    // Check audience if configured
    if let Some(ref audience) = config.auth.jwt.audience {
        if let Some(token_audience) = token_data.claims.get("aud") {
            if token_audience.as_str() != Some(audience) {
                return Err(ApiError::Unauthorized("Invalid JWT audience".to_string()));
            }
        }
    }

    // Extract user ID from token
    let user_id = token_data
        .claims
        .get("sub")
        .and_then(|sub| sub.as_str())
        .unwrap_or("unknown");

    // Add user context to request extensions
    let mut request = request;
    let context = RequestContext {
        request_id: Uuid::new_v4().to_string(),
        start_time: std::time::Instant::now(),
        user_id: Some(format!("jwt:{}", user_id)),
        metadata: std::collections::HashMap::new(),
    };
    request.extensions_mut().insert(context);

    Ok(next.run(request).await)
}

/// OAuth authentication
async fn auth_oauth(
    config: &BridgeAPIConfig,
    headers: &HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract OAuth token from headers
    if let Some(auth_header) = headers.get("authorization") {
        let auth_value = auth_header.to_str().unwrap_or("");

        if auth_value.starts_with("Bearer ") {
            let token = &auth_value[7..]; // Remove "Bearer " prefix

            // Validate OAuth token
            match validate_oauth_token(token, config).await {
                Ok(user_info) => {
                    debug!(
                        "OAuth request authenticated for user: {}",
                        user_info.user_id
                    );

                    // Add user context to request extensions
                    let mut request = request;
                    let context = RequestContext {
                        request_id: Uuid::new_v4().to_string(),
                        start_time: std::time::Instant::now(),
                        user_id: Some(format!("oauth:{}", user_info.user_id)),
                        metadata: {
                            let mut metadata = std::collections::HashMap::new();
                            if let Some(email) = &user_info.email {
                                metadata.insert("email".to_string(), email.clone());
                            }
                            if let Some(name) = &user_info.name {
                                metadata.insert("name".to_string(), name.clone());
                            }
                            metadata.insert("roles".to_string(), user_info.roles.join(","));
                            metadata
                        },
                    };
                    request.extensions_mut().insert(context);

                    Ok(next.run(request).await)
                }
                Err(e) => {
                    warn!("OAuth validation failed: {}", e);
                    Err(ApiError::Unauthorized(format!(
                        "Invalid OAuth token: {}",
                        e
                    )))
                }
            }
        } else {
            Err(ApiError::Unauthorized(
                "Invalid authorization header format".to_string(),
            ))
        }
    } else {
        // Check if OAuth authentication is required
        if !config.auth.oauth.client_id.is_empty() {
            Err(ApiError::Unauthorized(
                "OAuth authentication required".to_string(),
            ))
        } else {
            debug!("Request without OAuth authentication header (auth not required)");
            Ok(next.run(request).await)
        }
    }
}

/// Validate OAuth token
async fn validate_oauth_token(
    token: &str,
    config: &BridgeAPIConfig,
) -> Result<OAuthUserInfo, String> {
    // Create OAuth validator from bridge-api config
    let oauth_config = bridge_auth::config::OAuthConfig::from_bridge_api_format(
        &bridge_auth::config::BridgeApiOAuthConfig {
            client_id: config.auth.oauth.client_id.clone(),
            client_secret: config.auth.oauth.client_secret.clone(),
            authorization_url: config.auth.oauth.authorization_url.clone(),
            token_url: config.auth.oauth.token_url.clone(),
            user_info_url: config.auth.oauth.user_info_url.clone(),
        },
    );

    // Validate OAuth configuration
    if !oauth_config.enabled {
        return Err("OAuth is not enabled".to_string());
    }

    if oauth_config.providers.is_empty() {
        return Err("No OAuth providers configured".to_string());
    }

    // Get the first provider (or default provider)
    let provider = oauth_config
        .providers
        .get("default")
        .or_else(|| oauth_config.providers.values().next())
        .ok_or("No OAuth provider available")?;

    // Validate the token by making a request to the user info endpoint
    let user_info = validate_token_with_provider(token, provider).await?;

    Ok(user_info)
}

/// Validate OAuth token with a specific provider
async fn validate_token_with_provider(
    token: &str,
    provider: &bridge_auth::config::OAuthProviderConfig,
) -> Result<OAuthUserInfo, String> {
    // Create HTTP client
    let client = reqwest::Client::new();

    // Make request to user info endpoint
    let response = client
        .get(&provider.user_info_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "Orasi-Bridge-API/1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to validate token: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Token validation failed with status: {}",
            response.status()
        ));
    }

    // Parse user info from response
    let user_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse user info: {}", e))?;

    // Extract user information from the response
    let user_id = user_data
        .get("sub")
        .or_else(|| user_data.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let email = user_data
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let name = user_data
        .get("name")
        .or_else(|| user_data.get("display_name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract roles if available
    let roles = user_data
        .get("roles")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_else(|| vec!["user".to_string()]);

    Ok(OAuthUserInfo {
        user_id,
        email,
        name,
        roles,
    })
}

/// OAuth user information
#[derive(Debug, Clone)]
struct OAuthUserInfo {
    user_id: String,
    email: Option<String>,
    name: Option<String>,
    roles: Vec<String>,
}
