//! Delta Lake exporter health check implementation

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::exporter::LakehouseExporter;
use super::exporter::DeltaLakeExporter;
use async_trait::async_trait;
use tracing::{debug, error, info};

#[async_trait]
impl HealthCheckCallback for DeltaLakeExporter {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        debug!("Performing health check on Delta Lake exporter");

        // Check if exporter is running
        let running = self.running.read().await;
        if !*running {
            return Ok(HealthCheckResult {
                component: "delta_lake_exporter".to_string(),
                status: crate::health::types::HealthStatus::Unhealthy,
                message: "Delta Lake exporter is not running".to_string(),
                details: None,
                timestamp: chrono::Utc::now(),
                duration_ms: 0,
                errors: vec![],
            });
        }

        // Check if Delta table is accessible
        let delta_table = self.delta_table.read().await;
        if delta_table.is_none() {
            return Ok(HealthCheckResult {
                component: "delta_lake_exporter".to_string(),
                status: crate::health::types::HealthStatus::Degraded,
                message: "Delta table not initialized".to_string(),
                details: None,
                timestamp: chrono::Utc::now(),
                duration_ms: 0,
                errors: vec![],
            });
        }

        // Try to perform a simple operation to verify connectivity
        match self.perform_connectivity_check().await {
            Ok(_) => {
                debug!("Delta Lake exporter health check passed");
                Ok(HealthCheckResult {
                    component: "delta_lake_exporter".to_string(),
                    status: crate::health::types::HealthStatus::Healthy,
                    message: "Delta Lake exporter is healthy".to_string(),
                    details: None,
                    timestamp: chrono::Utc::now(),
                    duration_ms: 0,
                    errors: vec![],
                })
            }
            Err(e) => {
                error!("Delta Lake exporter health check failed: {}", e);
                Ok(HealthCheckResult {
                    component: "delta_lake_exporter".to_string(),
                    status: crate::health::types::HealthStatus::Unhealthy,
                    message: format!("Delta Lake exporter health check failed: {}", e),
                    details: None,
                    timestamp: chrono::Utc::now(),
                    duration_ms: 0,
                    errors: vec![],
                })
            }
        }
    }

    fn component_name(&self) -> &str {
        "delta_lake_exporter"
    }
}

impl DeltaLakeExporter {
    /// Perform a connectivity check to verify Delta table access
    async fn perform_connectivity_check(&self) -> crate::error::BridgeResult<()> {
        let table_path = format!("{}/{}", self.config.table_location, self.config.table_name);
        
        // Check if the table directory exists
        if !std::path::Path::new(&table_path).exists() {
            return Err(crate::error::BridgeError::lakehouse(
                "Delta table directory does not exist",
            ));
        }

        // Check if we can read the Delta log
        let delta_log_dir = format!("{}/_delta_log", table_path);
        if !std::path::Path::new(&delta_log_dir).exists() {
            return Err(crate::error::BridgeError::lakehouse(
                "Delta log directory does not exist",
            ));
        }

        // Try to list files in the log directory
        match tokio::fs::read_dir(&delta_log_dir).await {
            Ok(_) => {
                debug!("Delta table connectivity check passed");
                Ok(())
            }
            Err(e) => {
                Err(crate::error::BridgeError::lakehouse(&format!(
                    "Failed to access Delta log directory: {}",
                    e
                )))
            }
        }
    }
}
