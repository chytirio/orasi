//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling for multi-tenancy operations

use thiserror::Error;

/// Multi-tenancy result type
pub type MultiTenancyResult<T> = Result<T, MultiTenancyError>;

/// Multi-tenancy error types
#[derive(Error, Debug)]
pub enum MultiTenancyError {
    #[error("Tenant not found: {tenant_id}")]
    TenantNotFound { tenant_id: String },

    #[error("Tenant already exists: {tenant_id}")]
    TenantAlreadyExists { tenant_id: String },

    #[error("Invalid tenant ID: {tenant_id}")]
    InvalidTenantId { tenant_id: String },

    #[error("Tenant is inactive: {tenant_id}")]
    TenantInactive { tenant_id: String },

    #[error("Tenant is suspended: {tenant_id}")]
    TenantSuspended { tenant_id: String },

    #[error("Resource quota exceeded: {resource} for tenant {tenant_id}")]
    QuotaExceeded { tenant_id: String, resource: String },

    #[error("Isolation violation: {message}")]
    IsolationViolation { message: String },

    #[error("Cross-tenant access denied: {message}")]
    CrossTenantAccessDenied { message: String },

    #[error("Storage error: {message}")]
    StorageError { message: String },

    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Authentication error: {message}")]
    AuthenticationError { message: String },

    #[error("Authorization error: {message}")]
    AuthorizationError { message: String },

    #[error("Billing error: {message}")]
    BillingError { message: String },

    #[error("Usage tracking error: {message}")]
    UsageTrackingError { message: String },

    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Redis error: {message}")]
    RedisError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl MultiTenancyError {
    /// Create a tenant not found error
    pub fn tenant_not_found(tenant_id: impl Into<String>) -> Self {
        Self::TenantNotFound {
            tenant_id: tenant_id.into(),
        }
    }

    /// Create a tenant already exists error
    pub fn tenant_already_exists(tenant_id: impl Into<String>) -> Self {
        Self::TenantAlreadyExists {
            tenant_id: tenant_id.into(),
        }
    }

    /// Create an invalid tenant ID error
    pub fn invalid_tenant_id(tenant_id: impl Into<String>) -> Self {
        Self::InvalidTenantId {
            tenant_id: tenant_id.into(),
        }
    }

    /// Create a tenant inactive error
    pub fn tenant_inactive(tenant_id: impl Into<String>) -> Self {
        Self::TenantInactive {
            tenant_id: tenant_id.into(),
        }
    }

    /// Create a tenant suspended error
    pub fn tenant_suspended(tenant_id: impl Into<String>) -> Self {
        Self::TenantSuspended {
            tenant_id: tenant_id.into(),
        }
    }

    /// Create a quota exceeded error
    pub fn quota_exceeded(tenant_id: impl Into<String>, resource: impl Into<String>) -> Self {
        Self::QuotaExceeded {
            tenant_id: tenant_id.into(),
            resource: resource.into(),
        }
    }

    /// Create an isolation violation error
    pub fn isolation_violation(message: impl Into<String>) -> Self {
        Self::IsolationViolation {
            message: message.into(),
        }
    }

    /// Create a cross-tenant access denied error
    pub fn cross_tenant_access_denied(message: impl Into<String>) -> Self {
        Self::CrossTenantAccessDenied {
            message: message.into(),
        }
    }

    /// Create a storage error
    pub fn storage_error(message: impl Into<String>) -> Self {
        Self::StorageError {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Create an authentication error
    pub fn authentication_error(message: impl Into<String>) -> Self {
        Self::AuthenticationError {
            message: message.into(),
        }
    }

    /// Create an authorization error
    pub fn authorization_error(message: impl Into<String>) -> Self {
        Self::AuthorizationError {
            message: message.into(),
        }
    }

    /// Create a billing error
    pub fn billing_error(message: impl Into<String>) -> Self {
        Self::BillingError {
            message: message.into(),
        }
    }

    /// Create a usage tracking error
    pub fn usage_tracking_error(message: impl Into<String>) -> Self {
        Self::UsageTrackingError {
            message: message.into(),
        }
    }

    /// Create a database error
    pub fn database_error(message: impl Into<String>) -> Self {
        Self::DatabaseError {
            message: message.into(),
        }
    }

    /// Create a Redis error
    pub fn redis_error(message: impl Into<String>) -> Self {
        Self::RedisError {
            message: message.into(),
        }
    }

    /// Create a serialization error
    pub fn serialization_error(message: impl Into<String>) -> Self {
        Self::SerializationError {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}
