//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Permission management module

pub mod manager;
pub mod model;
pub mod stats;

// Re-export commonly used types
pub use manager::PermissionManager;
pub use model::AccessLevel;
pub use stats::PermissionStats;
