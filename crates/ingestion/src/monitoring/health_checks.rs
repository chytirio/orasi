//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Health check system for the ingestion platform
//!
//! This module provides comprehensive health checking capabilities including
//! component health checks, readiness probes, liveness probes, and dependency checks.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    pub enabled: bool,

    /// Health check interval in seconds
    pub check_interval_secs: u64,

    /// Health check timeout in seconds
    pub timeout_secs: u64,

    /// Maximum number of consecutive failures
    pub max_failures: u32,

    /// Enable readiness probe
    pub enable_readiness: bool,

    /// Enable liveness probe
    pub enable_liveness: bool,

    /// Enable startup probe
    pub enable_startup: bool,

    /// Health check endpoints
    pub endpoints: Vec<HealthCheckEndpoint>,

    /// Custom health checks
    pub custom_checks: Vec<CustomHealthCheck>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Health check endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckEndpoint {
    /// Endpoint name
    pub name: String,

    /// Endpoint URL
    pub url: String,

    /// Expected status code
    pub expected_status: u16,

    /// Timeout in seconds
    pub timeout_secs: u64,

    /// Authentication (optional)
    pub auth: Option<EndpointAuth>,

    /// Headers (optional)
    pub headers: HashMap<String, String>,

    /// Whether endpoint is required
    pub required: bool,
}

/// Endpoint authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointAuth {
    /// Authentication type
    pub auth_type: AuthType,

    /// Username (for basic auth)
    pub username: Option<String>,

    /// Password (for basic auth)
    pub password: Option<String>,

    /// Token (for bearer auth)
    pub token: Option<String>,
}

/// Authentication type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    None,
    Basic,
    Bearer,
}

/// Custom health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomHealthCheck {
    /// Check name
    pub name: String,

    /// Check description
    pub description: String,

    /// Check type
    pub check_type: CustomCheckType,

    /// Check parameters
    pub parameters: HashMap<String, String>,

    /// Whether check is required
    pub required: bool,

    /// Check interval in seconds
    pub interval_secs: u64,
}

/// Custom check type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CustomCheckType {
    /// Database connectivity check
    Database,

    /// File system check
    FileSystem,

    /// Memory usage check
    MemoryUsage,

    /// CPU usage check
    CpuUsage,

    /// Disk usage check
    DiskUsage,

    /// Network connectivity check
    Network,

    /// Custom script check
    Script,

    /// Custom function check
    Function,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Check name
    pub name: String,

    /// Check status
    pub status: HealthStatus,

    /// Check timestamp
    pub timestamp: DateTime<Utc>,

    /// Response time in milliseconds
    pub response_time_ms: u64,

    /// Error message (if failed)
    pub error_message: Option<String>,

    /// Additional details
    pub details: HashMap<String, String>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,

    /// Component is unhealthy
    Unhealthy,

    /// Component is degraded
    Degraded,

    /// Component is starting up
    Starting,

    /// Component is shutting down
    ShuttingDown,
}

/// Health check trait
#[async_trait]
pub trait HealthChecker: Send + Sync {
    /// Get health check name
    fn name(&self) -> &str;

    /// Perform health check
    async fn check(&self) -> HealthCheckResult;

    /// Get health check description
    fn description(&self) -> &str {
        "Health check"
    }

    /// Check if health check is required
    fn is_required(&self) -> bool {
        true
    }
}

/// Health check manager
pub struct HealthCheckManager {
    config: HealthCheckConfig,
    health_checks: Arc<RwLock<HashMap<String, Box<dyn HealthChecker>>>>,
    results: Arc<RwLock<HashMap<String, HealthCheckResult>>>,
    is_running: Arc<RwLock<bool>>,
    failure_counts: Arc<RwLock<HashMap<String, u32>>>,
}

impl HealthCheckConfig {
    /// Create new health check configuration
    pub fn new() -> Self {
        Self {
            enabled: true,
            check_interval_secs: 30,
            timeout_secs: 10,
            max_failures: 3,
            enable_readiness: true,
            enable_liveness: true,
            enable_startup: true,
            endpoints: Vec::new(),
            custom_checks: Vec::new(),
            additional_config: HashMap::new(),
        }
    }

