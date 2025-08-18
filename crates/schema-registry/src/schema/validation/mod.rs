//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Schema validation for the Schema Registry
//!
//! This module provides validation functionality for schemas and telemetry data.

pub mod error;
pub mod result;
pub mod rules;
pub mod validator;

#[cfg(test)]
mod tests;

use crate::error::SchemaRegistryResult;
use crate::schema::Schema;
use async_trait::async_trait;
use bridge_core::types::TelemetryBatch;

// Re-export main types for convenience
pub use error::{ValidationError, ValidationWarning};
pub use result::{ValidationResult, ValidationStatus};
pub use rules::{ValidationRule, ValidationRuleType};
pub use validator::{SchemaValidator, is_valid_json, is_valid_yaml};

/// Schema validator trait
#[async_trait]
pub trait SchemaValidatorTrait: Send + Sync {
    /// Validate a schema
    async fn validate_schema(&self, schema: &Schema) -> SchemaRegistryResult<ValidationResult>;

    /// Validate telemetry data against a schema
    async fn validate_data(
        &self,
        schema: &Schema,
        data: &TelemetryBatch,
    ) -> SchemaRegistryResult<ValidationResult>;
}
