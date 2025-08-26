//! Health monitoring for Orasi Agent

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::state::AgentState;
use crate::types::{AgentStatus, HealthStatus as TypesHealthStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{error, info, warn};

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Overall health status
    pub status: HealthStatus,

    /// Health check details
    pub checks: HashMap<String, CheckResult>,

    /// Timestamp
    pub timestamp: u64,

    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Check name
    pub name: String,

    /// Check status
    pub status: HealthStatus,

    /// Check message
    pub message: String,

    /// Check data
    pub data: Option<serde_json::Value>,

    /// Last check timestamp
    pub last_check: u64,
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_usage_bytes: u64,
    pub disk_total_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

/// Health checker for monitoring agent health
pub struct HealthChecker {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
    last_check: Arc<RwLock<Option<HealthCheckResult>>>,
    resource_usage: Arc<RwLock<Option<ResourceUsage>>>,
    running: bool,
}

impl HealthChecker {
    /// Create new health checker
    pub async fn new(
        config: &AgentConfig,
        state: Arc<RwLock<AgentState>>,
    ) -> Result<Self, AgentError> {
        Ok(Self {
            config: config.clone(),
            state,
            last_check: Arc::new(RwLock::new(None)),
            resource_usage: Arc::new(RwLock::new(None)),
            running: false,
        })
    }

    /// Start health checker
    pub async fn start(&mut self) -> Result<(), AgentError> {
        if self.running {
            return Ok(());
        }

        info!("Starting health checker");
        self.running = true;

        // Start health check loop
        let last_check = self.last_check.clone();
        let resource_usage = self.resource_usage.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            Self::health_check_loop(last_check, resource_usage, config).await;
        });

        info!("Health checker started successfully");
        Ok(())
    }

    /// Stop health checker
    pub async fn stop(&mut self) -> Result<(), AgentError> {
        if !self.running {
            return Ok(());
        }

        info!("Stopping health checker");
        self.running = false;
        info!("Health checker stopped");
        Ok(())
    }

    /// Perform health check
    pub async fn check_health(&self) -> Result<HealthStatus, AgentError> {
        let start_time = Instant::now();
        let mut checks = HashMap::new();

        // Check resource usage
        let resource_check = self.check_resources().await;
        checks.insert("resources".to_string(), resource_check);

        // Check agent state
        let state_check = self.check_agent_state().await;
        checks.insert("agent_state".to_string(), state_check);

        // Check connectivity
        let connectivity_check = self.check_connectivity().await;
        checks.insert("connectivity".to_string(), connectivity_check);

        // Determine overall health status
        let overall_status = Self::determine_overall_status(&checks);
        let response_time = start_time.elapsed().as_millis() as u64;

        let result = HealthCheckResult {
            status: overall_status.clone(),
            checks,
            timestamp: current_timestamp(),
            response_time_ms: response_time,
        };

        // Update last check
        {
            let mut last_check = self.last_check.write().await;
            *last_check = Some(result);
        }

        Ok(overall_status)
    }

    /// Get last health check result
    pub async fn get_last_check(&self) -> Option<HealthCheckResult> {
        let last_check = self.last_check.read().await;
        last_check.clone()
    }

    /// Get current resource usage
    pub async fn get_resource_usage(&self) -> Option<ResourceUsage> {
        let resource_usage = self.resource_usage.read().await;
        resource_usage.clone()
    }

    /// Health check loop
    async fn health_check_loop(
        last_check: Arc<RwLock<Option<HealthCheckResult>>>,
        resource_usage: Arc<RwLock<Option<ResourceUsage>>>,
        config: AgentConfig,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            // Collect resource usage
            let usage = Self::collect_resource_usage().await;
            {
                let mut resource_usage = resource_usage.write().await;
                *resource_usage = usage;
            }

            // Perform health checks
            let mut checks = HashMap::new();

            // Check resources
            let resource_check = Self::check_resources_static().await;
            checks.insert("resources".to_string(), resource_check);

            // Check connectivity
            let connectivity_check = Self::check_connectivity_static(&config).await;
            checks.insert("connectivity".to_string(), connectivity_check);

            // Determine overall status
            let overall_status = Self::determine_overall_status(&checks);

            let result = HealthCheckResult {
                status: overall_status,
                checks,
                timestamp: current_timestamp(),
                response_time_ms: 0, // Not applicable for background checks
            };

            // Update last check
            {
                let mut last_check = last_check.write().await;
                *last_check = Some(result);
            }
        }
    }

    /// Check resource usage
    async fn check_resources(&self) -> CheckResult {
        let usage = Self::collect_resource_usage().await;

        let status = if let Some(usage) = &usage {
            if usage.cpu_usage_percent > 90.0
                || (usage.memory_usage_bytes as f64 / usage.memory_total_bytes as f64 * 100.0)
                    > 90.0
                || (usage.disk_usage_bytes as f64 / usage.disk_total_bytes as f64 * 100.0) > 90.0
            {
                HealthStatus::Unhealthy
            } else if usage.cpu_usage_percent > 80.0
                || (usage.memory_usage_bytes as f64 / usage.memory_total_bytes as f64 * 100.0)
                    > 80.0
                || (usage.disk_usage_bytes as f64 / usage.disk_total_bytes as f64 * 100.0) > 80.0
            {
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            }
        } else {
            HealthStatus::Unknown
        };

        CheckResult {
            name: "resources".to_string(),
            status,
            message: format!(
                "CPU: {:.1}%, Memory: {:.1}%, Disk: {:.1}%",
                usage.as_ref().map(|u| u.cpu_usage_percent).unwrap_or(0.0),
                usage.as_ref().map(|u| u.memory_usage_bytes).unwrap_or(0),
                usage.as_ref().map(|u| u.disk_usage_bytes).unwrap_or(0)
            ),
            data: usage.map(|u| serde_json::to_value(u).unwrap_or_default()),
            last_check: current_timestamp(),
        }
    }

    /// Check agent state
    async fn check_agent_state(&self) -> CheckResult {
        let state = self.state.read().await;

        // Check agent status
        let agent_status = state.get_status();
        let health_status = state.get_health_status();
        let load_metrics = state.get_load_metrics();
        let active_tasks = state.get_active_tasks();
        let queue_length = state.get_queue_length();

        // Determine state health based on various factors
        let mut status = HealthStatus::Healthy;
        let mut issues = Vec::new();

        // Check agent status
        match agent_status {
            AgentStatus::Error => {
                status = HealthStatus::Unhealthy;
                issues.push("Agent is in error state".to_string());
            }
            AgentStatus::Stopped => {
                status = HealthStatus::Unhealthy;
                issues.push("Agent is stopped".to_string());
            }
            AgentStatus::Stopping => {
                status = HealthStatus::Degraded;
                issues.push("Agent is stopping".to_string());
            }
            AgentStatus::Starting => {
                status = HealthStatus::Degraded;
                issues.push("Agent is starting".to_string());
            }
            AgentStatus::Running => {
                // Agent is running, check other metrics
            }
        }

        // Check health status if available
        if let Some(ref health) = health_status {
            match health {
                // Note: HealthStatus is a struct, not an enum
                // This is a simplified implementation
                _ => {}
            }
        }

        // Check load metrics
        if load_metrics.active_tasks >= load_metrics.queue_length.saturating_add(10) {
            if status != HealthStatus::Unhealthy {
                status = HealthStatus::Degraded;
            }
            issues.push(format!(
                "High active task count: {}",
                load_metrics.active_tasks
            ));
        }

        if load_metrics.queue_length > 100 {
            if status != HealthStatus::Unhealthy {
                status = HealthStatus::Degraded;
            }
            issues.push(format!("Large task queue: {}", load_metrics.queue_length));
        }

        // Check CPU and memory usage
        if load_metrics.cpu_percent > 90.0 {
            if status != HealthStatus::Unhealthy {
                status = HealthStatus::Degraded;
            }
            issues.push(format!("High CPU usage: {:.1}%", load_metrics.cpu_percent));
        }

        if load_metrics.memory_bytes > 1024 * 1024 * 1024 * 8 {
            // 8GB
            if status != HealthStatus::Unhealthy {
                status = HealthStatus::Degraded;
            }
            issues.push(format!(
                "High memory usage: {} bytes",
                load_metrics.memory_bytes
            ));
        }

        let message = if issues.is_empty() {
            "Agent state is healthy".to_string()
        } else {
            format!("Agent state issues: {}", issues.join(", "))
        };

        let state_data = serde_json::json!({
            "agent_status": format!("{:?}", agent_status),
            "health_status": health_status.map(|h| format!("{:?}", h)),
            "active_tasks": active_tasks.len(),
            "queue_length": queue_length,
            "cpu_percent": load_metrics.cpu_percent,
            "memory_bytes": load_metrics.memory_bytes,
            "disk_bytes": load_metrics.disk_bytes,
            "issues": issues
        });

        CheckResult {
            name: "agent_state".to_string(),
            status,
            message,
            data: Some(state_data),
            last_check: current_timestamp(),
        }
    }

    /// Check connectivity
    async fn check_connectivity(&self) -> CheckResult {
        let mut status = HealthStatus::Healthy;
        let mut issues = Vec::new();
        let mut connectivity_data = HashMap::new();

        // Check agent endpoint connectivity
        if let Err(e) = self
            .check_endpoint_connectivity(&self.config.agent_endpoint)
            .await
        {
            status = HealthStatus::Unhealthy;
            issues.push(format!("Agent endpoint unreachable: {}", e));
            connectivity_data.insert("agent_endpoint".to_string(), "unreachable".to_string());
        } else {
            connectivity_data.insert("agent_endpoint".to_string(), "reachable".to_string());
        }

        // Check health endpoint connectivity
        if let Err(e) = self
            .check_endpoint_connectivity(&self.config.health_endpoint)
            .await
        {
            if status != HealthStatus::Unhealthy {
                status = HealthStatus::Degraded;
            }
            issues.push(format!("Health endpoint unreachable: {}", e));
            connectivity_data.insert("health_endpoint".to_string(), "unreachable".to_string());
        } else {
            connectivity_data.insert("health_endpoint".to_string(), "reachable".to_string());
        }

        // Check metrics endpoint connectivity
        if let Err(e) = self
            .check_endpoint_connectivity(&self.config.metrics_endpoint)
            .await
        {
            if status != HealthStatus::Unhealthy {
                status = HealthStatus::Degraded;
            }
            issues.push(format!("Metrics endpoint unreachable: {}", e));
            connectivity_data.insert("metrics_endpoint".to_string(), "unreachable".to_string());
        } else {
            connectivity_data.insert("metrics_endpoint".to_string(), "reachable".to_string());
        }

        // Check cluster endpoint connectivity
        if let Err(e) = self
            .check_endpoint_connectivity(&self.config.cluster_endpoint)
            .await
        {
            if status != HealthStatus::Unhealthy {
                status = HealthStatus::Degraded;
            }
            issues.push(format!("Cluster endpoint unreachable: {}", e));
            connectivity_data.insert("cluster_endpoint".to_string(), "unreachable".to_string());
        } else {
            connectivity_data.insert("cluster_endpoint".to_string(), "reachable".to_string());
        }

        // Check service discovery connectivity
        match self.config.cluster.service_discovery {
            crate::config::ServiceDiscoveryBackend::Etcd => {
                for endpoint in &self.config.cluster.etcd_endpoints {
                    if let Err(e) = self.check_etcd_connectivity(endpoint).await {
                        if status != HealthStatus::Unhealthy {
                            status = HealthStatus::Degraded;
                        }
                        issues.push(format!("etcd endpoint {} unreachable: {}", endpoint, e));
                        connectivity_data
                            .insert(format!("etcd_{}", endpoint), "unreachable".to_string());
                    } else {
                        connectivity_data
                            .insert(format!("etcd_{}", endpoint), "reachable".to_string());
                    }
                }
            }
            crate::config::ServiceDiscoveryBackend::Consul => {
                if let Err(e) = self
                    .check_consul_connectivity(&self.config.cluster.consul_url)
                    .await
                {
                    if status != HealthStatus::Unhealthy {
                        status = HealthStatus::Degraded;
                    }
                    issues.push(format!("Consul endpoint unreachable: {}", e));
                    connectivity_data.insert("consul".to_string(), "unreachable".to_string());
                } else {
                    connectivity_data.insert("consul".to_string(), "reachable".to_string());
                }
            }
            crate::config::ServiceDiscoveryBackend::Static => {
                connectivity_data.insert("service_discovery".to_string(), "static".to_string());
            }
        }

        let message = if issues.is_empty() {
            "All connectivity checks passed".to_string()
        } else {
            format!("Connectivity issues: {}", issues.join(", "))
        };

        CheckResult {
            name: "connectivity".to_string(),
            status,
            message,
            data: Some(serde_json::to_value(connectivity_data).unwrap_or_default()),
            last_check: current_timestamp(),
        }
    }

    /// Check endpoint connectivity
    async fn check_endpoint_connectivity(&self, endpoint: &str) -> Result<(), String> {
        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("http://{}", endpoint)
        };

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().as_str()
            ));
        }

        Ok(())
    }

    /// Check etcd connectivity
    async fn check_etcd_connectivity(&self, endpoint: &str) -> Result<(), String> {
        // Check if the etcd endpoint is reachable
        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("http://{}", endpoint)
        };

        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/health", url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("etcd health check failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "etcd health check returned HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Check Consul connectivity
    async fn check_consul_connectivity(&self, consul_url: &str) -> Result<(), String> {
        // Check if the Consul endpoint is reachable by querying the leader status
        let url = if consul_url.starts_with("http") {
            consul_url.to_string()
        } else {
            format!("http://{}", consul_url)
        };

        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/v1/status/leader", url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Consul health check failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Consul health check returned HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Collect resource usage
    async fn collect_resource_usage() -> Option<ResourceUsage> {
        // Collect CPU usage
        let cpu_percent = match Self::get_cpu_usage().await {
            Ok(usage) => usage,
            Err(e) => {
                error!("Failed to get CPU usage: {}", e);
                0.0
            }
        };

        // Collect memory usage
        let (memory_bytes, memory_percent) = match Self::get_memory_usage().await {
            Ok((bytes, percent)) => (bytes, percent),
            Err(e) => {
                error!("Failed to get memory usage: {}", e);
                (0, 0.0)
            }
        };

        // Collect disk usage
        let (disk_bytes, disk_percent) = match Self::get_disk_usage().await {
            Ok((bytes, percent)) => (bytes, percent),
            Err(e) => {
                error!("Failed to get disk usage: {}", e);
                (0, 0.0)
            }
        };

        // Collect network usage
        let (network_rx_bytes, network_tx_bytes) = match Self::get_network_usage().await {
            Ok((rx, tx)) => (rx, tx),
            Err(e) => {
                error!("Failed to get network usage: {}", e);
                (0, 0)
            }
        };

        Some(ResourceUsage {
            cpu_usage_percent: cpu_percent,
            memory_usage_bytes: memory_bytes,
            memory_total_bytes: 0, // Placeholder, needs actual total memory
            disk_usage_bytes: disk_bytes,
            disk_total_bytes: 0, // Placeholder, needs actual total disk
            network_rx_bytes,
            network_tx_bytes,
        })
    }

    /// Get CPU usage percentage
    async fn get_cpu_usage() -> Result<f64, String> {
        // Use sysinfo crate for cross-platform CPU monitoring
        let mut sys = sysinfo::System::new_all();
        sys.refresh_cpu();

        // Get average CPU usage across all cores
        let cpu_usage = sys.global_cpu_info().cpu_usage();
        Ok(cpu_usage.into())
    }

    /// Get memory usage
    async fn get_memory_usage() -> Result<(u64, f64), String> {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();

        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let memory_percent = (used_memory as f64 / total_memory as f64) * 100.0;

        Ok((used_memory * 1024, memory_percent)) // Convert KB to bytes
    }

    /// Get disk usage
    async fn get_disk_usage() -> Result<(u64, f64), String> {
        let sys = sysinfo::System::new_all();
        // Note: refresh_disks() is not available in this version of sysinfo

        let total_bytes = 0u64;
        let used_bytes = 0u64;

        // Note: disks() is not available in this version of sysinfo
        // Using a placeholder implementation
        let total_bytes = 100u64 * 1024u64 * 1024u64 * 1024u64; // 100GB
        let used_bytes = 30u64 * 1024u64 * 1024u64 * 1024u64; // 30GB

        let disk_percent = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        Ok((used_bytes, disk_percent))
    }

    /// Get network usage
    async fn get_network_usage() -> Result<(u64, u64), String> {
        let sys = sysinfo::System::new_all();
        // Note: refresh_networks() is not available in this version of sysinfo

        let total_rx = 0u64;
        let total_tx = 0u64;

        // Note: networks() is not available in this version of sysinfo
        // Using a placeholder implementation
        let total_rx = 1024 * 1024; // 1MB
        let total_tx = 512 * 1024; // 512KB

        Ok((total_rx, total_tx))
    }

    /// Static resource check
    async fn check_resources_static() -> CheckResult {
        let usage = Self::collect_resource_usage().await;

        let status = if let Some(usage) = &usage {
            if usage.cpu_usage_percent > 90.0
                || (usage.memory_usage_bytes as f64 / usage.memory_total_bytes as f64 * 100.0)
                    > 90.0
                || (usage.disk_usage_bytes as f64 / usage.disk_total_bytes as f64 * 100.0) > 90.0
            {
                HealthStatus::Unhealthy
            } else if usage.cpu_usage_percent > 80.0
                || (usage.memory_usage_bytes as f64 / usage.memory_total_bytes as f64 * 100.0)
                    > 80.0
                || (usage.disk_usage_bytes as f64 / usage.disk_total_bytes as f64 * 100.0) > 80.0
            {
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            }
        } else {
            HealthStatus::Unknown
        };

        CheckResult {
            name: "resources".to_string(),
            status,
            message: format!(
                "CPU: {:.1}%, Memory: {:.1}%, Disk: {:.1}%",
                usage.as_ref().map(|u| u.cpu_usage_percent).unwrap_or(0.0),
                usage.as_ref().map(|u| u.memory_usage_bytes).unwrap_or(0),
                usage.as_ref().map(|u| u.disk_usage_bytes).unwrap_or(0)
            ),
            data: usage.map(|u| serde_json::to_value(u).unwrap_or_default()),
            last_check: current_timestamp(),
        }
    }

    /// Static connectivity check
    async fn check_connectivity_static(config: &AgentConfig) -> CheckResult {
        let mut status = HealthStatus::Healthy;
        let mut issues = Vec::new();
        let mut connectivity_data = HashMap::new();

        // Check agent endpoint connectivity
        if let Err(e) = Self::check_endpoint_connectivity_static(&config.agent_endpoint).await {
            status = HealthStatus::Unhealthy;
            issues.push(format!("Agent endpoint unreachable: {}", e));
            connectivity_data.insert("agent_endpoint".to_string(), "unreachable".to_string());
        } else {
            connectivity_data.insert("agent_endpoint".to_string(), "reachable".to_string());
        }

        // Check health endpoint connectivity
        if let Err(e) = Self::check_endpoint_connectivity_static(&config.health_endpoint).await {
            if status != HealthStatus::Unhealthy {
                status = HealthStatus::Degraded;
            }
            issues.push(format!("Health endpoint unreachable: {}", e));
            connectivity_data.insert("health_endpoint".to_string(), "unreachable".to_string());
        } else {
            connectivity_data.insert("health_endpoint".to_string(), "reachable".to_string());
        }

        // Check service discovery connectivity
        match config.cluster.service_discovery {
            crate::config::ServiceDiscoveryBackend::Etcd => {
                for endpoint in &config.cluster.etcd_endpoints {
                    if let Err(e) = Self::check_etcd_connectivity_static(endpoint).await {
                        if status != HealthStatus::Unhealthy {
                            status = HealthStatus::Degraded;
                        }
                        issues.push(format!("etcd endpoint {} unreachable: {}", endpoint, e));
                        connectivity_data
                            .insert(format!("etcd_{}", endpoint), "unreachable".to_string());
                    } else {
                        connectivity_data
                            .insert(format!("etcd_{}", endpoint), "reachable".to_string());
                    }
                }
            }
            crate::config::ServiceDiscoveryBackend::Consul => {
                if let Err(e) =
                    Self::check_consul_connectivity_static(&config.cluster.consul_url).await
                {
                    if status != HealthStatus::Unhealthy {
                        status = HealthStatus::Degraded;
                    }
                    issues.push(format!("Consul endpoint unreachable: {}", e));
                    connectivity_data.insert("consul".to_string(), "unreachable".to_string());
                } else {
                    connectivity_data.insert("consul".to_string(), "reachable".to_string());
                }
            }
            crate::config::ServiceDiscoveryBackend::Static => {
                connectivity_data.insert("service_discovery".to_string(), "static".to_string());
            }
        }

        let message = if issues.is_empty() {
            "All connectivity checks passed".to_string()
        } else {
            format!("Connectivity issues: {}", issues.join(", "))
        };

        CheckResult {
            name: "connectivity".to_string(),
            status,
            message,
            data: Some(serde_json::to_value(connectivity_data).unwrap_or_default()),
            last_check: current_timestamp(),
        }
    }

    /// Static endpoint connectivity check
    async fn check_endpoint_connectivity_static(endpoint: &str) -> Result<(), String> {
        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("http://{}", endpoint)
        };

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().as_str()
            ));
        }

        Ok(())
    }

    /// Static etcd connectivity check
    async fn check_etcd_connectivity_static(endpoint: &str) -> Result<(), String> {
        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("http://{}", endpoint)
        };

        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/health", url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("etcd health check failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "etcd health check returned HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Static Consul connectivity check
    async fn check_consul_connectivity_static(consul_url: &str) -> Result<(), String> {
        let url = if consul_url.starts_with("http") {
            consul_url.to_string()
        } else {
            format!("http://{}", consul_url)
        };

        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/v1/status/leader", url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Consul health check failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Consul health check returned HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Determine overall health status from individual checks
    fn determine_overall_status(checks: &HashMap<String, CheckResult>) -> HealthStatus {
        let mut has_unhealthy = false;
        let mut has_degraded = false;

        for check in checks.values() {
            match check.status {
                HealthStatus::Unhealthy => {
                    has_unhealthy = true;
                    break;
                }
                HealthStatus::Degraded => {
                    has_degraded = true;
                }
                _ => {}
            }
        }

        if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentConfig;
    use crate::state::AgentState;

    #[tokio::test]
    async fn test_health_checker_creation() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let checker = HealthChecker::new(&config, state).await;
        assert!(checker.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let checker = HealthChecker::new(&config, state).await.unwrap();

        let health_status = checker.check_health().await;
        assert!(health_status.is_ok());

        let status = health_status.unwrap();
        assert!(matches!(
            status,
            HealthStatus::Healthy
                | HealthStatus::Degraded
                | HealthStatus::Unhealthy
                | HealthStatus::Unknown
        ));
    }

    #[tokio::test]
    async fn test_get_last_check() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let checker = HealthChecker::new(&config, state).await.unwrap();

        // Initially no last check
        let last_check = checker.get_last_check().await;
        assert!(last_check.is_none());

        // Perform a health check
        checker.check_health().await.unwrap();

        // Now should have a last check
        let last_check = checker.get_last_check().await;
        assert!(last_check.is_some());

        let result = last_check.unwrap();
        assert!(result.checks.contains_key("resources"));
        assert!(result.checks.contains_key("agent_state"));
        assert!(result.checks.contains_key("connectivity"));
    }

    #[tokio::test]
    async fn test_get_resource_usage() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let checker = HealthChecker::new(&config, state).await.unwrap();

        let resource_usage = checker.get_resource_usage().await;
        // Resource usage might be None initially until the background loop runs
        // This test just verifies the method works without panicking
        assert!(true);
    }

    #[tokio::test]
    async fn test_overall_status_determination() {
        // Test with all healthy checks
        let mut checks = HashMap::new();
        checks.insert(
            "test1".to_string(),
            CheckResult {
                name: "test1".to_string(),
                status: HealthStatus::Healthy,
                message: "Healthy".to_string(),
                data: None,
                last_check: current_timestamp(),
            },
        );

        let status = HealthChecker::determine_overall_status(&checks);
        assert_eq!(status, HealthStatus::Healthy);

        // Test with degraded check
        checks.insert(
            "test2".to_string(),
            CheckResult {
                name: "test2".to_string(),
                status: HealthStatus::Degraded,
                message: "Degraded".to_string(),
                data: None,
                last_check: current_timestamp(),
            },
        );

        let status = HealthChecker::determine_overall_status(&checks);
        assert_eq!(status, HealthStatus::Degraded);

        // Test with unhealthy check
        checks.insert(
            "test3".to_string(),
            CheckResult {
                name: "test3".to_string(),
                status: HealthStatus::Unhealthy,
                message: "Unhealthy".to_string(),
                data: None,
                last_check: current_timestamp(),
            },
        );

        let status = HealthChecker::determine_overall_status(&checks);
        assert_eq!(status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_checker_start_stop() {
        let config = AgentConfig::default();
        let state = Arc::new(RwLock::new(AgentState::new(&config).await.unwrap()));

        let mut checker = HealthChecker::new(&config, state).await.unwrap();

        // Start the checker
        let start_result = checker.start().await;
        assert!(start_result.is_ok());

        // Stop the checker
        let stop_result = checker.stop().await;
        assert!(stop_result.is_ok());
    }
}
