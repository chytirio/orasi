//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake connector unit tests
//! 
//! This module contains unit tests for individual components
//! of the Delta Lake connector.

use crate::config::DeltaLakeConfig;
use crate::connector::DeltaLakeConnector;
use crate::writer::DeltaLakeWriter;
use crate::reader::DeltaLakeReader;
use crate::schema::DeltaLakeSchema;
use crate::error::{DeltaLakeError, DeltaLakeResult};
use crate::tests::utils;

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = DeltaLakeConfig::new(
            "/tmp/test".to_string(),
            "test_table".to_string(),
        );
        
        assert_eq!(config.table_name(), "test_table");
        assert_eq!(config.storage_path(), "/tmp/test");
    }

    #[test]
    fn test_config_validation() {
        let config = DeltaLakeConfig::new(
            "/tmp/test".to_string(),
            "test_table".to_string(),
        );
        
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_validation_empty_path() {
        let mut config = DeltaLakeConfig::new(
            "".to_string(),
            "test_table".to_string(),
        );
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_empty_table() {
        let mut config = DeltaLakeConfig::new(
            "/tmp/test".to_string(),
            "".to_string(),
        );
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_default() {
        let config = DeltaLakeConfig::default();
        assert!(!config.storage_path().is_empty());
        assert!(!config.table_name().is_empty());
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = DeltaLakeError::configuration("Test error");
        assert_eq!(error.category(), "configuration");
        assert!(error.error_code().is_some());
    }

    #[test]
    fn test_error_retryable() {
        let error = DeltaLakeError::connection("Connection failed");
        assert!(error.is_retryable());
        
        let error = DeltaLakeError::configuration("Config error");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_error_transient() {
        let error = DeltaLakeError::timeout("Timeout");
        assert!(error.is_transient());
        
        let error = DeltaLakeError::validation("Validation error");
        assert!(!error.is_transient());
    }

    #[test]
    fn test_error_with_source() {
        let source_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error = DeltaLakeError::with_source("IO error", source_error);
        assert_eq!(error.category(), "wrapped");
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = DeltaLakeSchema::create_telemetry_schema();
        assert_eq!(schema.name, "telemetry_schema");
        assert_eq!(schema.version, 1);
        assert!(!schema.fields.is_empty());
    }

    #[test]
    fn test_schema_validation() {
        let schema = DeltaLakeSchema::create_telemetry_schema();
        let manager = DeltaLakeSchemaManager::new();
        let result = manager.validate_schema(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_evolution() {
        let mut manager = DeltaLakeSchemaManager::new();
        let schema1 = DeltaLakeSchema::create_telemetry_schema();
        manager.set_schema(schema1).unwrap();
        
        let mut schema2 = DeltaLakeSchema::create_telemetry_schema();
        schema2.version = 2;
        schema2.fields.push(DeltaLakeField {
            name: "new_field".to_string(),
            data_type: DeltaLakeDataType::String,
            nullable: true,
            metadata: std::collections::HashMap::new(),
        });
        
        let result = manager.evolve_schema(schema2);
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod writer_tests {
    use super::*;
    use bridge_core::types::{MetricsBatch, MetricsRecord};

    #[tokio::test]
    async fn test_writer_creation() {
        let config = utils::create_test_config();
        let writer = DeltaLakeWriter::new(config).await;
        assert!(writer.is_ok());
    }

    #[tokio::test]
    async fn test_writer_metrics() {
        let config = utils::create_test_config();
        let writer = DeltaLakeWriter::new(config).await.unwrap();
        
        let batch = MetricsBatch {
            records: vec![
                MetricsRecord {
                    name: "test_metric".to_string(),
                    value: 42.0,
                    timestamp: chrono::Utc::now(),
                    labels: std::collections::HashMap::new(),
                }
            ],
        };
        
        let result = writer.write_metrics(batch).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_writer_health_check() {
        let config = utils::create_test_config();
        let writer = DeltaLakeWriter::new(config).await.unwrap();
        
        let health = writer.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_writer_stats() {
        let config = utils::create_test_config();
        let writer = DeltaLakeWriter::new(config).await.unwrap();
        
        let stats = writer.get_stats().await;
        assert!(stats.is_ok());
    }
}

#[cfg(test)]
mod reader_tests {
    use super::*;
    use bridge_core::types::{MetricsQuery, TracesQuery, LogsQuery};

    #[tokio::test]
    async fn test_reader_creation() {
        let config = utils::create_test_config();
        let reader = DeltaLakeReader::new(config).await;
        assert!(reader.is_ok());
    }

    #[tokio::test]
    async fn test_reader_metrics_query() {
        let config = utils::create_test_config();
        let reader = DeltaLakeReader::new(config).await.unwrap();
        
        let query = MetricsQuery {
            name: Some("test_metric".to_string()),
            start_time: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            end_time: Some(chrono::Utc::now()),
            labels: std::collections::HashMap::new(),
            limit: Some(100),
        };
        
        let result = reader.query_metrics(query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reader_traces_query() {
        let config = utils::create_test_config();
        let reader = DeltaLakeReader::new(config).await.unwrap();
        
        let query = TracesQuery {
            trace_id: Some("test_trace".to_string()),
            start_time: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            end_time: Some(chrono::Utc::now()),
            service_name: Some("test_service".to_string()),
            limit: Some(100),
        };
        
        let result = reader.query_traces(query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reader_logs_query() {
        let config = utils::create_test_config();
        let reader = DeltaLakeReader::new(config).await.unwrap();
        
        let query = LogsQuery {
            start_time: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            end_time: Some(chrono::Utc::now()),
            service_name: Some("test_service".to_string()),
            level: Some("INFO".to_string()),
            limit: Some(100),
        };
        
        let result = reader.query_logs(query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reader_custom_query() {
        let config = utils::create_test_config();
        let reader = DeltaLakeReader::new(config).await.unwrap();
        
        let query = "SELECT * FROM telemetry WHERE timestamp > '2023-01-01'".to_string();
        let result = reader.execute_query(query).await;
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod connector_tests {
    use super::*;

    #[tokio::test]
    async fn test_connector_creation() {
        let connector = utils::create_test_connector().await;
        assert!(connector.is_ok());
    }

    #[tokio::test]
    async fn test_connector_writer() {
        let connector = utils::create_test_connector().await.unwrap();
        let writer = connector.writer().await;
        assert!(writer.is_ok());
    }

    #[tokio::test]
    async fn test_connector_reader() {
        let connector = utils::create_test_connector().await.unwrap();
        let reader = connector.reader().await;
        assert!(reader.is_ok());
    }

    #[tokio::test]
    async fn test_connector_name_version() {
        let connector = utils::create_test_connector().await.unwrap();
        assert_eq!(connector.name(), "deltalake-connector");
        assert!(!connector.version().is_empty());
    }

    #[tokio::test]
    async fn test_connector_shutdown() {
        let connector = utils::create_test_connector().await.unwrap();
        let result = connector.shutdown().await;
        assert!(result.is_ok());
    }
}
