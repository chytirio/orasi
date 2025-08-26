//! Metrics collection implementation

use crate::config::AgentConfig;
use crate::error::AgentError;
use super::types::{AgentMetrics, TaskMetrics, ResourceMetrics, PerformanceMetrics, ErrorMetrics, current_timestamp};
use super::alerts::AlertManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Metrics collector for agent operations
pub struct MetricsCollector {
    config: AgentConfig,
    last_collection: Arc<RwLock<Option<AgentMetrics>>>,
    alert_manager: Arc<AlertManager>,
    running: bool,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub async fn new(config: &AgentConfig) -> Result<Self, AgentError> {
        let alert_manager = Arc::new(AlertManager::new());

        Ok(Self {
            config: config.clone(),
            last_collection: Arc::new(RwLock::new(None)),
            alert_manager,
            running: false,
        })
    }

    /// Start metrics collector
    pub async fn start(&mut self) -> Result<(), AgentError> {
        if self.running {
            return Ok(());
        }

        info!("Starting metrics collector");
        self.running = true;

        // Start metrics collection loop
        let config = self.config.clone();
        let last_collection = self.last_collection.clone();
        let alert_manager = self.alert_manager.clone();

        tokio::spawn(async move {
            Self::metrics_collection_loop(config, last_collection, alert_manager).await;
        });

        info!("Metrics collector started successfully");
        Ok(())
    }

    /// Stop metrics collector
    pub async fn stop(&mut self) -> Result<(), AgentError> {
        if !self.running {
            return Ok(());
        }

        info!("Stopping metrics collector");
        self.running = false;

        info!("Metrics collector stopped");
        Ok(())
    }

    /// Collect metrics
    pub async fn collect_metrics(&self) -> Result<AgentMetrics, AgentError> {
        let start_time = Instant::now();

        // Collect different types of metrics
        let task_metrics = self.collect_task_metrics().await?;
        let resource_metrics = self.collect_resource_metrics().await?;
        let performance_metrics = self.collect_performance_metrics().await?;
        let error_metrics = self.collect_error_metrics().await?;

        let metrics = AgentMetrics {
            tasks: task_metrics,
            resources: resource_metrics,
            performance: performance_metrics,
            errors: error_metrics,
            timestamp: current_timestamp(),
        };

        // Update last collection
        {
            let mut last_collection = self.last_collection.write().await;
            *last_collection = Some(metrics.clone());
        }

        // Check alerts
        if let Err(e) = self.alert_manager.check_alerts(&metrics).await {
            warn!("Failed to check alerts: {}", e);
        }

        let collection_time = start_time.elapsed();
        info!("Metrics collection completed in {:?}", collection_time);

        Ok(metrics)
    }

    /// Collect task metrics
    async fn collect_task_metrics(&self) -> Result<TaskMetrics, AgentError> {
        // In a real implementation, this would collect actual task metrics
        // For now, we'll use static metrics
        Ok(Self::collect_task_metrics_static().await)
    }

    /// Collect resource metrics
    async fn collect_resource_metrics(&self) -> Result<ResourceMetrics, AgentError> {
        // In a real implementation, this would collect actual resource metrics
        // For now, we'll use static metrics
        Ok(Self::collect_resource_metrics_static().await)
    }

    /// Collect performance metrics
    async fn collect_performance_metrics(&self) -> Result<PerformanceMetrics, AgentError> {
        // In a real implementation, this would collect actual performance metrics
        // For now, we'll use static metrics
        Ok(Self::collect_performance_metrics_static().await)
    }

    /// Collect error metrics
    async fn collect_error_metrics(&self) -> Result<ErrorMetrics, AgentError> {
        // In a real implementation, this would collect actual error metrics
        // For now, we'll use static metrics
        Ok(Self::collect_error_metrics_static().await)
    }

    /// Metrics collection loop
    async fn metrics_collection_loop(
        config: AgentConfig,
        last_collection: Arc<RwLock<Option<AgentMetrics>>>,
        alert_manager: Arc<AlertManager>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(30)); // Default 30 seconds

        loop {
            interval.tick().await;

            // Create a temporary collector for collection
            let collector = match Self::new(&config).await {
                Ok(collector) => collector,
                Err(e) => {
                    error!("Failed to create metrics collector: {}", e);
                    continue;
                }
            };

            match collector.collect_metrics().await {
                Ok(metrics) => {
                    // Update last collection
                    {
                        let mut last = last_collection.write().await;
                        *last = Some(metrics.clone());
                    }

                    // Check alerts
                    if let Err(e) = alert_manager.check_alerts(&metrics).await {
                        warn!("Failed to check alerts: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to collect metrics: {}", e);
                }
            }
        }
    }

