//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake exporter for the OpenTelemetry Data Lake Bridge
//!
//! This module provides a Delta Lake exporter that can export telemetry data
//! to Delta Lake format.

pub mod config;
pub mod converter;
pub mod health;
pub mod demo;
pub mod exporter;

// Re-export main types for convenience
pub use config::*;
pub use converter::*;
pub use health::*;
pub use demo::*;
pub use exporter::*;

// Re-export the main exporter struct for backward compatibility
pub use exporter::DeltaLakeExporter as Exporter;
