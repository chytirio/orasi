//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Core trait definitions for the OpenTelemetry Data Lake Bridge
//!
//! This module provides the foundational traits that define the extensible
//! architecture for telemetry processing, lakehouse connectors, and plugin interfaces.

pub mod connector;
pub mod exporter;
pub mod plugin;
pub mod processor;
pub mod receiver;
pub mod resilience;
pub mod schema;
pub mod stream;

// Re-export commonly used traits
pub use connector::*;
pub use exporter::*;
pub use plugin::*;
pub use processor::*;
pub use receiver::*;
pub use resilience::*;
pub use schema::*;
pub use stream::*;
