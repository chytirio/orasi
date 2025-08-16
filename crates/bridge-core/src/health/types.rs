//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Health check types and data structures for the OpenTelemetry Data Lake Bridge

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,

    /// Component is unhealthy
    Unhealthy,

    /// Component health is unknown
    Unknown,

    /// Component is degraded
    Degraded,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Component name
    pub component: String,

    /// Health status
    pub status: HealthStatus,

    /// Health check timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Health check duration in milliseconds
    pub duration_ms: u64,

    /// Health check message
    pub message: String,

    /// Health check details
    pub details: Option<HashMap<String, serde_json::Value>>,

    /// Health check errors
    pub errors: Vec<String>,
}

impl HealthCheckResult {
    /// Create a new health check result
    pub fn new(
        component: String,
        status: HealthStatus,
        message: String,
    ) -> Self {
        Self {
            component,
            status,
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
            message,
            details: None,
            errors: Vec::new(),
        }
    }

    /// Add an error to the health check result
    pub fn with_error(mut self, error: String) -> Self {
        self.errors.push(error);
        self
    }

    /// Add details to the health check result
    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        self.details = Some(details);
        self
    }

    /// Set the duration of the health check
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,

    /// Component status
    pub status: HealthStatus,

    /// Component version
    pub version: Option<String>,

    /// Component uptime in seconds
    pub uptime_seconds: Option<u64>,

    /// Component last check time
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,

    /// Component details
    pub details: Option<HashMap<String, serde_json::Value>>,

    /// Component errors
    pub errors: Vec<String>,
}

impl ComponentHealth {
    /// Create a new component health
    pub fn new(name: String, status: HealthStatus) -> Self {
        Self {
            name,
            status,
            version: None,
            uptime_seconds: None,
            last_check: None,
            details: None,
            errors: Vec::new(),
        }
    }

    /// Set the component version
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Set the component uptime
    pub fn with_uptime(mut self, uptime_seconds: u64) -> Self {
        self.uptime_seconds = Some(uptime_seconds);
        self
    }

    /// Set the last check time
    pub fn with_last_check(mut self, last_check: chrono::DateTime<chrono::Utc>) -> Self {
        self.last_check = Some(last_check);
        self
    }

    /// Add details to the component health
    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        self.details = Some(details);
        self
    }

    /// Add an error to the component health
    pub fn with_error(mut self, error: String) -> Self {
        self.errors.push(error);
        self
    }
}

/// System health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// System status
    pub status: HealthStatus,

    /// System timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// System uptime in seconds
    pub uptime_seconds: u64,

    /// Component health information
    pub components: Vec<ComponentHealth>,

    /// System details
    pub details: Option<HashMap<String, serde_json::Value>>,

    /// System errors
    pub errors: Vec<String>,
}

impl SystemHealth {
    /// Create a new system health
    pub fn new(status: HealthStatus, uptime_seconds: u64) -> Self {
        Self {
            status,
            timestamp: chrono::Utc::now(),
            uptime_seconds,
            components: Vec::new(),
            details: None,
            errors: Vec::new(),
        }
    }

    /// Add a component to the system health
    pub fn with_component(mut self, component: ComponentHealth) -> Self {
        self.components.push(component);
        self
    }

    /// Add details to the system health
    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        self.details = Some(details);
        self
    }

    /// Add an error to the system health
    pub fn with_error(mut self, error: String) -> Self {
        self.errors.push(error);
        self
    }

    /// Get the overall system status based on component health
    pub fn calculate_overall_status(&mut self) {
        if self.components.is_empty() {
            self.status = HealthStatus::Unknown;
            return;
        }

        let has_unhealthy = self.components.iter().any(|c| c.status == HealthStatus::Unhealthy);
        let has_degraded = self.components.iter().any(|c| c.status == HealthStatus::Degraded);
        let all_healthy = self.components.iter().all(|c| c.status == HealthStatus::Healthy);

        self.status = if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else if all_healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        };
    }
}