    /// Create configuration with custom interval
    pub fn with_interval(check_interval_secs: u64, timeout_secs: u64) -> Self {
        let mut config = Self::new();
        config.check_interval_secs = check_interval_secs;
        config.timeout_secs = timeout_secs;
        config
    }
}

impl HealthCheckManager {
    /// Create new health check manager
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            failure_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add health check
    pub async fn add_health_check(&self, health_check: Box<dyn HealthChecker>) -> BridgeResult<()> {
        let name = health_check.name().to_string();
        let mut health_checks = self.health_checks.write().await;
        health_checks.insert(name, health_check);
        Ok(())
    }

    /// Remove health check
    pub async fn remove_health_check(&self, name: &str) -> BridgeResult<bool> {
        let mut health_checks = self.health_checks.write().await;
        let removed = health_checks.remove(name).is_some();

        if removed {
            let mut results = self.results.write().await;
            results.remove(name);

            let mut failure_counts = self.failure_counts.write().await;
            failure_counts.remove(name);
        }

        Ok(removed)
    }

    /// Start health check monitoring
    pub async fn start(&self) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }

        *is_running = true;
        drop(is_running);

        let manager = self.clone();
        tokio::spawn(async move {
            manager.run_health_checks().await;
        });

        info!("Health check manager started");
        Ok(())
    }

    /// Stop health check monitoring
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("Health check manager stopped");
        Ok(())
    }

    /// Run health checks
    async fn run_health_checks(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            self.config.check_interval_secs,
        ));

        while *self.is_running.read().await {
            interval.tick().await;
            self.perform_all_health_checks().await;
        }
    }

    /// Perform all health checks
    async fn perform_all_health_checks(&self) {
        let health_checks = self.health_checks.read().await;

        for (name, health_check) in health_checks.iter() {
            let result = self.perform_health_check(health_check.as_ref()).await;

            // Update results
            {
                let mut results = self.results.write().await;
                results.insert(name.clone(), result.clone());
            }

            // Update failure counts
            {
                let mut failure_counts = self.failure_counts.write().await;
                match result.status {
                    HealthStatus::Healthy => {
                        failure_counts.remove(name);
                    }
                    _ => {
                        *failure_counts.entry(name.clone()).or_insert(0) += 1;
                    }
                }
            }

            // Log results
            match result.status {
                HealthStatus::Healthy => {
                    info!(
                        "Health check '{}' passed in {}ms",
                        name, result.response_time_ms
                    );
                }
                HealthStatus::Degraded => {
                    warn!(
                        "Health check '{}' degraded in {}ms: {:?}",
                        name, result.response_time_ms, result.error_message
                    );
                }
                _ => {
                    error!(
                        "Health check '{}' failed in {}ms: {:?}",
                        name, result.response_time_ms, result.error_message
                    );
                }
            }
        }
    }

    /// Perform individual health check
    async fn perform_health_check(&self, health_check: &dyn HealthChecker) -> HealthCheckResult {
        let start_time = Instant::now();
        let name = health_check.name().to_string();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            health_check.check(),
        )
        .await;

        let response_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(check_result) => check_result,
            Err(_) => HealthCheckResult {
                name,
                status: HealthStatus::Unhealthy,
                timestamp: Utc::now(),
                response_time_ms: response_time,
                error_message: Some("Health check timeout".to_string()),
                details: HashMap::new(),
            },
        }
    }

    /// Get overall health status
    pub async fn get_overall_health(&self) -> HealthStatus {
        let results = self.results.read().await;
        let failure_counts = self.failure_counts.read().await;

        let mut has_unhealthy = false;
        let mut has_degraded = false;
        let mut required_failures = 0;

        for (name, result) in results.iter() {
            match result.status {
                HealthStatus::Unhealthy => {
                    has_unhealthy = true;
                    if let Some(&count) = failure_counts.get(name) {
                        if count >= self.config.max_failures {
                            required_failures += 1;
                        }
                    }
                }
                HealthStatus::Degraded => {
                    has_degraded = true;
                }
                _ => {}
            }
        }

        if has_unhealthy && required_failures > 0 {
            HealthStatus::Unhealthy
        } else if has_degraded || has_unhealthy {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Get health check results
    pub async fn get_results(&self) -> HashMap<String, HealthCheckResult> {
        let results = self.results.read().await;
        results.clone()
    }

    /// Get readiness status
    pub async fn is_ready(&self) -> bool {
        if !self.config.enable_readiness {
            return true;
        }

        let health_checks = self.health_checks.read().await;
        let results = self.results.read().await;

        for (name, health_check) in health_checks.iter() {
            if health_check.is_required() {
                if let Some(result) = results.get(name) {
                    if matches!(result.status, HealthStatus::Unhealthy) {
                        return false;
                    }
                } else {
                    // No result yet, consider not ready
                    return false;
                }
            }
        }

        true
    }

    /// Get liveness status
    pub async fn is_alive(&self) -> bool {
        if !self.config.enable_liveness {
            return true;
        }

        let overall_health = self.get_overall_health().await;
        !matches!(overall_health, HealthStatus::Unhealthy)
    }

    /// Get startup status
    pub async fn is_started(&self) -> bool {
        if !self.config.enable_startup {
            return true;
        }

        let health_checks = self.health_checks.read().await;
        let results = self.results.read().await;

        // Check if all required health checks have been performed at least once
        for (name, health_check) in health_checks.iter() {
            if health_check.is_required() && !results.contains_key(name) {
                return false;
            }
        }

        true
    }

    /// Get health check statistics
    pub async fn get_statistics(&self) -> HealthCheckStatistics {
        let results = self.results.read().await;
        let failure_counts = self.failure_counts.read().await;

        let mut healthy_count = 0;
        let mut unhealthy_count = 0;
        let mut degraded_count = 0;
        let mut total_response_time = 0;

        for result in results.values() {
            match result.status {
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Unhealthy => unhealthy_count += 1,
                HealthStatus::Degraded => degraded_count += 1,
                _ => {}
            }
            total_response_time += result.response_time_ms;
        }

        let total_checks = results.len();
        let avg_response_time = if total_checks > 0 {
            total_response_time / total_checks as u64
        } else {
            0
        };

        HealthCheckStatistics {
            total_checks,
            healthy_count,
            unhealthy_count,
            degraded_count,
            average_response_time_ms: avg_response_time,
            last_check_time: results.values().map(|r| r.timestamp).max(),
            failure_counts: failure_counts.clone(),
        }
    }
}

impl Clone for HealthCheckManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            health_checks: Arc::clone(&self.health_checks),
            results: Arc::clone(&self.results),
            is_running: Arc::clone(&self.is_running),
            failure_counts: Arc::clone(&self.failure_counts),
        }
    }
}

