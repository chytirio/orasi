//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! User management module

pub mod manager;
pub mod model;
pub mod stats;

// Re-export commonly used types
pub use manager::UserManager;
pub use model::{User, UserRole};
pub use stats::UserStats;
