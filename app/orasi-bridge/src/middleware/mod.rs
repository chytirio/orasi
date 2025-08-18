//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!
//! Middleware for Bridge API

pub mod auth;
pub mod health;
pub mod logging;
pub mod rate_limit;
pub mod security;
pub mod utils;

// Re-export main middleware functions for convenience
pub use auth::auth_middleware;
pub use health::health_check_middleware;
pub use logging::{logging_middleware, metrics_middleware};
pub use rate_limit::rate_limit_middleware;
pub use security::{
    cors_middleware, security_headers_middleware,
};
pub use utils::{
    error_handling_middleware, request_id_middleware, timeout_middleware, size_limit_middleware,
    compression_middleware, keep_alive_middleware, RequestContext,
};
