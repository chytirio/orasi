//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Data export capabilities
//!
//! This module provides export capabilities for generated test data.

// Placeholder for now - will be implemented later
pub struct DataExporter {
    pub name: String,
}

impl DataExporter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}
