//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka error handling
//! 
//! This module provides error types and handling for Kafka operations.

use std::fmt;
use thiserror::Error;

/// Kafka error types
#[derive(Error, Debug)]
pub enum KafkaError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Connection errors
    #[error("Connection error: {0}")]
    Connection(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Producer errors
    #[error("Producer error: {0}")]
    Producer(String),

    /// Consumer errors
    #[error("Consumer error: {0}")]
    Consumer(String),

    /// Topic errors
    #[error("Topic error: {0}")]
    Topic(String),

    /// Cluster errors
    #[error("Cluster error: {0}")]
    Cluster(String),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization errors
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Timeout errors
    #[error("Timeout error: {0}")]
    Timeout(String),

    /// Resource errors
    #[error("Resource error: {0}")]
    Resource(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Internal errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// Unknown errors
    #[error("Unknown error: {0}")]
    Unknown(String),

    /// Wrapped errors from underlying libraries
    #[error("Wrapped error: {0}")]
    Wrapped(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl KafkaError {
    /// Create a configuration error
    pub fn configuration(message: &str) -> Self {
        KafkaError::Configuration(message.to_string())
    }

    /// Create a connection error
    pub fn connection(message: &str) -> Self {
        KafkaError::Connection(message.to_string())
    }

    /// Create an authentication error
    pub fn authentication(message: &str) -> Self {
        KafkaError::Authentication(message.to_string())
    }

    /// Create a producer error
    pub fn producer(message: &str) -> Self {
        KafkaError::Producer(message.to_string())
    }

    /// Create a consumer error
    pub fn consumer(message: &str) -> Self {
        KafkaError::Consumer(message.to_string())
    }

    /// Create a topic error
    pub fn topic(message: &str) -> Self {
        KafkaError::Topic(message.to_string())
    }

    /// Create a cluster error
    pub fn cluster(message: &str) -> Self {
        KafkaError::Cluster(message.to_string())
    }

    /// Create a serialization error
    pub fn serialization(message: &str) -> Self {
        KafkaError::Serialization(message.to_string())
    }

    /// Create a deserialization error
    pub fn deserialization(message: &str) -> Self {
        KafkaError::Deserialization(message.to_string())
    }

    /// Create a timeout error
    pub fn timeout(message: &str) -> Self {
        KafkaError::Timeout(message.to_string())
    }

    /// Create a resource error
    pub fn resource(message: &str) -> Self {
        KafkaError::Resource(message.to_string())
    }

    /// Create a validation error
    pub fn validation(message: &str) -> Self {
        KafkaError::Validation(message.to_string())
    }

    /// Create an internal error
    pub fn internal(message: &str) -> Self {
        KafkaError::Internal(message.to_string())
    }

    /// Create an unknown error
    pub fn unknown(message: &str) -> Self {
        KafkaError::Unknown(message.to_string())
    }

    /// Create an error with source
    pub fn with_source(message: &str, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        KafkaError::Wrapped(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}: {}", message, source))))
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            KafkaError::Connection(_) => true,
            KafkaError::Timeout(_) => true,
            KafkaError::Resource(_) => true,
            KafkaError::Producer(_) => true,
            KafkaError::Consumer(_) => true,
            _ => false,
        }
    }

    /// Check if the error is transient
    pub fn is_transient(&self) -> bool {
        match self {
            KafkaError::Connection(_) => true,
            KafkaError::Timeout(_) => true,
            KafkaError::Resource(_) => true,
            KafkaError::Producer(_) => true,
            KafkaError::Consumer(_) => true,
            KafkaError::Internal(_) => true,
            _ => false,
        }
    }

    /// Get error code if available
    pub fn error_code(&self) -> Option<u32> {
        match self {
            KafkaError::Connection(_) => Some(2001),
            KafkaError::Authentication(_) => Some(2002),
            KafkaError::Producer(_) => Some(2003),
            KafkaError::Consumer(_) => Some(2004),
            KafkaError::Topic(_) => Some(2005),
            KafkaError::Cluster(_) => Some(2006),
            KafkaError::Serialization(_) => Some(2007),
            KafkaError::Deserialization(_) => Some(2008),
            KafkaError::Timeout(_) => Some(2009),
            KafkaError::Resource(_) => Some(2010),
            KafkaError::Validation(_) => Some(2011),
            KafkaError::Internal(_) => Some(2012),
            KafkaError::Unknown(_) => Some(2099),
            KafkaError::Configuration(_) => Some(2100),
            KafkaError::Wrapped(_) => Some(2999),
        }
    }

    /// Get error category
    pub fn category(&self) -> &'static str {
        match self {
            KafkaError::Configuration(_) => "configuration",
            KafkaError::Connection(_) => "connection",
            KafkaError::Authentication(_) => "authentication",
            KafkaError::Producer(_) => "producer",
            KafkaError::Consumer(_) => "consumer",
            KafkaError::Topic(_) => "topic",
            KafkaError::Cluster(_) => "cluster",
            KafkaError::Serialization(_) => "serialization",
            KafkaError::Deserialization(_) => "deserialization",
            KafkaError::Timeout(_) => "timeout",
            KafkaError::Resource(_) => "resource",
            KafkaError::Validation(_) => "validation",
            KafkaError::Internal(_) => "internal",
            KafkaError::Unknown(_) => "unknown",
            KafkaError::Wrapped(_) => "wrapped",
        }
    }
}



/// Kafka result type
pub type KafkaResult<T> = Result<T, KafkaError>;

/// Kafka error context
#[derive(Debug, Clone)]
pub struct KafkaErrorContext {
    /// Error code
    pub code: Option<u32>,
    /// Error category
    pub category: String,
    /// Error message
    pub message: String,
    /// Error details
    pub details: Option<String>,
    /// Timestamp when error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional context
    pub context: std::collections::HashMap<String, String>,
}

impl KafkaErrorContext {
    /// Create a new error context
    pub fn new(error: &KafkaError) -> Self {
        Self {
            code: error.error_code(),
            category: error.category().to_string(),
            message: error.to_string(),
            details: None,
            timestamp: chrono::Utc::now(),
            context: std::collections::HashMap::new(),
        }
    }

    /// Add context information
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Add details
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

impl From<KafkaError> for KafkaErrorContext {
    fn from(error: KafkaError) -> Self {
        KafkaErrorContext::new(&error)
    }
}
