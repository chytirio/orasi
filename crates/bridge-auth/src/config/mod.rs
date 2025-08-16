//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Authentication configuration module

pub mod api_keys;
pub mod auth;
pub mod jwt;
pub mod oauth;
pub mod rbac;
pub mod security;
pub mod session;
pub mod user;

// Re-export commonly used types
pub use api_keys::ApiKeyConfig;
pub use auth::AuthConfig;
pub use jwt::{JwtAlgorithm, JwtConfig};
pub use oauth::{OAuthConfig, OAuthProviderConfig, BridgeApiOAuthConfig, OAuthConfigConverter};
pub use rbac::RbacConfig;
pub use security::{SecurityConfig, SecurityHeaders};
pub use session::SessionConfig;
pub use user::{UserConfig, PasswordComplexity};
