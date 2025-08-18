//! Validation error types

use serde::{Deserialize, Serialize};

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error location (optional)
    pub location: Option<String>,

    /// Additional details (optional)
    pub details: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,

    /// Warning message
    pub message: String,

    /// Warning location (optional)
    pub location: Option<String>,

    /// Additional details (optional)
    pub details: Option<String>,
}
