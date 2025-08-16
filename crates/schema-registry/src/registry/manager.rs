//! Schema Registry Manager
//!
//! This module provides the main registry manager for coordinating
//! schema operations and managing the registry lifecycle.

use crate::config::SchemaRegistryConfig;
use crate::error::{SchemaRegistryError, SchemaRegistryResult};
use crate::schema::{Schema, SchemaMetadata, SchemaSearchCriteria, SchemaVersion};
use crate::storage::{MemoryStorage, PostgresStorage, SqliteStorage, StorageBackend};
use crate::validation::{SchemaValidator, SchemaValidatorTrait, ValidationResult};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{
    operations::SchemaOperations,
    state::{RegistryMetrics, RegistryState, RegistryStats},
    validation::{MetricsCollector, ValidationTracker},
};

/// Schema Registry Manager
pub struct SchemaRegistryManager {
    /// Configuration
    config: SchemaRegistryConfig,

    /// Storage backend
    storage: Arc<dyn StorageBackend>,

    /// Schema validator
    validator: Arc<SchemaValidator>,

    /// Registry state
    state: Arc<RwLock<RegistryState>>,

    /// Schema operations
    operations: SchemaOperations,

    /// Validation tracker
    validation_tracker: ValidationTracker,

    /// Metrics collector
    metrics_collector: MetricsCollector,
}

impl SchemaRegistryManager {
    /// Create a new schema registry manager
    pub fn new(config: SchemaRegistryConfig) -> Self {
        let storage = Arc::new(MemoryStorage::new().expect("Failed to create memory storage"));
        let validator = Arc::new(SchemaValidator::new());
        let state = Arc::new(RwLock::new(RegistryState::new()));

        let operations = SchemaOperations::new(storage.clone(), validator.clone());
        let validation_tracker = ValidationTracker::new(state.clone());
        let metrics_collector = MetricsCollector::new(storage.clone(), state.clone());

        Self {
            config,
            storage,
            validator,
            state,
            operations,
            validation_tracker,
            metrics_collector,
        }
    }

    /// Create storage backend based on configuration
    async fn create_storage_backend(
        config: &SchemaRegistryConfig,
    ) -> SchemaRegistryResult<Arc<dyn StorageBackend>> {
        match config.storage.backend {
            crate::config::StorageBackendType::Memory => {
                Ok(Arc::new(MemoryStorage::new()?))
            }
            crate::config::StorageBackendType::Postgres => {
                let storage = PostgresStorage::new(config.storage.postgres.url.clone()).await?;
                Ok(Arc::new(storage))
            }
            crate::config::StorageBackendType::Sqlite => {
                let storage = SqliteStorage::new(config.storage.sqlite.database_path.clone()).await?;
                Ok(Arc::new(storage))
            }
            crate::config::StorageBackendType::Redis => {
                let storage = crate::storage::RedisStorage::new_with_config(&config.storage.redis).await?;
                Ok(Arc::new(storage))
            }
        }
    }

    /// Create a new schema registry manager with proper storage backend
    pub async fn new_with_storage(config: SchemaRegistryConfig) -> SchemaRegistryResult<Self> {
        let storage = Self::create_storage_backend(&config).await?;
        let validator = Arc::new(SchemaValidator::new());
        let state = Arc::new(RwLock::new(RegistryState::new()));

        let operations = SchemaOperations::new(storage.clone(), validator.clone());
        let validation_tracker = ValidationTracker::new(state.clone());
        let metrics_collector = MetricsCollector::new(storage.clone(), state.clone());

        Ok(Self {
            config,
            storage,
            validator,
            state,
            operations,
            validation_tracker,
            metrics_collector,
        })
    }

    /// Initialize the registry
    pub async fn initialize(&self) -> SchemaRegistryResult<()> {
        // Validate configuration
        self.config
            .validate()
            .map_err(|e| SchemaRegistryError::config(&e))?;

        // Perform health check
        let healthy = self.storage.health_check().await?;

        // Update state
        {
            let mut state = self.state.write().await;
            state.mark_initialized();
            state.update_health(healthy);
        }

        // Update statistics (but don't fail initialization if this fails)
        let _ = self.metrics_collector.update_stats().await;

        Ok(())
    }

