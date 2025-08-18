//! Component management handlers
//!
//! This module provides comprehensive component management functionality including:
//! - HTTP API handlers for component operations
//! - Component restart handlers for different bridge components
//! - Health monitoring and status reporting
//! - Component discovery and listing

pub mod api;
pub mod handlers;
pub mod health;
pub mod discovery;

// Re-export main functionality for convenience
pub use api::*;
pub use handlers::*;
pub use health::*;
pub use discovery::*;
