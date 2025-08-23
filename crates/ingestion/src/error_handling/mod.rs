//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error handling system for managing errors, retries, circuit breakers, and dead letter queues
//!
//! This module provides comprehensive error handling capabilities including
//! circuit breakers, retry policies, dead letter queues, and error classification.

pub mod circuit_breaker;
pub mod dead_letter_queue;
pub mod error_classification;
pub mod retry_policy;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use dead_letter_queue::{DeadLetterQueue, DeadLetterQueueConfig, DeadLetterRecord};
pub use error_classification::{
    ErrorClassification, ErrorClassificationConfig, ErrorClassifier, ErrorSeverity, ErrorType,
};
pub use retry_policy::{RetryPolicy, RetryPolicyConfig, RetryStrategy};

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Error context for tracking error information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Source of the error
    pub source: String,

    /// Retry count
    pub retry_count: u32,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Error handling configuration
#[derive(Debug, Clone)]
pub struct ErrorHandlingConfig {
    /// Circuit breaker configuration
    pub circuit_breaker: circuit_breaker::CircuitBreakerConfig,

    /// Retry policy configuration
    pub retry_policy: retry_policy::RetryPolicyConfig,

    /// Error classification configuration
    pub error_classification: ErrorClassificationConfig,

    /// Dead letter queue configuration
    pub dead_letter_queue: dead_letter_queue::DeadLetterQueueConfig,
}

/// Error handler for managing error handling strategies
pub struct ErrorHandler {
    /// Circuit breakers by operation
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,

    /// Retry policies by operation
    retry_policies: Arc<RwLock<HashMap<String, RetryPolicy>>>,

    /// Error classifier
    error_classifier: Arc<RwLock<ErrorClassifier>>,

    /// Dead letter queue
    dead_letter_queue: Arc<DeadLetterQueue>,

    /// Configuration
    config: ErrorHandlingConfig,

    /// Total errors handled counter
    total_errors_handled: Arc<RwLock<u64>>,
}

impl ErrorHandler {
    /// Create new error handler
    pub async fn new(config: ErrorHandlingConfig) -> BridgeResult<Self> {
        let error_classifier = ErrorClassifier::new(config.error_classification.clone());
        let dead_letter_queue = DeadLetterQueue::new(config.dead_letter_queue.clone()).await?;

        Ok(Self {
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            retry_policies: Arc::new(RwLock::new(HashMap::new())),
            error_classifier: Arc::new(RwLock::new(error_classifier)),
            dead_letter_queue: Arc::new(dead_letter_queue),
            config,
            total_errors_handled: Arc::new(RwLock::new(0)),
        })
    }

    /// Handle error with retry and circuit breaker logic
    pub async fn handle_error<T>(
        &self,
        operation: &str,
        error: &(dyn std::error::Error + Send + Sync),
        context: &ErrorContext,
    ) -> BridgeResult<T> {
        // Increment the total errors handled counter
        self.increment_error_counter().await;

        // Classify the error
        let classification = {
            let mut classifier = self.error_classifier.write().await;
            classifier.classify_error(error).await?
        };

        // Check circuit breaker
        let circuit_breaker = self.get_or_create_circuit_breaker(operation).await?;
        if !circuit_breaker.can_execute().await? {
            return Err(bridge_core::BridgeError::circuit_breaker(format!(
                "Circuit breaker is open for operation: {}",
                operation
            )));
        }

        // Check retry policy
        let retry_policy = self.get_or_create_retry_policy(operation).await?;
        if context.retry_count >= retry_policy.max_attempts() {
            // Max retries exceeded, send to dead letter queue
            self.send_to_dead_letter_queue(operation, error, context)
                .await?;
            return Err(bridge_core::BridgeError::timeout(format!(
                "Max retries exceeded for operation: {}",
                operation
            )));
        }

        // Record error in circuit breaker
        circuit_breaker.record_error().await?;

        // Determine if we should retry
        if retry_policy
            .should_retry(&classification, context.retry_count)
            .await?
        {
            let delay = retry_policy.calculate_delay(context.retry_count).await?;
            warn!(
                "Retrying operation {} after {}ms (attempt {})",
                operation,
                delay.as_millis(),
                context.retry_count + 1
            );
            tokio::time::sleep(delay).await;
            return Err(bridge_core::BridgeError::timeout(format!(
                "Retrying operation: {}",
                operation
            )));
        }

        // Non-retryable error, send to dead letter queue
        self.send_to_dead_letter_queue(operation, error, context)
            .await?;

        Err(bridge_core::BridgeError::validation(format!(
            "Non-retryable error for operation: {}",
            operation
        )))
    }

    /// Get or create circuit breaker for operation
    async fn get_or_create_circuit_breaker(&self, operation: &str) -> BridgeResult<CircuitBreaker> {
        let mut circuit_breakers = self.circuit_breakers.write().await;

        if let Some(circuit_breaker) = circuit_breakers.get(operation) {
            Ok(circuit_breaker.clone())
        } else {
            let circuit_breaker = CircuitBreaker::new(self.config.circuit_breaker.clone()).await?;
            circuit_breakers.insert(operation.to_string(), circuit_breaker.clone());
            Ok(circuit_breaker)
        }
    }

