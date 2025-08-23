//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query plan visualization and performance analysis
//!
//! This module provides tools for visualizing query plans and analyzing
//! query performance.

pub mod metrics_collector;
pub mod performance_analyzer;
pub mod plan_visualizer;
pub mod report_generator;

// Re-export main types
pub use metrics_collector::{MetricsCollector, MetricsCollectorConfig, QueryMetrics};
pub use performance_analyzer::{PerformanceAnalyzer, PerformanceAnalyzerConfig, PerformanceReport};
pub use plan_visualizer::{PlanVisualizer, PlanVisualizerConfig};
pub use report_generator::{ReportFormat, ReportGenerator, ReportGeneratorConfig};

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Query plan node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlanNode {
    /// Node ID
    pub id: Uuid,

    /// Node type
    pub node_type: String,

    /// Node name
    pub name: String,

    /// Node description
    pub description: String,

    /// Estimated cost
    pub estimated_cost: f64,

    /// Actual cost
    pub actual_cost: Option<f64>,

    /// Estimated rows
    pub estimated_rows: u64,

    /// Actual rows
    pub actual_rows: Option<u64>,

    /// Execution time in milliseconds
    pub execution_time_ms: Option<u64>,

    /// Node metadata
    pub metadata: HashMap<String, String>,

    /// Child nodes
    pub children: Vec<Uuid>,
}

/// Query plan edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlanEdge {
    /// Edge ID
    pub id: Uuid,

    /// Source node ID
    pub source: Uuid,

    /// Target node ID
    pub target: Uuid,

    /// Edge type
    pub edge_type: String,

    /// Edge metadata
    pub metadata: HashMap<String, String>,
}

/// Query plan graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlanGraph {
    /// Graph ID
    pub id: Uuid,

    /// Query ID
    pub query_id: Uuid,

    /// Graph name
    pub name: String,

    /// Graph description
    pub description: String,

    /// Nodes
    pub nodes: Vec<QueryPlanNode>,

    /// Edges
    pub edges: Vec<QueryPlanEdge>,

    /// Total estimated cost
    pub total_estimated_cost: f64,

    /// Total actual cost
    pub total_actual_cost: Option<f64>,

    /// Total execution time in milliseconds
    pub total_execution_time_ms: Option<u64>,

    /// Graph metadata
    pub metadata: HashMap<String, String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Query performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformanceMetrics {
    /// Query ID
    pub query_id: Uuid,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// CPU usage percentage
    pub cpu_usage_percent: f64,

    /// Memory usage in bytes
    pub memory_usage_bytes: u64,

    /// Disk I/O in bytes
    pub disk_io_bytes: u64,

    /// Network I/O in bytes
    pub network_io_bytes: u64,

    /// Cache hit ratio
    pub cache_hit_ratio: f64,

    /// Number of rows processed
    pub rows_processed: u64,

    /// Number of rows returned
    pub rows_returned: u64,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Visualization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationConfig {
    /// Enable query plan visualization
    pub enable_plan_visualization: bool,

    /// Enable performance analysis
    pub enable_performance_analysis: bool,

    /// Enable metrics collection
    pub enable_metrics_collection: bool,

    /// Enable report generation
    pub enable_report_generation: bool,

    /// Default visualization format
    pub default_format: VisualizationFormat,

    /// Output directory for visualizations
    pub output_directory: String,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Visualization format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VisualizationFormat {
    /// DOT format for Graphviz
    Dot,

    /// JSON format
    Json,

    /// HTML format
    Html,

    /// SVG format
    Svg,

    /// PNG format
    Png,

    /// PDF format
    Pdf,
}

/// Visualization trait
#[async_trait]
pub trait Visualizer: Send + Sync {
    /// Generate visualization
    async fn visualize(
        &self,
        graph: &QueryPlanGraph,
        format: &VisualizationFormat,
    ) -> BridgeResult<Vec<u8>>;

    /// Save visualization to file
    async fn save_visualization(
        &self,
        graph: &QueryPlanGraph,
        format: &VisualizationFormat,
        path: &str,
    ) -> BridgeResult<()>;

    /// Get supported formats
    fn get_supported_formats(&self) -> Vec<VisualizationFormat>;
}
