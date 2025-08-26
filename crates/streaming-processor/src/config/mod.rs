//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration for streaming processor

pub mod main;
pub mod sources;
pub mod processors;
pub mod sinks;
pub mod processing;
pub mod state;
pub mod metrics;
pub mod security;
pub mod connection;
pub mod rate_limiting;

// Re-export main types for convenience
pub use main::*;
pub use sources::*;
pub use processors::*;
pub use sinks::*;
pub use processing::*;
pub use state::*;
pub use metrics::*;
pub use security::*;
pub use connection::*;
pub use rate_limiting::*;

// Re-export the main config struct for backward compatibility
pub use main::StreamingProcessorConfig as Config;
