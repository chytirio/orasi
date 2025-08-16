//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Pipeline health checking for the OpenTelemetry Data Lake Bridge
//!
//! This module provides health checking functionality for the telemetry
//! ingestion pipeline.

use crate::error::BridgeResult;
use crate::traits::{LakehouseExporter, TelemetryProcessor, TelemetryReceiver};
use std::sync::Arc;
use tracing::{error, info};

/// Pipeline health checker
pub struct PipelineHealthChecker {
    /// Receivers to check
    receivers: Vec<Arc<dyn TelemetryReceiver>>,

    /// Processors to check
    processors: Vec<Arc<dyn TelemetryProcessor>>,

    /// Exporters to check
    exporters: Vec<Arc<dyn LakehouseExporter>>,
}

impl PipelineHealthChecker {
    /// Create a new pipeline health checker
    pub fn new() -> Self {
        Self {
            receivers: Vec::new(),
            processors: Vec::new(),
            exporters: Vec::new(),
        }
    }

    /// Add a receiver to check
    pub fn add_receiver(&mut self, receiver: Arc<dyn TelemetryReceiver>) {
        self.receivers.push(receiver);
    }

    /// Add a processor to check
    pub fn add_processor(&mut self, processor: Arc<dyn TelemetryProcessor>) {
        self.processors.push(processor);
    }

    /// Add an exporter to check
    pub fn add_exporter(&mut self, exporter: Arc<dyn LakehouseExporter>) {
        self.exporters.push(exporter);
    }

    /// Perform health check on all components
    pub async fn health_check(&self) -> BridgeResult<HealthCheckResult> {
        let mut result = HealthCheckResult {
            overall_healthy: true,
            receivers: Vec::new(),
            processors: Vec::new(),
            exporters: Vec::new(),
        };

        // Check receivers
        for (i, receiver) in self.receivers.iter().enumerate() {
            match receiver.health_check().await {
                Ok(healthy) => {
                    result.receivers.push(ComponentHealth {
                        index: i,
                        name: format!("receiver-{}", i),
                        healthy,
                        error: None,
                    });
                    if !healthy {
                        result.overall_healthy = false;
                    }
                }
                Err(e) => {
                    error!("Health check failed for receiver {}: {}", i, e);
                    result.receivers.push(ComponentHealth {
                        index: i,
                        name: format!("receiver-{}", i),
                        healthy: false,
                        error: Some(e.to_string()),
                    });
                    result.overall_healthy = false;
                }
            }
        }

        // Check processors
        for (i, processor) in self.processors.iter().enumerate() {
            match processor.health_check().await {
                Ok(healthy) => {
                    result.processors.push(ComponentHealth {
                        index: i,
                        name: processor.name().to_string(),
                        healthy,
                        error: None,
                    });
                    if !healthy {
                        result.overall_healthy = false;
                    }
                }
                Err(e) => {
                    error!("Health check failed for processor {}: {}", i, e);
                    result.processors.push(ComponentHealth {
                        index: i,
                        name: processor.name().to_string(),
                        healthy: false,
                        error: Some(e.to_string()),
                    });
                    result.overall_healthy = false;
                }
            }
        }

        // Check exporters
        for (i, exporter) in self.exporters.iter().enumerate() {
            match exporter.health_check().await {
                Ok(healthy) => {
                    result.exporters.push(ComponentHealth {
                        index: i,
                        name: exporter.name().to_string(),
                        healthy,
                        error: None,
                    });
                    if !healthy {
                        result.overall_healthy = false;
                    }
                }
                Err(e) => {
                    error!("Health check failed for exporter {}: {}", i, e);
                    result.exporters.push(ComponentHealth {
                        index: i,
                        name: exporter.name().to_string(),
                        healthy: false,
                        error: Some(e.to_string()),
                    });
                    result.overall_healthy = false;
                }
            }
        }

        if result.overall_healthy {
            info!("All pipeline components are healthy");
        } else {
            error!("Some pipeline components are unhealthy");
        }

        Ok(result)
    }
}

impl Default for PipelineHealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Overall health status
    pub overall_healthy: bool,

    /// Receiver health status
    pub receivers: Vec<ComponentHealth>,

    /// Processor health status
    pub processors: Vec<ComponentHealth>,

    /// Exporter health status
    pub exporters: Vec<ComponentHealth>,
}

