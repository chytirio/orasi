//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error classification system for categorizing and prioritizing errors
//!
//! This module provides error classification capabilities for categorizing
//! errors by type, severity, and other characteristics.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use tracing::{error, info, warn};

/// Error type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ErrorType {
    /// Network-related errors
    Network,

    /// Timeout errors
    Timeout,

    /// Authentication errors
    Authentication,

    /// Authorization errors
    Authorization,

    /// Validation errors
    Validation,

    /// Configuration errors
    Configuration,

    /// Resource errors (memory, disk, etc.)
    Resource,

    /// Temporary errors that may resolve
    Temporary,

    /// Permanent errors that won't resolve
    Permanent,

    /// Unknown error type
    Unknown,
}

/// Error severity classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ErrorSeverity {
    /// Debug level errors
    Debug,

    /// Info level errors
    Info,

    /// Warning level errors
    Warning,

    /// Error level errors
    Error,

    /// Critical level errors
    Critical,

    /// Fatal level errors
    Fatal,
}

/// Error classification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorClassification {
    /// Error type
    pub error_type: ErrorType,

    /// Error severity
    pub severity: ErrorSeverity,

    /// Error message
    pub message: String,

    /// Error timestamp
    pub timestamp: DateTime<Utc>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Error classification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorClassificationConfig {
    /// Error type mappings
    pub error_type_mappings: HashMap<String, ErrorType>,

    /// Error severity mappings
    pub error_severity_mappings: HashMap<String, ErrorSeverity>,

    /// Default error type
    pub default_error_type: ErrorType,

    /// Default error severity
    pub default_error_severity: ErrorSeverity,

    /// Error patterns for classification
    pub error_patterns: Vec<ErrorPattern>,

    /// Enable automatic classification
    pub enable_automatic_classification: bool,
}

/// Error pattern for classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    /// Pattern name
    pub name: String,

    /// Regex pattern
    pub pattern: String,

    /// Error type to assign
    pub error_type: ErrorType,

    /// Error severity to assign
    pub error_severity: ErrorSeverity,

    /// Pattern priority (higher = more specific)
    pub priority: u32,

    /// Whether pattern is enabled
    pub enabled: bool,
}

/// Error classification implementation
pub struct ErrorClassifier {
    config: ErrorClassificationConfig,
    patterns: Vec<ErrorPattern>,
    last_classification_time: Option<DateTime<Utc>>,
}

impl ErrorClassificationConfig {
    /// Create new error classification configuration
    pub fn new() -> Self {
        let mut error_type_mappings = HashMap::new();
        error_type_mappings.insert("network".to_string(), ErrorType::Network);
        error_type_mappings.insert("timeout".to_string(), ErrorType::Timeout);
        error_type_mappings.insert("auth".to_string(), ErrorType::Authentication);
        error_type_mappings.insert("permission".to_string(), ErrorType::Authorization);
        error_type_mappings.insert("validation".to_string(), ErrorType::Validation);
        error_type_mappings.insert("config".to_string(), ErrorType::Configuration);
        error_type_mappings.insert("resource".to_string(), ErrorType::Resource);
        error_type_mappings.insert("temporary".to_string(), ErrorType::Temporary);
        error_type_mappings.insert("permanent".to_string(), ErrorType::Permanent);

        let mut error_severity_mappings = HashMap::new();
        error_severity_mappings.insert("debug".to_string(), ErrorSeverity::Debug);
        error_severity_mappings.insert("info".to_string(), ErrorSeverity::Info);
        error_severity_mappings.insert("warning".to_string(), ErrorSeverity::Warning);
        error_severity_mappings.insert("error".to_string(), ErrorSeverity::Error);
        error_severity_mappings.insert("critical".to_string(), ErrorSeverity::Critical);
        error_severity_mappings.insert("fatal".to_string(), ErrorSeverity::Fatal);

        let error_patterns = vec![
            ErrorPattern {
                name: "network_timeout".to_string(),
                pattern: r"timeout|deadline|connection.*refused|Connection refused".to_string(),
                error_type: ErrorType::Timeout,
                error_severity: ErrorSeverity::Warning,
                priority: 100,
                enabled: true,
            },
            ErrorPattern {
                name: "network_error".to_string(),
                pattern: r"network.*error|connection.*failed".to_string(),
                error_type: ErrorType::Network,
                error_severity: ErrorSeverity::Error,
                priority: 90,
                enabled: true,
            },
            ErrorPattern {
                name: "authentication_error".to_string(),
                pattern: r"unauthorized|authentication.*failed|invalid.*token|Authentication failed|Permission denied".to_string(),
                error_type: ErrorType::Authentication,
                error_severity: ErrorSeverity::Error,
                priority: 80,
                enabled: true,
            },
            ErrorPattern {
                name: "authorization_error".to_string(),
                pattern: r"forbidden|permission.*denied|access.*denied".to_string(),
                error_type: ErrorType::Authorization,
                error_severity: ErrorSeverity::Error,
                priority: 80,
                enabled: true,
            },
            ErrorPattern {
                name: "validation_error".to_string(),
                pattern: r"validation.*failed|invalid.*input|malformed|custom.*error".to_string(),
                error_type: ErrorType::Validation,
                error_severity: ErrorSeverity::Warning,
                priority: 70,
                enabled: true,
            },
            ErrorPattern {
                name: "resource_error".to_string(),
                pattern: r"out.*of.*memory|disk.*full|resource.*exhausted".to_string(),
                error_type: ErrorType::Resource,
                error_severity: ErrorSeverity::Critical,
                priority: 95,
                enabled: true,
            },
            ErrorPattern {
                name: "temporary_error".to_string(),
                pattern: r"temporary.*failure|retry.*later|service.*unavailable".to_string(),
                error_type: ErrorType::Temporary,
                error_severity: ErrorSeverity::Warning,
                priority: 60,
                enabled: true,
            },
            ErrorPattern {
                name: "permanent_error".to_string(),
                pattern: r"permanent.*failure|fatal.*error|unrecoverable".to_string(),
                error_type: ErrorType::Permanent,
                error_severity: ErrorSeverity::Fatal,
                priority: 100,
                enabled: true,
            },
        ];

        Self {
            error_type_mappings,
            error_severity_mappings,
            default_error_type: ErrorType::Unknown,
            default_error_severity: ErrorSeverity::Error,
            error_patterns,
            enable_automatic_classification: true,
        }
    }
}