    /// Register a new schema
    pub async fn register_schema(&self, schema: Schema) -> SchemaRegistryResult<SchemaVersion> {
        // Track validation results first
        let validation_result = self.validator.validate_schema(&schema).await?;
        self.validation_tracker.track_validation_result(&validation_result).await;

        // Register the schema
        let version = self.operations.register_schema(schema).await?;

        // Update statistics
        self.metrics_collector.update_stats().await?;

        Ok(version)
    }

    /// Retrieve a schema by fingerprint
    pub async fn get_schema(&self, fingerprint: &str) -> SchemaRegistryResult<Option<Schema>> {
        self.operations.get_schema(fingerprint).await
    }

    /// List all schemas
    pub async fn list_schemas(&self) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        self.operations.list_schemas().await
    }

    /// Search schemas
    pub async fn search_schemas(
        &self,
        criteria: &SchemaSearchCriteria,
    ) -> SchemaRegistryResult<Vec<SchemaMetadata>> {
        self.operations.search_schemas(criteria).await
    }

    /// Delete a schema
    pub async fn delete_schema(&self, fingerprint: &str) -> SchemaRegistryResult<bool> {
        let deleted = self.operations.delete_schema(fingerprint).await?;

        if deleted {
            // Update statistics
            self.metrics_collector.update_stats().await?;
        }

        Ok(deleted)
    }

    /// Get schema versions
    pub async fn get_schema_versions(
        &self,
        name: &str,
    ) -> SchemaRegistryResult<Vec<SchemaVersion>> {
        self.operations.get_schema_versions(name).await
    }

    /// Get latest schema version
    pub async fn get_latest_schema(&self, name: &str) -> SchemaRegistryResult<Option<Schema>> {
        self.operations.get_latest_schema(name).await
    }

    /// Validate telemetry data against a schema
    pub async fn validate_data(
        &self,
        fingerprint: &str,
        data: &bridge_core::types::TelemetryBatch,
    ) -> SchemaRegistryResult<ValidationResult> {
        let validation_result = self.operations.validate_data(fingerprint, data).await?;

        // Track validation results
        self.validation_tracker.track_validation_result(&validation_result).await;

        Ok(validation_result)
    }

    /// Validate a schema
    pub async fn validate_schema(&self, schema: &Schema) -> SchemaRegistryResult<ValidationResult> {
        let validation_result = self.operations.validate_schema(schema).await?;

        // Track validation results
        self.validation_tracker.track_validation_result(&validation_result).await;

        Ok(validation_result)
    }

    /// Evolve a schema
    pub async fn evolve_schema(&self, schema: &Schema) -> SchemaRegistryResult<Schema> {
        self.operations.evolve_schema(schema).await
    }

    /// Get registry metrics
    pub async fn get_metrics(&self) -> SchemaRegistryResult<RegistryMetrics> {
        self.metrics_collector.get_metrics().await
    }

    /// Perform health check
    pub async fn health_check(&self) -> SchemaRegistryResult<bool> {
        let healthy = self.storage.health_check().await?;

        // Update state
        {
            let mut state = self.state.write().await;
            state.update_health(healthy);
        }

        Ok(healthy)
    }

    /// Get registry state
    pub async fn get_state(&self) -> RegistryState {
        self.state.read().await.clone()
    }

    /// Get registry statistics
    pub async fn get_stats(&self) -> SchemaRegistryResult<RegistryStats> {
        self.metrics_collector.get_stats().await
    }

    /// Shutdown the registry
    pub async fn shutdown(&self) -> SchemaRegistryResult<()> {
        // Update state
        {
            let mut state = self.state.write().await;
            state.initialized = false;
            state.healthy = false;
        }

        Ok(())
    }
}