    /// Get or create retry policy for operation
    async fn get_or_create_retry_policy(&self, operation: &str) -> BridgeResult<RetryPolicy> {
        let mut retry_policies = self.retry_policies.write().await;

        if let Some(retry_policy) = retry_policies.get(operation) {
            Ok(retry_policy.clone())
        } else {
            let retry_policy = RetryPolicy::new(self.config.retry_policy.clone()).await?;
            retry_policies.insert(operation.to_string(), retry_policy.clone());
            Ok(retry_policy)
        }
    }

    /// Send error to dead letter queue
    async fn send_to_dead_letter_queue(
        &self,
        operation: &str,
        error: &(dyn std::error::Error + Send + Sync),
        context: &ErrorContext,
    ) -> BridgeResult<()> {
        let dead_letter_record = DeadLetterRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            operation: operation.to_string(),
            error_message: error.to_string(),
            error_type: "unknown".to_string(),
            source: context.source.clone(),
            retry_count: context.retry_count,
            metadata: context.metadata.clone(),
        };

        self.dead_letter_queue
            .add_record(dead_letter_record)
            .await?;
        info!(
            "Error sent to dead letter queue for operation: {}",
            operation
        );
        Ok(())
    }

    /// Record successful operation
    pub async fn record_success(&self, operation: &str) -> BridgeResult<()> {
        let circuit_breaker = self.get_or_create_circuit_breaker(operation).await?;
        circuit_breaker.record_success().await?;
        Ok(())
    }

    /// Increment the total errors handled counter
    async fn increment_error_counter(&self) {
        let mut counter = self.total_errors_handled.write().await;
        *counter += 1;
    }

    /// Get error handling statistics
    pub async fn get_statistics(&self) -> BridgeResult<ErrorHandlingStatistics> {
        let mut circuit_breaker_stats = HashMap::new();
        let circuit_breakers = self.circuit_breakers.read().await;

        for (operation, circuit_breaker) in circuit_breakers.iter() {
            circuit_breaker_stats
                .insert(operation.clone(), circuit_breaker.get_statistics().await?);
        }

        let dead_letter_queue_stats = self.dead_letter_queue.get_statistics().await?;

        let total_errors = *self.total_errors_handled.read().await;

        Ok(ErrorHandlingStatistics {
            circuit_breaker_stats,
            dead_letter_queue_stats,
            total_errors_handled: total_errors,
        })
    }

    /// Clean up expired records
    pub async fn cleanup_expired_records(&self) -> BridgeResult<()> {
        self.dead_letter_queue.cleanup_expired_records().await?;
        Ok(())
    }
}

/// Error handling statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandlingStatistics {
    /// Circuit breaker statistics by operation
    pub circuit_breaker_stats: HashMap<String, circuit_breaker::CircuitBreakerStatistics>,

    /// Dead letter queue statistics
    pub dead_letter_queue_stats: dead_letter_queue::DeadLetterQueueStatistics,

    /// Total errors handled
    pub total_errors_handled: u64,
}

/// Error handling trait for components
#[async_trait]
pub trait ErrorHandling {
    /// Get error handling manager
    fn error_handling_manager(&self) -> &ErrorHandler;

    /// Handle error with retry logic
    async fn handle_with_retry<T: Send, F, Fut>(
        &self,
        operation: &str,
        context: &ErrorContext,
        f: F,
    ) -> BridgeResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = BridgeResult<T>> + Send,
    {
        let mut retry_count = 0;
        let max_retries = self
            .error_handling_manager()
            .config
            .retry_policy
            .max_attempts;

        loop {
            match f().await {
                Ok(result) => {
                    // Record success
                    self.error_handling_manager()
                        .record_success(operation)
                        .await?;
                    return Ok(result);
                }
                Err(error) => {
                    let mut error_context = context.clone();
                    error_context.retry_count = retry_count;

                    match self
                        .error_handling_manager()
                        .handle_error::<T>(operation, &error, &error_context)
                        .await
                    {
                        Ok(result) => return Ok(result),
                        Err(handled_error) => {
                            if handled_error.is_retryable() && retry_count < max_retries {
                                retry_count += 1;
                                continue;
                            } else {
                                return Err(handled_error);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extension trait for BridgeResult to add error handling
pub trait BridgeResultExt<T> {
    /// Handle error with error handling manager
    async fn handle_with_error_manager(
        self,
        error_manager: &ErrorHandler,
        operation: &str,
        context: &ErrorContext,
    ) -> BridgeResult<T>;
}

impl<T> BridgeResultExt<T> for BridgeResult<T> {
    async fn handle_with_error_manager(
        self,
        error_manager: &ErrorHandler,
        operation: &str,
        context: &ErrorContext,
    ) -> BridgeResult<T> {
        match self {
            Ok(result) => {
                error_manager.record_success(operation).await?;
                Ok(result)
            }
            Err(error) => {
                error_manager
                    .handle_error::<T>(operation, &error, context)
                    .await
            }
        }
    }
}
