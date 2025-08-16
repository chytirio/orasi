//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Circuit breaker implementation for the OpenTelemetry Data Lake Bridge

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

use crate::error::{BridgeError, BridgeResult};

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold
    pub failure_threshold: u32,

    /// Success threshold
    pub success_threshold: u32,

    /// Timeout in milliseconds
    pub timeout_ms: u64,

    /// Half-open timeout in milliseconds
    pub half_open_timeout_ms: u64,

    /// Enable monitoring
    pub enable_monitoring: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_timeout_ms: 30000,
            enable_monitoring: true,
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
    stats: Arc<RwLock<CircuitBreakerStats>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Current state
    pub state: CircuitBreakerState,

    /// Total requests
    pub total_requests: u64,

    /// Successful requests
    pub successful_requests: u64,

    /// Failed requests
    pub failed_requests: u64,

    /// Success rate
    pub success_rate: f64,

    /// Last state change timestamp
    pub last_state_change: Option<chrono::DateTime<chrono::Utc>>,

    /// Last request timestamp
    pub last_request_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for CircuitBreakerStats {
    fn default() -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            success_rate: 1.0,
            last_state_change: None,
            last_request_time: None,
        }
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            stats: Arc::new(RwLock::new(CircuitBreakerStats::default())),
            last_failure_time: Arc::new(RwLock::new(None)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Check if circuit breaker is open
    pub async fn is_open(&self) -> bool {
        matches!(*self.state.read().await, CircuitBreakerState::Open)
    }

    /// Check if circuit breaker is half-open
    pub async fn is_half_open(&self) -> bool {
        matches!(*self.state.read().await, CircuitBreakerState::HalfOpen)
    }

    /// Check if circuit breaker is closed
    pub async fn is_closed(&self) -> bool {
        matches!(*self.state.read().await, CircuitBreakerState::Closed)
    }

    /// Execute a function with circuit breaker logic
    pub async fn execute<F, Fut, T>(&self, operation: F) -> BridgeResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = BridgeResult<T>> + Send,
    {
        let current_state = self.state.read().await.clone();

        match current_state {
            CircuitBreakerState::Open => {
                if self.should_attempt_reset().await {
                    self.transition_to_half_open().await;
                } else {
                    return Err(BridgeError::circuit_breaker("Circuit breaker is open"));
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Allow the operation to proceed
            }
            CircuitBreakerState::Closed => {
                // Allow the operation to proceed
            }
        }

        let start_time = Instant::now();
        let result = operation().await;

        self.update_stats(result.is_ok()).await;

        match result {
            Ok(value) => {
                self.record_success().await;
                Ok(value)
            }
            Err(e) => {
                self.record_failure().await;
                Err(e)
            }
        }
    }

    /// Record success
    pub async fn record_success(&self) -> BridgeResult<()> {
        let mut success_count = self.success_count.write().await;
        *success_count += 1;

        let mut failure_count = self.failure_count.write().await;
        *failure_count = 0;

        let current_state = self.state.read().await.clone();

        match current_state {
            CircuitBreakerState::HalfOpen => {
                if *success_count >= self.config.success_threshold {
                    self.transition_to_closed().await;
                }
            }
            CircuitBreakerState::Closed => {
                // Stay closed
            }
            CircuitBreakerState::Open => {
                // Should not happen, but transition to half-open
                self.transition_to_half_open().await;
            }
        }

        Ok(())
    }

    /// Record failure
    pub async fn record_failure(&self) -> BridgeResult<()> {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;

        let mut success_count = self.success_count.write().await;
        *success_count = 0;

        let mut last_failure_time = self.last_failure_time.write().await;
        *last_failure_time = Some(Instant::now());

        let current_state = self.state.read().await.clone();

        match current_state {
            CircuitBreakerState::Closed => {
                if *failure_count >= self.config.failure_threshold {
                    self.transition_to_open().await;
                }
            }
            CircuitBreakerState::HalfOpen => {
                self.transition_to_open().await;
            }
            CircuitBreakerState::Open => {
                // Stay open
            }
        }

        Ok(())
    }

    /// Check if circuit breaker should attempt reset
    async fn should_attempt_reset(&self) -> bool {
        let last_failure = self.last_failure_time.read().await;
        if let Some(failure_time) = *last_failure {
            let elapsed = failure_time.elapsed();
            elapsed >= Duration::from_millis(self.config.half_open_timeout_ms)
        } else {
            false
        }
    }

    /// Transition to open state
    async fn transition_to_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::Open;

        let mut stats = self.stats.write().await;
        stats.state = CircuitBreakerState::Open;
        stats.last_state_change = Some(chrono::Utc::now());

        info!("Circuit breaker transitioned to OPEN state");
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::HalfOpen;

        let mut stats = self.stats.write().await;
        stats.state = CircuitBreakerState::HalfOpen;
        stats.last_state_change = Some(chrono::Utc::now());

        info!("Circuit breaker transitioned to HALF-OPEN state");
    }

    /// Transition to closed state
    async fn transition_to_closed(&self) {
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::Closed;

        let mut stats = self.stats.write().await;
        stats.state = CircuitBreakerState::Closed;
        stats.last_state_change = Some(chrono::Utc::now());

        info!("Circuit breaker transitioned to CLOSED state");
    }

    /// Update statistics
    async fn update_stats(&self, success: bool) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.last_request_time = Some(chrono::Utc::now());

        if success {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        if stats.total_requests > 0 {
            stats.success_rate = stats.successful_requests as f64 / stats.total_requests as f64;
        }
    }

    /// Get circuit breaker statistics
    pub async fn get_stats(&self) -> CircuitBreakerStats {
        self.stats.read().await.clone()
    }

    /// Reset circuit breaker to closed state
    pub async fn reset(&self) -> BridgeResult<()> {
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::Closed;

        let mut failure_count = self.failure_count.write().await;
        *failure_count = 0;

        let mut success_count = self.success_count.write().await;
        *success_count = 0;

        let mut last_failure_time = self.last_failure_time.write().await;
        *last_failure_time = None;

        let mut stats = self.stats.write().await;
        stats.state = CircuitBreakerState::Closed;
        stats.last_state_change = Some(chrono::Utc::now());

        info!("Circuit breaker reset to CLOSED state");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_creation() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new(config);
        assert!(breaker.is_closed().await);
    }

    #[tokio::test]
    async fn test_successful_operation() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new(config);

        let result = breaker
            .execute(|| async { Ok::<i32, BridgeError>(42) })
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_failure_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // First failure
        let result = breaker
            .execute(|| async { Err::<i32, BridgeError>(BridgeError::internal("test error")) })
            .await;
        assert!(result.is_err());
        assert!(breaker.is_closed().await);

        // Second failure - should open circuit
        let result = breaker
            .execute(|| async { Err::<i32, BridgeError>(BridgeError::internal("test error")) })
            .await;
        assert!(result.is_err());
        assert!(breaker.is_open().await);
    }
}
