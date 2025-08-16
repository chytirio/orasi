//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake connector tests
//! 
//! This module contains comprehensive unit and integration tests
//! for the Delta Lake connector functionality.

pub mod unit;
pub mod integration;
pub mod common;

use crate::config::DeltaLakeConfig;
use crate::connector::DeltaLakeConnector;
use crate::error::DeltaLakeResult;

/// Test utilities and helpers
pub mod utils {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;

    /// Create a temporary test directory
    pub fn create_temp_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp directory")
    }

    /// Create a test configuration
    pub fn create_test_config() -> DeltaLakeConfig {
        let temp_dir = create_temp_dir();
        DeltaLakeConfig::new(
            temp_dir.path().to_string_lossy().to_string(),
            "test_table".to_string(),
        )
    }

    /// Create a test connector
    pub async fn create_test_connector() -> DeltaLakeResult<DeltaLakeConnector> {
        let config = create_test_config();
        DeltaLakeConnector::connect(config).await
    }

    /// Clean up test resources
    pub fn cleanup_test_resources(temp_dir: TempDir) {
        // Cleanup will happen automatically when TempDir is dropped
        drop(temp_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connector_creation() {
        let connector = utils::create_test_connector().await;
        assert!(connector.is_ok());
    }

    #[tokio::test]
    async fn test_connector_health_check() {
        let connector = utils::create_test_connector().await.unwrap();
        let health = connector.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_connector_stats() {
        let connector = utils::create_test_connector().await.unwrap();
        let stats = connector.get_stats().await;
        assert!(stats.is_ok());
    }
}