    /// Static task metrics collection
    async fn collect_task_metrics_static() -> TaskMetrics {
        use std::sync::atomic::{AtomicU64, Ordering};
        
        // Use static counters for demonstration
        static TOTAL_PROCESSED: AtomicU64 = AtomicU64::new(0);
        static SUCCESSFUL: AtomicU64 = AtomicU64::new(0);
        static FAILED: AtomicU64 = AtomicU64::new(0);
        static PENDING: AtomicU64 = AtomicU64::new(0);
        static ACTIVE: AtomicU64 = AtomicU64::new(0);
        
        let total_processed = TOTAL_PROCESSED.load(Ordering::Relaxed);
        let successful = SUCCESSFUL.load(Ordering::Relaxed);
        let failed = FAILED.load(Ordering::Relaxed);
        let pending = PENDING.load(Ordering::Relaxed);
        let active = ACTIVE.load(Ordering::Relaxed);
        
        // Calculate average processing time (mock data)
        let avg_processing_time_ms = if total_processed > 0 {
            150.0 // Mock average processing time
        } else {
            0.0
        };
        
        // Build task type breakdown
        let mut by_type = std::collections::HashMap::new();
        by_type.insert("ingestion".to_string(), total_processed / 3);
        by_type.insert("indexing".to_string(), total_processed / 3);
        by_type.insert("processing".to_string(), total_processed / 3);
        
        TaskMetrics {
            total_processed,
            successful,
            failed,
            pending,
            active,
            avg_processing_time_ms,
            by_type,
        }
    }

    /// Static resource metrics collection
    async fn collect_resource_metrics_static() -> ResourceMetrics {
        // In a real implementation, this would use sysinfo or similar
        // For now, we'll return mock data
        ResourceMetrics {
            cpu_percent: 25.5,
            memory_bytes: 1024 * 1024 * 512, // 512 MB
            memory_percent: 50.0,
            disk_bytes: 1024 * 1024 * 1024 * 10, // 10 GB
            disk_percent: 30.0,
            network_rx_bytes: 1024 * 1024 * 100, // 100 MB
            network_tx_bytes: 1024 * 1024 * 50,  // 50 MB
        }
    }

    /// Static performance metrics collection
    async fn collect_performance_metrics_static() -> PerformanceMetrics {
        // In a real implementation, this would calculate actual performance metrics
        // For now, we'll return mock data
        PerformanceMetrics {
            requests_per_second: 150.0,
            avg_response_time_ms: 45.0,
            p95_response_time_ms: 120.0,
            p99_response_time_ms: 250.0,
            throughput_bytes_per_sec: 1024.0 * 1024.0 * 10.0, // 10 MB/s
        }
    }

    /// Static error metrics collection
    async fn collect_error_metrics_static() -> ErrorMetrics {
        use std::sync::atomic::{AtomicU64, Ordering};
        
        // Use the same static error counters
        static TOTAL_ERRORS: AtomicU64 = AtomicU64::new(0);
        static NETWORK_ERRORS: AtomicU64 = AtomicU64::new(0);
        static PROCESSING_ERRORS: AtomicU64 = AtomicU64::new(0);
        static VALIDATION_ERRORS: AtomicU64 = AtomicU64::new(0);
        static TOTAL_REQUESTS: AtomicU64 = AtomicU64::new(0);
        
        let total_errors = TOTAL_ERRORS.load(Ordering::Relaxed);
        let network_errors = NETWORK_ERRORS.load(Ordering::Relaxed);
        let processing_errors = PROCESSING_ERRORS.load(Ordering::Relaxed);
        let validation_errors = VALIDATION_ERRORS.load(Ordering::Relaxed);
        let total_requests = TOTAL_REQUESTS.load(Ordering::Relaxed);
        
        // Calculate error rate
        let error_rate = if total_requests > 0 {
            total_errors as f64 / total_requests as f64
        } else {
            0.0
        };
        
        // Build error type breakdown
        let mut by_type = std::collections::HashMap::new();
        by_type.insert("network".to_string(), network_errors);
        by_type.insert("processing".to_string(), processing_errors);
        by_type.insert("validation".to_string(), validation_errors);
        
        ErrorMetrics {
            total_errors,
            by_type,
            error_rate,
        }
    }

    /// Get last collected metrics
    pub async fn get_last_metrics(&self) -> Option<AgentMetrics> {
        let last_collection = self.last_collection.read().await;
        last_collection.clone()
    }

    /// Get alert manager
    pub fn get_alert_manager(&self) -> Arc<AlertManager> {
        self.alert_manager.clone()
    }
}
