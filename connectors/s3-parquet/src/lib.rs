//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! S3 Parquet connector for OpenTelemetry Data Lake Bridge
//!
//! This module provides integration with S3 Parquet for storing and querying
//! telemetry data in a lakehouse format.

pub mod config;
pub mod connector;
pub mod error;
pub mod exporter;
pub mod reader;
pub mod writer;

// Re-export main types for easy access
pub use config::S3ParquetConfig;
pub use connector::S3ParquetConnector;
pub use error::{S3ParquetError, S3ParquetResult};
pub use exporter::S3ParquetExporter;
pub use reader::S3ParquetReader;
pub use writer::S3ParquetWriter;
