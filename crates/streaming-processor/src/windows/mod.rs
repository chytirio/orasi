//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data windowing for the bridge
//!
//! This module provides windowing functionality for streaming data
//! including time windows, count windows, and session windows.

pub mod types;
pub mod fun;
pub mod manager;

// Re-export main types for convenience
pub use types::*;
pub use fun::*;
pub use manager::*;
