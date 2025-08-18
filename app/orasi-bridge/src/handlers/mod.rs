//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Request handlers for Bridge API

pub mod health;
pub mod telemetry;
pub mod query;
pub mod analytics;
pub mod config;
pub mod components;
pub mod plugins;
pub mod utils;

// Re-export all public handlers for convenience
pub use health::*;
pub use telemetry::*;
pub use query::*;
pub use analytics::*;
pub use config::*;
pub use components::*;
pub use plugins::*;
pub use utils::*;
