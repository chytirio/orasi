//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Utility functions and helper modules for the OpenTelemetry Data Lake Bridge
//! 
//! This module provides common utility functions, retry logic, circuit breakers,
//! and data processing utilities used throughout the bridge.

pub mod retry;
pub mod circuit_breaker;
pub mod data_processing;
pub mod time;

// Re-export commonly used types
pub use retry::{RetryConfig, RetryPolicy, RetryStats};
pub use circuit_breaker::{CircuitBreakerConfig, CircuitBreaker, CircuitBreakerState, CircuitBreakerStats};
pub use data_processing::{DataProcessor, ProcessingConfig, ProcessingStats};
pub use time::{TimeUtils, TimeConfig};
