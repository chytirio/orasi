//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Performance analyzer for query plans
//!
//! This module provides performance analysis capabilities for query plans.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::metrics_collector::{MetricsCollector, QueryExecutionRecord, QueryExecutionStatus};
use bridge_core::metrics::MetricsConfig;

/// Performance analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalyzerConfig {
    /// Analyzer name
    pub name: String,

    /// Analyzer version
    pub version: String,

    /// Enable analysis
    pub enable_analysis: bool,

    /// Analysis window in seconds
    pub analysis_window_secs: u64,

    /// Enable bottleneck detection
    pub enable_bottleneck_detection: bool,

    /// Enable trend analysis
    pub enable_trend_analysis: bool,

    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,

    /// Performance threshold for slow queries (ms)
    pub slow_query_threshold_ms: u64,

    /// Error rate threshold for alerts
    pub error_rate_threshold: f64,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for PerformanceAnalyzerConfig {
    fn default() -> Self {
        Self {
            name: "query_performance_analyzer".to_string(),
            version: "1.0.0".to_string(),
            enable_analysis: true,
            analysis_window_secs: 3600, // 1 hour
            enable_bottleneck_detection: true,
            enable_trend_analysis: true,
            enable_anomaly_detection: true,
            slow_query_threshold_ms: 1000, // 1 second
            error_rate_threshold: 0.05,    // 5%
            additional_config: HashMap::new(),
        }
    }
}

/// Performance bottleneck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBottleneck {
    /// Bottleneck type
    pub bottleneck_type: BottleneckType,

    /// Severity level
    pub severity: SeverityLevel,

    /// Description
    pub description: String,

    /// Impact score (0.0 to 1.0)
    pub impact_score: f64,

    /// Affected queries count
    pub affected_queries_count: usize,

    /// Recommended actions
    pub recommended_actions: Vec<String>,

    /// Detection timestamp
    pub detected_at: DateTime<Utc>,
}

/// Bottleneck type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckType {
    /// Slow query execution
    SlowQueryExecution,

    /// High memory usage
    HighMemoryUsage,

    /// High CPU usage
    HighCpuUsage,

    /// High disk I/O
    HighDiskIo,

    /// High network I/O
    HighNetworkIo,

    /// Cache misses
    CacheMisses,

    /// Error rate spike
    ErrorRateSpike,

    /// Resource exhaustion
    ResourceExhaustion,

    /// Query timeout
    QueryTimeout,

    /// Connection pool exhaustion
    ConnectionPoolExhaustion,
}

/// Severity level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeverityLevel {
    /// Low severity
    Low,

    /// Medium severity
    Medium,

    /// High severity
    High,

    /// Critical severity
    Critical,
}

/// Performance trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    /// Trend type
    pub trend_type: TrendType,

    /// Metric name
    pub metric_name: String,

    /// Current value
    pub current_value: f64,

    /// Previous value
    pub previous_value: f64,

    /// Change percentage
    pub change_percentage: f64,

    /// Trend direction
    pub direction: TrendDirection,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,

    /// Analysis period
    pub analysis_period: Duration,
}

/// Trend type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendType {
    /// Query execution time trend
    QueryExecutionTime,

    /// Error rate trend
    ErrorRate,

    /// Resource usage trend
    ResourceUsage,

    /// Throughput trend
    Throughput,

    /// Cache performance trend
    CachePerformance,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    /// Improving trend
    Improving,

    /// Stable trend
    Stable,

    /// Degrading trend
    Degrading,
}

/// Performance anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnomaly {
    /// Anomaly type
    pub anomaly_type: AnomalyType,

    /// Severity level
    pub severity: SeverityLevel,

    /// Description
    pub description: String,

    /// Detected value
    pub detected_value: f64,

    /// Expected range
    pub expected_range: (f64, f64),

    /// Deviation from normal
    pub deviation: f64,

    /// Detection timestamp
    pub detected_at: DateTime<Utc>,

    /// Affected queries
    pub affected_queries: Vec<Uuid>,
}

