//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry processors for the OpenTelemetry Data Lake Bridge
//!
//! This module provides concrete implementations of telemetry processors
//! that can transform, filter, and aggregate telemetry data.

pub mod aggregate;
pub mod filter;
pub mod mock;
pub mod transform;

// Re-export commonly used processors
pub use aggregate::AggregateProcessor;
pub use filter::FilterProcessor;
pub use mock::MockProcessor;
pub use transform::TransformProcessor;
