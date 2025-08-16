//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Hudi connector for OpenTelemetry Data Lake Bridge
//!
//! This module provides integration with Apache Hudi for storing and querying
//! telemetry data in a lakehouse format.

pub mod config;
pub mod connector;
pub mod error;
pub mod exporter;
pub mod metrics;
pub mod reader;
pub mod schema;
pub mod table;
pub mod writer;

// Re-export main types
pub use bridge_core::traits::{LakehouseConnector, LakehouseReader, LakehouseWriter};
pub use bridge_core::types::{ExportResult, ProcessedBatch, TelemetryBatch};

// Re-export Hudi-specific types
pub use config::HudiConfig;
pub use connector::HudiConnector;
pub use error::{HudiError, HudiResult};
pub use exporter::RealHudiExporter as HudiExporter;
pub use reader::HudiReader;
pub use writer::HudiWriter;

#[cfg(test)]
mod tests;