impl ErrorClassifier {
    /// Create new error classification
    pub fn new(config: ErrorClassificationConfig) -> Self {
        Self {
            patterns: config.error_patterns.clone(),
            config,
            last_classification_time: None,
        }
    }

    /// Classify an error
    pub async fn classify_error(
        &mut self,
        error: &(dyn StdError + Send + Sync),
    ) -> BridgeResult<ErrorClassification> {
        let error_message = error.to_string();
        let lower_message = error_message.to_lowercase();

        // Try pattern-based classification first
        if let Some(classification) = self.classify_by_patterns(&error_message).await? {
            self.last_classification_time = Some(Utc::now());
            return Ok(classification);
        }

        // Try keyword-based classification
        if let Some(classification) = self.classify_by_keywords(&lower_message).await? {
            self.last_classification_time = Some(Utc::now());
            return Ok(classification);
        }

        // Use default classification
        let classification = ErrorClassification {
            error_type: self.config.default_error_type.clone(),
            severity: self.config.default_error_severity.clone(),
            message: error_message,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        self.last_classification_time = Some(Utc::now());
        Ok(classification)
    }

    /// Classify error by patterns
    async fn classify_by_patterns(
        &self,
        error_message: &str,
    ) -> BridgeResult<Option<ErrorClassification>> {
        let mut best_match: Option<(ErrorPattern, usize)> = None;

        for pattern in &self.patterns {
            if !pattern.enabled {
                continue;
            }

            if let Ok(regex) = regex::Regex::new(&pattern.pattern) {
                if let Some(matches) = regex.find(error_message) {
                    let match_length = matches.end() - matches.start();
                    let priority_score = pattern.priority as usize + match_length;

                    if let Some((best_pattern, best_score)) = &best_match {
                        if priority_score > *best_score {
                            best_match = Some((pattern.clone(), priority_score));
                        }
                    } else {
                        best_match = Some((pattern.clone(), priority_score));
                    }
                }
            }
        }

        if let Some((pattern, _)) = best_match {
            Ok(Some(ErrorClassification {
                error_type: pattern.error_type,
                severity: pattern.error_severity,
                message: error_message.to_string(),
                timestamp: Utc::now(),
                metadata: HashMap::from([
                    ("pattern_name".to_string(), pattern.name),
                    ("classification_method".to_string(), "pattern".to_string()),
                ]),
            }))
        } else {
            Ok(None)
        }
    }

    /// Classify error by keywords
    async fn classify_by_keywords(
        &self,
        lower_message: &str,
    ) -> BridgeResult<Option<ErrorClassification>> {
        // Check error type mappings
        for (keyword, error_type) in &self.config.error_type_mappings {
            if lower_message.contains(keyword) {
                return Ok(Some(ErrorClassification {
                    error_type: error_type.clone(),
                    severity: self.determine_severity_from_type(error_type),
                    message: lower_message.to_string(),
                    timestamp: Utc::now(),
                    metadata: HashMap::from([
                        ("classification_method".to_string(), "keyword".to_string()),
                        ("matched_keyword".to_string(), keyword.clone()),
                    ]),
                }));
            }
        }

        // Check error severity mappings
        for (keyword, severity) in &self.config.error_severity_mappings {
            if lower_message.contains(keyword) {
                return Ok(Some(ErrorClassification {
                    error_type: self.determine_type_from_severity(severity),
                    severity: severity.clone(),
                    message: lower_message.to_string(),
                    timestamp: Utc::now(),
                    metadata: HashMap::from([
                        ("classification_method".to_string(), "keyword".to_string()),
                        ("matched_keyword".to_string(), keyword.clone()),
                    ]),
                }));
            }
        }

        Ok(None)
    }

    /// Determine severity from error type
    fn determine_severity_from_type(&self, error_type: &ErrorType) -> ErrorSeverity {
        match error_type {
            ErrorType::Network => ErrorSeverity::Warning,
            ErrorType::Timeout => ErrorSeverity::Warning,
            ErrorType::Authentication => ErrorSeverity::Error,
            ErrorType::Authorization => ErrorSeverity::Error,
            ErrorType::Validation => ErrorSeverity::Warning,
            ErrorType::Configuration => ErrorSeverity::Error,
            ErrorType::Resource => ErrorSeverity::Critical,
            ErrorType::Temporary => ErrorSeverity::Warning,
            ErrorType::Permanent => ErrorSeverity::Fatal,
            ErrorType::Unknown => ErrorSeverity::Error,
        }
    }

    /// Determine type from error severity
    fn determine_type_from_severity(&self, severity: &ErrorSeverity) -> ErrorType {
        match severity {
            ErrorSeverity::Debug => ErrorType::Unknown,
            ErrorSeverity::Info => ErrorType::Unknown,
            ErrorSeverity::Warning => ErrorType::Temporary,
            ErrorSeverity::Error => ErrorType::Unknown,
            ErrorSeverity::Critical => ErrorType::Resource,
            ErrorSeverity::Fatal => ErrorType::Permanent,
        }
    }

    /// Add custom error pattern
    pub async fn add_pattern(&mut self, pattern: ErrorPattern) -> BridgeResult<()> {
        self.patterns.push(pattern);
        // Sort patterns by priority (highest first)
        self.patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(())
    }

    /// Remove error pattern by name
    pub async fn remove_pattern(&mut self, pattern_name: &str) -> BridgeResult<()> {
        self.patterns.retain(|p| p.name != pattern_name);
        Ok(())
    }

    /// Get all patterns
    pub async fn get_patterns(&self) -> Vec<ErrorPattern> {
        self.patterns.clone()
    }

    /// Update pattern
    pub async fn update_pattern(
        &mut self,
        pattern_name: &str,
        updated_pattern: ErrorPattern,
    ) -> BridgeResult<()> {
        if let Some(index) = self.patterns.iter().position(|p| p.name == pattern_name) {
            self.patterns[index] = updated_pattern;
            // Re-sort by priority
            self.patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
            Ok(())
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "Pattern '{}' not found",
                pattern_name
            )))
        }
    }

    /// Get classification statistics
    pub async fn get_statistics(&self) -> BridgeResult<ErrorClassificationStatistics> {
        let mut type_counts = HashMap::new();
        let mut severity_counts = HashMap::new();

        // Initialize counts
        for error_type in [
            ErrorType::Network,
            ErrorType::Timeout,
            ErrorType::Authentication,
            ErrorType::Authorization,
            ErrorType::Validation,
            ErrorType::Configuration,
            ErrorType::Resource,
            ErrorType::Temporary,
            ErrorType::Permanent,
            ErrorType::Unknown,
        ] {
            type_counts.insert(error_type, 0);
        }

        for severity in [
            ErrorSeverity::Debug,
            ErrorSeverity::Info,
            ErrorSeverity::Warning,
            ErrorSeverity::Error,
            ErrorSeverity::Critical,
            ErrorSeverity::Fatal,
        ] {
            severity_counts.insert(severity, 0);
        }

        Ok(ErrorClassificationStatistics {
            total_patterns: self.patterns.len() as u64,
            enabled_patterns: self.patterns.iter().filter(|p| p.enabled).count() as u64,
            type_counts,
            severity_counts,
            last_classification_time: self.last_classification_time,
        })
    }

    /// Get the last classification time
    pub fn get_last_classification_time(&self) -> Option<DateTime<Utc>> {
        self.last_classification_time
    }

    /// Reset the last classification time
    pub fn reset_last_classification_time(&mut self) {
        self.last_classification_time = None;
    }
}

