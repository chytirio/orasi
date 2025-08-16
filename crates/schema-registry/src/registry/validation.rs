//! Validation tracking and metrics
//!
//! This module handles validation tracking and metrics collection for the registry.

use crate::error::SchemaRegistryResult;
use crate::storage::StorageBackend;
use crate::validation::ValidationResult;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::state::{RegistryMetrics, RegistryStats};

/// Validation tracker
pub struct ValidationTracker {
    state: Arc<RwLock<super::state::RegistryState>>,
}

impl ValidationTracker {
    /// Create a new validation tracker
    pub fn new(state: Arc<RwLock<super::state::RegistryState>>) -> Self {
        Self { state }
    }

    /// Track validation errors
    pub async fn track_validation_errors(&self, count: u64) {
        let mut state = self.state.write().await;
        state.stats.track_validation_errors(count);
    }

    /// Track validation warnings
    pub async fn track_validation_warnings(&self, count: u64) {
        let mut state = self.state.write().await;
        state.stats.track_validation_warnings(count);
    }

    /// Track validation result
    pub async fn track_validation_result(&self, result: &ValidationResult) {
        self.track_validation_errors(result.errors.len() as u64)
            .await;
        self.track_validation_warnings(result.warnings.len() as u64)
            .await;
    }
}

/// Metrics collector
pub struct MetricsCollector {
    storage: Arc<dyn StorageBackend>,
    state: Arc<RwLock<super::state::RegistryState>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(
        storage: Arc<dyn StorageBackend>,
        state: Arc<RwLock<super::state::RegistryState>>,
    ) -> Self {
        Self { storage, state }
    }

    /// Get registry metrics
    pub async fn get_metrics(&self) -> SchemaRegistryResult<RegistryMetrics> {
        // Get storage statistics
        let storage_stats = self.storage.get_stats().await?;

        Ok(RegistryMetrics {
            total_schemas: storage_stats.total_schemas,
            total_versions: storage_stats.total_versions,
            total_size_bytes: storage_stats.total_size_bytes,
            avg_schema_size_bytes: storage_stats.avg_schema_size_bytes,
            last_activity: storage_stats.last_activity,
        })
    }

    /// Get registry statistics
    pub async fn get_stats(&self) -> SchemaRegistryResult<RegistryStats> {
        let storage_stats = self.storage.get_stats().await?;

        // Get current validation stats
        let validation_errors = {
            let state = self.state.read().await;
            state.stats.validation_errors
        };
        let validation_warnings = {
            let state = self.state.read().await;
            state.stats.validation_warnings
        };

        let stats = RegistryStats {
            total_schemas: storage_stats.total_schemas,
            total_versions: storage_stats.total_versions,
            total_size_bytes: storage_stats.total_size_bytes,
            validation_errors,
            validation_warnings,
            last_activity: storage_stats.last_activity,
        };

        // Update state
        {
            let mut state = self.state.write().await;
            state.update_stats(stats.clone());
        }

        Ok(stats)
    }

    /// Update statistics
    pub async fn update_stats(&self) -> SchemaRegistryResult<()> {
        let _ = self.get_stats().await?;
        Ok(())
    }
}
