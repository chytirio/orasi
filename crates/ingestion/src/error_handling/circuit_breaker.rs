//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Circuit breaker implementation for fault tolerance
//!
//! This module provides a circuit breaker pattern implementation that can
//! prevent cascading failures and provide fault tolerance.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold before opening circuit
    pub failure_threshold: u32,

    /// Success threshold before closing circuit
    pub success_threshold: u32,

    /// Timeout for circuit breaker operations
    pub timeout_ms: u64,

    /// Half-open timeout in milliseconds
    pub half_open_timeout_ms: u64,

    /// Window size for error counting in milliseconds
    pub window_size_ms: u64,

    /// Enable circuit breaker
    pub enabled: bool,
}

/// Circuit breaker state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CircuitState {
    /// Circuit is closed (normal operation)
    Closed,

    /// Circuit is open (failing, no requests allowed)
    Open,

    /// Circuit is half-open (testing if service is recovered)
    HalfOpen,
}

/// Circuit breaker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStatistics {
    /// Current circuit state
    pub state: CircuitState,

    /// Total requests
    pub total_requests: u64,

    /// Successful requests
    pub successful_requests: u64,

    /// Failed requests
    pub failed_requests: u64,

    /// Current failure count
    pub current_failure_count: u32,

    /// Current success count
    pub current_success_count: u32,

    /// Last state change time
    pub last_state_change: DateTime<Utc>,

    /// Last request time
    pub last_request_time: Option<DateTime<Utc>>,
}

/// Circuit breaker implementation
#[derive(Clone)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    last_success_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    last_state_change: Arc<RwLock<DateTime<Utc>>>,
    total_requests: Arc<RwLock<u64>>,
    successful_requests: Arc<RwLock<u64>>,
    failed_requests: Arc<RwLock<u64>>,
}

