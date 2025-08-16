//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry ingestion pipeline for the OpenTelemetry Data Lake Bridge
//!
//! This module provides the core orchestration logic for processing telemetry
//! data through the bridge pipeline, including receivers, processors, and exporters.

pub mod config;
pub mod health;
pub mod processor;
pub mod state;

// Re-export commonly used types
pub use config::PipelineConfig;
pub use health::PipelineHealthChecker;
pub use processor::TelemetryIngestionPipeline;
pub use state::{PipelineState, PipelineStats};
