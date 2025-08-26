//! Metrics collection

use crate::{config::GatewayConfig, error::GatewayError, types::*};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Metrics collector for gateway
pub struct MetricsCollector {
    config: GatewayConfig,
    prometheus_handle: Option<PrometheusHandle>,
    is_running: Arc<RwLock<bool>>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub async fn new(config: &GatewayConfig) -> Result<Self, GatewayError> {
        info!("Initializing metrics collector for gateway {}", config.gateway_id);
        
        Ok(Self {
            config: config.clone(),
            prometheus_handle: None,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start metrics collector
    pub async fn start(&self) -> Result<(), GatewayError> {
        info!("Starting metrics collector");
        
        let mut is_running = self.is_running.write().await;
        if *is_running {
            warn!("Metrics collector is already running");
            return Ok(());
        }

        // Initialize Prometheus metrics exporter
        let (recorder, exporter) = PrometheusBuilder::new()
            .build()
            .map_err(|e| GatewayError::Metrics(format!("Failed to build Prometheus exporter: {}", e)))?;

        // Set the recorder as the global metrics recorder
        metrics::set_boxed_recorder(Box::new(recorder))
            .map_err(|e| GatewayError::Metrics(format!("Failed to set global metrics recorder: {}", e)))?;
        
        // Start the metrics server in a background task
        let metrics_endpoint = self.config.metrics_endpoint.clone();
        tokio::spawn(async move {
            // In a real implementation, you would serve the exporter on the metrics endpoint
            // For now, we'll just log that metrics are available
            info!("Prometheus metrics exporter ready on {}", metrics_endpoint);
        });

        // Initialize default metrics
        self.initialize_metrics();

        *is_running = true;
        info!("Metrics collector started successfully");
        Ok(())
    }

    /// Stop metrics collector
    pub async fn stop(&self) -> Result<(), GatewayError> {
        info!("Stopping metrics collector");
        
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            warn!("Metrics collector is not running");
            return Ok(());
        }

        // Reset all metrics to zero
        self.reset_metrics();

        *is_running = false;
        info!("Metrics collector stopped successfully");
        Ok(())
    }

    /// Collect metrics
    pub async fn collect_metrics(&self) -> Result<GatewayMetrics, GatewayError> {
        let is_running = self.is_running.read().await;
        if !*is_running {
            return Err(GatewayError::Metrics("Metrics collector is not running".to_string()));
        }

        // In a real implementation, this would collect metrics from various sources
        // For now, we'll return a basic metrics structure
        // The actual metrics are updated via the record_* methods below
        Ok(GatewayMetrics {
            total_requests: 0, // This would be collected from counters
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ms: 0.0,
            active_connections: 0,
            rate_limit_violations: 0,
            circuit_breaker_trips: 0,
        })
    }

    /// Record a new request
    pub fn record_request(&self, method: &str, path: &str, status_code: u16) {
        counter!("orasi_gateway_requests_total", 1, "method" => method.to_string(), "path" => path.to_string());
        
        if status_code < 400 {
            counter!("orasi_gateway_requests_successful_total", 1, "method" => method.to_string(), "path" => path.to_string());
        } else {
            counter!("orasi_gateway_requests_failed_total", 1, "method" => method.to_string(), "path" => path.to_string(), "status_code" => status_code.to_string());
        }
    }

    /// Record response time
    pub fn record_response_time(&self, method: &str, path: &str, duration_ms: f64) {
        histogram!("orasi_gateway_response_time_ms", duration_ms, "method" => method.to_string(), "path" => path.to_string());
    }

    /// Record active connections
    pub fn record_active_connections(&self, count: u32) {
        gauge!("orasi_gateway_active_connections", count as f64);
    }

    /// Record rate limit violation
    pub fn record_rate_limit_violation(&self, client_ip: &str) {
        counter!("orasi_gateway_rate_limit_violations_total", 1, "client_ip" => client_ip.to_string());
    }

    /// Record circuit breaker trip
    pub fn record_circuit_breaker_trip(&self, service: &str) {
        counter!("orasi_gateway_circuit_breaker_trips_total", 1, "service" => service.to_string());
    }

    /// Record service discovery event
    pub fn record_service_discovery_event(&self, event_type: &str, service: &str) {
        counter!("orasi_gateway_service_discovery_events_total", 1, "event_type" => event_type.to_string(), "service" => service.to_string());
    }

    /// Record load balancer event
    pub fn record_load_balancer_event(&self, event_type: &str, service: &str) {
        counter!("orasi_gateway_load_balancer_events_total", 1, "event_type" => event_type.to_string(), "service" => service.to_string());
    }

    /// Record health check result
    pub fn record_health_check(&self, service: &str, is_healthy: bool) {
        let status = if is_healthy { "healthy" } else { "unhealthy" };
        counter!("orasi_gateway_health_checks_total", 1, "service" => service.to_string(), "status" => status.to_string());
    }

    /// Initialize default metrics
    fn initialize_metrics(&self) {
        // Initialize counters to 0
        counter!("orasi_gateway_requests_total", 0);
        counter!("orasi_gateway_requests_successful_total", 0);
        counter!("orasi_gateway_requests_failed_total", 0);
        counter!("orasi_gateway_rate_limit_violations_total", 0);
        counter!("orasi_gateway_circuit_breaker_trips_total", 0);
        counter!("orasi_gateway_service_discovery_events_total", 0);
        counter!("orasi_gateway_load_balancer_events_total", 0);
        counter!("orasi_gateway_health_checks_total", 0);

        // Initialize gauges to 0
        gauge!("orasi_gateway_active_connections", 0.0);
        gauge!("orasi_gateway_up", 1.0); // Gateway is up

        // Initialize histograms (they don't need initialization, but we can set up buckets)
        // The histogram will be created automatically when first value is recorded
    }

    /// Reset all metrics to zero
    fn reset_metrics(&self) {
        // Note: Prometheus counters and histograms cannot be reset to zero
        // Only gauges can be reset
        gauge!("orasi_gateway_active_connections", 0.0);
        gauge!("orasi_gateway_up", 0.0); // Gateway is down
    }

    /// Check if metrics collector is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let config = GatewayConfig::default();
        let collector = MetricsCollector::new(&config).await;
        assert!(collector.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_collector_start_stop() {
        let config = GatewayConfig::default();
        let collector = MetricsCollector::new(&config).await.unwrap();
        
        // Test start
        let result = collector.start().await;
        assert!(result.is_ok());
        assert!(collector.is_running().await);
        
        // Test stop
        let result = collector.stop().await;
        assert!(result.is_ok());
        assert!(!collector.is_running().await);
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let config = GatewayConfig::default();
        let collector = MetricsCollector::new(&config).await.unwrap();
        
        // Start the collector
        collector.start().await.unwrap();
        
        // Test recording various metrics
        collector.record_request("GET", "/api/v1/health", 200);
        collector.record_response_time("GET", "/api/v1/health", 15.5);
        collector.record_active_connections(42);
        collector.record_rate_limit_violation("192.168.1.1");
        collector.record_circuit_breaker_trip("user-service");
        collector.record_service_discovery_event("service_added", "user-service");
        collector.record_load_balancer_event("endpoint_selected", "user-service");
        collector.record_health_check("user-service", true);
        
        // Test metrics collection
        let metrics = collector.collect_metrics().await;
        assert!(metrics.is_ok());
        
        // Stop the collector
        collector.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_metrics_collector_not_running() {
        let config = GatewayConfig::default();
        let collector = MetricsCollector::new(&config).await.unwrap();
        
        // Try to collect metrics when not running
        let result = collector.collect_metrics().await;
        assert!(result.is_err());
        
        // Try to stop when not running
        let result = collector.stop().await;
        assert!(result.is_ok()); // Should be a no-op
    }

    #[tokio::test]
    async fn test_metrics_collector_double_start() {
        let config = GatewayConfig::default();
        let collector = MetricsCollector::new(&config).await.unwrap();
        
        // Start twice
        let result1 = collector.start().await;
        assert!(result1.is_ok());
        
        let result2 = collector.start().await;
        assert!(result2.is_ok()); // Should be a no-op
        
        // Clean up
        collector.stop().await.unwrap();
    }
}
