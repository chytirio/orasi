//! Security middleware (CORS, security headers)

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use tower_http::cors::{Any, CorsLayer};
use tracing;

use crate::{config::BridgeAPIConfig, rest::AppState};

/// CORS middleware
pub fn cors_middleware(config: &BridgeAPIConfig) -> CorsLayer {
    if !config.cors.enabled {
        return CorsLayer::new();
    }

    let mut cors = CorsLayer::new()
        .allow_methods(
            config
                .cors
                .allowed_methods
                .iter()
                .map(|m| m.parse().unwrap())
                .collect::<Vec<_>>(),
        )
        .allow_headers(
            config
                .cors
                .allowed_headers
                .iter()
                .map(|h| h.parse().unwrap())
                .collect::<Vec<_>>(),
        )
        .max_age(config.cors.max_age);

    // Set allowed origins and credentials
    if config.cors.allowed_origins.contains(&"*".to_string()) {
        // Cannot use wildcard origin with credentials
        if config.cors.allow_credentials {
            tracing::warn!(
                "CORS: Cannot use wildcard origin (*) with credentials. Disabling credentials."
            );
            cors = cors.allow_origin(Any);
        } else {
            cors = cors.allow_origin(Any);
        }
    } else {
        cors = cors.allow_origin(
            config
                .cors
                .allowed_origins
                .iter()
                .map(|origin| origin.parse().unwrap())
                .collect::<Vec<_>>(),
        );

        // Set credentials only if not using wildcard origin
        if config.cors.allow_credentials {
            cors = cors.allow_credentials(true);
        }
    }

    cors
}

/// Security headers middleware
pub async fn security_headers_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    if state.config.security.enable_security_headers {
        let headers = response.headers_mut();

        // Content Security Policy
        if let Some(csp) = &state.config.security.content_security_policy {
            headers.insert("Content-Security-Policy", csp.parse().unwrap());
        }

        // Strict Transport Security
        if let Some(hsts) = &state.config.security.strict_transport_security {
            headers.insert("Strict-Transport-Security", hsts.parse().unwrap());
        }

        // X-Frame-Options
        if let Some(xfo) = &state.config.security.x_frame_options {
            headers.insert("X-Frame-Options", xfo.parse().unwrap());
        }

        // X-Content-Type-Options
        if let Some(xcto) = &state.config.security.x_content_type_options {
            headers.insert("X-Content-Type-Options", xcto.parse().unwrap());
        }

        // Additional security headers
        headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
        headers.insert(
            "Referrer-Policy",
            "strict-origin-when-cross-origin".parse().unwrap(),
        );
    }

    response
}
