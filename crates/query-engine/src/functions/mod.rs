//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry-specific functions for the query engine
//!
//! This module provides OpenTelemetry-specific SQL functions for analyzing
//! telemetry data including traces, metrics, and logs.

pub mod telemetry_functions;

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::parsers::{ParsedQuery, QueryAst};

// Re-export telemetry functions
pub use telemetry_functions::{TelemetryFunctionConfig, TelemetryFunctions};

/// Function configuration trait
#[async_trait]
pub trait FunctionConfig: Send + Sync {
    /// Get function name
    fn name(&self) -> &str;

    /// Get function version
    fn version(&self) -> &str;

    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;

    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Function trait for executing custom functions
#[async_trait]
pub trait QueryFunction: Send + Sync {
    /// Initialize the function
    async fn init(&mut self) -> BridgeResult<()>;

    /// Execute function
    async fn execute(&self, args: Vec<FunctionValue>) -> BridgeResult<FunctionValue>;

    /// Get function name
    fn name(&self) -> &str;

    /// Get function version
    fn version(&self) -> &str;

    /// Get function description
    fn description(&self) -> &str;

    /// Get function signature
    fn signature(&self) -> &str;
}

/// Function value structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Array(Vec<FunctionValue>),
    Object(HashMap<String, FunctionValue>),
    Timestamp(DateTime<Utc>),
}

/// Function result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResult {
    /// Result ID
    pub id: Uuid,

    /// Function name
    pub function_name: String,

    /// Result value
    pub value: FunctionValue,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Execution timestamp
    pub execution_timestamp: DateTime<Utc>,

    /// Result metadata
    pub metadata: HashMap<String, String>,
}

/// Function statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionStats {
    /// Function name
    pub function: String,

    /// Total calls
    pub total_calls: u64,

    /// Calls in last minute
    pub calls_per_minute: u64,

    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,

    /// Average execution time per call in milliseconds
    pub avg_execution_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last execution timestamp
    pub last_execution_time: Option<DateTime<Utc>>,

    /// Function status
    pub is_executing: bool,
}

/// Function manager for managing multiple functions
pub struct FunctionManager {
    functions: HashMap<String, Box<dyn QueryFunction>>,
    stats: HashMap<String, FunctionStats>,
}

impl FunctionManager {
    /// Create new function manager
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            stats: HashMap::new(),
        }
    }

    /// Add function
    pub fn add_function(&mut self, name: String, function: Box<dyn QueryFunction>) {
        let function_name = name.clone();
        self.functions.insert(name, function);
        self.stats.insert(
            function_name.clone(),
            FunctionStats {
                function: function_name,
                total_calls: 0,
                calls_per_minute: 0,
                total_execution_time_ms: 0,
                avg_execution_time_ms: 0.0,
                error_count: 0,
                last_execution_time: None,
                is_executing: false,
            },
        );
    }

    /// Remove function
    pub fn remove_function(&mut self, name: &str) -> Option<Box<dyn QueryFunction>> {
        self.stats.remove(name);
        self.functions.remove(name)
    }

    /// Get function
    pub fn get_function(&self, name: &str) -> Option<&dyn QueryFunction> {
        self.functions.get(name).map(|f| f.as_ref())
    }

    /// Get all function names
    pub fn get_function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    /// Execute function
    pub async fn execute_function(
        &mut self,
        name: &str,
        args: Vec<FunctionValue>,
    ) -> BridgeResult<FunctionResult> {
        let start_time = std::time::Instant::now();

        // Update stats
        if let Some(stats) = self.stats.get_mut(name) {
            stats.total_calls += 1;
            stats.is_executing = true;
            stats.last_execution_time = Some(Utc::now());
        }

        // Execute function
        let result = if let Some(function) = self.get_function(name) {
            function.execute(args).await
        } else {
            return Err(bridge_core::BridgeError::configuration(format!(
                "Function not found: {}",
                name
            )));
        };

        let execution_time = start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;

        // Update stats
        if let Some(stats) = self.stats.get_mut(name) {
            stats.is_executing = false;
            stats.total_execution_time_ms += execution_time_ms;
            stats.avg_execution_time_ms =
                stats.total_execution_time_ms as f64 / stats.total_calls as f64;

            if result.is_err() {
                stats.error_count += 1;
            }
        }

        match result {
            Ok(value) => Ok(FunctionResult {
                id: Uuid::new_v4(),
                function_name: name.to_string(),
                value,
                execution_time_ms,
                execution_timestamp: Utc::now(),
                metadata: HashMap::new(),
            }),
            Err(e) => {
                error!("Function execution failed: {}", e);
                Err(e)
            }
        }
    }

    /// Get function statistics
    pub fn get_stats(&self) -> &HashMap<String, FunctionStats> {
        &self.stats
    }
}

/// Function factory for creating functions
pub struct FunctionFactory;

impl FunctionFactory {
    /// Create a function based on configuration
    pub async fn create_function(
        config: &dyn FunctionConfig,
    ) -> BridgeResult<Box<dyn QueryFunction>> {
        match config.name() {
            "time_series_aggregation" => Ok(Box::new(
                telemetry_functions::TimeSeriesAggregationFunction::new(),
            )),
            "anomaly_detection" => Ok(Box::new(
                telemetry_functions::AnomalyDetectionFunction::new(),
            )),
            "trend_analysis" => Ok(Box::new(telemetry_functions::TrendAnalysisFunction::new())),
            "forecast" => Ok(Box::new(telemetry_functions::ForecastFunction::new())),
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported function: {}",
                config.name()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_function_manager_creation() {
        let manager = FunctionManager::new();
        assert_eq!(manager.get_function_names().len(), 0);
    }

    #[tokio::test]
    async fn test_function_manager_add_function() {
        let mut manager = FunctionManager::new();

        // This would require a mock function implementation
        // For now, just test the basic structure
        assert_eq!(manager.get_function_names().len(), 0);
    }
}
