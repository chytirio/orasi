//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Time utilities for the OpenTelemetry Data Lake Bridge

use crate::error::{BridgeError, BridgeResult};
use chrono::{DateTime, Duration, Utc};

/// Time configuration
#[derive(Debug, Clone)]
pub struct TimeConfig {
    /// Default time format
    pub default_format: String,

    /// Timezone offset in seconds
    pub timezone_offset_seconds: i32,

    /// Enable time validation
    pub enable_validation: bool,
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self {
            default_format: "%Y-%m-%d %H:%M:%S".to_string(),
            timezone_offset_seconds: 0,
            enable_validation: true,
        }
    }
}

/// Time utilities
pub struct TimeUtils {
    config: TimeConfig,
}

impl TimeUtils {
    /// Create a new time utilities instance
    pub fn new(config: TimeConfig) -> Self {
        Self { config }
    }

    /// Get current timestamp
    pub fn now() -> DateTime<Utc> {
        Utc::now()
    }

    /// Get Unix timestamp from DateTime
    pub fn to_unix_timestamp(datetime: &DateTime<Utc>) -> i64 {
        datetime.timestamp()
    }

    /// Create DateTime from Unix timestamp
    pub fn from_unix_timestamp(timestamp: i64) -> BridgeResult<DateTime<Utc>> {
        DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| BridgeError::validation("Invalid Unix timestamp"))
    }

    /// Format timestamp for display
    pub fn format_timestamp(datetime: &DateTime<Utc>, format: &str) -> BridgeResult<String> {
        Ok(datetime.format(format).to_string())
    }

    /// Parse timestamp from string
    pub fn parse_timestamp(input: &str, format: &str) -> BridgeResult<DateTime<Utc>> {
        DateTime::parse_from_str(input, format)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| BridgeError::validation_with_source("Failed to parse timestamp", e))
    }

    /// Add duration to timestamp
    pub fn add_duration(datetime: &DateTime<Utc>, duration: Duration) -> DateTime<Utc> {
        *datetime + duration
    }

    /// Subtract duration from timestamp
    pub fn subtract_duration(datetime: &DateTime<Utc>, duration: Duration) -> DateTime<Utc> {
        *datetime - duration
    }

    /// Get time difference between two timestamps
    pub fn time_difference(later: &DateTime<Utc>, earlier: &DateTime<Utc>) -> Duration {
        *later - *earlier
    }

    /// Check if timestamp is in the future
    pub fn is_future(datetime: &DateTime<Utc>) -> bool {
        *datetime > Utc::now()
    }

    /// Check if timestamp is in the past
    pub fn is_past(datetime: &DateTime<Utc>) -> bool {
        *datetime < Utc::now()
    }

    /// Get timestamp rounded to nearest interval
    pub fn round_to_interval(datetime: &DateTime<Utc>, interval_seconds: i64) -> DateTime<Utc> {
        let timestamp = datetime.timestamp();
        let rounded_timestamp = (timestamp / interval_seconds) * interval_seconds;
        DateTime::from_timestamp(rounded_timestamp, 0).unwrap_or(*datetime)
    }

    /// Validate timestamp range
    pub fn validate_timestamp_range(
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
        max_duration: Duration,
    ) -> BridgeResult<()> {
        if *start >= *end {
            return Err(BridgeError::validation(
                "Start time must be before end time",
            ));
        }

        let duration = *end - *start;
        if duration > max_duration {
            return Err(BridgeError::validation(
                "Time range exceeds maximum allowed duration",
            ));
        }

        Ok(())
    }
}

impl Default for TimeUtils {
    fn default() -> Self {
        Self::new(TimeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_time_utilities() {
        let now = TimeUtils::now();
        assert!(now > Utc::now() - Duration::seconds(1));

        let timestamp = TimeUtils::to_unix_timestamp(&now);
        let parsed = TimeUtils::from_unix_timestamp(timestamp).unwrap();
        assert_eq!(now.timestamp(), parsed.timestamp());
    }

    #[test]
    fn test_timestamp_formatting() {
        let now = Utc::now();
        let formatted = TimeUtils::format_timestamp(&now, "%Y-%m-%d").unwrap();
        assert!(formatted.len() == 10);
    }

    #[test]
    fn test_timestamp_parsing() {
        let input = "2023-01-01 12:00:00 +00:00";
        let parsed = TimeUtils::parse_timestamp(input, "%Y-%m-%d %H:%M:%S %z").unwrap();
        assert_eq!(parsed.year(), 2023);
        assert_eq!(parsed.month(), 1);
        assert_eq!(parsed.day(), 1);
    }

    #[test]
    fn test_timestamp_parsing_with_timezone() {
        let input = "2023-01-01 12:00:00 +00:00";
        let parsed = TimeUtils::parse_timestamp(input, "%Y-%m-%d %H:%M:%S %z").unwrap();
        assert_eq!(parsed.year(), 2023);
        assert_eq!(parsed.month(), 1);
        assert_eq!(parsed.day(), 1);
    }

    #[test]
    fn test_duration_operations() {
        let now = Utc::now();
        let future = TimeUtils::add_duration(&now, Duration::hours(1));
        assert!(TimeUtils::is_future(&future));

        let past = TimeUtils::subtract_duration(&now, Duration::hours(1));
        assert!(TimeUtils::is_past(&past));
    }

    #[test]
    fn test_timestamp_validation() {
        let start = Utc::now();
        let end = TimeUtils::add_duration(&start, Duration::hours(1));

        // Valid range
        assert!(TimeUtils::validate_timestamp_range(&start, &end, Duration::hours(2)).is_ok());

        // Invalid range (start after end)
        assert!(TimeUtils::validate_timestamp_range(&end, &start, Duration::hours(2)).is_err());

        // Invalid range (too long)
        assert!(TimeUtils::validate_timestamp_range(&start, &end, Duration::minutes(30)).is_err());
    }
}
