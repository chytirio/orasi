//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Health check configuration for the OpenTelemetry Data Lake Bridge

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Enable health checks
    pub enable_health_checks: bool,

    /// Health check interval in milliseconds
    pub health_check_interval_ms: u64,

    /// Health check timeout in milliseconds
    pub health_check_timeout_ms: u64,

    /// Health check endpoint
    pub health_endpoint: String,

    /// Health check port
    pub health_port: u16,

    /// Enable detailed health reporting
    pub enable_detailed_health: bool,

    /// Health check retry count
    pub health_check_retry_count: u32,

    /// Health check retry delay in milliseconds
    pub health_check_retry_delay_ms: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enable_health_checks: true,
            health_check_interval_ms: 30000,
            health_check_timeout_ms: 5000,
            health_endpoint: "0.0.0.0".to_string(),
            health_port: 8080,
            enable_detailed_health: true,
            health_check_retry_count: 3,
            health_check_retry_delay_ms: 1000,
        }
    }
}

impl HealthCheckConfig {
    /// Validate health check configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.health_check_interval_ms == 0 {
            return Err("Health check interval must be greater than 0".to_string());
        }

        if self.health_check_timeout_ms == 0 {
            return Err("Health check timeout must be greater than 0".to_string());
        }

        if self.health_check_timeout_ms >= self.health_check_interval_ms {
            return Err("Health check timeout must be less than interval".to_string());
        }

        if self.health_check_retry_count == 0 {
            return Err("Health check retry count must be greater than 0".to_string());
        }

        if self.health_check_retry_delay_ms == 0 {
            return Err("Health check retry delay must be greater than 0".to_string());
        }

        Ok(())
    }
}