/// Anomaly type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Execution time spike
    ExecutionTimeSpike,

    /// Error rate spike
    ErrorRateSpike,

    /// Resource usage spike
    ResourceUsageSpike,

    /// Throughput drop
    ThroughputDrop,

    /// Cache miss spike
    CacheMissSpike,

    /// Unusual query pattern
    UnusualQueryPattern,
}

/// Query performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformanceStats {
    /// Total queries executed
    pub total_queries: usize,

    /// Successful queries
    pub successful_queries: usize,

    /// Failed queries
    pub failed_queries: usize,

    /// Average execution time (ms)
    pub avg_execution_time_ms: f64,

    /// Median execution time (ms)
    pub median_execution_time_ms: f64,

    /// 95th percentile execution time (ms)
    pub p95_execution_time_ms: f64,

    /// 99th percentile execution time (ms)
    pub p99_execution_time_ms: f64,

    /// Maximum execution time (ms)
    pub max_execution_time_ms: u64,

    /// Minimum execution time (ms)
    pub min_execution_time_ms: u64,

    /// Total rows processed
    pub total_rows_processed: u64,

    /// Total rows returned
    pub total_rows_returned: u64,

    /// Error rate
    pub error_rate: f64,

    /// Throughput (queries per second)
    pub throughput_qps: f64,
}

/// Performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Report ID
    pub id: String,

    /// Report name
    pub name: String,

    /// Report description
    pub description: String,

    /// Generation timestamp
    pub generated_at: DateTime<Utc>,

    /// Analysis window start
    pub analysis_window_start: DateTime<Utc>,

    /// Analysis window end
    pub analysis_window_end: DateTime<Utc>,

    /// Query performance statistics
    pub query_stats: QueryPerformanceStats,

    /// Detected bottlenecks
    pub bottlenecks: Vec<PerformanceBottleneck>,

    /// Performance trends
    pub trends: Vec<PerformanceTrend>,

    /// Performance anomalies
    pub anomalies: Vec<PerformanceAnomaly>,

    /// Top slow queries
    pub top_slow_queries: Vec<QueryExecutionRecord>,

    /// Top error-prone queries
    pub top_error_queries: Vec<QueryExecutionRecord>,

    /// Recommendations
    pub recommendations: Vec<String>,

    /// Overall health score (0.0 to 1.0)
    pub health_score: f64,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Performance analyzer trait
#[async_trait]
pub trait PerformanceAnalyzer: Send + Sync {
    /// Analyze performance
    async fn analyze(&self) -> BridgeResult<PerformanceReport>;

    /// Analyze specific time range
    async fn analyze_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> BridgeResult<PerformanceReport>;

    /// Get recent bottlenecks
    async fn get_recent_bottlenecks(
        &self,
        limit: usize,
    ) -> BridgeResult<Vec<PerformanceBottleneck>>;

    /// Get performance trends
    async fn get_performance_trends(&self) -> BridgeResult<Vec<PerformanceTrend>>;

    /// Get performance anomalies
    async fn get_performance_anomalies(&self) -> BridgeResult<Vec<PerformanceAnomaly>>;
}

/// Default performance analyzer implementation
pub struct DefaultPerformanceAnalyzer {
    config: PerformanceAnalyzerConfig,
    metrics_collector: Arc<dyn MetricsCollector>,
    analysis_history: Arc<RwLock<Vec<PerformanceReport>>>,
}

