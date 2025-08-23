//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query plan visualizer
//!
//! This module provides visualization capabilities for query plans.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{QueryPlanEdge, QueryPlanGraph, QueryPlanNode, VisualizationFormat, Visualizer};

/// Plan visualizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanVisualizerConfig {
    /// Visualizer name
    pub name: String,

    /// Visualizer version
    pub version: String,

    /// Enable visualization
    pub enable_visualization: bool,

    /// Default output format
    pub default_format: VisualizationFormat,

    /// Output directory
    pub output_directory: String,

    /// Enable color coding
    pub enable_color_coding: bool,

    /// Enable cost annotations
    pub enable_cost_annotations: bool,

    /// Enable performance annotations
    pub enable_performance_annotations: bool,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for PlanVisualizerConfig {
    fn default() -> Self {
        Self {
            name: "plan_visualizer".to_string(),
            version: "1.0.0".to_string(),
            enable_visualization: true,
            default_format: VisualizationFormat::Dot,
            output_directory: "./visualizations".to_string(),
            enable_color_coding: true,
            enable_cost_annotations: true,
            enable_performance_annotations: true,
            additional_config: HashMap::new(),
        }
    }
}

/// Plan visualizer implementation
pub struct PlanVisualizer {
    config: PlanVisualizerConfig,
}

impl PlanVisualizer {
    /// Create a new plan visualizer
    pub fn new(config: PlanVisualizerConfig) -> Self {
        Self { config }
    }