impl CircuitBreakerConfig {
    /// Create new circuit breaker configuration
    pub fn new() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_timeout_ms: 30000,
            window_size_ms: 60000,
            enabled: true,
        }
    }

    /// Create configuration with custom thresholds
    pub fn with_thresholds(failure_threshold: u32, success_threshold: u32) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout_ms: 5000,
            half_open_timeout_ms: 30000,
            window_size_ms: 60000,
            enabled: true,
        }
    }
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub async fn new(config: CircuitBreakerConfig) -> BridgeResult<Self> {
        Ok(Self {
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            last_success_time: Arc::new(RwLock::new(None)),
            last_state_change: Arc::new(RwLock::new(Utc::now())),
            total_requests: Arc::new(RwLock::new(0)),
            successful_requests: Arc::new(RwLock::new(0)),
            failed_requests: Arc::new(RwLock::new(0)),
        })
    }

    /// Check if circuit breaker can execute
    pub async fn can_execute(&self) -> BridgeResult<bool> {
        if !self.config.enabled {
            return Ok(true);
        }

        let state = self.state.read().await;
        match *state {
            CircuitState::Closed => Ok(true),
            CircuitState::Open => {
                // Check if enough time has passed to try half-open
                let last_failure = self.last_failure_time.read().await;
                if let Some(last_failure_time) = *last_failure {
                    let timeout = Duration::milliseconds(self.config.half_open_timeout_ms as i64);
                    if Utc::now() - last_failure_time >= timeout {
                        drop(state);
                        self.transition_to_half_open().await?;
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            CircuitState::HalfOpen => Ok(true),
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let now = Utc::now();
        *self.last_success_time.write().await = Some(now);
        *self.successful_requests.write().await += 1;
        *self.total_requests.write().await += 1;

        let mut success_count = self.success_count.write().await;
        *success_count += 1;

        let state = self.state.read().await;
        match *state {
            CircuitState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            CircuitState::HalfOpen => {
                // Check if we have enough successes to close circuit
                if *success_count >= self.config.success_threshold {
                    drop(state);
                    self.transition_to_closed().await?;
                }
            }
            CircuitState::Open => {
                // Should not happen, but handle gracefully
                warn!("Received success while circuit is open");
            }
        }

        Ok(())
    }

    /// Record a failed operation
    pub async fn record_error(&self) -> BridgeResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let now = Utc::now();
        *self.last_failure_time.write().await = Some(now);
        *self.failed_requests.write().await += 1;
        *self.total_requests.write().await += 1;

        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;

        let state = self.state.read().await;
        match *state {
            CircuitState::Closed => {
                // Check if we should open circuit
                if *failure_count >= self.config.failure_threshold {
                    drop(state);
                    self.transition_to_open().await?;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open state opens circuit
                drop(state);
                self.transition_to_open().await?;
            }
            CircuitState::Open => {
                // Already open, just update failure count
            }
        }

        // Reset success count on failure
        *self.success_count.write().await = 0;

        Ok(())
    }

    /// Transition to closed state
    async fn transition_to_closed(&self) -> BridgeResult<()> {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;
        drop(state);

        *self.last_state_change.write().await = Utc::now();
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;

        info!("Circuit breaker transitioned to CLOSED state");
        Ok(())
    }

    /// Transition to open state
    async fn transition_to_open(&self) -> BridgeResult<()> {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;
        drop(state);

        *self.last_state_change.write().await = Utc::now();

        warn!("Circuit breaker transitioned to OPEN state");
        Ok(())
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) -> BridgeResult<()> {
        let mut state = self.state.write().await;
        *state = CircuitState::HalfOpen;
        drop(state);

        *self.last_state_change.write().await = Utc::now();
        *self.success_count.write().await = 0;

        info!("Circuit breaker transitioned to HALF-OPEN state");
        Ok(())
    }

    /// Get current circuit state
    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }

    /// Get circuit breaker statistics
    pub async fn get_statistics(&self) -> BridgeResult<CircuitBreakerStatistics> {
        let state = self.state.read().await;
        let failure_count = self.failure_count.read().await;
        let success_count = self.success_count.read().await;
        let last_failure_time = self.last_failure_time.read().await;
        let last_success_time = self.last_success_time.read().await;
        let last_state_change = self.last_state_change.read().await;
        let total_requests = self.total_requests.read().await;
        let successful_requests = self.successful_requests.read().await;
        let failed_requests = self.failed_requests.read().await;

        Ok(CircuitBreakerStatistics {
            state: state.clone(),
            total_requests: *total_requests,
            successful_requests: *successful_requests,
            failed_requests: *failed_requests,
            current_failure_count: *failure_count,
            current_success_count: *success_count,
            last_state_change: *last_state_change,
            last_request_time: if *total_requests > 0 {
                Some(last_failure_time.unwrap_or(last_success_time.unwrap_or(Utc::now())))
            } else {
                None
            },
        })
    }

    /// Reset circuit breaker to closed state
    pub async fn reset(&self) -> BridgeResult<()> {
        self.transition_to_closed().await
    }

    /// Force circuit breaker to open state
    pub async fn force_open(&self) -> BridgeResult<()> {
        self.transition_to_open().await
    }

    /// Check if circuit breaker is healthy
    pub async fn is_healthy(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, CircuitState::Closed)
    }

    /// Get failure rate
    pub async fn get_failure_rate(&self) -> f64 {
        let total_requests = self.total_requests.read().await;
        let failed_requests = self.failed_requests.read().await;

        if *total_requests == 0 {
            0.0
        } else {
            *failed_requests as f64 / *total_requests as f64
        }
    }

    /// Clean up old error records
    pub async fn cleanup_old_records(&self) -> BridgeResult<()> {
        let now = Utc::now();
        let window_size = Duration::milliseconds(self.config.window_size_ms as i64);

        // Check if we should reset failure count based on time window
        let last_failure = self.last_failure_time.read().await;
        if let Some(last_failure_time) = *last_failure {
            if now - last_failure_time >= window_size {
                *self.failure_count.write().await = 0;
            }
        }

        Ok(())
    }
}

/// Circuit breaker guard for automatic state management
pub struct CircuitBreakerGuard {
    circuit_breaker: CircuitBreaker,
    operation_name: String,
}

impl CircuitBreakerGuard {
    /// Create new circuit breaker guard
    pub async fn new(
        circuit_breaker: CircuitBreaker,
        operation_name: String,
    ) -> BridgeResult<Self> {
        Ok(Self {
            circuit_breaker,
            operation_name,
        })
    }

    /// Execute operation with circuit breaker protection
    pub async fn execute<T, F, Fut>(&self, operation: F) -> BridgeResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = BridgeResult<T>>,
    {
        // Check if circuit breaker allows execution
        if !self.circuit_breaker.can_execute().await? {
            return Err(bridge_core::BridgeError::circuit_breaker(format!(
                "Circuit breaker is open for operation: {}",
                self.operation_name
            )));
        }

        // Execute the operation
        match operation().await {
            Ok(result) => {
                // Record success
                self.circuit_breaker.record_success().await?;
                Ok(result)
            }
            Err(error) => {
                // Record failure
                self.circuit_breaker.record_error().await?;
                Err(error)
            }
        }
    }
}

impl Drop for CircuitBreakerGuard {
    fn drop(&mut self) {
        // Note: We don't spawn background tasks in Drop to avoid hanging tests
        // Cleanup should be done explicitly when needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_creation() {
        let config = CircuitBreakerConfig::new();
        let circuit_breaker = CircuitBreaker::new(config).await.unwrap();

        assert!(circuit_breaker.can_execute().await.unwrap());
        assert_eq!(circuit_breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_success() {
        let config = CircuitBreakerConfig::with_thresholds(3, 2);
        let circuit_breaker = CircuitBreaker::new(config).await.unwrap();

        // Record some successes
        circuit_breaker.record_success().await.unwrap();
        circuit_breaker.record_success().await.unwrap();

        assert!(circuit_breaker.can_execute().await.unwrap());
        assert_eq!(circuit_breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_failure() {
        let config = CircuitBreakerConfig::with_thresholds(2, 1);
        let circuit_breaker = CircuitBreaker::new(config).await.unwrap();

        // Record failures until circuit opens
        circuit_breaker.record_error().await.unwrap();
        circuit_breaker.record_error().await.unwrap();

        assert!(!circuit_breaker.can_execute().await.unwrap());
        assert_eq!(circuit_breaker.get_state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            half_open_timeout_ms: 100, // Short timeout for testing
            ..CircuitBreakerConfig::new()
        };
        let circuit_breaker = CircuitBreaker::new(config).await.unwrap();

        // Open circuit
        circuit_breaker.record_error().await.unwrap();
        assert_eq!(circuit_breaker.get_state().await, CircuitState::Open);

        // Wait for half-open timeout
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // Should be able to execute in half-open state
        assert!(circuit_breaker.can_execute().await.unwrap());
        assert_eq!(circuit_breaker.get_state().await, CircuitState::HalfOpen);
    }
}
