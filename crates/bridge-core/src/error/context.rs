//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Error context for the OpenTelemetry Data Lake Bridge
//!
//! This module provides error context structures and utilities for logging
//! and monitoring error information.

use std::fmt;

/// Error context for logging and monitoring
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error_type: &'static str,
    pub retryable: bool,
    pub transient: bool,
    pub permanent: bool,
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ErrorContext {{ type: {}, retryable: {}, transient: {}, permanent: {} }}",
            self.error_type, self.retryable, self.transient, self.permanent
        )
    }
}