impl DefaultPerformanceAnalyzer {
    /// Create a new performance analyzer
    pub fn new(
        config: PerformanceAnalyzerConfig,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            config,
            metrics_collector,
            analysis_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Calculate percentiles for execution times
    fn calculate_percentiles(&self, durations: &[u64], percentile: f64) -> f64 {
        if durations.is_empty() {
            return 0.0;
        }

        let mut sorted = durations.to_vec();
        sorted.sort_unstable();

        let index = (percentile / 100.0 * (sorted.len() - 1) as f64).round() as usize;
        sorted[index] as f64
    }

    /// Detect performance bottlenecks
    async fn detect_bottlenecks(
        &self,
        records: &[QueryExecutionRecord],
    ) -> Vec<PerformanceBottleneck> {
        let mut bottlenecks = Vec::new();

        if !self.config.enable_bottleneck_detection {
            return bottlenecks;
        }

        // Detect slow queries
        let slow_queries: Vec<_> = records
            .iter()
            .filter(|r| r.duration_ms.unwrap_or(0) > self.config.slow_query_threshold_ms)
            .collect();

        if !slow_queries.is_empty() {
            bottlenecks.push(PerformanceBottleneck {
                bottleneck_type: BottleneckType::SlowQueryExecution,
                severity: if slow_queries.len() > records.len() / 10 {
                    SeverityLevel::High
                } else {
                    SeverityLevel::Medium
                },
                description: format!(
                    "{} slow queries detected (threshold: {}ms)",
                    slow_queries.len(),
                    self.config.slow_query_threshold_ms
                ),
                impact_score: slow_queries.len() as f64 / records.len() as f64,
                affected_queries_count: slow_queries.len(),
                recommended_actions: vec![
                    "Review query plans for optimization opportunities".to_string(),
                    "Consider adding indexes for frequently accessed columns".to_string(),
                    "Analyze query patterns for potential caching strategies".to_string(),
                ],
                detected_at: Utc::now(),
            });
        }

        // Detect high error rate
        let failed_queries: Vec<_> = records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Failed))
            .collect();

        let error_rate = failed_queries.len() as f64 / records.len() as f64;
        if error_rate > self.config.error_rate_threshold {
            bottlenecks.push(PerformanceBottleneck {
                bottleneck_type: BottleneckType::ErrorRateSpike,
                severity: if error_rate > 0.1 {
                    SeverityLevel::Critical
                } else {
                    SeverityLevel::High
                },
                description: format!("High error rate detected: {:.2}%", error_rate * 100.0),
                impact_score: error_rate,
                affected_queries_count: failed_queries.len(),
                recommended_actions: vec![
                    "Review error logs for common failure patterns".to_string(),
                    "Check system resources and connectivity".to_string(),
                    "Implement retry mechanisms for transient failures".to_string(),
                ],
                detected_at: Utc::now(),
            });
        }

