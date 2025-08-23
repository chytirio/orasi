//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Storage backends for the Schema Registry
//!
//! This module provides storage abstractions and implementations for
//! persisting schemas in the registry.

pub mod error;
pub mod memory;
pub mod postgres;
pub mod redis;
pub mod sqlite;

#[cfg(test)]
mod tests;

use crate::error::{SchemaRegistryError, SchemaRegistryResult};
use crate::schema::{Schema, SchemaMetadata, SchemaSearchCriteria, SchemaVersion};
use async_trait::async_trait;
use chrono;

/// Storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a schema
    async fn store_schema(&self, schema: Schema) -> SchemaRegistryResult<SchemaVersion>;

    /// Retrieve a schema by fingerprint
    async fn get_schema(&self, fingerprint: &str) -> SchemaRegistryResult<Option<Schema>>;

    /// List all schemas
    async fn list_schemas(&self) -> SchemaRegistryResult<Vec<SchemaMetadata>>;

    /// Search schemas
    async fn search_schemas(
        &self,
        criteria: &SchemaSearchCriteria,
    ) -> SchemaRegistryResult<Vec<SchemaMetadata>>;

    /// Delete a schema
    async fn delete_schema(&self, fingerprint: &str) -> SchemaRegistryResult<bool>;

    /// Get schema versions
    async fn get_schema_versions(&self, name: &str) -> SchemaRegistryResult<Vec<SchemaVersion>>;

    /// Get latest schema version
    async fn get_latest_schema(&self, name: &str) -> SchemaRegistryResult<Option<Schema>>;

    /// Health check
    async fn health_check(&self) -> SchemaRegistryResult<bool>;

    /// Get storage statistics
    async fn get_stats(&self) -> SchemaRegistryResult<StorageStats>;

    /// Shutdown the storage backend
    async fn shutdown(&self) -> SchemaRegistryResult<()>;
}

/// Storage statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageStats {
    /// Total number of schemas
    pub total_schemas: u64,

    /// Total number of versions
    pub total_versions: u64,

    /// Total storage size in bytes
    pub total_size_bytes: u64,

    /// Average schema size in bytes
    pub avg_schema_size_bytes: u64,

    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

// Re-export storage implementations
pub use error::StorageError;
pub use memory::MemoryStorage;
pub use postgres::PostgresStorage;
pub use redis::RedisStorage;
pub use sqlite::SqliteStorage;
