//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Security configuration

use serde::{Deserialize, Serialize};

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether to enable rate limiting
    pub enable_rate_limiting: bool,

    /// Rate limit requests per minute
    pub rate_limit_per_minute: usize,

    /// Whether to enable CORS
    pub enable_cors: bool,

    /// CORS allowed origins
    pub cors_allowed_origins: Vec<String>,

    /// Whether to enable HTTPS only
    pub https_only: bool,

    /// Security headers
    pub security_headers: SecurityHeaders,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_rate_limiting: true,
            rate_limit_per_minute: 100,
            enable_cors: true,
            cors_allowed_origins: vec!["http://localhost:3000".to_string()],
            https_only: false,
            security_headers: SecurityHeaders::default(),
        }
    }
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeaders {
    /// Whether to enable HSTS
    pub enable_hsts: bool,

    /// Whether to enable XSS protection
    pub enable_xss_protection: bool,

    /// Whether to enable content type sniffing protection
    pub enable_content_type_sniffing_protection: bool,

    /// Whether to enable frame options
    pub enable_frame_options: bool,
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            enable_xss_protection: true,
            enable_content_type_sniffing_protection: true,
            enable_frame_options: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.enable_rate_limiting);
        assert_eq!(config.rate_limit_per_minute, 100);
        assert!(config.enable_cors);
        assert!(!config.https_only);
    }

    #[test]
    fn test_security_headers_default() {
        let headers = SecurityHeaders::default();
        assert!(headers.enable_hsts);
        assert!(headers.enable_xss_protection);
        assert!(headers.enable_content_type_sniffing_protection);
        assert!(headers.enable_frame_options);
    }
}
