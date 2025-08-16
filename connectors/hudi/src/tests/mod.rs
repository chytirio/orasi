//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Hudi connector tests
//! 
//! This module contains comprehensive unit and integration tests
//! for the Hudi connector functionality.

pub mod unit;
pub mod integration;
pub mod common;

use crate::config::HudiConfig;
use crate::connector::HudiConnector;
use crate::error::HudiResult;
use bridge_core::traits::LakehouseConnector;

/// Test utilities and helpers
pub mod utils {
    use super::*;
    use std::env;

    /// Create a test configuration
    pub fn create_test_config() -> HudiConfig {
        // Use environment variables for test configuration
        let table_path = env::var("HUDI_TEST_TABLE_PATH")
            .unwrap_or_else(|_| "/tmp/test_hudi_table".to_string());
        let table_name = env::var("HUDI_TEST_TABLE_NAME")
            .unwrap_or_else(|_| "test_telemetry".to_string());
        
        HudiConfig::new(
            table_path,
            table_name,
        )
    }

    /// Create a test connector
    pub async fn create_test_connector() -> HudiResult<HudiConnector> {
        let config = create_test_config();
        let mut connector = HudiConnector::new(config);
        connector.initialize().await?;
        Ok(connector)
    }

    /// Check if Hudi is available for testing
    pub fn is_hudi_available() -> bool {
        env::var("HUDI_TEST_TABLE_PATH").is_ok() || 
        env::var("SKIP_HUDI_TESTS").is_err()
    }

    /// Skip test if Hudi is not available
    pub fn skip_if_no_hudi() {
        if !is_hudi_available() {
            eprintln!("Skipping Hudi test - no Hudi instance available");
            std::process::exit(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connector_creation() {
        utils::skip_if_no_hudi();
        let connector = utils::create_test_connector().await;
        assert!(connector.is_ok());
    }

    #[tokio::test]
    async fn test_connector_health_check() {
        utils::skip_if_no_hudi();
        let connector = utils::create_test_connector().await.unwrap();
        let health = connector.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_connector_stats() {
        utils::skip_if_no_hudi();
        let connector = utils::create_test_connector().await.unwrap();
        let stats = connector.get_stats().await;
        assert!(stats.is_ok());
    }
}
