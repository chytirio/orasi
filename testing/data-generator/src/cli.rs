//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Command-line interface for test data generation
//!
//! This module provides command-line interface capabilities for test data generation.

// Placeholder for now - will be implemented later
pub struct Cli {
    pub name: String,
}

impl Cli {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}
