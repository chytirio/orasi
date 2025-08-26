//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data state management for the bridge
//!
//! This module provides state management functionality for streaming data
//! including state stores and managers.

pub mod store;
pub mod manager;
pub mod factory;

// Re-export main types for convenience
pub use store::*;
pub use manager::*;
pub use factory::*;
