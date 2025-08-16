//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! JWT configuration

use serde::{Deserialize, Serialize};

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret key
    pub secret: String,

    /// JWT expiration time in seconds
    pub expiration_secs: u64,

    /// JWT issuer
    pub issuer: String,

    /// JWT audience
    pub audience: String,

    /// Algorithm to use for JWT signing
    pub algorithm: JwtAlgorithm,

    /// Whether to enable JWT refresh tokens
    pub enable_refresh_tokens: bool,

    /// Refresh token expiration time in seconds
    pub refresh_token_expiration_secs: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "your-secret-key-change-in-production".to_string(),
            expiration_secs: crate::DEFAULT_JWT_EXPIRATION_SECS,
            issuer: "otel-bridge".to_string(),
            audience: "otel-bridge-users".to_string(),
            algorithm: JwtAlgorithm::HS256,
            enable_refresh_tokens: true,
            refresh_token_expiration_secs: 7 * 24 * 60 * 60, // 7 days
        }
    }
}

/// JWT algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JwtAlgorithm {
    HS256,
    HS384,
    HS512,
    RS256,
    RS384,
    RS512,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_config_default() {
        let config = JwtConfig::default();
        assert!(!config.secret.is_empty());
        assert_eq!(config.expiration_secs, crate::DEFAULT_JWT_EXPIRATION_SECS);
        assert!(matches!(config.algorithm, JwtAlgorithm::HS256));
        assert_eq!(config.issuer, "otel-bridge");
        assert_eq!(config.audience, "otel-bridge-users");
        assert!(config.enable_refresh_tokens);
    }
}
