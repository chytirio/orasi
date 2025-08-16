//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Retry logic and policies for the OpenTelemetry Data Lake Bridge

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::debug;

use crate::error::{BridgeError, BridgeResult};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,

    /// Maximum backoff delay in milliseconds
    pub max_backoff_ms: u64,

    /// Backoff multiplier
    pub backoff_multiplier: f64,

    /// Enable exponential backoff
    pub enable_exponential_backoff: bool,

    /// Enable jitter
    pub enable_jitter: bool,

    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
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
            jitter_factor: 0.1,
        }
    }
}

/// Retry policy implementation
pub struct RetryPolicy {
    config: RetryConfig,
    stats: Arc<RwLock<RetryStats>>,
}

/// Retry statistics
#[derive(Debug, Clone)]
pub struct RetryStats {
    /// Total retry attempts
    pub total_retries: u64,

    /// Successful retries
    pub successful_retries: u64,

    /// Failed retries
    pub failed_retries: u64,

    /// Average retry attempts per operation
    pub avg_retry_attempts: f64,

    /// Last retry timestamp
    pub last_retry_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for RetryStats {
    fn default() -> Self {
        Self {
            total_retries: 0,
            successful_retries: 0,
            failed_retries: 0,
            avg_retry_attempts: 0.0,
            last_retry_time: None,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(RetryStats::default())),
        }
    }

    /// Execute a function with retry logic
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> BridgeResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        E: std::error::Error + Send + Sync + 'static,
    {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < self.config.max_attempts {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        self.update_stats(true).await;
                    }
                    return Ok(result);
                }
                Err(e) => {
                    attempt += 1;
                    last_error = Some(e);

                    if attempt < self.config.max_attempts {
                        let delay = self.calculate_delay(attempt);
                        debug!("Retry attempt {} after {:?} delay", attempt, delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        self.update_stats(false).await;
        Err(BridgeError::internal_with_source(
            "Operation failed after maximum retry attempts",
            last_error.unwrap(),
        ))
    }

    /// Calculate delay for retry attempt
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = if self.config.enable_exponential_backoff {
            let delay_ms = self.config.initial_backoff_ms as f64
                * self.config.backoff_multiplier.powi((attempt - 1) as i32);
            delay_ms.min(self.config.max_backoff_ms as f64) as u64
        } else {
            self.config.initial_backoff_ms
        };

        let final_delay = if self.config.enable_jitter {
            let jitter = (base_delay as f64
                * self.config.jitter_factor
                * (rand::random::<f64>() * 2.0 - 1.0)) as i64;
            (base_delay as i64 + jitter).max(1) as u64
        } else {
            base_delay
        };

        Duration::from_millis(final_delay)
    }

    /// Update retry statistics
    async fn update_stats(&self, success: bool) {
        let mut stats = self.stats.write().await;
        stats.total_retries += 1;

        if success {
            stats.successful_retries += 1;
        } else {
            stats.failed_retries += 1;
        }

        stats.avg_retry_attempts =
            stats.total_retries as f64 / (stats.successful_retries + stats.failed_retries) as f64;
        stats.last_retry_time = Some(chrono::Utc::now());
    }

    /// Get retry statistics
    pub async fn get_stats(&self) -> RetryStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_policy_creation() {
        let config = RetryConfig::default();
        let policy = RetryPolicy::new(config);
        let stats = policy.get_stats().await;
        assert_eq!(stats.total_retries, 0);
    }

    #[tokio::test]
    async fn test_successful_operation() {
        let config = RetryConfig::default();
        let policy = RetryPolicy::new(config);

        let result = policy
            .execute(|| async { Ok::<i32, std::io::Error>(42) })
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_failed_operation() {
        let config = RetryConfig {
            max_attempts: 2,
            ..Default::default()
        };
        let policy = RetryPolicy::new(config);

        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let result = policy
            .execute(move || {
                let attempts = attempts_clone.clone();
                async move {
                    let current_attempt =
                        attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if current_attempt < 1 {
                        Err(std::io::Error::new(std::io::ErrorKind::Other, "test error"))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
