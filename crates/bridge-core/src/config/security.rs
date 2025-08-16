//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Security configuration for the OpenTelemetry Data Lake Bridge
//!
//! This module provides configuration structures for security features.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::Validate;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SecurityConfig {
    /// Enable TLS encryption
    pub enable_tls: bool,

    /// TLS certificate path
    pub tls_cert_path: Option<PathBuf>,

    /// TLS private key path
    pub tls_key_path: Option<PathBuf>,

    /// Enable authentication
    pub enable_authentication: bool,

    /// Authentication methods
    pub authentication_methods: Vec<AuthenticationMethod>,

    /// Enable authorization
    pub enable_authorization: bool,

    /// Authorization policies
    pub authorization_policies: Option<Vec<AuthorizationPolicy>>,

    /// Enable audit logging
    pub enable_audit_logging: bool,

    /// Audit log path
    pub audit_log_path: Option<PathBuf>,

    /// Enable PII scrubbing
    pub enable_pii_scrubbing: bool,

    /// PII scrubbing rules
    pub pii_scrubbing_rules: Option<Vec<PiiScrubbingRule>>,

    /// Data retention policy in days
    #[validate(range(min = 1, max = 3650))]
    pub data_retention_days: u32,

    /// Enable data encryption at rest
    pub enable_encryption_at_rest: bool,

    /// Encryption key configuration
    pub encryption_key: Option<EncryptionKeyConfig>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            enable_authentication: false,
            authentication_methods: vec![AuthenticationMethod::None],
            enable_authorization: false,
            authorization_policies: None,
            enable_audit_logging: false,
            audit_log_path: None,
            enable_pii_scrubbing: false,
            pii_scrubbing_rules: None,
            data_retention_days: 90,
            enable_encryption_at_rest: false,
            encryption_key: None,
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AuthenticationConfig {
    /// Authentication method
    pub method: AuthenticationMethod,

    /// API key (for API key authentication)
    pub api_key: Option<String>,

    /// Username (for basic authentication)
    pub username: Option<String>,

    /// Password (for basic authentication)
    pub password: Option<String>,

    /// OAuth configuration
    pub oauth: Option<OAuthConfig>,

    /// Certificate path (for certificate authentication)
    pub certificate_path: Option<PathBuf>,

    /// Private key path (for certificate authentication)
    pub private_key_path: Option<PathBuf>,
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    None,
    ApiKey,
    Basic,
    OAuth,
    Certificate,
    JWT,
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct OAuthConfig {
    /// OAuth client ID
    pub client_id: String,

    /// OAuth client secret
    pub client_secret: String,

    /// OAuth token endpoint
    #[validate(url)]
    pub token_endpoint: String,

    /// OAuth scopes
    pub scopes: Vec<String>,

    /// OAuth redirect URI
    pub redirect_uri: Option<String>,
}

/// Authorization policy
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AuthorizationPolicy {
    /// Policy name
    pub name: String,

    /// Policy description
    pub description: Option<String>,

    /// Allowed roles
    pub allowed_roles: Vec<String>,

    /// Allowed actions
    pub allowed_actions: Vec<String>,

    /// Allowed resources
    pub allowed_resources: Vec<String>,

    /// Policy conditions
    pub conditions: Option<HashMap<String, serde_json::Value>>,
}

/// PII scrubbing rule
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PiiScrubbingRule {
    /// Rule name
    pub name: String,

    /// Field pattern to match
    pub field_pattern: String,

    /// Scrubbing method
    pub scrubbing_method: PiiScrubbingMethod,

    /// Replacement value
    pub replacement: Option<String>,
}

/// PII scrubbing methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PiiScrubbingMethod {
    Hash,
    Mask,
    Redact,
    Replace,
    Anonymize,
}

/// Encryption key configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct EncryptionKeyConfig {
    /// Key ID
    pub key_id: String,

    /// Key material (base64 encoded)
    pub key_material: String,

    /// Key algorithm
    pub algorithm: String,

    /// Key rotation interval in days
    pub rotation_interval_days: Option<u32>,
}