/// Health check statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckStatistics {
    /// Total number of health checks
    pub total_checks: usize,

    /// Number of healthy checks
    pub healthy_count: u32,

    /// Number of unhealthy checks
    pub unhealthy_count: u32,

    /// Number of degraded checks
    pub degraded_count: u32,

    /// Average response time in milliseconds
    pub average_response_time_ms: u64,

    /// Last check time
    pub last_check_time: Option<DateTime<Utc>>,

    /// Failure counts by check name
    pub failure_counts: HashMap<String, u32>,
}

/// HTTP endpoint health checker
pub struct HttpEndpointHealthChecker {
    name: String,
    endpoint: HealthCheckEndpoint,
}

impl HttpEndpointHealthChecker {
    /// Create new HTTP endpoint health checker
    pub fn new(name: String, endpoint: HealthCheckEndpoint) -> Self {
        Self { name, endpoint }
    }
}

#[async_trait]
impl HealthChecker for HttpEndpointHealthChecker {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "HTTP endpoint health check"
    }

    async fn check(&self) -> HealthCheckResult {
        let start_time = Instant::now();
        let client = reqwest::Client::new();

        let mut request = client.get(&self.endpoint.url);

        // Add headers
        for (key, value) in &self.endpoint.headers {
            request = request.header(key, value);
        }

        // Add authentication
        if let Some(auth) = &self.endpoint.auth {
            match auth.auth_type {
                AuthType::Basic => {
                    if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
                        request = request.basic_auth(username, Some(password));
                    }
                }
                AuthType::Bearer => {
                    if let Some(token) = &auth.token {
                        request = request.bearer_auth(token);
                    }
                }
                AuthType::None => {}
            }
        }

        let response = request.send().await;
        let response_time = start_time.elapsed().as_millis() as u64;

        match response {
            Ok(response) => {
                let status = response.status();
                if status.as_u16() == self.endpoint.expected_status {
                    HealthCheckResult {
                        name: self.name.clone(),
                        status: HealthStatus::Healthy,
                        timestamp: Utc::now(),
                        response_time_ms: response_time,
                        error_message: None,
                        details: HashMap::from([
                            ("status_code".to_string(), status.as_u16().to_string()),
                            ("url".to_string(), self.endpoint.url.clone()),
                        ]),
                    }
                } else {
                    HealthCheckResult {
                        name: self.name.clone(),
                        status: HealthStatus::Unhealthy,
                        timestamp: Utc::now(),
                        response_time_ms: response_time,
                        error_message: Some(format!(
                            "Expected status {}, got {}",
                            self.endpoint.expected_status,
                            status.as_u16()
                        )),
                        details: HashMap::from([
                            ("status_code".to_string(), status.as_u16().to_string()),
                            ("url".to_string(), self.endpoint.url.clone()),
                        ]),
                    }
                }
            }
            Err(e) => HealthCheckResult {
                name: self.name.clone(),
                status: HealthStatus::Unhealthy,
                timestamp: Utc::now(),
                response_time_ms: response_time,
                error_message: Some(format!("HTTP request failed: {}", e)),
                details: HashMap::from([("url".to_string(), self.endpoint.url.clone())]),
            },
        }
    }
}

