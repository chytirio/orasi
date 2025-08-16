//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Health checking system for the OpenTelemetry Data Lake Bridge
//!
//! This module provides comprehensive health monitoring and checking
//! capabilities for the bridge components and overall system health.

pub mod checker;
pub mod config;
pub mod manager;
pub mod types;

// Re-export commonly used types
pub use checker::HealthChecker;
pub use config::HealthCheckConfig;
pub use manager::HealthManager;
pub use types::{ComponentHealth, HealthCheckResult, HealthStatus, SystemHealth};
