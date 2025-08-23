//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Retry policy implementation for resilient operations
//!
//! This module provides a retry policy system with different strategies
//! and backoff mechanisms for handling transient failures.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration as StdDuration;
use tracing::{info, warn};

use super::error_classification::{ErrorClassification, ErrorSeverity, ErrorType};

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicyConfig {
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

    /// Retry strategy
    pub retry_strategy: RetryStrategy,

    /// Retryable error types
    pub retryable_error_types: Vec<ErrorType>,

    /// Non-retryable error types
    pub non_retryable_error_types: Vec<ErrorType>,

    /// Retryable error messages (regex patterns)
    pub retryable_error_patterns: Vec<String>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Retry strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed,

    /// Exponential backoff
    Exponential,

    /// Linear backoff
    Linear,

    /// Custom backoff function
    Custom,
}

/// Retry attempt information
#[derive(Debug, Clone)]
pub struct RetryAttempt {
    /// Attempt number (1-based)
    pub attempt_number: u32,

    /// Total attempts allowed
    pub max_attempts: u32,

    /// Delay before this attempt
    pub delay: StdDuration,

    /// Error that caused the retry
    pub error: String,

    /// Error classification
    pub error_classification: ErrorClassification,

    /// Timestamp of the attempt
    pub timestamp: DateTime<Utc>,
}

/// Retry policy implementation
#[derive(Clone)]
pub struct RetryPolicy {
    config: RetryPolicyConfig,
    attempt_history: Vec<RetryAttempt>,
}

impl RetryPolicyConfig {
    /// Create new retry policy configuration
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
            backoff_multiplier: 2.0,
            enable_exponential_backoff: true,
            enable_jitter: true,
            jitter_factor: 0.1,
            retry_strategy: RetryStrategy::Exponential,
            retryable_error_types: vec![
                ErrorType::Network,
                ErrorType::Timeout,
                ErrorType::Temporary,
            ],
            non_retryable_error_types: vec![
                ErrorType::Authentication,
                ErrorType::Authorization,
                ErrorType::Validation,
                ErrorType::Permanent,
            ],
            retryable_error_patterns: vec![
                r"timeout".to_string(),
                r"connection.*refused".to_string(),
                r"network.*error".to_string(),
                r"temporary.*failure".to_string(),
            ],
            additional_config: HashMap::new(),
        }
    }

    /// Create configuration with custom attempts
    pub fn with_attempts(max_attempts: u32) -> Self {
        let mut config = Self::new();
        config.max_attempts = max_attempts;
        config
    }

    /// Create configuration with exponential backoff
    pub fn with_exponential_backoff(
        max_attempts: u32,
        initial_backoff_ms: u64,
        max_backoff_ms: u64,
    ) -> Self {
        let mut config = Self::new();
        config.max_attempts = max_attempts;
        config.initial_backoff_ms = initial_backoff_ms;
        config.max_backoff_ms = max_backoff_ms;
        config.retry_strategy = RetryStrategy::Exponential;
        config
    }
}

impl RetryPolicy {
    /// Create new retry policy
    pub async fn new(config: RetryPolicyConfig) -> BridgeResult<Self> {
        Ok(Self {
            config,
            attempt_history: Vec::new(),
        })
    }

    /// Check if error should be retried
    pub async fn should_retry(
        &self,
        error_classification: &ErrorClassification,
        current_attempt: u32,
    ) -> BridgeResult<bool> {
        // Check if we've exceeded max attempts
        if current_attempt >= self.config.max_attempts {
            return Ok(false);
        }

        // Check error type
        if self
            .config
            .non_retryable_error_types
            .contains(&error_classification.error_type)
        {
            return Ok(false);
        }

        // Check if error type is retryable
        if self
            .config
            .retryable_error_types
            .contains(&error_classification.error_type)
        {
            return Ok(true);
        }

        // Check error severity
        match error_classification.severity {
            ErrorSeverity::Critical | ErrorSeverity::Fatal => Ok(false),
            ErrorSeverity::Error | ErrorSeverity::Warning => Ok(true),
            ErrorSeverity::Info | ErrorSeverity::Debug => Ok(false),
        }
    }

