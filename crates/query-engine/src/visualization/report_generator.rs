//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Report generator for query analysis
//!
//! This module provides report generation capabilities for query analysis.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::metrics_collector::{MetricsCollector, QueryExecutionRecord};
use super::performance_analyzer::{
    PerformanceAnalyzer, PerformanceAnomaly, PerformanceBottleneck, PerformanceReport,
    PerformanceTrend,
};

/// Report generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportGeneratorConfig {
    /// Generator name
    pub name: String,

    /// Generator version
    pub version: String,

    /// Enable generation
    pub enable_generation: bool,

    /// Output directory for reports
    pub output_directory: String,

    /// Default report format
    pub default_format: ReportFormat,

    /// Enable HTML reports
    pub enable_html_reports: bool,

    /// Enable JSON reports
    pub enable_json_reports: bool,

    /// Enable PDF reports
    pub enable_pdf_reports: bool,

    /// Include charts and visualizations
    pub include_charts: bool,

    /// Include detailed query analysis
    pub include_detailed_analysis: bool,

    /// Report retention days
    pub retention_days: u32,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for ReportGeneratorConfig {
    fn default() -> Self {
        Self {
            name: "query_report_generator".to_string(),
            version: "1.0.0".to_string(),
            enable_generation: true,
            output_directory: "./reports".to_string(),
            default_format: ReportFormat::Html,
            enable_html_reports: true,
            enable_json_reports: true,
            enable_pdf_reports: false,
            include_charts: true,
            include_detailed_analysis: true,
            retention_days: 30,
            additional_config: HashMap::new(),
        }
    }
}

/// Report format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportFormat {
    /// JSON format
    Json,

    /// HTML format
    Html,

    /// PDF format
    Pdf,

    /// Markdown format
    Markdown,
}

/// Report content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportContent {
    /// Report ID
    pub id: String,

    /// Report title
    pub title: String,

    /// Report description
    pub description: String,

    /// Generation timestamp
    pub generated_at: DateTime<Utc>,

    /// Report data
    pub data: ReportData,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Report data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    /// Performance report
    pub performance_report: Option<PerformanceReport>,

    /// Query execution records
    pub query_records: Vec<QueryExecutionRecord>,

    /// Summary statistics
    pub summary_stats: SummaryStatistics,

    /// Charts and visualizations
    pub charts: Vec<ChartData>,

    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryStatistics {
    /// Total queries analyzed
    pub total_queries: usize,

    /// Successful queries
    pub successful_queries: usize,

    /// Failed queries
    pub failed_queries: usize,

    /// Average execution time (ms)
    pub avg_execution_time_ms: f64,

    /// Error rate
    pub error_rate: f64,

    /// Throughput (queries per second)
    pub throughput_qps: f64,

    /// Health score
    pub health_score: f64,

    /// Critical issues count
    pub critical_issues: usize,

    /// High priority issues count
    pub high_priority_issues: usize,
}

/// Chart data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    /// Chart ID
    pub id: String,

    /// Chart title
    pub title: String,

    /// Chart type
    pub chart_type: ChartType,

    /// Chart data
    pub data: serde_json::Value,

    /// Chart options
    pub options: HashMap<String, serde_json::Value>,
}

/// Chart type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    /// Line chart
    Line,

    /// Bar chart
    Bar,

    /// Pie chart
    Pie,

    /// Scatter plot
    Scatter,

    /// Heatmap
    Heatmap,

    /// Histogram
    Histogram,
}

/// Generated report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedReport {
    /// Report ID
    pub id: String,

    /// Report format
    pub format: ReportFormat,

    /// Report content
    pub content: Vec<u8>,

    /// File path (if saved)
    pub file_path: Option<String>,

    /// Generation timestamp
    pub generated_at: DateTime<Utc>,

    /// File size in bytes
    pub file_size_bytes: usize,
}

