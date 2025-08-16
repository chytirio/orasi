//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Role-based access control module

pub mod manager;
pub mod model;
pub mod stats;

// Re-export commonly used types
pub use manager::RoleManager;
pub use model::{Permission, Role};
pub use stats::RoleStats;