        // Detect timeouts
        let timeout_queries: Vec<_> = records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Timeout))
            .collect();

        if !timeout_queries.is_empty() {
            bottlenecks.push(PerformanceBottleneck {
                bottleneck_type: BottleneckType::QueryTimeout,
                severity: SeverityLevel::High,
                description: format!("{} queries timed out", timeout_queries.len()),
                impact_score: timeout_queries.len() as f64 / records.len() as f64,
                affected_queries_count: timeout_queries.len(),
                recommended_actions: vec![
                    "Increase query timeout limits".to_string(),
                    "Optimize slow-running queries".to_string(),
                    "Consider query cancellation for long-running operations".to_string(),
                ],
                detected_at: Utc::now(),
            });
        }

        bottlenecks
    }

    /// Analyze performance trends
    async fn analyze_trends(&self, records: &[QueryExecutionRecord]) -> Vec<PerformanceTrend> {
        let mut trends = Vec::new();

        if !self.config.enable_trend_analysis || records.len() < 10 {
            return trends;
        }

        // Split records into two time periods for comparison
        let mid_point = records.len() / 2;
        let (earlier, later) = records.split_at(mid_point);

        // Analyze execution time trend
        let earlier_avg =
            earlier.iter().filter_map(|r| r.duration_ms).sum::<u64>() as f64 / earlier.len() as f64;

        let later_avg =
            later.iter().filter_map(|r| r.duration_ms).sum::<u64>() as f64 / later.len() as f64;

        let change_percentage = if earlier_avg > 0.0 {
            (later_avg - earlier_avg) / earlier_avg * 100.0
        } else {
            0.0
        };

        let direction = if change_percentage < -5.0 {
            TrendDirection::Improving
        } else if change_percentage > 5.0 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        };

        trends.push(PerformanceTrend {
            trend_type: TrendType::QueryExecutionTime,
            metric_name: "Average Execution Time".to_string(),
            current_value: later_avg,
            previous_value: earlier_avg,
            change_percentage,
            direction,
            confidence: 0.8, // Simplified confidence calculation
            analysis_period: Duration::minutes(30),
        });

        // Analyze error rate trend
        let earlier_errors = earlier
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Failed))
            .count() as f64
            / earlier.len() as f64;

        let later_errors = later
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Failed))
            .count() as f64
            / later.len() as f64;

        let error_change = if earlier_errors > 0.0 {
            (later_errors - earlier_errors) / earlier_errors * 100.0
        } else {
            0.0
        };

        let error_direction = if error_change < -10.0 {
            TrendDirection::Improving
        } else if error_change > 10.0 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        };

        trends.push(PerformanceTrend {
            trend_type: TrendType::ErrorRate,
            metric_name: "Error Rate".to_string(),
            current_value: later_errors,
            previous_value: earlier_errors,
            change_percentage: error_change,
            direction: error_direction,
            confidence: 0.7,
            analysis_period: Duration::minutes(30),
        });

        trends
    }

    /// Detect performance anomalies
    async fn detect_anomalies(&self, records: &[QueryExecutionRecord]) -> Vec<PerformanceAnomaly> {
        let mut anomalies = Vec::new();

        if !self.config.enable_anomaly_detection || records.len() < 5 {
            return anomalies;
        }

        // Calculate baseline statistics
        let durations: Vec<u64> = records.iter().filter_map(|r| r.duration_ms).collect();

        if durations.len() < 3 {
            return anomalies;
        }

        let mean = durations.iter().sum::<u64>() as f64 / durations.len() as f64;
        let variance = durations
            .iter()
            .map(|&d| (d as f64 - mean).powi(2))
            .sum::<f64>()
            / durations.len() as f64;
        let std_dev = variance.sqrt();

        // Detect execution time anomalies (3-sigma rule)
        for record in records {
            if let Some(duration) = record.duration_ms {
                let z_score = (duration as f64 - mean) / std_dev;
                if z_score.abs() > 3.0 {
                    anomalies.push(PerformanceAnomaly {
                        anomaly_type: AnomalyType::ExecutionTimeSpike,
                        severity: if z_score > 5.0 {
                            SeverityLevel::Critical
                        } else {
                            SeverityLevel::High
                        },
                        description: format!(
                            "Execution time spike detected: {}ms (z-score: {:.2})",
                            duration, z_score
                        ),
                        detected_value: duration as f64,
                        expected_range: (mean - 2.0 * std_dev, mean + 2.0 * std_dev),
                        deviation: z_score,
                        detected_at: Utc::now(),
                        affected_queries: vec![record.query_id],
                    });
                }
            }
        }

        anomalies
    }

    /// Calculate query performance statistics
    fn calculate_query_stats(&self, records: &[QueryExecutionRecord]) -> QueryPerformanceStats {
        let total_queries = records.len();
        let successful_queries = records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Completed))
            .count();
        let failed_queries = records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Failed))
            .count();

        let durations: Vec<u64> = records.iter().filter_map(|r| r.duration_ms).collect();

        let avg_execution_time_ms = if !durations.is_empty() {
            durations.iter().sum::<u64>() as f64 / durations.len() as f64
        } else {
            0.0
        };

        let median_execution_time_ms = if !durations.is_empty() {
            self.calculate_percentiles(&durations, 50.0)
        } else {
            0.0
        };

        let p95_execution_time_ms = if !durations.is_empty() {
            self.calculate_percentiles(&durations, 95.0)
        } else {
            0.0
        };

        let p99_execution_time_ms = if !durations.is_empty() {
            self.calculate_percentiles(&durations, 99.0)
        } else {
            0.0
        };

        let max_execution_time_ms = durations.iter().max().copied().unwrap_or(0);
        let min_execution_time_ms = durations.iter().min().copied().unwrap_or(0);

        let total_rows_processed = records.iter().filter_map(|r| r.rows_processed).sum();

        let total_rows_returned = records.iter().filter_map(|r| r.rows_returned).sum();

        let error_rate = if total_queries > 0 {
            failed_queries as f64 / total_queries as f64
        } else {
            0.0
        };

        // Calculate throughput (queries per second)
        let analysis_duration = self.config.analysis_window_secs as f64;
        let throughput_qps = if analysis_duration > 0.0 {
            total_queries as f64 / analysis_duration
        } else {
            0.0
        };

        QueryPerformanceStats {
            total_queries,
            successful_queries,
            failed_queries,
            avg_execution_time_ms,
            median_execution_time_ms,
            p95_execution_time_ms,
            p99_execution_time_ms,
            max_execution_time_ms,
            min_execution_time_ms,
            total_rows_processed,
            total_rows_returned,
            error_rate,
            throughput_qps,
        }
    }

    /// Generate recommendations based on analysis
    fn generate_recommendations(
        &self,
        stats: &QueryPerformanceStats,
        bottlenecks: &[PerformanceBottleneck],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Performance recommendations
        if stats.avg_execution_time_ms > 1000.0 {
            recommendations.push("Consider query optimization and indexing strategies".to_string());
        }

        if stats.error_rate > 0.05 {
            recommendations.push("Investigate and address error patterns".to_string());
        }

        if stats.throughput_qps < 10.0 {
            recommendations
                .push("Consider scaling resources or optimizing query patterns".to_string());
        }

        // Bottleneck-specific recommendations
        for bottleneck in bottlenecks {
            recommendations.extend(bottleneck.recommended_actions.clone());
        }

        // General recommendations
        if recommendations.is_empty() {
            recommendations.push("System performance is within acceptable parameters".to_string());
        }

        recommendations
    }

    /// Calculate overall health score
    fn calculate_health_score(
        &self,
        stats: &QueryPerformanceStats,
        bottlenecks: &[PerformanceBottleneck],
    ) -> f64 {
        let mut score = 1.0;

        // Penalize for high error rate
        score -= stats.error_rate * 0.5;

        // Penalize for slow queries (if average > 1 second)
        if stats.avg_execution_time_ms > 1000.0 {
            score -= 0.2;
        }

        // Penalize for bottlenecks
        for bottleneck in bottlenecks {
            match bottleneck.severity {
                SeverityLevel::Critical => score -= 0.3,
                SeverityLevel::High => score -= 0.2,
                SeverityLevel::Medium => score -= 0.1,
                SeverityLevel::Low => score -= 0.05,
            }
        }

        score.max(0.0).min(1.0)
    }
}

