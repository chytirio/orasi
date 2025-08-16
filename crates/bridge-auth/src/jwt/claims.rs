//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! JWT claims definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::config::JwtConfig;

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: String,

    /// Issued at
    pub iat: i64,

    /// Expiration time
    pub exp: i64,

    /// Not before
    pub nbf: Option<i64>,

    /// JWT ID
    pub jti: String,

    /// User roles
    pub roles: Vec<String>,

    /// Custom claims
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl JwtClaims {
    /// Create new JWT claims
    pub fn new(
        user_id: String,
        roles: Vec<String>,
        config: &JwtConfig,
        custom_claims: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        let now = Utc::now();
        let exp = now.timestamp() + config.expiration_secs as i64;

        Self {
            sub: user_id,
            iss: config.issuer.clone(),
            aud: config.audience.clone(),
            iat: now.timestamp(),
            exp,
            nbf: Some(now.timestamp()),
            jti: Uuid::new_v4().to_string(),
            roles,
            custom: custom_claims.unwrap_or_default(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp < now
    }

    /// Check if token is not yet valid
    pub fn is_not_yet_valid(&self) -> bool {
        if let Some(nbf) = self.nbf {
            let now = Utc::now().timestamp();
            nbf > now
        } else {
            false
        }
    }

    /// Get user ID
    pub fn user_id(&self) -> &str {
        &self.sub
    }

    /// Get roles
    pub fn roles(&self) -> &[String] {
        &self.roles
    }

    /// Check if user has role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[String]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Get custom claim
    pub fn get_custom_claim(&self, key: &str) -> Option<&serde_json::Value> {
        self.custom.get(key)
    }

    /// Get expiration time as DateTime
    pub fn expiration_time(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.exp, 0).unwrap_or_else(|| Utc::now())
    }

    /// Get issued at time as DateTime
    pub fn issued_at_time(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.iat, 0).unwrap_or_else(|| Utc::now())
    }

    /// Get not before time as DateTime (if set)
    pub fn not_before_time(&self) -> Option<DateTime<Utc>> {
        self.nbf.and_then(|nbf| DateTime::from_timestamp(nbf, 0))
    }

    /// Get time until expiration
    pub fn time_until_expiration(&self) -> chrono::Duration {
        let now = Utc::now();
        let exp = self.expiration_time();
        exp - now
    }

    /// Check if token will expire soon (within the given duration)
    pub fn expires_soon(&self, duration: chrono::Duration) -> bool {
        self.time_until_expiration() <= duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::JwtConfig;

    #[test]
    fn test_jwt_claims_creation() {
        let config = JwtConfig::default();
        let user_id = "test-user".to_string();
        let roles = vec!["user".to_string(), "admin".to_string()];

        let claims = JwtClaims::new(user_id.clone(), roles.clone(), &config, None);
        
        assert_eq!(claims.user_id(), user_id);
        assert_eq!(claims.roles(), roles.as_slice());
        assert!(!claims.is_expired());
        assert!(!claims.is_not_yet_valid());
    }

    #[test]
    fn test_jwt_claims_role_checking() {
        let config = JwtConfig::default();
        let user_id = "test-user".to_string();
        let roles = vec!["user".to_string(), "admin".to_string()];

        let claims = JwtClaims::new(user_id, roles, &config, None);
        
        assert!(claims.has_role("user"));
        assert!(claims.has_role("admin"));
        assert!(!claims.has_role("nonexistent"));

        assert!(claims.has_any_role(&["user".to_string(), "moderator".to_string()]));
        assert!(claims.has_any_role(&["admin".to_string(), "moderator".to_string()]));
        assert!(!claims.has_any_role(&["moderator".to_string(), "guest".to_string()]));
    }

    #[test]
    fn test_jwt_claims_custom_claims() {
        let config = JwtConfig::default();
        let user_id = "test-user".to_string();
        let roles = vec!["user".to_string()];
        let mut custom_claims = HashMap::new();
        custom_claims.insert("premium".to_string(), serde_json::json!(true));
        custom_claims.insert("subscription".to_string(), serde_json::json!("pro"));

        let claims = JwtClaims::new(user_id, roles, &config, Some(custom_claims));
        
        assert_eq!(claims.get_custom_claim("premium"), Some(&serde_json::json!(true)));
        assert_eq!(claims.get_custom_claim("subscription"), Some(&serde_json::json!("pro")));
        assert_eq!(claims.get_custom_claim("nonexistent"), None);
    }
}
