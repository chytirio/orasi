//! Prometheus metrics integration

use super::types::AgentMetrics;
use metrics::{counter, gauge, histogram};
use tracing::info;

/// Prometheus metrics manager
pub struct PrometheusMetrics {
    initialized: bool,
}

impl PrometheusMetrics {
    /// Create new Prometheus metrics manager
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    /// Initialize Prometheus metrics
    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }

        // Task metrics
        counter!("orasi_agent_tasks_completed_total", 0);
        counter!("orasi_agent_tasks_failed_total", 0);
        histogram!("orasi_agent_task_processing_time_ms", 0.0);

        // Resource metrics
        gauge!("orasi_agent_cpu_usage_percent", 0.0);
        gauge!("orasi_agent_memory_usage_percent", 0.0);
        gauge!("orasi_agent_disk_usage_percent", 0.0);

        // Error metrics
        counter!("orasi_agent_errors_total", 0);

        // Performance metrics
        gauge!("orasi_agent_requests_per_second", 0.0);
        gauge!("orasi_agent_avg_response_time_ms", 0.0);
        gauge!("orasi_agent_throughput_bytes_per_sec", 0.0);

        self.initialized = true;
        info!("Prometheus metrics initialized");
    }

    /// Update Prometheus metrics with agent metrics
    pub fn update_metrics(&self, metrics: &AgentMetrics) {
        if !self.initialized {
            return;
        }

        // Update resource metrics
        gauge!(
            "orasi_agent_cpu_usage_percent",
            metrics.resources.cpu_percent
        );
        gauge!(
            "orasi_agent_memory_usage_percent",
            metrics.resources.memory_percent
        );
        gauge!(
            "orasi_agent_disk_usage_percent",
            metrics.resources.disk_percent
        );

        // Update performance metrics
        gauge!(
            "orasi_agent_requests_per_second",
            metrics.performance.requests_per_second
        );
        gauge!(
            "orasi_agent_avg_response_time_ms",
            metrics.performance.avg_response_time_ms
        );
        gauge!(
            "orasi_agent_throughput_bytes_per_sec",
            metrics.performance.throughput_bytes_per_sec
        );

        // Update task metrics
        gauge!("orasi_agent_tasks_pending", metrics.tasks.pending as f64);
        gauge!("orasi_agent_tasks_active", metrics.tasks.active as f64);
        gauge!(
            "orasi_agent_tasks_avg_processing_time_ms",
            metrics.tasks.avg_processing_time_ms
        );

        // Update error metrics
        counter!("orasi_agent_errors_total", metrics.errors.total_errors);

        // Update task type metrics
        for (task_type, count) in &metrics.tasks.by_type {
            gauge!("orasi_agent_tasks_by_type", *count as f64, "type" => task_type.clone());
        }

        // Update error type metrics
        for (error_type, count) in &metrics.errors.by_type {
            counter!("orasi_agent_errors_by_type", *count, "type" => error_type.clone());
        }
    }

    /// Record task completion
    pub fn record_task_completion(&self, task_type: &str, processing_time_ms: f64) {
        if !self.initialized {
            return;
        }

        counter!("orasi_agent_tasks_completed_total", 1, "type" => task_type.to_string());
        histogram!("orasi_agent_task_processing_time_ms", processing_time_ms, "type" => task_type.to_string());
    }

    /// Record task failure
    pub fn record_task_failure(&self, task_type: &str, error_type: &str) {
        if !self.initialized {
            return;
        }

        counter!("orasi_agent_tasks_failed_total", 1, "type" => task_type.to_string(), "error" => error_type.to_string());
    }

    /// Record error
    pub fn record_error(&self, error_type: &str) {
        if !self.initialized {
            return;
        }

        counter!("orasi_agent_errors_total", 1, "type" => error_type.to_string());
    }

    /// Record request
    pub fn record_request(&self, response_time_ms: f64) {
        if !self.initialized {
            return;
        }

        histogram!("orasi_agent_request_duration_ms", response_time_ms);
    }

    /// Record resource usage
    pub fn record_resource_usage(&self, cpu_percent: f64, memory_percent: f64, disk_percent: f64) {
        if !self.initialized {
            return;
        }

        gauge!("orasi_agent_cpu_usage_percent", cpu_percent);
        gauge!("orasi_agent_memory_usage_percent", memory_percent);
        gauge!("orasi_agent_disk_usage_percent", disk_percent);
    }
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new()
    }
}
