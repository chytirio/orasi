//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Monitoring and observability modules for telemetry ingestion
//!
//! This module provides comprehensive monitoring, metrics collection, and
//! observability features for the telemetry ingestion system.

use bridge_core::{BridgeResult, traits::MetricsCollector as BridgeMetricsCollector};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub mod metrics;

use metrics::{MetricsCollector as LocalMetricsCollector, MetricsConfig, AllMetrics, IngestionMetrics, ProtocolMetrics, PerformanceMetrics, ErrorMetrics};

/// Monitoring manager for the ingestion system
pub struct MonitoringManager {
    metrics_collector: Arc<dyn BridgeMetricsCollector>,
    local_metrics_collector: Arc<LocalMetricsCollector>,
    health_checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
    is_running: Arc<RwLock<bool>>,
}

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub last_check: chrono::DateTime<Utc>,
    pub details: Option<String>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub condition: AlertCondition,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub last_triggered: Option<chrono::DateTime<Utc>>,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub metric_name: String,
    pub operator: AlertOperator,
    pub threshold: f64,
    pub duration: Duration,
}

/// Alert operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertOperator {
    GreaterThan,
    LessThan,
    Equals,
    NotEquals,
}

impl MonitoringManager {
    /// Create new monitoring manager
    pub fn new(config: MetricsConfig) -> BridgeResult<Self> {
        let local_collector = LocalMetricsCollector::new(config)?;
        let metrics_collector = Arc::new(local_collector.clone()) as Arc<dyn BridgeMetricsCollector>;
        let local_metrics_collector = Arc::new(local_collector);
        let health_checks = Arc::new(RwLock::new(HashMap::new()));
        let alerts = Arc::new(RwLock::new(Vec::new()));
        let is_running = Arc::new(RwLock::new(false));

        Ok(Self {
            metrics_collector,
            local_metrics_collector,
            health_checks,
            alerts,
            is_running,
        })
    }

    /// Start monitoring
    pub async fn start(&self) -> BridgeResult<()> {
        info!("Starting monitoring manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        
        // Start metrics collection
        self.local_metrics_collector.start_collection().await?;
        
        info!("Monitoring manager started");
        Ok(())
    }

    /// Stop monitoring
    pub async fn stop(&self) -> BridgeResult<()> {
        info!("Stopping monitoring manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        // Stop metrics collection
        self.local_metrics_collector.stop_collection().await?;
        
        info!("Monitoring manager stopped");
        Ok(())
    }

    /// Record batch processed
    pub async fn record_batch_processed(&self, batch_size: usize, processing_time: Duration) {
        self.local_metrics_collector.record_batch_processed(batch_size, processing_time).await;
    }

    /// Record error
    pub async fn record_error(&self, error_type: &str) {
        self.local_metrics_collector.record_error(error_type).await;
    }

    /// Get all metrics
    pub async fn get_metrics(&self) -> AllMetrics {
        self.local_metrics_collector.get_all_metrics().await
    }

    /// Add health check
    pub async fn add_health_check(&self, name: String, health_check: HealthCheck) {
        let mut health_checks = self.health_checks.write().await;
        health_checks.insert(name, health_check);
    }

    /// Get health checks
    pub async fn get_health_checks(&self) -> HashMap<String, HealthCheck> {
        let health_checks = self.health_checks.read().await;
        health_checks.clone()
    }

    /// Add alert
    pub async fn add_alert(&self, alert: Alert) {
        let mut alerts = self.alerts.write().await;
        alerts.push(alert);
    }

    /// Get alerts
    pub async fn get_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.clone()
    }

    /// Check alerts
    pub async fn check_alerts(&self) -> BridgeResult<Vec<Alert>> {
        let metrics = self.get_metrics().await;
        let mut triggered_alerts = Vec::new();
        
        let mut alerts = self.alerts.write().await;
        for alert in alerts.iter_mut() {
            if self.evaluate_alert_condition(&alert.condition, &metrics).await? {
                alert.is_active = true;
                alert.last_triggered = Some(Utc::now());
                triggered_alerts.push(alert.clone());
                
                match alert.severity {
                    AlertSeverity::Info => info!("Alert triggered: {}", alert.name),
                    AlertSeverity::Warning => warn!("Alert triggered: {}", alert.name),
                    AlertSeverity::Error => error!("Alert triggered: {}", alert.name),
                    AlertSeverity::Critical => error!("CRITICAL ALERT: {}", alert.name),
                }
            } else {
                alert.is_active = false;
            }
        }
        
        Ok(triggered_alerts)
    }

    /// Evaluate alert condition
    async fn evaluate_alert_condition(&self, condition: &AlertCondition, metrics: &AllMetrics) -> BridgeResult<bool> {
        let value = match condition.metric_name.as_str() {
            "total_records" => metrics.ingestion.total_records as f64,
            "total_batches" => metrics.ingestion.total_batches as f64,
            "error_count" => metrics.ingestion.error_count as f64,
            "avg_processing_time_ms" => metrics.ingestion.avg_processing_time_ms,
            "total_errors" => metrics.error.total_errors as f64,
            "error_rate_percentage" => metrics.error.error_rate_percentage,
            _ => return Ok(false),
        };

        let result = match condition.operator {
            AlertOperator::GreaterThan => value > condition.threshold,
            AlertOperator::LessThan => value < condition.threshold,
            AlertOperator::Equals => (value - condition.threshold).abs() < f64::EPSILON,
            AlertOperator::NotEquals => (value - condition.threshold).abs() >= f64::EPSILON,
        };

        Ok(result)
    }
}
