//! Validation result types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::error::{ValidationError, ValidationWarning};

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Validation status
    pub status: ValidationStatus,

    /// Validation errors
    pub errors: Vec<ValidationError>,

    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ValidationResult {
    /// Check if validation is successful
    pub fn is_valid(&self) -> bool {
        matches!(
            self.status,
            ValidationStatus::Valid | ValidationStatus::Warning
        )
    }

    /// Check if validation has errors
    pub fn has_errors(&self) -> bool {
        matches!(self.status, ValidationStatus::Invalid)
    }

    /// Check if validation has warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

/// Validation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Validation passed
    Valid,

    /// Validation passed with warnings
    Warning,

    /// Validation failed
    Invalid,
}
