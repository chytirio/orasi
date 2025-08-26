//! Metrics collection module for the Orasi Agent

pub mod types;
pub mod alerts;
pub mod collection;
pub mod prometheus;

// Re-export main types for convenience
pub use types::*;
pub use alerts::*;
pub use collection::*;
pub use prometheus::*;

// Re-export the main collector struct for backward compatibility
pub use collection::MetricsCollector as Collector;