/// Report generator trait
#[async_trait]
pub trait ReportGenerator: Send + Sync {
    /// Generate report
    async fn generate_report(&self, format: ReportFormat) -> BridgeResult<GeneratedReport>;

    /// Generate report from performance data
    async fn generate_performance_report(
        &self,
        performance_report: &PerformanceReport,
        format: ReportFormat,
    ) -> BridgeResult<GeneratedReport>;

    /// Save report to file
    async fn save_report(&self, report: &GeneratedReport, filename: &str) -> BridgeResult<String>;

    /// Get report history
    async fn get_report_history(&self, limit: usize) -> BridgeResult<Vec<GeneratedReport>>;

    /// Clean up old reports
    async fn cleanup_old_reports(&self) -> BridgeResult<usize>;
}

/// Default report generator implementation
pub struct DefaultReportGenerator {
    config: ReportGeneratorConfig,
    performance_analyzer: Arc<dyn PerformanceAnalyzer>,
    metrics_collector: Arc<dyn MetricsCollector>,
    report_history: Arc<RwLock<Vec<GeneratedReport>>>,
}

impl DefaultReportGenerator {
    /// Create a new report generator
    pub fn new(
        config: ReportGeneratorConfig,
        performance_analyzer: Arc<dyn PerformanceAnalyzer>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            config,
            performance_analyzer,
            metrics_collector,
            report_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Generate JSON report
    async fn generate_json_report(&self, content: &ReportContent) -> BridgeResult<Vec<u8>> {
        let json_content = serde_json::to_string_pretty(content).map_err(|e| {
            bridge_core::BridgeError::serialization(format!("Failed to serialize JSON: {}", e))
        })?;

        Ok(json_content.into_bytes())
    }

    /// Generate HTML report
    async fn generate_html_report(&self, content: &ReportContent) -> BridgeResult<Vec<u8>> {
        let html_template = self.generate_html_template(content).await?;
        Ok(html_template.into_bytes())
    }

    /// Generate HTML template
    async fn generate_html_template(&self, content: &ReportContent) -> BridgeResult<String> {
        let mut html = String::new();

        // HTML header
        html.push_str(&format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background-color: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .header {{ text-align: center; border-bottom: 2px solid #007bff; padding-bottom: 20px; margin-bottom: 30px; }}
        .section {{ margin-bottom: 30px; }}
        .section h2 {{ color: #007bff; border-bottom: 1px solid #ddd; padding-bottom: 10px; }}
        .stats-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin: 20px 0; }}
        .stat-card {{ background-color: #f8f9fa; padding: 15px; border-radius: 5px; border-left: 4px solid #007bff; }}
        .stat-value {{ font-size: 24px; font-weight: bold; color: #007bff; }}
        .stat-label {{ color: #666; font-size: 14px; }}
        .issue-card {{ background-color: #fff3cd; border: 1px solid #ffeaa7; border-radius: 5px; padding: 15px; margin: 10px 0; }}
        .critical {{ border-left-color: #dc3545; }}
        .high {{ border-left-color: #fd7e14; }}
        .medium {{ border-left-color: #ffc107; }}
        .low {{ border-left-color: #28a745; }}
        .recommendation {{ background-color: #d1ecf1; border: 1px solid #bee5eb; border-radius: 5px; padding: 15px; margin: 10px 0; }}
        .chart-container {{ margin: 20px 0; padding: 20px; background-color: #f8f9fa; border-radius: 5px; }}
        .footer {{ text-align: center; margin-top: 40px; padding-top: 20px; border-top: 1px solid #ddd; color: #666; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>{}</h1>
            <p>{}</p>
            <p><strong>Generated:</strong> {}</p>
        </div>
"#, 
            content.title, 
            content.title, 
            content.description, 
            content.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Summary statistics
        let stats = &content.data.summary_stats;
        html.push_str(&format!(
            r#"
        <div class="section">
            <h2>Summary Statistics</h2>
            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Total Queries</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">{:.1}ms</div>
                    <div class="stat-label">Avg Execution Time</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">{:.2}%</div>
                    <div class="stat-label">Error Rate</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">{:.1}</div>
                    <div class="stat-label">Health Score</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">{:.1} QPS</div>
                    <div class="stat-label">Throughput</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Critical Issues</div>
                </div>
            </div>
        </div>
"#,
            stats.total_queries,
            stats.avg_execution_time_ms,
            stats.error_rate * 100.0,
            stats.health_score,
            stats.throughput_qps,
            stats.critical_issues
        ));

        // Performance issues
        if let Some(perf_report) = &content.data.performance_report {
            if !perf_report.bottlenecks.is_empty() {
                html.push_str(
                    r#"
        <div class="section">
            <h2>Performance Issues</h2>
"#,
                );

                for bottleneck in &perf_report.bottlenecks {
                    let severity_class = match bottleneck.severity {
                        super::performance_analyzer::SeverityLevel::Critical => "critical",
                        super::performance_analyzer::SeverityLevel::High => "high",
                        super::performance_analyzer::SeverityLevel::Medium => "medium",
                        super::performance_analyzer::SeverityLevel::Low => "low",
                    };

                    html.push_str(&format!(
                        r#"
            <div class="issue-card {}">
                <h3>{}</h3>
                <p><strong>Severity:</strong> {:?}</p>
                <p><strong>Impact Score:</strong> {:.2}</p>
                <p><strong>Affected Queries:</strong> {}</p>
                <p><strong>Description:</strong> {}</p>
                <p><strong>Recommended Actions:</strong></p>
                <ul>
"#,
                        severity_class,
                        format!("{:?}", bottleneck.bottleneck_type),
                        bottleneck.severity,
                        bottleneck.impact_score,
                        bottleneck.affected_queries_count,
                        bottleneck.description
                    ));

                    for action in &bottleneck.recommended_actions {
                        html.push_str(&format!("                    <li>{}</li>\n", action));
                    }

                    html.push_str("                </ul>\n            </div>\n");
                }

                html.push_str("        </div>\n");
            }
        }

        // Recommendations
        if !content.data.recommendations.is_empty() {
            html.push_str(
                r#"
        <div class="section">
            <h2>Recommendations</h2>
"#,
            );

            for recommendation in &content.data.recommendations {
                html.push_str(&format!(
                    r#"
            <div class="recommendation">
                <p>{}</p>
            </div>
"#,
                    recommendation
                ));
            }

            html.push_str("        </div>\n");
        }

        // Footer
        html.push_str(&format!(
            r#"
        <div class="footer">
            <p>Report generated by {} v{} | Generated at {}</p>
        </div>
    </div>
</body>
</html>
"#,
            self.config.name,
            self.config.version,
            content.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        Ok(html)
    }

    /// Generate Markdown report
    async fn generate_markdown_report(&self, content: &ReportContent) -> BridgeResult<Vec<u8>> {
        let mut markdown = String::new();

        // Header
        markdown.push_str(&format!("# {}\n\n", content.title));
        markdown.push_str(&format!("{}\n\n", content.description));
        markdown.push_str(&format!(
            "**Generated:** {}\n\n",
            content.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Summary statistics
        let stats = &content.data.summary_stats;
        markdown.push_str("## Summary Statistics\n\n");
        markdown.push_str(&format!("- **Total Queries:** {}\n", stats.total_queries));
        markdown.push_str(&format!(
            "- **Average Execution Time:** {:.1}ms\n",
            stats.avg_execution_time_ms
        ));
        markdown.push_str(&format!(
            "- **Error Rate:** {:.2}%\n",
            stats.error_rate * 100.0
        ));
        markdown.push_str(&format!("- **Health Score:** {:.1}\n", stats.health_score));
        markdown.push_str(&format!(
            "- **Throughput:** {:.1} QPS\n",
            stats.throughput_qps
        ));
        markdown.push_str(&format!(
            "- **Critical Issues:** {}\n\n",
            stats.critical_issues
        ));

        // Performance issues
        if let Some(perf_report) = &content.data.performance_report {
            if !perf_report.bottlenecks.is_empty() {
                markdown.push_str("## Performance Issues\n\n");

                for bottleneck in &perf_report.bottlenecks {
                    markdown.push_str(&format!("### {:?}\n\n", bottleneck.bottleneck_type));
                    markdown.push_str(&format!("- **Severity:** {:?}\n", bottleneck.severity));
                    markdown.push_str(&format!(
                        "- **Impact Score:** {:.2}\n",
                        bottleneck.impact_score
                    ));
                    markdown.push_str(&format!(
                        "- **Affected Queries:** {}\n",
                        bottleneck.affected_queries_count
                    ));
                    markdown.push_str(&format!(
                        "- **Description:** {}\n\n",
                        bottleneck.description
                    ));

                    markdown.push_str("**Recommended Actions:**\n");
                    for action in &bottleneck.recommended_actions {
                        markdown.push_str(&format!("- {}\n", action));
                    }
                    markdown.push_str("\n");
                }
            }
        }

        // Recommendations
        if !content.data.recommendations.is_empty() {
            markdown.push_str("## Recommendations\n\n");
            for recommendation in &content.data.recommendations {
                markdown.push_str(&format!("- {}\n", recommendation));
            }
            markdown.push_str("\n");
        }

        // Footer
        markdown.push_str(&format!(
            "---\n\n*Report generated by {} v{}*",
            self.config.name, self.config.version
        ));

        Ok(markdown.into_bytes())
    }

    /// Calculate summary statistics
    fn calculate_summary_stats(
        &self,
        performance_report: &PerformanceReport,
        query_records: &[QueryExecutionRecord],
    ) -> SummaryStatistics {
        let total_queries = query_records.len();
        let successful_queries = query_records
            .iter()
            .filter(|r| {
                matches!(
                    r.status,
                    super::metrics_collector::QueryExecutionStatus::Completed
                )
            })
            .count();
        let failed_queries = query_records
            .iter()
            .filter(|r| {
                matches!(
                    r.status,
                    super::metrics_collector::QueryExecutionStatus::Failed
                )
            })
            .count();

        let avg_execution_time_ms = if total_queries > 0 {
            query_records
                .iter()
                .filter_map(|r| r.duration_ms)
                .sum::<u64>() as f64
                / total_queries as f64
        } else {
            0.0
        };

        let error_rate = if total_queries > 0 {
            failed_queries as f64 / total_queries as f64
        } else {
            0.0
        };

        let critical_issues = performance_report
            .bottlenecks
            .iter()
            .filter(|b| {
                matches!(
                    b.severity,
                    super::performance_analyzer::SeverityLevel::Critical
                )
            })
            .count();

        let high_priority_issues = performance_report
            .bottlenecks
            .iter()
            .filter(|b| matches!(b.severity, super::performance_analyzer::SeverityLevel::High))
            .count();

        SummaryStatistics {
            total_queries,
            successful_queries,
            failed_queries,
            avg_execution_time_ms,
            error_rate,
            throughput_qps: performance_report.query_stats.throughput_qps,
            health_score: performance_report.health_score,
            critical_issues,
            high_priority_issues,
        }
    }
}

#[async_trait]
impl ReportGenerator for DefaultReportGenerator {
    async fn generate_report(&self, format: ReportFormat) -> BridgeResult<GeneratedReport> {
        if !self.config.enable_generation {
            return Err(bridge_core::BridgeError::configuration(
                "Report generation is disabled".to_string(),
            ));
        }

        // Get performance report
        let performance_report = self.performance_analyzer.analyze().await?;

        self.generate_performance_report(&performance_report, format)
            .await
    }

    async fn generate_performance_report(
        &self,
        performance_report: &PerformanceReport,
        format: ReportFormat,
    ) -> BridgeResult<GeneratedReport> {
        // Get recent query records (simplified - in real implementation, get from metrics collector)
        let query_records = Vec::new(); // Placeholder

        // Calculate summary statistics
        let summary_stats = self.calculate_summary_stats(performance_report, &query_records);

        // Create report content
        let content = ReportContent {
            id: Uuid::new_v4().to_string(),
            title: format!(
                "Query Performance Report - {}",
                performance_report.generated_at.format("%Y-%m-%d")
            ),
            description: "Comprehensive analysis of query performance and system health"
                .to_string(),
            generated_at: Utc::now(),
            data: ReportData {
                performance_report: Some(performance_report.clone()),
                query_records,
                summary_stats,
                charts: Vec::new(), // Placeholder for charts
                recommendations: performance_report.recommendations.clone(),
            },
            metadata: HashMap::new(),
        };

        // Generate content based on format
        let report_content = match format {
            ReportFormat::Json => self.generate_json_report(&content).await?,
            ReportFormat::Html => self.generate_html_report(&content).await?,
            ReportFormat::Markdown => self.generate_markdown_report(&content).await?,
            ReportFormat::Pdf => {
                // For now, generate HTML and convert to PDF placeholder
                let html_content = self.generate_html_report(&content).await?;
                html_content // Placeholder - would convert HTML to PDF
            }
        };

        let generated_report = GeneratedReport {
            id: content.id.clone(),
            format: format.clone(),
            content: report_content.clone(),
            file_path: None,
            generated_at: content.generated_at,
            file_size_bytes: report_content.len(),
        };

        // Store in history
        let mut history = self.report_history.write().await;
        history.push(generated_report.clone());

        debug!("Generated report: {} ({:?})", content.id, format);
        Ok(generated_report)
    }

    async fn save_report(&self, report: &GeneratedReport, filename: &str) -> BridgeResult<String> {
        // Ensure output directory exists
        let output_dir = Path::new(&self.config.output_directory);
        if !output_dir.exists() {
            fs::create_dir_all(output_dir).map_err(|e| {
                bridge_core::BridgeError::configuration(format!(
                    "Failed to create output directory: {}",
                    e
                ))
            })?;
        }

        // Determine file extension
        let extension = match report.format {
            ReportFormat::Json => "json",
            ReportFormat::Html => "html",
            ReportFormat::Pdf => "pdf",
            ReportFormat::Markdown => "md",
        };

        let file_path = output_dir.join(format!("{}.{}", filename, extension));

        // Write file
        fs::write(&file_path, &report.content).map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Failed to write report file: {}", e))
        })?;

        let file_path_str = file_path.to_string_lossy().to_string();
        info!("Saved report to: {}", file_path_str);

        Ok(file_path_str)
    }

    async fn get_report_history(&self, limit: usize) -> BridgeResult<Vec<GeneratedReport>> {
        let history = self.report_history.read().await;
        let mut reports: Vec<_> = history.iter().cloned().collect();
        reports.sort_by(|a, b| b.generated_at.cmp(&a.generated_at));
        reports.truncate(limit);

        Ok(reports)
    }

    async fn cleanup_old_reports(&self) -> BridgeResult<usize> {
        let retention_duration = chrono::Duration::days(self.config.retention_days as i64);
        let cutoff_time = Utc::now() - retention_duration;

        let mut history = self.report_history.write().await;
        let initial_count = history.len();

        history.retain(|r| r.generated_at >= cutoff_time);

        let removed_count = initial_count - history.len();
        info!("Cleaned up {} old reports", removed_count);

        Ok(removed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::metrics_collector::DefaultMetricsCollector;
    use crate::visualization::performance_analyzer::DefaultPerformanceAnalyzer;
    use bridge_core::metrics::{BridgeMetrics, MetricsConfig};

    #[tokio::test]
    async fn test_report_generator_creation() {
        let config = ReportGeneratorConfig::default();
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let metrics_collector = Arc::new(DefaultMetricsCollector::new(
            Default::default(),
            bridge_metrics.clone(),
        ));
        let performance_analyzer = Arc::new(DefaultPerformanceAnalyzer::new(
            Default::default(),
            metrics_collector,
        ));

        let generator = DefaultReportGenerator::new(
            config,
            performance_analyzer,
            Arc::new(DefaultMetricsCollector::new(
                Default::default(),
                bridge_metrics,
            )),
        );

        assert_eq!(generator.config.name, "query_report_generator");
    }

    #[tokio::test]
    async fn test_json_report_generation() {
        let config = ReportGeneratorConfig::default();
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let metrics_collector = Arc::new(DefaultMetricsCollector::new(
            Default::default(),
            bridge_metrics.clone(),
        ));
        let performance_analyzer = Arc::new(DefaultPerformanceAnalyzer::new(
            Default::default(),
            metrics_collector,
        ));

        let generator = DefaultReportGenerator::new(
            config,
            performance_analyzer,
            Arc::new(DefaultMetricsCollector::new(
                Default::default(),
                bridge_metrics,
            )),
        );

        let content = ReportContent {
            id: "test".to_string(),
            title: "Test Report".to_string(),
            description: "Test description".to_string(),
            generated_at: Utc::now(),
            data: ReportData {
                performance_report: None,
                query_records: Vec::new(),
                summary_stats: SummaryStatistics {
                    total_queries: 100,
                    successful_queries: 95,
                    failed_queries: 5,
                    avg_execution_time_ms: 150.0,
                    error_rate: 0.05,
                    throughput_qps: 10.0,
                    health_score: 0.85,
                    critical_issues: 0,
                    high_priority_issues: 1,
                },
                charts: Vec::new(),
                recommendations: vec!["Test recommendation".to_string()],
            },
            metadata: HashMap::new(),
        };

        let json_content = generator.generate_json_report(&content).await.unwrap();
        assert!(!json_content.is_empty());

        // Verify it's valid JSON
        let json_str = String::from_utf8(json_content).unwrap();
        serde_json::from_str::<serde_json::Value>(&json_str).unwrap();
    }

    #[tokio::test]
    async fn test_html_report_generation() {
        let config = ReportGeneratorConfig::default();
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let metrics_collector = Arc::new(DefaultMetricsCollector::new(
            Default::default(),
            bridge_metrics.clone(),
        ));
        let performance_analyzer = Arc::new(DefaultPerformanceAnalyzer::new(
            Default::default(),
            metrics_collector,
        ));

        let generator = DefaultReportGenerator::new(
            config,
            performance_analyzer,
            Arc::new(DefaultMetricsCollector::new(
                Default::default(),
                bridge_metrics,
            )),
        );

        let content = ReportContent {
            id: "test".to_string(),
            title: "Test Report".to_string(),
            description: "Test description".to_string(),
            generated_at: Utc::now(),
            data: ReportData {
                performance_report: None,
                query_records: Vec::new(),
                summary_stats: SummaryStatistics {
                    total_queries: 100,
                    successful_queries: 95,
                    failed_queries: 5,
                    avg_execution_time_ms: 150.0,
                    error_rate: 0.05,
                    throughput_qps: 10.0,
                    health_score: 0.85,
                    critical_issues: 0,
                    high_priority_issues: 1,
                },
                charts: Vec::new(),
                recommendations: vec!["Test recommendation".to_string()],
            },
            metadata: HashMap::new(),
        };

        let html_content = generator.generate_html_report(&content).await.unwrap();
        let html_str = String::from_utf8(html_content).unwrap();

        assert!(html_str.contains("<!DOCTYPE html>"));
        assert!(html_str.contains("Test Report"));
        assert!(html_str.contains("100")); // Total queries
    }
}
