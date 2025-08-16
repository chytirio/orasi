//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Health manager and simple health check implementations for the OpenTelemetry Data Lake Bridge

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{BridgeError, BridgeResult};
use super::{checker::{HealthChecker, HealthCheckCallback}, types::{HealthStatus, HealthCheckResult, ComponentHealth, SystemHealth}};

/// Health manager for coordinating multiple health checkers
pub struct HealthManager {
    /// Health checkers by name
    checkers: Arc<RwLock<HashMap<String, Arc<HealthChecker>>>>,

    /// System health information
    system_health: Arc<RwLock<SystemHealth>>,
}

impl HealthManager {
    /// Create a new health manager
    pub fn new() -> Self {
        Self {
            checkers: Arc::new(RwLock::new(HashMap::new())),
            system_health: Arc::new(RwLock::new(SystemHealth::new(HealthStatus::Unknown, 0))),
        }
    }

    /// Add a health checker
    pub async fn add_checker(&self, name: String, checker: Arc<HealthChecker>) -> BridgeResult<()> {
        let mut checkers = self.checkers.write().await;
        checkers.insert(name.clone(), checker);
        info!("Added health checker: {}", name);
        Ok(())
    }

    /// Remove a health checker
    pub async fn remove_checker(&self, name: &str) -> BridgeResult<()> {
        let mut checkers = self.checkers.write().await;
        checkers.remove(name);
        info!("Removed health checker: {}", name);
        Ok(())
    }

    /// Get a health checker
    pub async fn get_checker(&self, name: &str) -> Option<Arc<HealthChecker>> {
        let checkers = self.checkers.read().await;
        checkers.get(name).cloned()
    }

    /// Get all health checkers
    pub async fn get_all_checkers(&self) -> HashMap<String, Arc<HealthChecker>> {
        self.checkers.read().await.clone()
    }

    /// Perform health check for all components across all checkers
    pub async fn check_all_components(&self) -> BridgeResult<Vec<HealthCheckResult>> {
        let checkers = self.checkers.read().await;
        let mut all_results = Vec::new();

        for (checker_name, checker) in checkers.iter() {
            match checker.check_all_components().await {
                Ok(results) => {
                    all_results.extend(results);
                }
                Err(e) => {
                    error!("Health check failed for checker {}: {}", checker_name, e);
                    let error_result = HealthCheckResult {
                        component: checker_name.clone(),
                        status: HealthStatus::Unhealthy,
                        timestamp: chrono::Utc::now(),
                        duration_ms: 0,
                        message: format!("Health check failed: {}", e),
                        details: None,
                        errors: vec![e.to_string()],
                    };
                    all_results.push(error_result);
                }
            }
        }

        // Update system health
        self.update_system_health(&all_results).await;

        Ok(all_results)
    }

    /// Get system health information
    pub async fn get_system_health(&self) -> SystemHealth {
        self.system_health.read().await.clone()
    }

    /// Update system health based on component results
    async fn update_system_health(&self, results: &[HealthCheckResult]) {
        let mut system_health = self.system_health.write().await;
        system_health.timestamp = chrono::Utc::now();

        // Convert results to component health
        let mut components = Vec::new();
        for result in results {
            let component_health = ComponentHealth::new(result.component.clone(), result.status.clone())
                .with_last_check(result.timestamp)
                .with_details(result.details.clone().unwrap_or_default());

            if !result.errors.is_empty() {
                let mut component = component_health;
                for error in &result.errors {
                    component = component.with_error(error.clone());
                }
                components.push(component);
            } else {
                components.push(component_health);
            }
        }

        system_health.components = components;
        system_health.calculate_overall_status();
    }

    /// Initialize all health checkers
    pub async fn init_all_checkers(&self) -> BridgeResult<()> {
        let checkers = self.checkers.read().await;
        
        for (name, checker) in checkers.iter() {
            match checker.init().await {
                Ok(()) => {
                    info!("Initialized health checker: {}", name);
                }
                Err(e) => {
                    error!("Failed to initialize health checker {}: {}", name, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Shutdown all health checkers
    pub async fn shutdown_all_checkers(&self) -> BridgeResult<()> {
        let checkers = self.checkers.read().await;
        
        for (name, checker) in checkers.iter() {
            match checker.shutdown().await {
                Ok(()) => {
                    info!("Shutdown health checker: {}", name);
                }
                Err(e) => {
                    error!("Failed to shutdown health checker {}: {}", name, e);
                    // Continue with other checkers even if one fails
                }
            }
        }

        Ok(())
    }
}

/// Simple health check callback implementation
pub struct SimpleHealthCheck {
    component_name: String,
    check_fn: Box<dyn Fn() -> BridgeResult<bool> + Send + Sync>,
}

impl SimpleHealthCheck {
    /// Create a new simple health check
    pub fn new<F>(component_name: String, check_fn: F) -> Self
    where
        F: Fn() -> BridgeResult<bool> + Send + Sync + 'static,
    {
        Self {
            component_name,
            check_fn: Box::new(check_fn),
        }
    }
}

#[async_trait]
impl HealthCheckCallback for SimpleHealthCheck {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let is_healthy = (self.check_fn)()?;

        Ok(HealthCheckResult {
            component: self.component_name.clone(),
            status: if is_healthy {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            },
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
            message: if is_healthy {
                "Component is healthy".to_string()
            } else {
                "Component is unhealthy".to_string()
            },
            details: None,
            errors: Vec::new(),
        })
    }

    fn component_name(&self) -> &str {
        &self.component_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_manager_creation() {
        let manager = HealthManager::new();
        let system_health = manager.get_system_health().await;
        assert!(matches!(system_health.status, HealthStatus::Unknown));
    }

    #[tokio::test]
    async fn test_simple_health_check() {
        let config = super::super::config::HealthCheckConfig::default();
        let checker = HealthChecker::new(config);

        // Add a simple health check
        let health_check = SimpleHealthCheck::new("test_component".to_string(), || Ok(true));

        checker
            .add_health_check(Arc::new(health_check))
            .await
            .unwrap();

        // Check the component
        let result = checker.check_component("test_component").await.unwrap();
        assert_eq!(result.component, "test_component");
        assert!(matches!(result.status, HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_unhealthy_component() {
        let config = super::super::config::HealthCheckConfig::default();
        let checker = HealthChecker::new(config);

        // Add an unhealthy health check
        let health_check = SimpleHealthCheck::new("unhealthy_component".to_string(), || Ok(false));

        checker
            .add_health_check(Arc::new(health_check))
            .await
            .unwrap();

        // Check the component
        let result = checker
            .check_component("unhealthy_component")
            .await
            .unwrap();
        assert_eq!(result.component, "unhealthy_component");
        assert!(matches!(result.status, HealthStatus::Unhealthy));
    }

    #[tokio::test]
    async fn test_unknown_component() {
        let config = super::super::config::HealthCheckConfig::default();
        let checker = HealthChecker::new(config);

        // Check a non-existent component
        let result = checker.check_component("unknown_component").await.unwrap();
        assert_eq!(result.component, "unknown_component");
        assert!(matches!(result.status, HealthStatus::Unknown));
    }
}