    /// Generate DOT format visualization
    fn generate_dot(&self, graph: &QueryPlanGraph) -> String {
        let mut dot = String::new();

        // Header
        dot.push_str("digraph QueryPlan {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, style=filled, fontname=\"Arial\"];\n");
        dot.push_str("  edge [fontname=\"Arial\"];\n\n");

        // Nodes
        for node in &graph.nodes {
            let color = self.get_node_color(&node.node_type);
            let label = self.get_node_label(node);

            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\", fillcolor=\"{}\"];\n",
                node.id, label, color
            ));
        }

        dot.push_str("\n");

        // Edges
        for edge in &graph.edges {
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                edge.source, edge.target, edge.edge_type
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Generate JSON format visualization
    fn generate_json(&self, graph: &QueryPlanGraph) -> String {
        serde_json::to_string_pretty(graph).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate HTML format visualization
    fn generate_html(&self, graph: &QueryPlanGraph) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html>\n");
        html.push_str("<head>\n");
        html.push_str("  <title>Query Plan Visualization</title>\n");
        html.push_str("  <style>\n");
        html.push_str("    body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str("    .node { border: 1px solid #ccc; padding: 10px; margin: 5px; border-radius: 5px; }\n");
        html.push_str("    .node-type { font-weight: bold; color: #333; }\n");
        html.push_str("    .node-cost { color: #666; font-size: 0.9em; }\n");
        html.push_str("    .node-performance { color: #888; font-size: 0.8em; }\n");
        html.push_str("  </style>\n");
        html.push_str("</head>\n");
        html.push_str("<body>\n");
        html.push_str(&format!("<h1>Query Plan: {}</h1>\n", graph.name));
        html.push_str(&format!(
            "<p><strong>Description:</strong> {}</p>\n",
            graph.description
        ));
        html.push_str(&format!(
            "<p><strong>Total Estimated Cost:</strong> {:.2}</p>\n",
            graph.total_estimated_cost
        ));

        if let Some(actual_cost) = graph.total_actual_cost {
            html.push_str(&format!(
                "<p><strong>Total Actual Cost:</strong> {:.2}</p>\n",
                actual_cost
            ));
        }

        if let Some(execution_time) = graph.total_execution_time_ms {
            html.push_str(&format!(
                "<p><strong>Total Execution Time:</strong> {}ms</p>\n",
                execution_time
            ));
        }

        html.push_str("<h2>Query Plan Nodes</h2>\n");

        for node in &graph.nodes {
            html.push_str("<div class=\"node\">\n");
            html.push_str(&format!(
                "  <div class=\"node-type\">{}</div>\n",
                node.node_type
            ));
            html.push_str(&format!(
                "  <div><strong>Name:</strong> {}</div>\n",
                node.name
            ));
            html.push_str(&format!(
                "  <div><strong>Description:</strong> {}</div>\n",
                node.description
            ));
            html.push_str(&format!(
                "  <div class=\"node-cost\"><strong>Estimated Cost:</strong> {:.2}</div>\n",
                node.estimated_cost
            ));
            html.push_str(&format!(
                "  <div class=\"node-cost\"><strong>Estimated Rows:</strong> {}</div>\n",
                node.estimated_rows
            ));

            if let Some(actual_cost) = node.actual_cost {
                html.push_str(&format!(
                    "  <div class=\"node-cost\"><strong>Actual Cost:</strong> {:.2}</div>\n",
                    actual_cost
                ));
            }

            if let Some(actual_rows) = node.actual_rows {
                html.push_str(&format!(
                    "  <div class=\"node-cost\"><strong>Actual Rows:</strong> {}</div>\n",
                    actual_rows
                ));
            }

            if let Some(execution_time) = node.execution_time_ms {
                html.push_str(&format!("  <div class=\"node-performance\"><strong>Execution Time:</strong> {}ms</div>\n", execution_time));
            }

            html.push_str("</div>\n");
        }

        html.push_str("</body>\n");
        html.push_str("</html>\n");

        html
    }

    /// Get node color based on type
    fn get_node_color(&self, node_type: &str) -> &str {
        match node_type.to_lowercase().as_str() {
            "scan" | "table scan" => "#e6f3ff",
            "filter" | "where" => "#fff2e6",
            "join" | "hash join" | "merge join" => "#e6ffe6",
            "aggregate" | "group by" => "#ffe6f3",
            "sort" | "order by" => "#f3e6ff",
            "project" | "select" => "#ffffe6",
            _ => "#f0f0f0",
        }
    }

    /// Get node label
    fn get_node_label(&self, node: &QueryPlanNode) -> String {
        let mut label = format!("{}: {}", node.node_type, node.name);

        if self.config.enable_cost_annotations {
            label.push_str(&format!("\\nCost: {:.2}", node.estimated_cost));
            label.push_str(&format!("\\nRows: {}", node.estimated_rows));
        }

        if self.config.enable_performance_annotations {
            if let Some(actual_cost) = node.actual_cost {
                label.push_str(&format!("\\nActual Cost: {:.2}", actual_cost));
            }

            if let Some(actual_rows) = node.actual_rows {
                label.push_str(&format!("\\nActual Rows: {}", actual_rows));
            }

            if let Some(execution_time) = node.execution_time_ms {
                label.push_str(&format!("\\nTime: {}ms", execution_time));
            }
        }

        label
    }
}

#[async_trait]
impl Visualizer for PlanVisualizer {
    async fn visualize(
        &self,
        graph: &QueryPlanGraph,
        format: &VisualizationFormat,
    ) -> BridgeResult<Vec<u8>> {
        let content = match format {
            VisualizationFormat::Dot => self.generate_dot(graph),
            VisualizationFormat::Json => self.generate_json(graph),
            VisualizationFormat::Html => self.generate_html(graph),
            VisualizationFormat::Svg => {
                // For SVG, we would need to convert DOT to SVG using Graphviz
                // For now, return the DOT content
                self.generate_dot(graph)
            }
            VisualizationFormat::Png => {
                // For PNG, we would need to convert DOT to PNG using Graphviz
                // For now, return the DOT content
                self.generate_dot(graph)
            }
            VisualizationFormat::Pdf => {
                // For PDF, we would need to convert DOT to PDF using Graphviz
                // For now, return the DOT content
                self.generate_dot(graph)
            }
        };

        Ok(content.into_bytes())
    }

    async fn save_visualization(
        &self,
        graph: &QueryPlanGraph,
        format: &VisualizationFormat,
        path: &str,
    ) -> BridgeResult<()> {
        let content = self.visualize(graph, format).await?;

        // Ensure output directory exists
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, content)?;
        info!("Saved visualization to: {}", path);
        Ok(())
    }

    fn get_supported_formats(&self) -> Vec<VisualizationFormat> {
        vec![
            VisualizationFormat::Dot,
            VisualizationFormat::Json,
            VisualizationFormat::Html,
            VisualizationFormat::Svg,
            VisualizationFormat::Png,
            VisualizationFormat::Pdf,
        ]
    }
}
