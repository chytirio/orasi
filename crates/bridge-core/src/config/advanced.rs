//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Advanced configuration for the OpenTelemetry Data Lake Bridge
//!
//! This module provides advanced configuration options and performance tuning.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::Validate;

/// Advanced configuration options
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AdvancedConfig {
    /// Enable experimental features
    pub enable_experimental_features: bool,

    /// Custom configuration overrides
    pub custom_config: Option<HashMap<String, serde_json::Value>>,

    /// Performance tuning options
    pub performance_tuning: PerformanceTuningConfig,

    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,

    /// Retry configuration
    pub retry: RetryConfig,

    /// Connection pooling configuration
    pub connection_pooling: ConnectionPoolingConfig,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            enable_experimental_features: false,
            custom_config: None,
            performance_tuning: PerformanceTuningConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
            retry: RetryConfig::default(),
            connection_pooling: ConnectionPoolingConfig::default(),
        }
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PerformanceTuningConfig {
    /// Enable SIMD optimizations
    pub enable_simd: bool,

    /// Enable custom allocators
    pub enable_custom_allocators: bool,

    /// Memory pool size in bytes
    pub memory_pool_size: Option<u64>,

    /// Enable profile-guided optimization
    pub enable_pgo: bool,

    /// PGO profile path
    pub pgo_profile_path: Option<PathBuf>,
}

impl Default for PerformanceTuningConfig {
    fn default() -> Self {
        Self {
            enable_simd: true,
            enable_custom_allocators: false,
            memory_pool_size: None,
            enable_pgo: false,
            pgo_profile_path: None,
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CircuitBreakerConfig {
    /// Failure threshold
    #[validate(range(min = 1, max = 1000))]
    pub failure_threshold: u32,

    /// Success threshold
    #[validate(range(min = 1, max = 1000))]
    pub success_threshold: u32,

    /// Timeout in milliseconds
    #[validate(range(min = 100, max = 60000))]
    pub timeout_ms: u64,

    /// Half-open timeout in milliseconds
    #[validate(range(min = 1000, max = 300000))]
    pub half_open_timeout_ms: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_timeout_ms: 30000,
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RetryConfig {
    /// Maximum retry attempts
    #[validate(range(min = 0, max = 10))]
    pub max_attempts: u32,

    /// Initial backoff in milliseconds
    #[validate(range(min = 100, max = 10000))]
    pub initial_backoff_ms: u64,

    /// Maximum backoff in milliseconds
    #[validate(range(min = 1000, max = 300000))]
    pub max_backoff_ms: u64,

    /// Backoff multiplier
    #[validate(range(min = 1.0, max = 10.0))]
    pub backoff_multiplier: f64,

    /// Enable exponential backoff
    pub enable_exponential_backoff: bool,

    /// Enable jitter
    pub enable_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
            backoff_multiplier: 2.0,
            enable_exponential_backoff: true,
            enable_jitter: true,
        }
    }
}

/// Connection pooling configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ConnectionPoolingConfig {
    /// Minimum pool size
    #[validate(range(min = 1, max = 100))]
    pub min_pool_size: u32,

    /// Maximum pool size
    #[validate(range(min = 1, max = 1000))]
    pub max_pool_size: u32,

    /// Connection timeout in seconds
    #[validate(range(min = 1, max = 300))]
    pub connection_timeout_secs: u64,

    /// Idle timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub idle_timeout_secs: u64,

    /// Max lifetime in seconds
    #[validate(range(min = 1, max = 86400))]
    pub max_lifetime_secs: u64,
}

impl Default for ConnectionPoolingConfig {
    fn default() -> Self {
        Self {
            min_pool_size: 5,
            max_pool_size: 20,
            connection_timeout_secs: 30,
            idle_timeout_secs: 300,
            max_lifetime_secs: 3600,
        }
    }
}
