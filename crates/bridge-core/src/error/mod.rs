//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling for the OpenTelemetry Data Lake Bridge
//!
//! This module provides structured error types with context and recovery strategies
//! for all components of the bridge.

pub mod context;
pub mod conversions;
pub mod types;

// Re-export commonly used types
pub use context::ErrorContext;
pub use conversions::*;
pub use types::{BridgeError, BridgeResult};
