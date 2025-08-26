//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry converter trait and factory

use bridge_core::BridgeResult;

/// Conversion trait for telemetry data
#[async_trait::async_trait]
pub trait TelemetryConverter: Send + Sync {
    /// Convert from source format to target format
    async fn convert(
        &self,
        data: &[u8],
        source_format: &str,
        target_format: &str,
    ) -> BridgeResult<Vec<u8>>;

    /// Get supported source formats
    fn supported_source_formats(&self) -> Vec<String>;

    /// Get supported target formats
    fn supported_target_formats(&self) -> Vec<String>;
}

/// Conversion factory
pub struct ConversionFactory;

impl ConversionFactory {
    /// Create converter for specific format
    pub fn create_converter(format: &str) -> Option<Box<dyn TelemetryConverter>> {
        match format {
            "otlp" => Some(Box::new(super::otlp::OtlpConverter::new())),
            "arrow" => Some(Box::new(super::arrow::ArrowConverter::new())),
            "json" => Some(Box::new(super::json::JsonConverter::new())),
            _ => None,
        }
    }

    /// Get supported formats
    pub fn get_supported_formats() -> Vec<String> {
        vec!["otlp".to_string(), "arrow".to_string(), "json".to_string()]
    }
}
