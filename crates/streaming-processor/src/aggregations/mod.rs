//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data aggregations for the bridge
//!
//! This module provides aggregation functionality for streaming data
//! including various aggregation functions and managers.

pub mod trait_def;
pub mod fun;
pub mod manager;
pub mod factory;

// Re-export main types for convenience
pub use trait_def::*;
pub use fun::*;
pub use manager::*;
pub use factory::*;
