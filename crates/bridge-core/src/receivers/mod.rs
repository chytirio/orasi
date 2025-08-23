//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry receivers for the OpenTelemetry Data Lake Bridge
//!
//! This module provides concrete implementations of telemetry receivers
//! that can ingest data from various sources.

pub mod mock;
pub mod otlp;

// Re-export commonly used receivers
pub use mock::MockReceiver;
pub use otlp::OtlpReceiver;
