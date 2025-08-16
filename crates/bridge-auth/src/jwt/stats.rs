//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! JWT statistics

use chrono::{DateTime, Utc};
use serde::Serialize;

/// JWT statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct JwtStats {
    /// Number of tokens generated
    pub tokens_generated: u64,

    /// Number of tokens validated
    pub tokens_validated: u64,

    /// Number of tokens blacklisted
    pub tokens_blacklisted: u64,

    /// Number of tokens refreshed
    pub tokens_refreshed: u64,

    /// Number of validation failures
    pub validation_failures: u64,

    /// Last token generated
    pub last_token_generated: Option<DateTime<Utc>>,

    /// Last token validated
    pub last_token_validated: Option<DateTime<Utc>>,

    /// Last token blacklisted
    pub last_token_blacklisted: Option<DateTime<Utc>>,

    /// Last token refreshed
    pub last_token_refreshed: Option<DateTime<Utc>>,
}

impl JwtStats {
    /// Create new JWT statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment tokens generated count
    pub fn increment_tokens_generated(&mut self) {
        self.tokens_generated += 1;
        self.last_token_generated = Some(Utc::now());
    }

    /// Increment tokens validated count
    pub fn increment_tokens_validated(&mut self) {
        self.tokens_validated += 1;
        self.last_token_validated = Some(Utc::now());
    }

    /// Increment tokens blacklisted count
    pub fn increment_tokens_blacklisted(&mut self) {
        self.tokens_blacklisted += 1;
        self.last_token_blacklisted = Some(Utc::now());
    }

    /// Increment tokens refreshed count
    pub fn increment_tokens_refreshed(&mut self) {
        self.tokens_refreshed += 1;
        self.last_token_refreshed = Some(Utc::now());
    }

    /// Increment validation failures count
    pub fn increment_validation_failures(&mut self) {
        self.validation_failures += 1;
    }

    /// Get validation success rate
    pub fn validation_success_rate(&self) -> f64 {
        let total_validations = self.tokens_validated + self.validation_failures;
        if total_validations == 0 {
            0.0
        } else {
            self.tokens_validated as f64 / total_validations as f64
        }
    }

    /// Get token generation rate (tokens per minute)
    pub fn generation_rate_per_minute(&self) -> f64 {
        if let Some(last_generated) = self.last_token_generated {
            let now = Utc::now();
            let duration = now - last_generated;
            let minutes = duration.num_minutes() as f64;
            if minutes > 0.0 {
                self.tokens_generated as f64 / minutes
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_stats_creation() {
        let stats = JwtStats::new();
        assert_eq!(stats.tokens_generated, 0);
        assert_eq!(stats.tokens_validated, 0);
        assert_eq!(stats.tokens_blacklisted, 0);
        assert_eq!(stats.tokens_refreshed, 0);
        assert_eq!(stats.validation_failures, 0);
        assert!(stats.last_token_generated.is_none());
        assert!(stats.last_token_validated.is_none());
        assert!(stats.last_token_blacklisted.is_none());
        assert!(stats.last_token_refreshed.is_none());
    }

    #[test]
    fn test_jwt_stats_increment() {
        let mut stats = JwtStats::new();
        
        stats.increment_tokens_generated();
        assert_eq!(stats.tokens_generated, 1);
        assert!(stats.last_token_generated.is_some());
        
        stats.increment_tokens_validated();
        assert_eq!(stats.tokens_validated, 1);
        assert!(stats.last_token_validated.is_some());
        
        stats.increment_tokens_blacklisted();
        assert_eq!(stats.tokens_blacklisted, 1);
        assert!(stats.last_token_blacklisted.is_some());
        
        stats.increment_tokens_refreshed();
        assert_eq!(stats.tokens_refreshed, 1);
        assert!(stats.last_token_refreshed.is_some());
        
        stats.increment_validation_failures();
        assert_eq!(stats.validation_failures, 1);
    }

    #[test]
    fn test_jwt_stats_validation_success_rate() {
        let mut stats = JwtStats::new();
        
        // No validations yet
        assert_eq!(stats.validation_success_rate(), 0.0);
        
        // All successful
        stats.increment_tokens_validated();
        stats.increment_tokens_validated();
        assert_eq!(stats.validation_success_rate(), 1.0);
        
        // Mixed results
        stats.increment_validation_failures();
        assert_eq!(stats.validation_success_rate(), 2.0 / 3.0);
        
        // All failures
        stats.increment_validation_failures();
        stats.increment_validation_failures();
        assert_eq!(stats.validation_success_rate(), 2.0 / 5.0);
    }

    #[test]
    fn test_jwt_stats_generation_rate() {
        let mut stats = JwtStats::new();
        
        // No tokens generated yet
        assert_eq!(stats.generation_rate_per_minute(), 0.0);
        
        // Generate some tokens
        stats.increment_tokens_generated();
        stats.increment_tokens_generated();
        
        // Rate should be calculated (though exact value depends on timing)
        let rate = stats.generation_rate_per_minute();
        assert!(rate >= 0.0);
    }
}