/// Memory usage health checker
pub struct MemoryUsageHealthChecker {
    name: String,
    max_usage_percent: f64,
}

impl MemoryUsageHealthChecker {
    /// Create new memory usage health checker
    pub fn new(name: String, max_usage_percent: f64) -> Self {
        Self {
            name,
            max_usage_percent,
        }
    }
}

#[async_trait]
impl HealthChecker for MemoryUsageHealthChecker {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Memory usage health check"
    }

    async fn check(&self) -> HealthCheckResult {
        let start_time = Instant::now();

        // Get memory usage (simplified implementation)
        let mut system = sysinfo::System::new_all();
        system.refresh_memory();
        let memory_usage = (system.used_memory() as f64 / system.total_memory() as f64) * 100.0;
        let response_time = start_time.elapsed().as_millis() as u64;

        let status = if memory_usage <= self.max_usage_percent {
            HealthStatus::Healthy
        } else if memory_usage <= self.max_usage_percent * 1.2 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        let error_message = if memory_usage > self.max_usage_percent {
            Some(format!(
                "Memory usage {}% exceeds threshold {}%",
                memory_usage, self.max_usage_percent
            ))
        } else {
            None
        };

        HealthCheckResult {
            name: self.name.clone(),
            status,
            timestamp: Utc::now(),
            response_time_ms: response_time,
            error_message,
            details: HashMap::from([
                ("memory_usage_percent".to_string(), memory_usage.to_string()),
                (
                    "max_usage_percent".to_string(),
                    self.max_usage_percent.to_string(),
                ),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_manager_creation() {
        let config = HealthCheckConfig::new();
        let manager = HealthCheckManager::new(config);

        assert_eq!(manager.config.check_interval_secs, 30);
        assert_eq!(manager.config.timeout_secs, 10);
    }

    #[tokio::test]
    async fn test_health_check_manager_overall_health() {
        let config = HealthCheckConfig::new();
        let manager = HealthCheckManager::new(config);

        let health_status = manager.get_overall_health().await;
        assert_eq!(health_status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_memory_usage_health_checker() {
        let checker = MemoryUsageHealthChecker::new("memory_check".to_string(), 90.0);
        let result = checker.check().await;

        assert_eq!(result.name, "memory_check");
        assert!(result.response_time_ms > 0);
    }
}
