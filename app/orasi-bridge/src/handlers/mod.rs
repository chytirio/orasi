//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Request handlers for Bridge API

pub mod analytics;
pub mod components;
pub mod config;
pub mod health;
pub mod plugins;
pub mod query;
pub mod telemetry;
pub mod utils;

// Re-export all public handlers for convenience
pub use analytics::*;
pub use components::*;
pub use config::*;
pub use health::*;
pub use plugins::*;
pub use query::*;
pub use telemetry::*;
pub use utils::*;