    /// Calculate delay for next retry attempt
    pub async fn calculate_delay(&self, current_attempt: u32) -> BridgeResult<StdDuration> {
        let base_delay = match self.config.retry_strategy {
            RetryStrategy::Fixed => self.config.initial_backoff_ms,
            RetryStrategy::Linear => self.config.initial_backoff_ms * current_attempt as u64,
            RetryStrategy::Exponential => {
                let exponential_delay = (self.config.initial_backoff_ms as f64)
                    * self
                        .config
                        .backoff_multiplier
                        .powi((current_attempt - 1) as i32);
                exponential_delay.min(self.config.max_backoff_ms as f64) as u64
            }
            RetryStrategy::Custom => {
                // Custom backoff logic
                self.config.initial_backoff_ms * current_attempt as u64
            }
        };

        let final_delay = if self.config.enable_jitter {
            self.add_jitter(base_delay)
        } else {
            base_delay
        };

        Ok(StdDuration::from_millis(final_delay))
    }

    /// Add jitter to delay
    fn add_jitter(&self, delay: u64) -> u64 {
        let mut rng = rand::thread_rng();
        let jitter_range = (delay as f64 * self.config.jitter_factor) as u64;
        let jitter = rng.gen_range(0..=jitter_range);

        if rng.gen_bool(0.5) {
            delay + jitter
        } else {
            delay.saturating_sub(jitter)
        }
    }

    /// Record retry attempt
    pub async fn record_attempt(&mut self, attempt: RetryAttempt) {
        self.attempt_history.push(attempt);

        // Keep only recent attempts to prevent memory growth
        if self.attempt_history.len() > 100 {
            self.attempt_history.remove(0);
        }
    }

    /// Get retry statistics
    pub async fn get_statistics(&self) -> BridgeResult<RetryStatistics> {
        let total_attempts = self.attempt_history.len() as u64;
        let successful_retries = self
            .attempt_history
            .iter()
            .filter(|attempt| attempt.attempt_number < attempt.max_attempts)
            .count() as u64;

        let avg_delay = if !self.attempt_history.is_empty() {
            let total_delay: u64 = self
                .attempt_history
                .iter()
                .map(|attempt| attempt.delay.as_millis() as u64)
                .sum();
            total_delay / total_attempts
        } else {
            0
        };

        Ok(RetryStatistics {
            total_attempts,
            successful_retries,
            failed_retries: total_attempts.saturating_sub(successful_retries),
            average_delay_ms: avg_delay,
            last_attempt_time: self.attempt_history.last().map(|a| a.timestamp),
            retry_success_rate: if total_attempts > 0 {
                successful_retries as f64 / total_attempts as f64
            } else {
                0.0
            },
        })
    }

    /// Get maximum attempts
    pub fn max_attempts(&self) -> u32 {
        self.config.max_attempts
    }

    /// Check if error message matches retryable patterns
    pub async fn is_retryable_error_message(&self, error_message: &str) -> bool {
        for pattern in &self.config.retryable_error_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(error_message) {
                    return true;
                }
            }
        }
        false
    }

    /// Reset retry policy
    pub async fn reset(&mut self) {
        self.attempt_history.clear();
    }

    /// Get attempt history
    pub async fn get_attempt_history(&self) -> Vec<RetryAttempt> {
        self.attempt_history.clone()
    }
}

/// Retry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStatistics {
    /// Total retry attempts
    pub total_attempts: u64,

    /// Successful retries
    pub successful_retries: u64,

    /// Failed retries
    pub failed_retries: u64,

    /// Average delay in milliseconds
    pub average_delay_ms: u64,

    /// Last attempt time
    pub last_attempt_time: Option<DateTime<Utc>>,

    /// Retry success rate
    pub retry_success_rate: f64,
}

/// Retry executor for automatic retry logic
pub struct RetryExecutor {
    retry_policy: RetryPolicy,
    operation_name: String,
}

impl RetryExecutor {
    /// Create new retry executor
    pub async fn new(retry_policy: RetryPolicy, operation_name: String) -> BridgeResult<Self> {
        Ok(Self {
            retry_policy,
            operation_name,
        })
    }

