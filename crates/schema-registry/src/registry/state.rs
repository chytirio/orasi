//! Registry state management
//!
//! This module contains the state structures and management for the schema registry.

use chrono;
use serde::{Deserialize, Serialize};

/// Registry state
#[derive(Debug, Clone)]
pub struct RegistryState {
    /// Whether the registry is initialized
    pub initialized: bool,

    /// Whether the registry is healthy
    pub healthy: bool,

    /// Last health check timestamp
    pub last_health_check: chrono::DateTime<chrono::Utc>,

    /// Registry statistics
    pub stats: RegistryStats,
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    /// Total number of schemas
    pub total_schemas: u64,

    /// Total number of versions
    pub total_versions: u64,

    /// Total storage size in bytes
    pub total_size_bytes: u64,

    /// Number of validation errors
    pub validation_errors: u64,

    /// Number of validation warnings
    pub validation_warnings: u64,

    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Registry metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryMetrics {
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

impl RegistryState {
    /// Create a new registry state
    pub fn new() -> Self {
        Self {
            initialized: false,
            healthy: false,
            last_health_check: chrono::Utc::now(),
            stats: RegistryStats::new(),
        }
    }

    /// Mark the registry as initialized
    pub fn mark_initialized(&mut self) {
        self.initialized = true;
    }

    /// Update health status
    pub fn update_health(&mut self, healthy: bool) {
        self.healthy = healthy;
        self.last_health_check = chrono::Utc::now();
    }

    /// Update statistics
    pub fn update_stats(&mut self, stats: RegistryStats) {
        self.stats = stats;
    }
}

impl RegistryStats {
    /// Create new registry statistics
    pub fn new() -> Self {
        Self {
            total_schemas: 0,
            total_versions: 0,
            total_size_bytes: 0,
            validation_errors: 0,
            validation_warnings: 0,
            last_activity: chrono::Utc::now(),
        }
    }

    /// Track validation errors
    pub fn track_validation_errors(&mut self, count: u64) {
        self.validation_errors += count;
        self.last_activity = chrono::Utc::now();
    }

    /// Track validation warnings
    pub fn track_validation_warnings(&mut self, count: u64) {
        self.validation_warnings += count;
        self.last_activity = chrono::Utc::now();
    }
}

impl Default for RegistryState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RegistryStats {
    fn default() -> Self {
        Self::new()
    }
}
