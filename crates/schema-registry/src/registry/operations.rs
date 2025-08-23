//! Schema operations
//!
//! This module contains the core CRUD operations for schemas.

use crate::error::{SchemaRegistryError, SchemaRegistryResult};
use crate::schema::validation::{SchemaValidator, SchemaValidatorTrait, ValidationResult};
use crate::schema::{Schema, SchemaMetadata, SchemaSearchCriteria, SchemaVersion};
use crate::storage::StorageBackend;
use std::sync::Arc;

/// Schema operations handler
pub struct SchemaOperations {
    storage: Arc<dyn StorageBackend>,
    validator: Arc<SchemaValidator>,
}

impl SchemaOperations {
    /// Create a new schema operations handler
    pub fn new(storage: Arc<dyn StorageBackend>, validator: Arc<SchemaValidator>) -> Self {
        Self { storage, validator }
    }

    /// Register a new schema
    pub async fn register_schema(&self, schema: Schema) -> SchemaRegistryResult<SchemaVersion> {
        // Validate the schema
        let validation_result = self.validator.validate_schema(&schema).await?;

        if !validation_result.is_valid() {
            return Err(SchemaRegistryError::ValidationFailed(
                validation_result
                    .errors
                    .into_iter()
                    .map(|e| format!("{:?}", e))
                    .collect(),
            ));
        }

        // Store the schema
        let version = self.storage.store_schema(schema).await?;

        Ok(version)
    }

    /// Retrieve a schema by fingerprint
    pub async fn get_schema(&self, fingerprint: &str) -> SchemaRegistryResult<Option<Schema>> {
        self.storage.get_schema(fingerprint).await
    }

    /// List all schemas
    pub async fn list_schemas(&self) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        self.storage.list_schemas().await
    }

    /// Search schemas
    pub async fn search_schemas(
        &self,
        criteria: &SchemaSearchCriteria,
    ) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        self.storage.search_schemas(criteria).await
    }

    /// Delete a schema
    pub async fn delete_schema(&self, fingerprint: &str) -> SchemaRegistryResult<bool> {
        self.storage.delete_schema(fingerprint).await
    }

    /// Get schema versions
    pub async fn get_schema_versions(
        &self,
        name: &str,
    ) -> SchemaRegistryResult<Vec<SchemaVersion>> {
        self.storage.get_schema_versions(name).await
    }

    /// Get latest schema version
    pub async fn get_latest_schema(&self, name: &str) -> SchemaRegistryResult<Option<Schema>> {
        self.storage.get_latest_schema(name).await
    }

    /// Validate telemetry data against a schema
    pub async fn validate_data(
        &self,
        fingerprint: &str,
        data: &bridge_core::types::TelemetryBatch,
    ) -> SchemaRegistryResult<ValidationResult> {
        let schema = self
            .get_schema(fingerprint)
            .await?
            .ok_or_else(|| SchemaRegistryError::SchemaNotFound(fingerprint.to_string()))?;

        let validation_result = self.validator.validate_data(&schema, data).await?;

        Ok(validation_result)
    }

    /// Validate a schema
    pub async fn validate_schema(&self, schema: &Schema) -> SchemaRegistryResult<ValidationResult> {
        let validation_result = self.validator.validate_schema(schema).await?;

        Ok(validation_result)
    }

    /// Evolve a schema
    pub async fn evolve_schema(&self, schema: &Schema) -> SchemaRegistryResult<Schema> {
        // Get existing schemas with the same name
        let existing_versions = self.get_schema_versions(&schema.name).await?;

        if existing_versions.is_empty() {
            // First version of this schema, no evolution needed
            return Ok(schema.clone());
        }

        // Get the latest existing schema
        let latest_schema = self
            .get_latest_schema(&schema.name)
            .await?
            .ok_or_else(|| SchemaRegistryError::SchemaNotFound(schema.name.clone()))?;

        // Check compatibility based on the schema's compatibility mode
        let is_compatible =
            crate::registry::compatibility::CompatibilityChecker::check_schema_compatibility(
                &latest_schema,
                schema,
            )
            .await?;

        if !is_compatible {
            return Err(SchemaRegistryError::ValidationFailed(vec![
                format!(
                    "Schema evolution failed: new schema is not compatible with existing schema version {}",
                    latest_schema.version
                ),
            ]));
        }

        // Create evolved schema with incremented version
        let evolved_version =
            crate::registry::compatibility::CompatibilityChecker::increment_schema_version(
                &latest_schema.version,
            );
        let mut evolved_schema = schema.clone();
        evolved_schema.version = evolved_version;
        evolved_schema.updated_at = chrono::Utc::now();
        evolved_schema.fingerprint = Schema::generate_fingerprint(&evolved_schema.content);

        Ok(evolved_schema)
    }
}
