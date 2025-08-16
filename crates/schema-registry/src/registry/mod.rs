//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Schema Registry Manager
//!
//! This module provides the main registry manager for coordinating
//! schema operations and managing the registry lifecycle.

pub mod compatibility;
pub mod manager;
pub mod operations;
pub mod state;
pub mod validation;

// Re-export main types for convenience
pub use manager::SchemaRegistryManager;
pub use state::{RegistryMetrics, RegistryState, RegistryStats};