    /// Execute operation with retry logic
    pub async fn execute<T, F, Fut>(&mut self, operation: F) -> BridgeResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = BridgeResult<T>> + Send,
    {
        let mut attempt = 1;
        let max_attempts = self.retry_policy.max_attempts();

        loop {
            match operation().await {
                Ok(result) => {
                    info!(
                        "Operation '{}' succeeded on attempt {}",
                        self.operation_name, attempt
                    );
                    return Ok(result);
                }
                Err(error) => {
                    let error_message = error.to_string();

                    // Create error classification
                    let error_classification = ErrorClassification {
                        error_type: self.classify_error_type(&error_message),
                        severity: self.classify_error_severity(&error_message),
                        message: error_message.clone(),
                        timestamp: Utc::now(),
                        metadata: HashMap::new(),
                    };

                    // Check if we should retry
                    if !self
                        .retry_policy
                        .should_retry(&error_classification, attempt)
                        .await?
                    {
                        warn!(
                            "Operation '{}' failed with non-retryable error on attempt {}: {}",
                            self.operation_name, attempt, error_message
                        );
                        return Err(error);
                    }

                    // Check if we've exceeded max attempts
                    if attempt >= max_attempts {
                        warn!(
                            "Operation '{}' failed after {} attempts: {}",
                            self.operation_name, max_attempts, error_message
                        );
                        return Err(error);
                    }

                    // Calculate delay for next attempt
                    let delay = self.retry_policy.calculate_delay(attempt).await?;

                    // Record attempt
                    let retry_attempt = RetryAttempt {
                        attempt_number: attempt,
                        max_attempts,
                        delay,
                        error: error_message.clone(),
                        error_classification,
                        timestamp: Utc::now(),
                    };
                    self.retry_policy.record_attempt(retry_attempt).await;

                    warn!(
                        "Operation '{}' failed on attempt {}, retrying in {}ms: {}",
                        self.operation_name,
                        attempt,
                        delay.as_millis(),
                        error_message
                    );

                    // Wait before retry
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Classify error type based on error message
    fn classify_error_type(&self, error_message: &str) -> ErrorType {
        let lower_message = error_message.to_lowercase();

        if lower_message.contains("timeout") || lower_message.contains("deadline") {
            ErrorType::Timeout
        } else if lower_message.contains("connection") || lower_message.contains("network") {
            ErrorType::Network
        } else if lower_message.contains("auth") || lower_message.contains("unauthorized") {
            ErrorType::Authentication
        } else if lower_message.contains("permission") || lower_message.contains("forbidden") {
            ErrorType::Authorization
        } else if lower_message.contains("validation") || lower_message.contains("invalid") {
            ErrorType::Validation
        } else if lower_message.contains("temporary") || lower_message.contains("retry") {
            ErrorType::Temporary
        } else {
            ErrorType::Unknown
        }
    }

    /// Classify error severity based on error message
    fn classify_error_severity(&self, error_message: &str) -> ErrorSeverity {
        let lower_message = error_message.to_lowercase();

        if lower_message.contains("fatal") || lower_message.contains("critical") {
            ErrorSeverity::Fatal
        } else if lower_message.contains("error") {
            ErrorSeverity::Error
        } else if lower_message.contains("warning") {
            ErrorSeverity::Warning
        } else if lower_message.contains("info") {
            ErrorSeverity::Info
        } else {
            ErrorSeverity::Error // Default to error
        }
    }

    /// Get retry statistics
    pub async fn get_statistics(&self) -> BridgeResult<RetryStatistics> {
        self.retry_policy.get_statistics().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_policy_creation() {
        let config = RetryPolicyConfig::new();
        let retry_policy = RetryPolicy::new(config).await.unwrap();

        assert_eq!(retry_policy.max_attempts(), 3);
    }

    #[tokio::test]
    async fn test_retry_policy_should_retry() {
        let config = RetryPolicyConfig::new();
        let retry_policy = RetryPolicy::new(config).await.unwrap();

        let classification = ErrorClassification {
            error_type: ErrorType::Network,
            severity: ErrorSeverity::Error,
            message: "Connection failed".to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        assert!(retry_policy.should_retry(&classification, 1).await.unwrap());
        assert!(!retry_policy.should_retry(&classification, 3).await.unwrap());
    }

    #[tokio::test]
    async fn test_retry_policy_delay_calculation() {
        let config = RetryPolicyConfig::with_exponential_backoff(3, 1000, 10000);
        let retry_policy = RetryPolicy::new(config).await.unwrap();

        let delay1 = retry_policy.calculate_delay(1).await.unwrap();
        let delay2 = retry_policy.calculate_delay(2).await.unwrap();

        assert!(delay2 > delay1);
    }

    #[tokio::test]
    async fn test_retry_executor() {
        let config = RetryPolicyConfig::with_attempts(2);
        let retry_policy = RetryPolicy::new(config).await.unwrap();
        let mut executor = RetryExecutor::new(retry_policy, "test_operation".to_string())
            .await
            .unwrap();

        let attempts = std::sync::Arc::new(std::sync::Mutex::new(0));
        let attempts_clone = attempts.clone();

        let result = executor
            .execute(move || {
                let attempts = attempts_clone.clone();
                async move {
                    let mut count = attempts.lock().unwrap();
                    *count += 1;
                    if *count < 2 {
                        Err(bridge_core::BridgeError::network("Temporary failure"))
                    } else {
                        Ok("Success")
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(*attempts.lock().unwrap(), 2);
    }
}
