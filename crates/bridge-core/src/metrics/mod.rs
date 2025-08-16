//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Metrics collection and monitoring for the OpenTelemetry Data Lake Bridge
//!
//! This module provides metrics collection, monitoring, and observability
//! capabilities for the bridge components.

pub mod collector;
pub mod config;
pub mod factory;

// Re-export commonly used types
pub use collector::BridgeMetrics;
pub use config::MetricsConfig;
pub use factory::MetricsFactory;