#[async_trait]
impl PerformanceAnalyzer for DefaultPerformanceAnalyzer {
    async fn analyze(&self) -> BridgeResult<PerformanceReport> {
        if !self.config.enable_analysis {
            return Ok(PerformanceReport {
                id: "disabled".to_string(),
                name: "Analysis Disabled".to_string(),
                description: "Performance analysis is disabled".to_string(),
                generated_at: Utc::now(),
                analysis_window_start: Utc::now(),
                analysis_window_end: Utc::now(),
                query_stats: QueryPerformanceStats {
                    total_queries: 0,
                    successful_queries: 0,
                    failed_queries: 0,
                    avg_execution_time_ms: 0.0,
                    median_execution_time_ms: 0.0,
                    p95_execution_time_ms: 0.0,
                    p99_execution_time_ms: 0.0,
                    max_execution_time_ms: 0,
                    min_execution_time_ms: 0,
                    total_rows_processed: 0,
                    total_rows_returned: 0,
                    error_rate: 0.0,
                    throughput_qps: 0.0,
                },
                bottlenecks: Vec::new(),
                trends: Vec::new(),
                anomalies: Vec::new(),
                top_slow_queries: Vec::new(),
                top_error_queries: Vec::new(),
                recommendations: vec![
                    "Enable performance analysis for detailed insights".to_string()
                ],
                health_score: 1.0,
                additional_config: HashMap::new(),
            });
        }

        let end_time = Utc::now();
        let start_time = end_time - Duration::seconds(self.config.analysis_window_secs as i64);

        self.analyze_time_range(start_time, end_time).await
    }

