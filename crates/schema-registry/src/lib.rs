//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Schema Registry for OpenTelemetry Data Lake Bridge
//!
//! This module provides schema management and validation capabilities
//! for telemetry data structures.

pub mod api;
pub mod config;
pub mod error;
pub mod metrics;
pub mod registry;
pub mod schema;
pub mod storage;
pub mod types;
pub mod utils;
pub mod validation;

// Re-export main types
pub use bridge_core::types::{ProcessedBatch, TelemetryBatch};
pub use config::SchemaRegistryConfig;
pub use error::{SchemaRegistryError, SchemaRegistryResult};
pub use registry::{RegistryMetrics, RegistryState, RegistryStats, SchemaRegistryManager};
pub use schema::{Schema, SchemaMetadata, SchemaVersion};
pub use storage::{MemoryStorage, RedisStorage, StorageBackend, StorageError};
pub use validation::{SchemaValidator, SchemaValidatorTrait, ValidationError, ValidationResult};

/// Schema Registry version
pub const SCHEMA_REGISTRY_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Schema Registry name
pub const SCHEMA_REGISTRY_NAME: &str = "orasi-schema-registry";

/// Initialize the schema registry
pub async fn init_schema_registry(
    config: SchemaRegistryConfig,
) -> SchemaRegistryResult<SchemaRegistry> {
    SchemaRegistry::new(config).await
}

/// Shutdown the schema registry
pub async fn shutdown_schema_registry(registry: SchemaRegistry) -> SchemaRegistryResult<()> {
    registry.shutdown().await
}

/// Schema Registry for OpenTelemetry Data Lake Bridge
///
/// This provides a centralized service for managing and validating
/// telemetry data schemas across the bridge ecosystem.
pub struct SchemaRegistry {
    /// Configuration
    config: SchemaRegistryConfig,
    /// Storage backend
    storage: Box<dyn StorageBackend>,
    /// Schema validator
    validator: SchemaValidator,
    /// Registry manager
    manager: SchemaRegistryManager,
}

impl SchemaRegistry {
    /// Create a new schema registry
    pub async fn new(config: SchemaRegistryConfig) -> SchemaRegistryResult<Self> {
        let storage = MemoryStorage::new()?;
        let validator = SchemaValidator::new();
        let manager = SchemaRegistryManager::new(config.clone());

        Ok(Self {
            config,
            storage: Box::new(storage),
            validator,
            manager,
        })
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

    /// Validate telemetry data against a schema
    pub async fn validate_data(
        &self,
        fingerprint: &str,
        data: &TelemetryBatch,
    ) -> SchemaRegistryResult<ValidationResult> {
        let schema = self
            .get_schema(fingerprint)
            .await?
            .ok_or_else(|| SchemaRegistryError::SchemaNotFound(fingerprint.to_string()))?;

        self.validator.validate_data(&schema, data).await
    }

    /// Get registry statistics
    pub async fn get_stats(&self) -> SchemaRegistryResult<RegistryStats> {
        self.manager.get_stats().await
    }

    /// Health check
    pub async fn health_check(&self) -> SchemaRegistryResult<bool> {
        // Basic health check - ensure storage is accessible
        let _ = self.storage.list_schemas().await?;
        Ok(true)
    }

    /// Get the registry manager
    pub fn manager(&self) -> &SchemaRegistryManager {
        &self.manager
    }

    /// Shutdown the schema registry
    pub async fn shutdown(self) -> SchemaRegistryResult<()> {
        // Shutdown the registry manager
        self.manager.shutdown().await?;

        // Shutdown the storage backend
        self.storage.shutdown().await?;

        // Log shutdown completion
        tracing::info!("Schema registry shutdown completed");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Schema, SchemaType};

    #[tokio::test]
    async fn test_schema_registry_creation() {
        let config = SchemaRegistryConfig::default();
        let registry = SchemaRegistry::new(config).await;
        assert!(registry.is_ok());
    }

    #[tokio::test]
    async fn test_schema_registry_health_check() {
        let config = SchemaRegistryConfig::default();
        let registry = SchemaRegistry::new(config).await.unwrap();
        let health = registry.health_check().await;
        assert!(health.is_ok());
        assert!(health.unwrap());
    }

    #[tokio::test]
    async fn test_schema_registry_shutdown() {
        let config = SchemaRegistryConfig::default();
        let registry = SchemaRegistry::new(config).await.unwrap();

        // Shutdown should complete successfully
        let shutdown_result = registry.shutdown().await;
        assert!(shutdown_result.is_ok());
    }
}
