//! Rate limiting middleware

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::time::{SystemTime, UNIX_EPOCH};

use super::utils::RequestContext;
use crate::{config::BridgeAPIConfig, rest::AppState};

/// Rate limiting state for a client
#[derive(Debug, Clone)]
struct RateLimitState {
    /// Last request timestamp
    last_request: u64,
    /// Current token count
    tokens: u32,
    /// Maximum tokens
    max_tokens: u32,
    /// Token refill rate (tokens per second)
    refill_rate: f64,
}

impl RateLimitState {
    fn new(max_tokens: u32, refill_rate: f64) -> Self {
        Self {
            last_request: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tokens: max_tokens,
            max_tokens,
            refill_rate,
        }
    }

    fn try_consume_token(&mut self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Refill tokens based on time elapsed
        let time_elapsed = now - self.last_request;
        let tokens_to_add = (time_elapsed as f64 * self.refill_rate) as u32;
        self.tokens = (self.tokens + tokens_to_add).min(self.max_tokens);

        // Try to consume a token
        if self.tokens > 0 {
            self.tokens -= 1;
            self.last_request = now;
            true
        } else {
            false
        }
    }
}

// Global rate limiting state
static RATE_LIMIT_STORE: Lazy<DashMap<String, RateLimitState>> = Lazy::new(DashMap::new);

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if !state.config.rate_limit.enabled {
        return next.run(request).await;
    }

    // Get client identifier (IP address or user ID)
    let client_id = get_client_identifier(&request);

    // Get or create rate limit state for this client
    let mut rate_limit_state = RATE_LIMIT_STORE
        .entry(client_id.clone())
        .or_insert_with(|| {
            RateLimitState::new(
                state.config.rate_limit.burst_size,
                state.config.rate_limit.requests_per_second as f64,
            )
        })
        .clone();

    // Try to consume a token
    if !rate_limit_state.try_consume_token() {
        // Rate limit exceeded
        return Response::builder()
            .status(429)
            .header(
                "X-RateLimit-Limit",
                state.config.rate_limit.burst_size.to_string(),
            )
            .header("X-RateLimit-Remaining", "0")
            .header(
                "X-RateLimit-Reset",
                (SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 1)
                .to_string(),
            )
            .body(axum::body::Body::from("Rate limit exceeded"))
            .unwrap();
    }

    // Get remaining tokens before updating
    let remaining_tokens = rate_limit_state.tokens - 1;

    // Update the rate limit state
    RATE_LIMIT_STORE.insert(client_id, rate_limit_state);

    // Add rate limit headers to response
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "X-RateLimit-Limit",
        state
            .config
            .rate_limit
            .burst_size
            .to_string()
            .parse()
            .unwrap(),
    );
    headers.insert(
        "X-RateLimit-Remaining",
        remaining_tokens.to_string().parse().unwrap(),
    );

    response
}

/// Get client identifier for rate limiting
fn get_client_identifier(request: &Request) -> String {
    // Try to get user ID from request context first
    if let Some(context) = request.extensions().get::<RequestContext>() {
        if let Some(user_id) = &context.user_id {
            return format!("user:{}", user_id);
        }
    }

    // Fall back to IP address
    request
        .extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| format!("ip:{}", addr.ip()))
        .unwrap_or_else(|| "unknown".to_string())
}