/// Component health status
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// Component index
    pub index: usize,

    /// Component name
    pub name: String,

    /// Health status
    pub healthy: bool,

    /// Error message if unhealthy
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::predicate::*;
    use mockall::*;

    mock! {
        TestReceiver {}

        #[async_trait]
        impl TelemetryReceiver for TestReceiver {
            async fn receive(&self) -> crate::error::BridgeResult<crate::types::TelemetryBatch>;
            async fn health_check(&self) -> crate::error::BridgeResult<bool>;
            async fn get_stats(&self) -> crate::error::BridgeResult<crate::traits::ReceiverStats>;
            async fn shutdown(&self) -> crate::error::BridgeResult<()>;
        }
    }

    mock! {
        TestProcessor {}

        #[async_trait]
        impl TelemetryProcessor for TestProcessor {
            async fn process(&self, batch: crate::types::TelemetryBatch) -> crate::error::BridgeResult<crate::types::ProcessedBatch>;
            fn name(&self) -> &str;
            fn version(&self) -> &str;
            async fn health_check(&self) -> crate::error::BridgeResult<bool>;
            async fn get_stats(&self) -> crate::error::BridgeResult<crate::traits::ProcessorStats>;
            async fn shutdown(&self) -> crate::error::BridgeResult<()>;
        }
    }

    mock! {
        TestExporter {}

        #[async_trait]
        impl LakehouseExporter for TestExporter {
            async fn export(&self, batch: crate::types::ProcessedBatch) -> crate::error::BridgeResult<crate::types::ExportResult>;
            fn name(&self) -> &str;
            fn version(&self) -> &str;
            async fn health_check(&self) -> crate::error::BridgeResult<bool>;
            async fn get_stats(&self) -> crate::error::BridgeResult<crate::traits::ExporterStats>;
            async fn shutdown(&self) -> crate::error::BridgeResult<()>;
        }
    }

    #[tokio::test]
    async fn test_health_checker_creation() {
        let checker = PipelineHealthChecker::new();
        let result = checker.health_check().await.unwrap();
        assert!(result.overall_healthy);
        assert!(result.receivers.is_empty());
        assert!(result.processors.is_empty());
        assert!(result.exporters.is_empty());
    }

    #[tokio::test]
    async fn test_health_check_with_healthy_components() {
        let mut checker = PipelineHealthChecker::new();

        // Add healthy receiver
        let mut mock_receiver = MockTestReceiver::new();
        mock_receiver
            .expect_health_check()
            .times(1)
            .returning(|| Ok(true));
        checker.add_receiver(Arc::new(mock_receiver));

        // Add healthy processor
        let mut mock_processor = MockTestProcessor::new();
        mock_processor
            .expect_health_check()
            .times(1)
            .returning(|| Ok(true));
        mock_processor
            .expect_name()
            .times(1)
            .return_const("test-processor".to_string());
        checker.add_processor(Arc::new(mock_processor));

        // Add healthy exporter
        let mut mock_exporter = MockTestExporter::new();
        mock_exporter
            .expect_health_check()
            .times(1)
            .returning(|| Ok(true));
        mock_exporter
            .expect_name()
            .times(1)
            .return_const("test-exporter".to_string());
        checker.add_exporter(Arc::new(mock_exporter));

        let result = checker.health_check().await.unwrap();
        assert!(result.overall_healthy);
        assert_eq!(result.receivers.len(), 1);
        assert_eq!(result.processors.len(), 1);
        assert_eq!(result.exporters.len(), 1);
    }

    #[tokio::test]
    async fn test_health_check_with_unhealthy_components() {
        let mut checker = PipelineHealthChecker::new();

        // Add unhealthy receiver
        let mut mock_receiver = MockTestReceiver::new();
        mock_receiver
            .expect_health_check()
            .times(1)
            .returning(|| Ok(false));
        checker.add_receiver(Arc::new(mock_receiver));

        let result = checker.health_check().await.unwrap();
        assert!(!result.overall_healthy);
        assert_eq!(result.receivers.len(), 1);
        assert!(!result.receivers[0].healthy);
    }
}