/// Error classification statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorClassificationStatistics {
    /// Total number of patterns
    pub total_patterns: u64,

    /// Number of enabled patterns
    pub enabled_patterns: u64,

    /// Counts by error type
    pub type_counts: HashMap<ErrorType, u64>,

    /// Counts by error severity
    pub severity_counts: HashMap<ErrorSeverity, u64>,

    /// Last classification time
    pub last_classification_time: Option<DateTime<Utc>>,
}

/// Error classification trait for components
#[async_trait]
pub trait ErrorClassifiable {
    /// Get error classification
    fn get_error_classification(&self) -> ErrorClassification;

    /// Check if error is retryable
    fn is_retryable(&self) -> bool {
        let classification = self.get_error_classification();
        matches!(
            classification.error_type,
            ErrorType::Network | ErrorType::Timeout | ErrorType::Temporary
        )
    }

    /// Check if error is critical
    fn is_critical(&self) -> bool {
        let classification = self.get_error_classification();
        matches!(
            classification.severity,
            ErrorSeverity::Critical | ErrorSeverity::Fatal
        )
    }

    /// Get error priority (higher = more important)
    fn get_priority(&self) -> u32 {
        let classification = self.get_error_classification();
        match classification.severity {
            ErrorSeverity::Fatal => 100,
            ErrorSeverity::Critical => 90,
            ErrorSeverity::Error => 70,
            ErrorSeverity::Warning => 50,
            ErrorSeverity::Info => 30,
            ErrorSeverity::Debug => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_classification_creation() {
        let config = ErrorClassificationConfig::new();
        let classifier = ErrorClassifier::new(config);

        assert!(!classifier.patterns.is_empty());
    }

    #[tokio::test]
    async fn test_network_error_classification() {
        let config = ErrorClassificationConfig::new();
        let mut classifier = ErrorClassifier::new(config);

        let error =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused");
        let classification = classifier.classify_error(&error).await.unwrap();

        assert_eq!(classification.error_type, ErrorType::Timeout);
        assert_eq!(classification.severity, ErrorSeverity::Warning);
    }

    #[tokio::test]
    async fn test_authentication_error_classification() {
        let config = ErrorClassificationConfig::new();
        let mut classifier = ErrorClassifier::new(config);

        let error = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Authentication failed",
        );
        let classification = classifier.classify_error(&error).await.unwrap();

        assert_eq!(classification.error_type, ErrorType::Authentication);
        assert_eq!(classification.severity, ErrorSeverity::Error);
    }

    #[tokio::test]
    async fn test_add_custom_pattern() {
        let config = ErrorClassificationConfig::new();
        let mut classifier = ErrorClassifier::new(config);

        let custom_pattern = ErrorPattern {
            name: "custom_test".to_string(),
            pattern: r"(?i)custom.*error".to_string(), // Case-insensitive flag
            error_type: ErrorType::Validation,
            error_severity: ErrorSeverity::Warning,
            priority: 50,
            enabled: true,
        };

        classifier.add_pattern(custom_pattern).await.unwrap();

        let error = std::io::Error::new(std::io::ErrorKind::Other, "Custom error occurred");
        let classification = classifier.classify_error(&error).await.unwrap();

        assert_eq!(classification.error_type, ErrorType::Validation);
        assert_eq!(classification.severity, ErrorSeverity::Warning);
    }

    #[tokio::test]
    async fn test_last_classification_time_tracking() {
        let config = ErrorClassificationConfig::new();
        let mut classifier = ErrorClassifier::new(config);

        // Initially, no classification has been performed
        let stats = classifier.get_statistics().await.unwrap();
        assert!(stats.last_classification_time.is_none());
        assert!(classifier.get_last_classification_time().is_none());

        // Perform a classification
        let error =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused");
        let _classification = classifier.classify_error(&error).await.unwrap();

        // Now the last classification time should be set
        let stats = classifier.get_statistics().await.unwrap();
        assert!(stats.last_classification_time.is_some());
        assert!(classifier.get_last_classification_time().is_some());

        // Verify the timestamp is recent (within the last second)
        let last_time = stats.last_classification_time.unwrap();
        let now = Utc::now();
        let time_diff = now.signed_duration_since(last_time);
        assert!(time_diff.num_seconds() <= 1);

        // Test reset functionality
        classifier.reset_last_classification_time();
        assert!(classifier.get_last_classification_time().is_none());

        let stats = classifier.get_statistics().await.unwrap();
        assert!(stats.last_classification_time.is_none());
    }
}
