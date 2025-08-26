//! Security configuration types

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::rate_limiting::RateLimitingConfig;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub enable_auth: bool,

    /// Authentication type
    pub auth_type: AuthType,

    /// TLS configuration
    pub tls: Option<TlsConfig>,

    /// Rate limiting configuration
    pub rate_limiting: Option<RateLimitingConfig>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_auth: false,
            auth_type: AuthType::None,
            tls: None,
            rate_limiting: None,
        }
    }
}

impl SecurityConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if let Some(tls_config) = &self.tls {
            tls_config.validate()?;
        }

        if let Some(rate_limiting_config) = &self.rate_limiting {
            rate_limiting_config.validate()?;
        }

        Ok(())
    }
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    None,
    ApiKey,
    Jwt,
    OAuth,
    Basic,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: AuthType,

    /// Authentication credentials
    pub credentials: HashMap<String, String>,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Certificate file path
    pub cert_file: String,

    /// Private key file path
    pub key_file: String,

    /// CA certificate file path
    pub ca_file: Option<String>,
}

impl TlsConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.cert_file.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "TLS certificate file path cannot be empty".to_string(),
            ));
        }

        if self.key_file.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "TLS private key file path cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}