    async fn analyze_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> BridgeResult<PerformanceReport> {
        // Get query execution records from metrics collector
        // Note: This is a simplified implementation. In a real system, you would
        // need to implement a method to get records for a specific time range
        let records = Vec::new(); // Placeholder - would get from metrics collector

        // Calculate statistics
        let query_stats = self.calculate_query_stats(&records);

        // Detect bottlenecks
        let bottlenecks = self.detect_bottlenecks(&records).await;

        // Analyze trends
        let trends = self.analyze_trends(&records).await;

        // Detect anomalies
        let anomalies = self.detect_anomalies(&records).await;

        // Get top slow queries
        let mut top_slow_queries: Vec<_> =
            records.iter().filter(|r| r.duration_ms.is_some()).collect();
        top_slow_queries
            .sort_by(|a, b| b.duration_ms.unwrap_or(0).cmp(&a.duration_ms.unwrap_or(0)));
        let top_slow_queries: Vec<_> = top_slow_queries
            .iter()
            .take(10)
            .map(|r| (*r).clone())
            .collect();

        // Get top error queries
        let top_error_queries: Vec<_> = records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Failed))
            .take(10)
            .cloned()
            .collect();

        // Generate recommendations
        let recommendations = self.generate_recommendations(&query_stats, &bottlenecks);

        // Calculate health score
        let health_score = self.calculate_health_score(&query_stats, &bottlenecks);

        let report = PerformanceReport {
            id: Uuid::new_v4().to_string(),
            name: format!(
                "Performance Analysis Report - {}",
                end.format("%Y-%m-%d %H:%M:%S")
            ),
            description: format!(
                "Performance analysis for the period from {} to {}",
                start.format("%Y-%m-%d %H:%M:%S"),
                end.format("%Y-%m-%d %H:%M:%S")
            ),
            generated_at: Utc::now(),
            analysis_window_start: start,
            analysis_window_end: end,
            query_stats,
            bottlenecks,
            trends,
            anomalies,
            top_slow_queries,
            top_error_queries,
            recommendations,
            health_score,
            additional_config: self.config.additional_config.clone(),
        };

        // Store in history
        let mut history = self.analysis_history.write().await;
        history.push(report.clone());

        // Keep only recent reports
        let retention_duration = Duration::hours(24);
        let cutoff_time = Utc::now() - retention_duration;
        history.retain(|r| r.generated_at >= cutoff_time);

        debug!("Generated performance report: {}", report.id);
        Ok(report)
    }

    async fn get_recent_bottlenecks(
        &self,
        limit: usize,
    ) -> BridgeResult<Vec<PerformanceBottleneck>> {
        let history = self.analysis_history.read().await;
        let mut all_bottlenecks: Vec<_> =
            history.iter().flat_map(|r| r.bottlenecks.clone()).collect();

        all_bottlenecks.sort_by(|a, b| b.detected_at.cmp(&a.detected_at));
        all_bottlenecks.truncate(limit);

        Ok(all_bottlenecks)
    }

    async fn get_performance_trends(&self) -> BridgeResult<Vec<PerformanceTrend>> {
        let history = self.analysis_history.read().await;
        let mut all_trends: Vec<_> = history.iter().flat_map(|r| r.trends.clone()).collect();

        all_trends.sort_by(|a, b| b.analysis_period.cmp(&a.analysis_period));

        Ok(all_trends)
    }

    async fn get_performance_anomalies(&self) -> BridgeResult<Vec<PerformanceAnomaly>> {
        let history = self.analysis_history.read().await;
        let mut all_anomalies: Vec<_> = history.iter().flat_map(|r| r.anomalies.clone()).collect();

        all_anomalies.sort_by(|a, b| b.detected_at.cmp(&a.detected_at));

        Ok(all_anomalies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::metrics_collector::DefaultMetricsCollector;
    use bridge_core::metrics::BridgeMetrics;

    #[tokio::test]
    async fn test_performance_analyzer_creation() {
        let config = PerformanceAnalyzerConfig::default();
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let metrics_collector = Arc::new(DefaultMetricsCollector::new(
            Default::default(),
            bridge_metrics,
        ));

        let analyzer = DefaultPerformanceAnalyzer::new(config, metrics_collector);
        assert_eq!(analyzer.config.name, "query_performance_analyzer");
    }

    #[tokio::test]
    async fn test_performance_analyzer_disabled() {
        let mut config = PerformanceAnalyzerConfig::default();
        config.enable_analysis = false;

        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let metrics_collector = Arc::new(DefaultMetricsCollector::new(
            Default::default(),
            bridge_metrics,
        ));

        let analyzer = DefaultPerformanceAnalyzer::new(config, metrics_collector);
        let report = analyzer.analyze().await.unwrap();

        assert_eq!(report.id, "disabled");
        assert_eq!(report.health_score, 1.0);
    }

    #[tokio::test]
    async fn test_percentile_calculation() {
        let config = PerformanceAnalyzerConfig::default();
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let metrics_collector = Arc::new(DefaultMetricsCollector::new(
            Default::default(),
            bridge_metrics,
        ));

        let analyzer = DefaultPerformanceAnalyzer::new(config, metrics_collector);

        let durations = vec![10, 20, 30, 40, 50];
        let p50 = analyzer.calculate_percentiles(&durations, 50.0);
        let p95 = analyzer.calculate_percentiles(&durations, 95.0);

        assert_eq!(p50, 30.0);
        assert_eq!(p95, 50.0);
    }

    #[tokio::test]
    async fn test_bottleneck_detection() {
        let mut config = PerformanceAnalyzerConfig::default();
        config.enable_bottleneck_detection = true;
        config.slow_query_threshold_ms = 100;

        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let metrics_collector = Arc::new(DefaultMetricsCollector::new(
            Default::default(),
            bridge_metrics,
        ));

        let analyzer = DefaultPerformanceAnalyzer::new(config, metrics_collector);

        let records = vec![
            QueryExecutionRecord {
                query_id: Uuid::new_v4(),
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                duration_ms: Some(150), // Slow query
                query_type: "SELECT".to_string(),
                status: QueryExecutionStatus::Completed,
                error_message: None,
                rows_processed: Some(1000),
                rows_returned: Some(100),
            },
            QueryExecutionRecord {
                query_id: Uuid::new_v4(),
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                duration_ms: Some(50), // Fast query
                query_type: "SELECT".to_string(),
                status: QueryExecutionStatus::Completed,
                error_message: None,
                rows_processed: Some(100),
                rows_returned: Some(10),
            },
        ];

        let bottlenecks = analyzer.detect_bottlenecks(&records).await;
        assert!(!bottlenecks.is_empty());

        let slow_query_bottleneck = bottlenecks
            .iter()
            .find(|b| matches!(b.bottleneck_type, BottleneckType::SlowQueryExecution))
            .expect("Should detect slow query bottleneck");

        assert_eq!(slow_query_bottleneck.affected_queries_count, 1);
    }
}
