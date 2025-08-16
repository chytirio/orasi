//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Health checker implementation for the OpenTelemetry Data Lake Bridge

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{BridgeError, BridgeResult};
use super::{config::HealthCheckConfig, types::{HealthStatus, HealthCheckResult}};

/// Health state
#[derive(Debug, Clone)]
pub struct HealthState {
    /// Overall health status
    pub overall_status: HealthStatus,

    /// Last health check timestamp
    pub last_check_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Total health checks performed
    pub total_checks: u64,

    /// Successful health checks
    pub successful_checks: u64,

    /// Failed health checks
    pub failed_checks: u64,

    /// Health check errors
    pub errors: Vec<String>,
}

/// Health check callback trait
#[async_trait]
pub trait HealthCheckCallback: Send + Sync {
    /// Perform health check
    async fn check(&self) -> BridgeResult<HealthCheckResult>;

    /// Get component name
    fn component_name(&self) -> &str;
}

impl Default for HealthState {
    fn default() -> Self {
        Self {
            overall_status: HealthStatus::Unknown,
            last_check_time: None,
            total_checks: 0,
            successful_checks: 0,
            failed_checks: 0,
            errors: Vec::new(),
        }
    }
}

/// Health checker
pub struct HealthChecker {
    /// Health check configuration
    config: HealthCheckConfig,

    /// Health check state
    state: Arc<RwLock<HealthState>>,

    /// Health check results
    results: Arc<RwLock<HashMap<String, HealthCheckResult>>>,

