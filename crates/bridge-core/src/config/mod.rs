//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration management for the OpenTelemetry Data Lake Bridge
//!
//! This module provides type-safe configuration structures with validation
//! and hierarchical configuration management.

pub mod advanced;
pub mod bridge;
pub mod ingestion;
pub mod lakehouse;
pub mod monitoring;
pub mod processing;
pub mod security;

// Re-export commonly used types
pub use advanced::*;
pub use bridge::{BridgeConfig, DEFAULT_CONFIG_PATH};
pub use ingestion::*;
pub use lakehouse::*;
pub use monitoring::*;
pub use processing::*;
pub use security::*;
