//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query time range types for the OpenTelemetry Data Lake Bridge

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;

/// Time range for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start time
    pub start: DateTime<Utc>,

    /// End time
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Create a time range from start to end
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Create a time range for the last N hours
    pub fn last_hours(hours: u64) -> Self {
        let end = Utc::now();
        let start = end - Duration::hours(hours as i64);
        Self { start, end }
    }

    /// Create a time range for the last N days
    pub fn last_days(days: u64) -> Self {
        let end = Utc::now();
        let start = end - Duration::days(days as i64);
        Self { start, end }
    }

    /// Get the duration of the time range
    pub fn duration(&self) -> StdDuration {
        let start = self.start.timestamp_nanos_opt().unwrap_or(0) as u64;
        let end = self.end.timestamp_nanos_opt().unwrap_or(0) as u64;
        StdDuration::from_nanos(end.saturating_sub(start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range_creation() {
        let time_range = TimeRange::last_hours(1);
        assert!(time_range.duration() > StdDuration::from_secs(0));
    }
}