impl Default for SchemaRegistryManager {
    fn default() -> Self {
        Self::new(SchemaRegistryConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Schema, SchemaFormat, SchemaType, SchemaVersion};

    #[tokio::test]
    async fn test_registry_manager_creation() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);

        let state = manager.get_state().await;
        assert!(!state.initialized);
        assert!(!state.healthy);
    }

    #[tokio::test]
    async fn test_registry_initialization() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);

        let result = manager.initialize().await;
        assert!(result.is_ok());

        let state = manager.get_state().await;
        assert!(state.initialized);
        assert!(state.healthy);
    }

    #[tokio::test]
    async fn test_schema_registration() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);
        manager.initialize().await.unwrap();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object", "properties": {"value": {"type": "number"}}}"#.to_string(),
            SchemaFormat::Json,
        );

        let version = manager.register_schema(schema).await.unwrap();
        assert_eq!(version.to_string(), "1.0.0");

        let schemas = manager.list_schemas().await.unwrap();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "test-schema");
    }

    #[tokio::test]
    async fn test_schema_retrieval() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);
        manager.initialize().await.unwrap();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        let fingerprint = schema.fingerprint.clone();
        manager.register_schema(schema).await.unwrap();

        let retrieved = manager.get_schema(&fingerprint).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-schema");
    }

    #[tokio::test]
    async fn test_schema_deletion() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);
        manager.initialize().await.unwrap();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        let fingerprint = schema.fingerprint.clone();
        manager.register_schema(schema).await.unwrap();

        let deleted = manager.delete_schema(&fingerprint).await.unwrap();
        assert!(deleted);

        let retrieved = manager.get_schema(&fingerprint).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);
        manager.initialize().await.unwrap();

        let healthy = manager.health_check().await.unwrap();
        assert!(healthy);

        let state = manager.get_state().await;
        assert!(state.healthy);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);
        manager.initialize().await.unwrap();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        manager.register_schema(schema).await.unwrap();

        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_schemas, 1);
        assert_eq!(stats.total_versions, 1);
        assert!(stats.total_size_bytes > 0);
    }

    #[tokio::test]
    async fn test_schema_evolution() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);
        manager.initialize().await.unwrap();

        // Register initial schema
        let initial_schema = Schema::new(
            "test-evolution".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object", "properties": {"value": {"type": "number"}}}"#.to_string(),
            SchemaFormat::Json,
        );

        manager.register_schema(initial_schema).await.unwrap();

        // Create evolved schema (backward compatible)
        let evolved_schema = Schema::new(
            "test-evolution".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object", "properties": {"value": {"type": "number"}, "unit": {"type": "string"}}}"#.to_string(),
            SchemaFormat::Json,
        );

        let evolved = manager.evolve_schema(&evolved_schema).await.unwrap();
        assert_eq!(evolved.version.to_string(), "1.0.1");
        assert!(evolved.content.contains("unit"));
    }

    #[tokio::test]
    async fn test_validation_tracking() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);
        manager.initialize().await.unwrap();

        // Register a schema
        let schema = Schema::new(
            "test-validation".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        manager.register_schema(schema.clone()).await.unwrap();

        // Validate the schema (should track any validation results)
        let _validation_result = manager.validate_schema(&schema).await.unwrap();

        // Check that stats are updated
        let stats = manager.get_stats().await.unwrap();
        // Validation errors and warnings should be tracked (even if 0)
        assert_eq!(stats.validation_errors, 0);
        assert_eq!(stats.validation_warnings, 0);
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let config = SchemaRegistryConfig::default();
        let manager = SchemaRegistryManager::new(config);
        manager.initialize().await.unwrap();

        let schema = Schema::new(
            "test-metrics".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        manager.register_schema(schema).await.unwrap();

        let metrics = manager.get_metrics().await.unwrap();
        assert_eq!(metrics.total_schemas, 1);
        assert_eq!(metrics.total_versions, 1);
        assert!(metrics.total_size_bytes > 0);
        assert!(metrics.avg_schema_size_bytes > 0);
    }
}