    /// Health check callbacks
    callbacks: Arc<RwLock<HashMap<String, Arc<dyn HealthCheckCallback>>>>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(HealthState::default())),
            results: Arc::new(RwLock::new(HashMap::new())),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize health checker
    pub async fn init(&self) -> BridgeResult<()> {
        if !self.config.enable_health_checks {
            info!("Health checks are disabled");
            return Ok(());
        }

        info!("Initializing health checker");

        // Start health check task
        self.start_health_check_task().await?;

        info!("Health checker initialized successfully");
        Ok(())
    }

    /// Add health check callback
    pub async fn add_health_check(
        &self,
        callback: Arc<dyn HealthCheckCallback>,
    ) -> BridgeResult<()> {
        let component_name = callback.component_name().to_string();
        let mut callbacks = self.callbacks.write().await;
        callbacks.insert(component_name.clone(), callback);
        info!("Added health check for component: {}", component_name);
        Ok(())
    }

    /// Remove health check callback
    pub async fn remove_health_check(&self, component_name: &str) -> BridgeResult<()> {
        let mut callbacks = self.callbacks.write().await;
        callbacks.remove(component_name);
        info!("Removed health check for component: {}", component_name);
        Ok(())
    }

    /// Perform health check for a specific component
    pub async fn check_component(&self, component_name: &str) -> BridgeResult<HealthCheckResult> {
        let callbacks = self.callbacks.read().await;

        if let Some(callback) = callbacks.get(component_name) {
            let start_time = Instant::now();
            let result = callback.check().await?;
            let duration = start_time.elapsed();

            let result = HealthCheckResult {
                duration_ms: duration.as_millis() as u64,
                ..result
            };

            // Store the result
            {
                let mut results = self.results.write().await;
                results.insert(component_name.to_string(), result.clone());
            }

            // Update health state
            self.update_health_state(&result).await;

            info!(
                "Health check for component {}: {:?}",
                component_name, result.status
            );
            Ok(result)
        } else {
            // Return unknown status for non-existent component
            let result = HealthCheckResult {
                component: component_name.to_string(),
                status: HealthStatus::Unknown,
                timestamp: chrono::Utc::now(),
                duration_ms: 0,
                message: "Component not found".to_string(),
                details: None,
                errors: vec!["Component not registered".to_string()],
            };
            Ok(result)
        }
    }

    /// Perform health check for all components
    pub async fn check_all_components(&self) -> BridgeResult<Vec<HealthCheckResult>> {
        let callbacks = self.callbacks.read().await;
        let mut results = Vec::new();

        for (component_name, callback) in callbacks.iter() {
            match self.check_component(component_name).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!(
                        "Health check failed for component {}: {}",
                        component_name, e
                    );
                    let error_result = HealthCheckResult {
                        component: component_name.clone(),
                        status: HealthStatus::Unhealthy,
                        timestamp: chrono::Utc::now(),
                        duration_ms: 0,
                        message: format!("Health check failed: {}", e),
                        details: None,
                        errors: vec![e.to_string()],
                    };
                    results.push(error_result);
                }
            }
        }

        Ok(results)
    }

    /// Get health check result for a component
    pub async fn get_component_health(&self, component_name: &str) -> Option<HealthCheckResult> {
        let results = self.results.read().await;
        results.get(component_name).cloned()
    }

    /// Get all health check results
    pub async fn get_all_health_results(&self) -> HashMap<String, HealthCheckResult> {
        self.results.read().await.clone()
    }

    /// Get overall health status
    pub async fn get_overall_health(&self) -> HealthStatus {
        let state = self.state.read().await;
        state.overall_status.clone()
    }

    /// Start health check task
    async fn start_health_check_task(&self) -> BridgeResult<()> {
        let config = self.config.clone();
        let state = self.state.clone();
        let results = self.results.clone();
        let callbacks = self.callbacks.clone();

        tokio::spawn(async move {
            let interval = Duration::from_millis(config.health_check_interval_ms);
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                // Perform health checks for all components
                let callbacks_guard = callbacks.read().await;
                let mut component_results = Vec::new();

                for (component_name, callback) in callbacks_guard.iter() {
                    let start_time = Instant::now();
                    match callback.check().await {
                        Ok(mut result) => {
                            let duration = start_time.elapsed();
                            result.duration_ms = duration.as_millis() as u64;
                            component_results.push(result.clone());

                            // Store the result
                            {
                                let mut results_guard = results.write().await;
                                results_guard.insert(component_name.clone(), result);
                            }
                        }
                        Err(e) => {
                            error!(
                                "Health check failed for component {}: {}",
                                component_name, e
                            );
                            let error_result = HealthCheckResult {
                                component: component_name.clone(),
                                status: HealthStatus::Unhealthy,
                                timestamp: chrono::Utc::now(),
                                duration_ms: 0,
                                message: format!("Health check failed: {}", e),
                                details: None,
                                errors: vec![e.to_string()],
                            };
                            component_results.push(error_result.clone());

                            // Store the error result
                            {
                                let mut results_guard = results.write().await;
                                results_guard.insert(component_name.clone(), error_result);
                            }
                        }
                    }
                }

                // Update overall health state
                {
                    let mut state_guard = state.write().await;
                    state_guard.last_check_time = Some(chrono::Utc::now());
                    state_guard.total_checks += 1;

                    let healthy_count = component_results
                        .iter()
                        .filter(|r| r.status == HealthStatus::Healthy)
                        .count();
                    let unhealthy_count = component_results
                        .iter()
                        .filter(|r| r.status == HealthStatus::Unhealthy)
                        .count();

                    if unhealthy_count > 0 {
                        state_guard.overall_status = HealthStatus::Unhealthy;
                        state_guard.failed_checks += 1;
                    } else if healthy_count == component_results.len() {
                        state_guard.overall_status = HealthStatus::Healthy;
                        state_guard.successful_checks += 1;
                    } else {
                        state_guard.overall_status = HealthStatus::Degraded;
                        state_guard.failed_checks += 1;
                    }
                }

                debug!("Health check cycle completed");
            }
        });

        Ok(())
    }

    /// Update health state based on check result
    async fn update_health_state(&self, result: &HealthCheckResult) {
        let mut state = self.state.write().await;
        state.last_check_time = Some(chrono::Utc::now());
        state.total_checks += 1;

        match result.status {
            HealthStatus::Healthy => {
                state.successful_checks += 1;
            }
            HealthStatus::Unhealthy | HealthStatus::Degraded => {
                state.failed_checks += 1;
                if !result.errors.is_empty() {
                    state.errors.extend(result.errors.clone());
                }
            }
            HealthStatus::Unknown => {
                // Don't count unknown status in statistics
            }
        }
    }

    /// Shutdown health checker
    pub async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down health checker");
        // The health check task will be automatically cancelled when the HealthChecker is dropped
        Ok(())
    }
}
