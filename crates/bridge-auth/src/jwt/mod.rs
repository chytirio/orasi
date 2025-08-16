//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! JWT (JSON Web Token) authentication module

pub mod claims;
pub mod manager;
pub mod stats;

// Re-export commonly used types
pub use claims::JwtClaims;
pub use manager::JwtManager;
pub use stats::JwtStats;
