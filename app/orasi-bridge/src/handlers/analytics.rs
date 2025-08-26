//! Analytics handlers

use axum::{extract::State, response::Json};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    rest::AppState,
    types::*,
};
use bridge_core::types::analytics::{AnalyticsError, AnalyticsStatus};
use query_engine::{
    analytics::{
        Analytics, AnalyticsConfig, AnomalyDetector, AnomalyDetectorConfig, AnomalyResult,
        AnomalyType, ClusterAnalyzer, ClusterAnalyzerConfig, ClusterResult, ClusterType,
        ForecastModel, ForecastResult, Forecaster, ForecasterConfig, MLModel, MLModelConfig,
        MLPredictor, MLPredictorConfig, PredictionResult, TimeSeriesAnalyzer,
        TimeSeriesAnalyzerConfig, TimeSeriesData, TimeSeriesPoint, TimeSeriesResult,
        time_series::{TrendAnalysis, SeasonalityAnalysis, TrendDirection, StatisticalSummary},
        anomaly_detection::DefaultAnomalyDetector,
        clustering::DefaultClusterAnalyzer,
        forecasting::DefaultForecaster,
    },
    executors::{QueryExecutor, QueryResult, MockExecutor, datafusion_executor::{DataFusionExecutor, DataFusionConfig}},
    parsers::ParsedQuery,
};
use bridge_core::BridgeResult;

/// Analytics service for processing analytics requests
pub struct AnalyticsService {
    config: AnalyticsConfig,
    time_series_analyzer: TimeSeriesAnalyzer,
    query_executor: DataFusionExecutor,
    anomaly_detector: DefaultAnomalyDetector,
    cluster_analyzer: DefaultClusterAnalyzer,
    forecaster: DefaultForecaster,
}

impl AnalyticsService {
    /// Create a new analytics service
    pub fn new() -> Self {
        let analytics_config = AnalyticsConfig::default();
        let time_series_config = TimeSeriesAnalyzerConfig::default();
        let query_config = DataFusionConfig::new();
        let anomaly_config = AnomalyDetectorConfig::default();
        let cluster_config = ClusterAnalyzerConfig::default();
        let forecaster_config = ForecasterConfig::default();

        Self {
            config: analytics_config,
            time_series_analyzer: TimeSeriesAnalyzer::new(time_series_config),
            query_executor: DataFusionExecutor::new(DataFusionConfig::new()),
            anomaly_detector: DefaultAnomalyDetector::new(anomaly_config),
            cluster_analyzer: DefaultClusterAnalyzer::new(cluster_config),
            forecaster: DefaultForecaster::new(forecaster_config),
        }
    }

    /// Initialize the analytics service
    pub async fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Initialize query executor
        self.query_executor.init().await?;

        // Initialize analytics components
        // Note: TimeSeriesAnalyzer doesn't have an init method, so we skip it
        // self.time_series_analyzer.init().await?;
        // self.anomaly_detector.init().await?;
        // self.cluster_analyzer.init().await?;
        // self.forecaster.init().await?;

        Ok(())
    }

    /// Calculate statistical summary from a vector of values
    fn calculate_statistical_summary(&self, values: &[f64]) -> StatisticalSummary {
        if values.is_empty() {
            return StatisticalSummary {
                count: 0,
                mean: 0.0,
                std_dev: 0.0,
                min: 0.0,
                max: 0.0,
                median: 0.0,
                q1: 0.0,
                q3: 0.0,
                skewness: 0.0,
                kurtosis: 0.0,
            };
        }

        let count = values.len();
        let mean = values.iter().sum::<f64>() / count as f64;
        
        // Calculate standard deviation
        let variance = values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        // Calculate min and max
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        // Calculate median and quartiles
        let mut sorted_values = values.to_vec();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let median = if count % 2 == 0 {
            (sorted_values[count / 2 - 1] + sorted_values[count / 2]) / 2.0
        } else {
            sorted_values[count / 2]
        };

        let q1 = if count >= 4 {
            let q1_idx = count / 4;
            if count % 4 == 0 {
                (sorted_values[q1_idx - 1] + sorted_values[q1_idx]) / 2.0
            } else {
                sorted_values[q1_idx]
            }
        } else {
            sorted_values.first().copied().unwrap_or(0.0)
        };

        let q3 = if count >= 4 {
            let q3_idx = 3 * count / 4;
            if count % 4 == 0 {
                (sorted_values[q3_idx - 1] + sorted_values[q3_idx]) / 2.0
            } else {
                sorted_values[q3_idx]
            }
        } else {
            sorted_values.last().copied().unwrap_or(0.0)
        };

        // Calculate skewness
        let skewness = if std_dev > 0.0 {
            values.iter()
                .map(|x| ((x - mean) / std_dev).powi(3))
                .sum::<f64>() / count as f64
        } else {
            0.0
        };

        // Calculate kurtosis
        let kurtosis = if std_dev > 0.0 {
            values.iter()
                .map(|x| ((x - mean) / std_dev).powi(4))
                .sum::<f64>() / count as f64 - 3.0 // Excess kurtosis
        } else {
            0.0
        };

        StatisticalSummary {
            count,
            mean,
            std_dev,
            min,
            max,
            median,
            q1,
            q3,
            skewness,
            kurtosis,
        }
    }

    /// Detect anomalies in time series data
    async fn detect_anomalies(&self, time_series_data: &TimeSeriesData) -> Result<Vec<AnomalyResult>, Box<dyn std::error::Error + Send + Sync>> {
        // Use the anomaly detector to find anomalies
        let anomalies = self.anomaly_detector.detect_anomalies(time_series_data).await?;
        
        Ok(anomalies)
    }

    /// Perform trend analysis on time series data
    async fn analyze_trends(&self, time_series_data: &TimeSeriesData) -> Result<Option<TrendAnalysis>, Box<dyn std::error::Error + Send + Sync>> {
        if time_series_data.points.len() < 2 {
            return Ok(None);
        }

        // Simple linear regression for trend analysis
        let points = &time_series_data.points;
        let n = points.len() as f64;
        let x_values: Vec<f64> = points.iter().enumerate().map(|(i, _)| i as f64).collect();
        let y_values: Vec<f64> = points.iter().map(|p| p.value).collect();

        let sum_x: f64 = x_values.iter().sum();
        let sum_y: f64 = y_values.iter().sum();
        let sum_xy: f64 = x_values
            .iter()
            .zip(y_values.iter())
            .map(|(x, y)| x * y)
            .sum();
        let sum_x2: f64 = x_values.iter().map(|x| x * x).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        // Calculate R-squared
        let y_mean = sum_y / n;
        let ss_tot: f64 = y_values.iter().map(|y| (y - y_mean).powi(2)).sum();
        let ss_res: f64 = x_values
            .iter()
            .zip(y_values.iter())
            .map(|(x, y)| (y - (slope * x + intercept)).powi(2))
            .sum();
        let r_squared = 1.0 - (ss_res / ss_tot);

        let direction = if slope > 0.01 {
            TrendDirection::Increasing
        } else if slope < -0.01 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        let description = match direction {
            TrendDirection::Increasing => format!("Increasing trend with slope {:.4}", slope),
            TrendDirection::Decreasing => format!("Decreasing trend with slope {:.4}", slope),
            TrendDirection::Stable => "No significant trend detected".to_string(),
        };

        Ok(Some(TrendAnalysis {
            direction,
            slope,
            strength: r_squared,
            confidence: r_squared.min(1.0),
            description,
        }))
    }

    /// Perform seasonality analysis on time series data
    async fn analyze_seasonality(&self, time_series_data: &TimeSeriesData) -> Result<Option<SeasonalityAnalysis>, Box<dyn std::error::Error + Send + Sync>> {
        if time_series_data.points.len() < 10 {
            return Ok(None);
        }

        // Simple seasonality detection using autocorrelation
        let values: Vec<f64> = time_series_data.points.iter().map(|p| p.value).collect();
        let n = values.len();

        // Calculate autocorrelation for different lags
        let max_lag = (n / 2).min(100);
        let mut autocorr = Vec::new();

        for lag in 1..=max_lag {
            let mut sum = 0.0;
            let mut count = 0;

            for i in 0..(n - lag) {
                sum += values[i] * values[i + lag];
                count += 1;
            }

            if count > 0 {
                autocorr.push(sum / count as f64);
            }
        }

        // Find the lag with maximum autocorrelation
        if let Some((max_lag_idx, _)) = autocorr
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        {
            let period_secs = (max_lag_idx + 1) as u64 * 3600; // Assuming 1-hour intervals
            let strength = autocorr[max_lag_idx];

            if strength > 0.3 {
                let description = format!(
                    "Seasonal pattern detected with period {} seconds",
                    period_secs
                );

                return Ok(Some(SeasonalityAnalysis {
                    period_secs,
                    strength,
                    pattern: vec![strength; 1], // Simple pattern with single value
                    confidence: strength.min(1.0),
                    description,
                }));
            }
        }

        Ok(None)
    }

    /// Perform clustering analysis on data
    async fn perform_clustering(&self, data: &[Vec<f64>], labels: &[String]) -> Result<Vec<ClusterResult>, Box<dyn std::error::Error + Send + Sync>> {
        // Use the cluster analyzer to perform clustering
        let cluster_results = self.cluster_analyzer.cluster_data(data, labels).await?;
        
        Ok(cluster_results)
    }

    /// Process workflow analytics
    async fn process_workflow_analytics(
        &self,
        request: &bridge_core::types::AnalyticsRequest,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Extract workflow data from request parameters
        let time_range = request.parameters.get("time_range")
            .and_then(|v| v.as_str())
            .unwrap_or("1h");

        // Query workflow telemetry data
        let query = format!(
            "SELECT timestamp, service_name, operation_name, duration_ms, status_code 
             FROM telemetry 
             WHERE timestamp >= NOW() - INTERVAL '{}' 
             AND record_type = 'trace'
             ORDER BY timestamp",
            time_range
        );

        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: query.clone(),
            ast: query_engine::parsers::QueryAst {
                root: query_engine::parsers::AstNode {
                    node_type: query_engine::parsers::NodeType::Statement,
                    value: Some(query),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                node_count: 1,
                depth: 1,
            },
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let query_result = self.query_executor.execute(parsed_query).await?;

        // Convert query results to time series data
        let time_series_data = self.convert_query_results_to_time_series(&query_result, "duration_ms")?;

        // Calculate statistical summary
        let values: Vec<f64> = time_series_data.points.iter().map(|p| p.value).collect();
        let statistics = self.calculate_statistical_summary(&values);

        // Perform trend analysis
        let trend_analysis = self.analyze_trends(&time_series_data).await?;

        // Perform seasonality analysis
        let seasonality_analysis = self.analyze_seasonality(&time_series_data).await?;

        // Detect anomalies
        let anomalies = self.detect_anomalies(&time_series_data).await?;

        let time_series_result = TimeSeriesResult {
            id: Uuid::new_v4(),
            series_id: time_series_data.id,
            timestamp: chrono::Utc::now(),
            statistics,
            trend_analysis,
            seasonality_analysis,
            metadata: HashMap::new(),
        };

        // Generate workflow insights
        let insights = self.generate_workflow_insights(&time_series_result, &anomalies)?;

        Ok(serde_json::json!({
            "analytics_type": "workflow_analytics",
            "timestamp": chrono::Utc::now(),
            "data_points": time_series_data.points.len(),
            "time_series_analysis": {
                "trend": time_series_result.trend_analysis,
                "seasonality": time_series_result.seasonality_analysis,
                "statistics": time_series_result.statistics,
            },
            "anomalies": anomalies.iter().map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "type": format!("{:?}", a.anomaly_type),
                    "timestamp": a.timestamp,
                    "confidence": a.confidence,
                    "severity": a.severity,
                    "description": a.description,
                })
            }).collect::<Vec<_>>(),
            "insights": insights,
            "workflow_metrics": {
                "total_operations": query_result.data.len(),
                "avg_duration": time_series_result.statistics.mean,
                "std_dev_duration": time_series_result.statistics.std_dev,
                "error_rate": self.calculate_error_rate(&query_result),
            }
        }))
    }

    /// Process agent analytics
    async fn process_agent_analytics(
        &self,
        request: &bridge_core::types::AnalyticsRequest,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Extract agent data from request parameters
        let agent_id = request.parameters.get("agent_id")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        // Query agent performance data
        let query = format!(
            "SELECT timestamp, agent_id, task_type, execution_time_ms, success_rate, cpu_usage, memory_usage
             FROM agent_metrics 
             WHERE timestamp >= NOW() - INTERVAL '1h'
             {} 
             ORDER BY timestamp",
            if agent_id != "all" { format!("AND agent_id = '{}'", agent_id) } else { String::new() }
        );

        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: query.clone(),
            ast: query_engine::parsers::QueryAst {
                root: query_engine::parsers::AstNode {
                    node_type: query_engine::parsers::NodeType::Statement,
                    value: Some(query),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                node_count: 1,
                depth: 1,
            },
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let query_result = self.query_executor.execute(parsed_query).await?;

        // Convert to time series data for different metrics
        let execution_time_data = self.convert_query_results_to_time_series(&query_result, "execution_time_ms")?;
        let cpu_usage_data = self.convert_query_results_to_time_series(&query_result, "cpu_usage")?;
        let memory_usage_data = self.convert_query_results_to_time_series(&query_result, "memory_usage")?;

        // Calculate statistical summaries
        let execution_values: Vec<f64> = execution_time_data.points.iter().map(|p| p.value).collect();
        let cpu_values: Vec<f64> = cpu_usage_data.points.iter().map(|p| p.value).collect();
        let memory_values: Vec<f64> = memory_usage_data.points.iter().map(|p| p.value).collect();

        let execution_stats = self.calculate_statistical_summary(&execution_values);
        let cpu_stats = self.calculate_statistical_summary(&cpu_values);
        let memory_stats = self.calculate_statistical_summary(&memory_values);

        // Perform trend analysis
        let execution_trend = self.analyze_trends(&execution_time_data).await?;
        let cpu_trend = self.analyze_trends(&cpu_usage_data).await?;
        let memory_trend = self.analyze_trends(&memory_usage_data).await?;

        // Perform seasonality analysis
        let execution_seasonality = self.analyze_seasonality(&execution_time_data).await?;
        let cpu_seasonality = self.analyze_seasonality(&cpu_usage_data).await?;
        let memory_seasonality = self.analyze_seasonality(&memory_usage_data).await?;

        // Detect anomalies
        let execution_anomalies = self.detect_anomalies(&execution_time_data).await?;
        let cpu_anomalies = self.detect_anomalies(&cpu_usage_data).await?;
        let memory_anomalies = self.detect_anomalies(&memory_usage_data).await?;

        let execution_analysis = TimeSeriesResult {
            id: Uuid::new_v4(),
            series_id: execution_time_data.id,
            timestamp: chrono::Utc::now(),
            statistics: execution_stats,
            trend_analysis: execution_trend,
            seasonality_analysis: execution_seasonality,
            metadata: HashMap::new(),
        };
        let cpu_analysis = TimeSeriesResult {
            id: Uuid::new_v4(),
            series_id: cpu_usage_data.id,
            timestamp: chrono::Utc::now(),
            statistics: cpu_stats,
            trend_analysis: cpu_trend,
            seasonality_analysis: cpu_seasonality,
            metadata: HashMap::new(),
        };
        let memory_analysis = TimeSeriesResult {
            id: Uuid::new_v4(),
            series_id: memory_usage_data.id,
            timestamp: chrono::Utc::now(),
            statistics: memory_stats,
            trend_analysis: memory_trend,
            seasonality_analysis: memory_seasonality,
            metadata: HashMap::new(),
        };

        // Generate agent insights
        let insights = self.generate_agent_insights(&execution_analysis, &cpu_analysis, &memory_analysis)?;

        Ok(serde_json::json!({
            "analytics_type": "agent_analytics",
            "timestamp": chrono::Utc::now(),
            "data_points": execution_time_data.points.len(),
            "agent_id": agent_id,
            "performance_analysis": {
                "execution_time": {
                    "trend": execution_analysis.trend_analysis,
                    "statistics": execution_analysis.statistics,
                    "anomalies": execution_anomalies.len(),
                },
                "cpu_usage": {
                    "trend": cpu_analysis.trend_analysis,
                    "statistics": cpu_analysis.statistics,
                    "anomalies": cpu_anomalies.len(),
                },
                "memory_usage": {
                    "trend": memory_analysis.trend_analysis,
                    "statistics": memory_analysis.statistics,
                    "anomalies": memory_anomalies.len(),
                }
            },
            "anomalies": {
                "execution_time": execution_anomalies.iter().map(|a: &AnomalyResult| {
                    serde_json::json!({
                        "id": a.id,
                        "timestamp": a.timestamp,
                        "confidence": a.confidence,
                        "severity": a.severity,
                        "description": a.description,
                    })
                }).collect::<Vec<_>>(),
                "cpu_usage": cpu_anomalies.iter().map(|a: &AnomalyResult| {
                    serde_json::json!({
                        "id": a.id,
                        "timestamp": a.timestamp,
                        "confidence": a.confidence,
                        "severity": a.severity,
                        "description": a.description,
                    })
                }).collect::<Vec<_>>(),
                "memory_usage": memory_anomalies.iter().map(|a: &AnomalyResult| {
                    serde_json::json!({
                        "id": a.id,
                        "timestamp": a.timestamp,
                        "confidence": a.confidence,
                        "severity": a.severity,
                        "description": a.description,
                    })
                }).collect::<Vec<_>>(),
            },
            "insights": insights,
        }))
    }

    /// Process multi-repo analytics
    async fn process_multi_repo_analytics(
        &self,
        request: &bridge_core::types::AnalyticsRequest,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Query multi-repo coordination data
        let query = "
            SELECT timestamp, repo_name, branch_name, commit_count, merge_count, 
                   conflict_count, collaboration_score
            FROM repo_metrics 
            WHERE timestamp >= NOW() - INTERVAL '24h'
            ORDER BY timestamp
        ";

        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: query.to_string(),
            ast: query_engine::parsers::QueryAst {
                root: query_engine::parsers::AstNode {
                    node_type: query_engine::parsers::NodeType::Statement,
                    value: Some(query.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                node_count: 1,
                depth: 1,
            },
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let query_result = self.query_executor.execute(parsed_query).await?;

        // Convert to time series data for different metrics
        let commit_data = self.convert_query_results_to_time_series(&query_result, "commit_count")?;
        let merge_data = self.convert_query_results_to_time_series(&query_result, "merge_count")?;
        let collaboration_data = self.convert_query_results_to_time_series(&query_result, "collaboration_score")?;

        // Prepare data for clustering
        let mut cluster_data = Vec::new();
        let mut cluster_labels = Vec::new();
        
        for row in &query_result.data {
            if let (Some(commit_count), Some(merge_count), Some(collaboration_score)) = (
                row.data.get("commit_count"),
                row.data.get("merge_count"),
                row.data.get("collaboration_score")
            ) {
                let commit_val = match commit_count {
                    query_engine::executors::QueryValue::Float(v) => *v,
                    query_engine::executors::QueryValue::Integer(v) => *v as f64,
                    _ => 0.0,
                };
                let merge_val = match merge_count {
                    query_engine::executors::QueryValue::Float(v) => *v,
                    query_engine::executors::QueryValue::Integer(v) => *v as f64,
                    _ => 0.0,
                };
                let collab_val = match collaboration_score {
                    query_engine::executors::QueryValue::Float(v) => *v,
                    query_engine::executors::QueryValue::Integer(v) => *v as f64,
                    _ => 0.0,
                };
                
                cluster_data.push(vec![commit_val, merge_val, collab_val]);
                
                if let Some(repo_name) = row.data.get("repo_name") {
                    let repo = match repo_name {
                        query_engine::executors::QueryValue::String(s) => s.clone(),
                        _ => "unknown".to_string(),
                    };
                    cluster_labels.push(repo);
                } else {
                    cluster_labels.push("unknown".to_string());
                }
            }
        }

        // Perform clustering analysis
        let cluster_results = if !cluster_data.is_empty() {
            self.perform_clustering(&cluster_data, &cluster_labels).await?
        } else {
            Vec::new()
        };

        // Calculate statistical summaries
        let commit_values: Vec<f64> = commit_data.points.iter().map(|p| p.value).collect();
        let merge_values: Vec<f64> = merge_data.points.iter().map(|p| p.value).collect();

        let commit_stats = self.calculate_statistical_summary(&commit_values);
        let merge_stats = self.calculate_statistical_summary(&merge_values);

        // Perform trend analysis
        let commit_trend = self.analyze_trends(&commit_data).await?;
        let merge_trend = self.analyze_trends(&merge_data).await?;

        // Perform seasonality analysis
        let commit_seasonality = self.analyze_seasonality(&commit_data).await?;
        let merge_seasonality = self.analyze_seasonality(&merge_data).await?;

        let commit_analysis = TimeSeriesResult {
            id: Uuid::new_v4(),
            series_id: commit_data.id,
            timestamp: chrono::Utc::now(),
            statistics: commit_stats,
            trend_analysis: commit_trend,
            seasonality_analysis: commit_seasonality,
            metadata: HashMap::new(),
        };
        let merge_analysis = TimeSeriesResult {
            id: Uuid::new_v4(),
            series_id: merge_data.id,
            timestamp: chrono::Utc::now(),
            statistics: merge_stats,
            trend_analysis: merge_trend,
            seasonality_analysis: merge_seasonality,
            metadata: HashMap::new(),
        };

        // Generate multi-repo insights
        let insights = self.generate_multi_repo_insights(&commit_analysis, &merge_analysis, &cluster_results)?;

        Ok(serde_json::json!({
            "analytics_type": "multi_repo_analytics",
            "timestamp": chrono::Utc::now(),
            "data_points": commit_data.points.len(),
            "repository_analysis": {
                "commit_trends": {
                    "trend": commit_analysis.trend_analysis,
                    "statistics": commit_analysis.statistics,
                },
                "merge_trends": {
                    "trend": merge_analysis.trend_analysis,
                    "statistics": merge_analysis.statistics,
                },
                "collaboration_clusters": cluster_results.iter().map(|c| {
                    serde_json::json!({
                        "cluster_id": c.id,
                        "cluster_type": format!("{:?}", c.cluster_type),
                        "label": c.label,
                        "size": c.size,
                        "score": c.score,
                    })
                }).collect::<Vec<_>>(),
            },
            "insights": insights,
            "coordination_metrics": {
                "total_repos": self.count_unique_values(&query_result, "repo_name"),
                "total_branches": self.count_unique_values(&query_result, "branch_name"),
                "avg_collaboration_score": collaboration_data.points.iter()
                    .map(|p| p.value)
                    .sum::<f64>() / collaboration_data.points.len() as f64,
            }
        }))
    }

    /// Process real-time alerting
    async fn process_real_time_alerting(
        &self,
        request: &bridge_core::types::AnalyticsRequest,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
            // Extract alert thresholds from request
    let empty_map = serde_json::Map::new();
    let thresholds = request.parameters.get("thresholds")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty_map);

        // Query recent telemetry data for alerting
        let query = "
            SELECT timestamp, service_name, error_rate, response_time_ms, cpu_usage, memory_usage
            FROM telemetry 
            WHERE timestamp >= NOW() - INTERVAL '5m'
            ORDER BY timestamp DESC
        ";

        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: query.to_string(),
            ast: query_engine::parsers::QueryAst {
                root: query_engine::parsers::AstNode {
                    node_type: query_engine::parsers::NodeType::Statement,
                    value: Some(query.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                node_count: 1,
                depth: 1,
            },
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let query_result = self.query_executor.execute(parsed_query).await?;

        // Convert to time series data
        let error_rate_data = self.convert_query_results_to_time_series(&query_result, "error_rate")?;
        let response_time_data = self.convert_query_results_to_time_series(&query_result, "response_time_ms")?;
        let cpu_data = self.convert_query_results_to_time_series(&query_result, "cpu_usage")?;
        let memory_data = self.convert_query_results_to_time_series(&query_result, "memory_usage")?;

        // Detect anomalies
        let error_anomalies = self.detect_anomalies(&error_rate_data).await?;
        let response_anomalies = self.detect_anomalies(&response_time_data).await?;
        let cpu_anomalies = self.detect_anomalies(&cpu_data).await?;
        let memory_anomalies = self.detect_anomalies(&memory_data).await?;

        // Generate alerts based on thresholds and anomalies
        let alerts = self.generate_alerts(
            &error_anomalies,
            &response_anomalies,
            &cpu_anomalies,
            &memory_anomalies,
            thresholds,
        )?;

        Ok(serde_json::json!({
            "analytics_type": "real_time_alerting",
            "timestamp": chrono::Utc::now(),
            "data_points": error_rate_data.points.len(),
            "alerts": alerts,
            "anomaly_summary": {
                "error_rate_anomalies": error_anomalies.len(),
                "response_time_anomalies": response_anomalies.len(),
                "cpu_anomalies": cpu_anomalies.len(),
                "memory_anomalies": memory_anomalies.len(),
            },
            "current_metrics": {
                "avg_error_rate": error_rate_data.points.iter().map(|p| p.value).sum::<f64>() / error_rate_data.points.len() as f64,
                "avg_response_time": response_time_data.points.iter().map(|p| p.value).sum::<f64>() / response_time_data.points.len() as f64,
                "avg_cpu_usage": cpu_data.points.iter().map(|p| p.value).sum::<f64>() / cpu_data.points.len() as f64,
                "avg_memory_usage": memory_data.points.iter().map(|p| p.value).sum::<f64>() / memory_data.points.len() as f64,
            }
        }))
    }

    /// Process interactive querying
    async fn process_interactive_querying(
        &self,
        request: &bridge_core::types::AnalyticsRequest,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Extract custom query from request
        let custom_query = request.parameters.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("SELECT * FROM telemetry LIMIT 100");

        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: custom_query.to_string(),
            ast: query_engine::parsers::QueryAst {
                root: query_engine::parsers::AstNode {
                    node_type: query_engine::parsers::NodeType::Statement,
                    value: Some(custom_query.to_string()),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                node_count: 1,
                depth: 1,
            },
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let query_result = self.query_executor.execute(parsed_query).await?;

        // Get query execution statistics
        let execution_stats = self.query_executor.get_execution_stats(custom_query).await?;

        Ok(serde_json::json!({
            "analytics_type": "interactive_querying",
            "timestamp": chrono::Utc::now(),
            "query": custom_query,
            "results": {
                "row_count": query_result.data.len(),
                "execution_time_ms": query_result.execution_time_ms,
                "columns": if !query_result.data.is_empty() {
                    query_result.data[0].data.keys().cloned().collect::<Vec<_>>()
                } else {
                    vec![]
                },
                "sample_data": query_result.data.iter().take(10).map(|row| {
                    serde_json::json!(row.data)
                }).collect::<Vec<_>>(),
            },
            "execution_stats": {
                "logical_plan": execution_stats.logical_plan,
                "physical_plan": execution_stats.physical_plan,
                "estimated_rows": execution_stats.estimated_rows,
                "estimated_cost": execution_stats.estimated_cost,
            }
        }))
    }

    /// Process data visualization
    async fn process_data_visualization(
        &self,
        request: &bridge_core::types::AnalyticsRequest,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Extract visualization parameters
        let chart_type = request.parameters.get("chart_type")
            .and_then(|v| v.as_str())
            .unwrap_or("line");

        let time_range = request.parameters.get("time_range")
            .and_then(|v| v.as_str())
            .unwrap_or("1h");

        // Query data for visualization
        let query = format!(
            "SELECT timestamp, service_name, response_time_ms, error_rate, request_count
             FROM telemetry 
             WHERE timestamp >= NOW() - INTERVAL '{}'
             ORDER BY timestamp",
            time_range
        );

        let parsed_query = ParsedQuery {
            id: Uuid::new_v4(),
            query_text: query.clone(),
            ast: query_engine::parsers::QueryAst {
                root: query_engine::parsers::AstNode {
                    node_type: query_engine::parsers::NodeType::Statement,
                    value: Some(query),
                    children: vec![],
                    metadata: HashMap::new(),
                },
                node_count: 1,
                depth: 1,
            },
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let query_result = self.query_executor.execute(parsed_query).await?;

        // Generate visualization data
        let visualization_data = self.generate_visualization_data(&query_result, chart_type)?;

        Ok(serde_json::json!({
            "analytics_type": "data_visualization",
            "timestamp": chrono::Utc::now(),
            "chart_type": chart_type,
            "time_range": time_range,
            "data_points": query_result.data.len(),
            "visualization_data": visualization_data,
            "chart_options": {
                "title": format!("Telemetry Data - {}", time_range),
                "x_axis": "timestamp",
                "y_axis": "value",
                "series": ["response_time_ms", "error_rate", "request_count"],
            }
        }))
    }

    /// Convert query results to time series data
    pub fn convert_query_results_to_time_series(
        &self,
        query_result: &query_engine::executors::QueryResult,
        value_column: &str,
    ) -> Result<TimeSeriesData, Box<dyn std::error::Error + Send + Sync>> {
        let mut points = Vec::new();

        for row in &query_result.data {
            if let Some(timestamp_str) = row.data.get("timestamp") {
                if let Some(value) = row.data.get(value_column) {
                    let timestamp = match timestamp_str {
                        query_engine::executors::QueryValue::String(ts) => {
                            chrono::DateTime::parse_from_rfc3339(ts)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now())
                        }
                        query_engine::executors::QueryValue::Integer(ts) => {
                            chrono::DateTime::from_timestamp_millis(*ts)
                                .unwrap_or_else(|| chrono::Utc::now())
                        }
                        _ => chrono::Utc::now(),
                    };

                    let value = match value {
                        query_engine::executors::QueryValue::Float(v) => *v,
                        query_engine::executors::QueryValue::Integer(v) => *v as f64,
                        query_engine::executors::QueryValue::String(v) => v.parse().unwrap_or(0.0),
                        _ => 0.0,
                    };

                    points.push(TimeSeriesPoint {
                        timestamp,
                        value,
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Ok(TimeSeriesData {
            id: Uuid::new_v4(),
            name: value_column.to_string(),
            points,
            metadata: HashMap::new(),
        })
    }

    /// Generate workflow insights
    fn generate_workflow_insights(
        &self,
        time_series_result: &TimeSeriesResult,
        anomalies: &[AnomalyResult],
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut insights = Vec::new();

        // Trend insights
        if let Some(trend) = &time_series_result.trend_analysis {
            insights.push(serde_json::json!({
                "type": "trend",
                "description": format!("Workflow performance shows a {} trend with slope {:.2}", 
                    if trend.slope > 0.0 { "positive" } else { "negative" }, trend.slope),
                "confidence": trend.confidence,
                "severity": if trend.slope.abs() > 0.1 { "high" } else { "low" },
            }));
        }

        // Anomaly insights
        if !anomalies.is_empty() {
            insights.push(serde_json::json!({
                "type": "anomaly",
                "description": format!("Detected {} anomalies in workflow performance", anomalies.len()),
                "count": anomalies.len(),
                "severity": if anomalies.iter().any(|a| a.severity > 0.8) { "high" } else { "medium" },
            }));
        }

        // Statistical insights
        let stats = &time_series_result.statistics;
        insights.push(serde_json::json!({
            "type": "statistics",
            "description": format!("Average workflow duration: {:.2}ms, Std Dev: {:.2}ms", 
                stats.mean, stats.std_dev),
            "mean": stats.mean,
            "std_dev": stats.std_dev,
            "min": stats.min,
            "max": stats.max,
        }));

        Ok(insights)
    }

    /// Generate agent insights
    fn generate_agent_insights(
        &self,
        execution_analysis: &TimeSeriesResult,
        cpu_analysis: &TimeSeriesResult,
        memory_analysis: &TimeSeriesResult,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut insights = Vec::new();

        // Performance insights
        if let Some(trend) = &execution_analysis.trend_analysis {
            insights.push(serde_json::json!({
                "type": "performance",
                "description": format!("Agent execution time shows a {} trend", 
                    if trend.slope > 0.0 { "degrading" } else { "improving" }),
                "trend": if trend.slope > 0.0 { "degrading" } else { "improving" },
                "confidence": trend.confidence,
            }));
        }

        // Resource utilization insights
        let avg_cpu = cpu_analysis.statistics.mean;
        let avg_memory = memory_analysis.statistics.mean;

        insights.push(serde_json::json!({
            "type": "resource_utilization",
            "description": format!("Average CPU usage: {:.1}%, Memory usage: {:.1}%", avg_cpu, avg_memory),
            "cpu_usage": avg_cpu,
            "memory_usage": avg_memory,
            "status": if avg_cpu > 80.0 || avg_memory > 80.0 { "warning" } else { "normal" },
        }));

        Ok(insights)
    }

    /// Generate multi-repo insights
    fn generate_multi_repo_insights(
        &self,
        commit_analysis: &TimeSeriesResult,
        merge_analysis: &TimeSeriesResult,
        cluster_results: &[ClusterResult],
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut insights = Vec::new();

        // Collaboration insights
        insights.push(serde_json::json!({
            "type": "collaboration",
            "description": format!("Identified {} collaboration patterns across repositories", cluster_results.len()),
            "cluster_count": cluster_results.len(),
            "collaboration_score": cluster_results.iter().map(|c| c.score).sum::<f64>() / cluster_results.len() as f64,
        }));

        // Activity insights
        if let Some(trend) = &commit_analysis.trend_analysis {
            insights.push(serde_json::json!({
                "type": "activity",
                "description": format!("Commit activity shows a {} trend", 
                    if trend.slope > 0.0 { "increasing" } else { "decreasing" }),
                "trend": if trend.slope > 0.0 { "increasing" } else { "decreasing" },
                "confidence": trend.confidence,
            }));
        }

        Ok(insights)
    }

    /// Generate alerts
    pub fn generate_alerts(
        &self,
        error_anomalies: &[AnomalyResult],
        response_anomalies: &[AnomalyResult],
        cpu_anomalies: &[AnomalyResult],
        memory_anomalies: &[AnomalyResult],
        thresholds: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut alerts = Vec::new();

        // Error rate alerts
        if !error_anomalies.is_empty() {
            alerts.push(serde_json::json!({
                "type": "error_rate_anomaly",
                "severity": "high",
                "description": format!("Detected {} error rate anomalies", error_anomalies.len()),
                "count": error_anomalies.len(),
                "timestamp": chrono::Utc::now(),
            }));
        }

        // Response time alerts
        if !response_anomalies.is_empty() {
            alerts.push(serde_json::json!({
                "type": "response_time_anomaly",
                "severity": "medium",
                "description": format!("Detected {} response time anomalies", response_anomalies.len()),
                "count": response_anomalies.len(),
                "timestamp": chrono::Utc::now(),
            }));
        }

        // Resource alerts
        if !cpu_anomalies.is_empty() || !memory_anomalies.is_empty() {
            alerts.push(serde_json::json!({
                "type": "resource_anomaly",
                "severity": "medium",
                "description": format!("Detected {} CPU and {} memory anomalies", 
                    cpu_anomalies.len(), memory_anomalies.len()),
                "cpu_anomalies": cpu_anomalies.len(),
                "memory_anomalies": memory_anomalies.len(),
                "timestamp": chrono::Utc::now(),
            }));
        }

        Ok(alerts)
    }

    /// Generate visualization data
    fn generate_visualization_data(
        &self,
        query_result: &query_engine::executors::QueryResult,
        chart_type: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        match chart_type {
            "line" => {
                let mut series_data = HashMap::new();
                
                for row in &query_result.data {
                    if let Some(timestamp) = row.data.get("timestamp") {
                        let ts = match timestamp {
                            query_engine::executors::QueryValue::String(ts) => ts.clone(),
                            query_engine::executors::QueryValue::Integer(ts) => ts.to_string(),
                            _ => continue,
                        };

                        for (key, value) in &row.data {
                            if key != "timestamp" {
                                let val = match value {
                                    query_engine::executors::QueryValue::Float(v) => *v,
                                    query_engine::executors::QueryValue::Integer(v) => *v as f64,
                                    query_engine::executors::QueryValue::String(v) => v.parse().unwrap_or(0.0),
                                    _ => 0.0,
                                };

                                series_data.entry(key.clone())
                                    .or_insert_with(Vec::new)
                                    .push(serde_json::json!({
                                        "timestamp": ts,
                                        "value": val,
                                    }));
                            }
                        }
                    }
                }

                Ok(serde_json::json!(series_data))
            }
            "bar" => {
                // Generate aggregated data for bar charts
                let mut aggregated_data = HashMap::new();
                
                for row in &query_result.data {
                    if let Some(service_name) = row.data.get("service_name") {
                        let service = match service_name {
                            query_engine::executors::QueryValue::String(s) => s.clone(),
                            _ => "unknown".to_string(),
                        };

                        if let Some(response_time) = row.data.get("response_time_ms") {
                            let rt = match response_time {
                                query_engine::executors::QueryValue::Float(v) => *v,
                                query_engine::executors::QueryValue::Integer(v) => *v as f64,
                                _ => 0.0,
                            };

                            let entry = aggregated_data.entry(service.clone())
                                .or_insert_with(|| {
                                    serde_json::json!({
                                        "service": service,
                                        "avg_response_time": 0.0,
                                        "count": 0,
                                        "total_response_time": 0.0,
                                    })
                                });

                            if let serde_json::Value::Object(ref mut obj) = entry {
                                let count = obj.get("count").and_then(|v| v.as_u64()).unwrap_or(0) + 1;
                                let total = obj.get("total_response_time").and_then(|v| v.as_f64()).unwrap_or(0.0) + rt;
                                
                                obj.insert("count".to_string(), serde_json::Value::Number(count.into()));
                                obj.insert("total_response_time".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(total).unwrap()));
                                obj.insert("avg_response_time".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(total / count as f64).unwrap()));
                            }
                        }
                    }
                }

                Ok(serde_json::json!(aggregated_data.values().collect::<Vec<_>>()))
            }
            _ => {
                // Default to line chart
                self.generate_visualization_data(query_result, "line")
            }
        }
    }

    /// Calculate error rate from query results
    fn calculate_error_rate(&self, query_result: &query_engine::executors::QueryResult) -> f64 {
        let mut total_requests = 0;
        let mut error_requests = 0;

        for row in &query_result.data {
            if let Some(status_code) = row.data.get("status_code") {
                total_requests += 1;
                
                let status = match status_code {
                    query_engine::executors::QueryValue::String(s) => s.parse::<u16>().unwrap_or(200),
                    query_engine::executors::QueryValue::Integer(i) => *i as u16,
                    _ => 200,
                };

                if status >= 400 {
                    error_requests += 1;
                }
            }
        }

        if total_requests > 0 {
            error_requests as f64 / total_requests as f64
        } else {
            0.0
        }
    }

    /// Count unique values in a column
    fn count_unique_values(&self, query_result: &query_engine::executors::QueryResult, column: &str) -> usize {
        let mut unique_values = std::collections::HashSet::new();
        
        for row in &query_result.data {
            if let Some(value) = row.data.get(column) {
                let val_str = match value {
                    query_engine::executors::QueryValue::String(s) => s.clone(),
                    query_engine::executors::QueryValue::Integer(i) => i.to_string(),
                    query_engine::executors::QueryValue::Float(f) => f.to_string(),
                    _ => continue,
                };
                unique_values.insert(val_str);
            }
        }

        unique_values.len()
    }
}

/// Execute analytics processing
async fn execute_analytics_processing(
    request: &AnalyticsRequest,
) -> Result<bridge_core::types::AnalyticsResponse, Box<dyn std::error::Error + Send + Sync>> {
    // Create analytics service
    let mut analytics_service = AnalyticsService::new();
    analytics_service.init().await?;

    // Process analytics based on type
    let data = match request.analytics_type {
        bridge_core::types::AnalyticsType::WorkflowAnalytics => {
            analytics_service.process_workflow_analytics(&request.parameters).await?
        }
        bridge_core::types::AnalyticsType::AgentAnalytics => {
            analytics_service.process_agent_analytics(&request.parameters).await?
        }
        bridge_core::types::AnalyticsType::MultiRepoAnalytics => {
            analytics_service.process_multi_repo_analytics(&request.parameters).await?
        }
        bridge_core::types::AnalyticsType::RealTimeAlerting => {
            analytics_service.process_real_time_alerting(&request.parameters).await?
        }
        bridge_core::types::AnalyticsType::InteractiveQuerying => {
            analytics_service.process_interactive_querying(&request.parameters).await?
        }
        bridge_core::types::AnalyticsType::DataVisualization => {
            analytics_service.process_data_visualization(&request.parameters).await?
        }
        bridge_core::types::AnalyticsType::Custom(ref name) => {
            // Handle custom analytics types
            serde_json::json!({
                "analytics_type": format!("custom_{}", name),
                "timestamp": chrono::Utc::now(),
                "message": "Custom analytics type not implemented",
                "data_points": 0,
            })
        }
    };

    Ok(bridge_core::types::AnalyticsResponse {
        request_id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        status: AnalyticsStatus::Success,
        data,
        metadata: HashMap::new(),
        errors: Vec::new(),
    })
}

/// Get number of data points processed in analytics
fn get_analytics_data_points_processed(results: &bridge_core::types::AnalyticsResponse) -> u64 {
    // Extract data points count from analytics results
    if let Some(data_points) = results.data.get("data_points") {
        data_points.as_u64().unwrap_or(0)
    } else {
        0
    }
}

/// Calculate insights count from analytics results
fn calculate_insights_count(results: &bridge_core::types::AnalyticsResponse) -> u64 {
    // Extract insights count from analytics results
    if let Some(insights) = results.data.get("insights") {
        insights.as_array().map(|arr| arr.len() as u64).unwrap_or(0)
    } else {
        0
    }
}

/// Check for alerts in analytics results
fn check_for_alerts(results: &bridge_core::types::AnalyticsResponse) -> Vec<String> {
    let mut alerts = Vec::new();

    // Check for alerts in the analytics results
    if let Some(alerts_data) = results.data.get("alerts") {
        if let Some(alerts_array) = alerts_data.as_array() {
            for alert in alerts_array {
                if let Some(alert_obj) = alert.as_object() {
                    if let Some(description) = alert_obj.get("description") {
                        if let Some(desc_str) = description.as_str() {
                            alerts.push(desc_str.to_string());
                        }
                    }
                }
            }
        }
    }

    // Check for anomalies that might trigger alerts
    if let Some(anomalies) = results.data.get("anomalies") {
        if let Some(anomalies_array) = anomalies.as_array() {
            for anomaly in anomalies_array {
                if let Some(anomaly_obj) = anomaly.as_object() {
                    if let Some(severity) = anomaly_obj.get("severity") {
                        if let Some(severity_val) = severity.as_f64() {
                            if severity_val > 0.8 {
                                if let Some(description) = anomaly_obj.get("description") {
                                    if let Some(desc_str) = description.as_str() {
                                        alerts.push(format!("High severity anomaly: {}", desc_str));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for anomaly summaries
    if let Some(anomaly_summary) = results.data.get("anomaly_summary") {
        if let Some(summary_obj) = anomaly_summary.as_object() {
            for (key, value) in summary_obj {
                if let Some(count) = value.as_u64() {
                    if count > 0 {
                        alerts.push(format!("{}: {} anomalies detected", key, count));
                    }
                }
            }
        }
    }

    alerts
}

/// Analytics handler
pub async fn analytics_handler(
    State(state): State<AppState>,
    Json(request): Json<AnalyticsRequest>,
) -> ApiResult<Json<ApiResponse<AnalyticsResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    let processing_start = Instant::now();

    // Execute analytics processing
    let results = match execute_analytics_processing(&request).await {
        Ok(analytics_response) => analytics_response,
        Err(e) => {
            // Fallback to basic response if processing fails
            bridge_core::types::AnalyticsResponse {
                request_id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                status: AnalyticsStatus::Failed,
                data: serde_json::Value::Null,
                metadata: HashMap::new(),
                errors: vec![AnalyticsError {
                    code: "PROCESSING_ERROR".to_string(),
                    message: e.to_string(),
                    details: None,
                }],
            }
        }
    };

    let processing_time = processing_start.elapsed();

    // Record analytics metrics
    let analytics_type_str = match request.analytics_type {
        bridge_core::types::AnalyticsType::WorkflowAnalytics => "workflow_analytics",
        bridge_core::types::AnalyticsType::AgentAnalytics => "agent_analytics",
        bridge_core::types::AnalyticsType::MultiRepoAnalytics => "multi_repo_analytics",
        bridge_core::types::AnalyticsType::RealTimeAlerting => "real_time_alerting",
        bridge_core::types::AnalyticsType::InteractiveQuerying => "interactive_querying",
        bridge_core::types::AnalyticsType::DataVisualization => "data_visualization",
        bridge_core::types::AnalyticsType::Custom(ref name) => name,
    };

    state
        .metrics
        .record_processing(analytics_type_str, processing_time, true);

    let metadata = AnalyticsMetadata {
        analytics_id: Uuid::new_v4(),
        processing_time_ms: processing_time.as_millis() as u64,
        data_points_processed: get_analytics_data_points_processed(&results) as usize,
        insights_count: calculate_insights_count(&results) as usize,
        alerts_triggered: check_for_alerts(&results),
    };

    let response = AnalyticsResponse { results, metadata };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Analytics insights handler
pub async fn analytics_insights_handler(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Create analytics service
    let mut analytics_service = AnalyticsService::new();
    analytics_service.init().await.map_err(|e| {
        ApiError::Internal(format!("Failed to initialize analytics service: {}", e))
    })?;

    // Generate insights from recent data
    let insights = generate_system_insights(&analytics_service).await.map_err(|e| {
        ApiError::Internal(format!("Failed to generate insights: {}", e))
    })?;

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(insights, request_id, total_time);

    Ok(Json(api_response))
}

/// Analytics trends handler
pub async fn analytics_trends_handler(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Create analytics service
    let mut analytics_service = AnalyticsService::new();
    analytics_service.init().await.map_err(|e| {
        ApiError::Internal(format!("Failed to initialize analytics service: {}", e))
    })?;

    // Generate trends from recent data
    let trends = generate_system_trends(&analytics_service).await.map_err(|e| {
        ApiError::Internal(format!("Failed to generate trends: {}", e))
    })?;

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(trends, request_id, total_time);

    Ok(Json(api_response))
}

/// Analytics alerts handler
pub async fn analytics_alerts_handler(
    State(state): State<AppState>,
    Json(request): Json<AnalyticsRequest>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Create analytics service
    let mut analytics_service = AnalyticsService::new();
    analytics_service.init().await.map_err(|e| {
        ApiError::Internal(format!("Failed to initialize analytics service: {}", e))
    })?;

    // Generate alerts from recent data
    let alerts = generate_system_alerts(&analytics_service, &request).await.map_err(|e| {
        ApiError::Internal(format!("Failed to generate alerts: {}", e))
    })?;

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(alerts, request_id, total_time);

    Ok(Json(api_response))
}

/// Generate system insights
async fn generate_system_insights(
    analytics_service: &AnalyticsService,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Query recent system data for insights
    let query = "
        SELECT timestamp, service_name, response_time_ms, error_rate, cpu_usage, memory_usage
        FROM telemetry 
        WHERE timestamp >= NOW() - INTERVAL '1h'
        ORDER BY timestamp
    ";

    let parsed_query = ParsedQuery {
        id: Uuid::new_v4(),
        query_text: query.to_string(),
        ast: query_engine::parsers::QueryAst {
            root: query_engine::parsers::AstNode {
                node_type: query_engine::parsers::NodeType::Statement,
                value: Some(query.to_string()),
                children: vec![],
                metadata: HashMap::new(),
            },
            node_count: 1,
            depth: 1,
        },
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    };

    let query_result = analytics_service.query_executor.execute(parsed_query).await?;

    // Convert to time series data
    let response_time_data = analytics_service.convert_query_results_to_time_series(&query_result, "response_time_ms")?;
    let error_rate_data = analytics_service.convert_query_results_to_time_series(&query_result, "error_rate")?;
    let cpu_data = analytics_service.convert_query_results_to_time_series(&query_result, "cpu_usage")?;
    let memory_data = analytics_service.convert_query_results_to_time_series(&query_result, "memory_usage")?;

    // Calculate statistical summaries
    let response_values: Vec<f64> = response_time_data.points.iter().map(|p| p.value).collect();
    let error_values: Vec<f64> = error_rate_data.points.iter().map(|p| p.value).collect();
    let cpu_values: Vec<f64> = cpu_data.points.iter().map(|p| p.value).collect();
    let memory_values: Vec<f64> = memory_data.points.iter().map(|p| p.value).collect();

    let response_stats = analytics_service.calculate_statistical_summary(&response_values);
    let error_stats = analytics_service.calculate_statistical_summary(&error_values);
    let cpu_stats = analytics_service.calculate_statistical_summary(&cpu_values);
    let memory_stats = analytics_service.calculate_statistical_summary(&memory_values);

    // Perform trend analysis
    let response_trend = analytics_service.analyze_trends(&response_time_data).await?;
    let error_trend = analytics_service.analyze_trends(&error_rate_data).await?;
    let cpu_trend = analytics_service.analyze_trends(&cpu_data).await?;
    let memory_trend = analytics_service.analyze_trends(&memory_data).await?;

    // Perform seasonality analysis
    let response_seasonality = analytics_service.analyze_seasonality(&response_time_data).await?;
    let error_seasonality = analytics_service.analyze_seasonality(&error_rate_data).await?;
    let cpu_seasonality = analytics_service.analyze_seasonality(&cpu_data).await?;
    let memory_seasonality = analytics_service.analyze_seasonality(&memory_data).await?;

    let response_analysis = TimeSeriesResult {
        id: Uuid::new_v4(),
        series_id: response_time_data.id,
        timestamp: chrono::Utc::now(),
        statistics: response_stats,
        trend_analysis: response_trend,
        seasonality_analysis: response_seasonality,
        metadata: HashMap::new(),
    };
    let error_analysis = TimeSeriesResult {
        id: Uuid::new_v4(),
        series_id: error_rate_data.id,
        timestamp: chrono::Utc::now(),
        statistics: error_stats,
        trend_analysis: error_trend,
        seasonality_analysis: error_seasonality,
        metadata: HashMap::new(),
    };
    let cpu_analysis = TimeSeriesResult {
        id: Uuid::new_v4(),
        series_id: cpu_data.id,
        timestamp: chrono::Utc::now(),
        statistics: cpu_stats,
        trend_analysis: cpu_trend,
        seasonality_analysis: cpu_seasonality,
        metadata: HashMap::new(),
    };
    let memory_analysis = TimeSeriesResult {
        id: Uuid::new_v4(),
        series_id: memory_data.id,
        timestamp: chrono::Utc::now(),
        statistics: memory_stats,
        trend_analysis: memory_trend,
        seasonality_analysis: memory_seasonality,
        metadata: HashMap::new(),
    };

    // Generate insights
    let mut insights = Vec::new();

    // Performance insights
    if let Some(trend) = &response_analysis.trend_analysis {
        insights.push(serde_json::json!({
            "type": "performance",
            "category": "response_time",
            "description": format!("Response time shows a {} trend", 
                if trend.slope > 0.0 { "degrading" } else { "improving" }),
            "trend": if trend.slope > 0.0 { "degrading" } else { "improving" },
            "confidence": trend.confidence,
            "severity": if trend.slope.abs() > 0.1 { "high" } else { "low" },
        }));
    }

    // Error rate insights
    if let Some(trend) = &error_analysis.trend_analysis {
        insights.push(serde_json::json!({
            "type": "reliability",
            "category": "error_rate",
            "description": format!("Error rate shows a {} trend", 
                if trend.slope > 0.0 { "increasing" } else { "decreasing" }),
            "trend": if trend.slope > 0.0 { "increasing" } else { "decreasing" },
            "confidence": trend.confidence,
            "severity": if trend.slope > 0.05 { "high" } else { "medium" },
        }));
    }

    // Resource utilization insights
    let avg_cpu = cpu_analysis.statistics.mean;
    let avg_memory = memory_analysis.statistics.mean;

    insights.push(serde_json::json!({
        "type": "resource_utilization",
        "category": "system_resources",
        "description": format!("Average CPU usage: {:.1}%, Memory usage: {:.1}%", avg_cpu, avg_memory),
        "cpu_usage": avg_cpu,
        "memory_usage": avg_memory,
        "status": if avg_cpu > 80.0 || avg_memory > 80.0 { "warning" } else { "normal" },
        "severity": if avg_cpu > 90.0 || avg_memory > 90.0 { "high" } else if avg_cpu > 80.0 || avg_memory > 80.0 { "medium" } else { "low" },
    }));

    Ok(serde_json::json!({
        "insights": insights,
        "timestamp": chrono::Utc::now(),
        "data_points": response_time_data.points.len(),
        "summary": {
            "total_insights": insights.len(),
            "high_severity": insights.iter().filter(|i| i.get("severity").and_then(|s| s.as_str()) == Some("high")).count(),
            "medium_severity": insights.iter().filter(|i| i.get("severity").and_then(|s| s.as_str()) == Some("medium")).count(),
            "low_severity": insights.iter().filter(|i| i.get("severity").and_then(|s| s.as_str()) == Some("low")).count(),
        }
    }))
}

/// Generate system trends
async fn generate_system_trends(
    analytics_service: &AnalyticsService,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Query historical data for trends
    let query = "
        SELECT timestamp, service_name, response_time_ms, error_rate, request_count
        FROM telemetry 
        WHERE timestamp >= NOW() - INTERVAL '24h'
        ORDER BY timestamp
    ";

    let parsed_query = ParsedQuery {
        id: Uuid::new_v4(),
        query_text: query.to_string(),
        ast: query_engine::parsers::QueryAst {
            root: query_engine::parsers::AstNode {
                node_type: query_engine::parsers::NodeType::Statement,
                value: Some(query.to_string()),
                children: vec![],
                metadata: HashMap::new(),
            },
            node_count: 1,
            depth: 1,
        },
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    };

    let query_result = analytics_service.query_executor.execute(parsed_query).await?;

    // Convert to time series data
    let response_time_data = analytics_service.convert_query_results_to_time_series(&query_result, "response_time_ms")?;
    let error_rate_data = analytics_service.convert_query_results_to_time_series(&query_result, "error_rate")?;
    let request_count_data = analytics_service.convert_query_results_to_time_series(&query_result, "request_count")?;

    // Calculate statistical summaries
    let response_values: Vec<f64> = response_time_data.points.iter().map(|p| p.value).collect();
    let error_values: Vec<f64> = error_rate_data.points.iter().map(|p| p.value).collect();
    let request_values: Vec<f64> = request_count_data.points.iter().map(|p| p.value).collect();

    let response_stats = analytics_service.calculate_statistical_summary(&response_values);
    let error_stats = analytics_service.calculate_statistical_summary(&error_values);
    let request_stats = analytics_service.calculate_statistical_summary(&request_values);

    // Perform trend analysis
    let response_trend = analytics_service.analyze_trends(&response_time_data).await?;
    let error_trend = analytics_service.analyze_trends(&error_rate_data).await?;
    let request_trend = analytics_service.analyze_trends(&request_count_data).await?;

    // Perform seasonality analysis
    let response_seasonality = analytics_service.analyze_seasonality(&response_time_data).await?;
    let error_seasonality = analytics_service.analyze_seasonality(&error_rate_data).await?;
    let request_seasonality = analytics_service.analyze_seasonality(&request_count_data).await?;

    let response_analysis = TimeSeriesResult {
        id: Uuid::new_v4(),
        series_id: response_time_data.id,
        timestamp: chrono::Utc::now(),
        statistics: response_stats,
        trend_analysis: response_trend,
        seasonality_analysis: response_seasonality,
        metadata: HashMap::new(),
    };
    let error_analysis = TimeSeriesResult {
        id: Uuid::new_v4(),
        series_id: error_rate_data.id,
        timestamp: chrono::Utc::now(),
        statistics: error_stats,
        trend_analysis: error_trend,
        seasonality_analysis: error_seasonality,
        metadata: HashMap::new(),
    };
    let request_analysis = TimeSeriesResult {
        id: Uuid::new_v4(),
        series_id: request_count_data.id,
        timestamp: chrono::Utc::now(),
        statistics: request_stats,
        trend_analysis: request_trend,
        seasonality_analysis: request_seasonality,
        metadata: HashMap::new(),
    };

    // Generate trend data
    let trends = vec![
        serde_json::json!({
            "metric": "response_time",
            "trend": response_analysis.trend_analysis,
            "seasonality": response_analysis.seasonality_analysis,
            "statistics": response_analysis.statistics,
            "data_points": response_time_data.points.len(),
        }),
        serde_json::json!({
            "metric": "error_rate",
            "trend": error_analysis.trend_analysis,
            "seasonality": error_analysis.seasonality_analysis,
            "statistics": error_analysis.statistics,
            "data_points": error_rate_data.points.len(),
        }),
        serde_json::json!({
            "metric": "request_count",
            "trend": request_analysis.trend_analysis,
            "seasonality": request_analysis.seasonality_analysis,
            "statistics": request_analysis.statistics,
            "data_points": request_count_data.points.len(),
        }),
    ];

    Ok(serde_json::json!({
        "trends": trends,
        "timestamp": chrono::Utc::now(),
        "time_range": "24h",
        "summary": {
            "total_metrics": trends.len(),
            "trending_up": trends.iter().filter(|t| {
                t.get("trend").and_then(|tr| tr.get("slope")).and_then(|s| s.as_f64()).unwrap_or(0.0) > 0.0
            }).count(),
            "trending_down": trends.iter().filter(|t| {
                t.get("trend").and_then(|tr| tr.get("slope")).and_then(|s| s.as_f64()).unwrap_or(0.0) < 0.0
            }).count(),
        }
    }))
}

/// Generate system alerts
async fn generate_system_alerts(
    analytics_service: &AnalyticsService,
    request: &AnalyticsRequest,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Extract alert thresholds from request
    let empty_map = serde_json::Map::new();
    let thresholds = request.parameters.parameters.get("thresholds")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty_map);

    // Query recent data for alerting
    let query = "
        SELECT timestamp, service_name, error_rate, response_time_ms, cpu_usage, memory_usage
        FROM telemetry 
        WHERE timestamp >= NOW() - INTERVAL '5m'
        ORDER BY timestamp DESC
    ";

    let parsed_query = ParsedQuery {
        id: Uuid::new_v4(),
        query_text: query.to_string(),
        ast: query_engine::parsers::QueryAst {
            root: query_engine::parsers::AstNode {
                node_type: query_engine::parsers::NodeType::Statement,
                value: Some(query.to_string()),
                children: vec![],
                metadata: HashMap::new(),
            },
            node_count: 1,
            depth: 1,
        },
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    };

    let query_result = analytics_service.query_executor.execute(parsed_query).await?;

    // Convert to time series data
    let error_rate_data = analytics_service.convert_query_results_to_time_series(&query_result, "error_rate")?;
    let response_time_data = analytics_service.convert_query_results_to_time_series(&query_result, "response_time_ms")?;
    let cpu_data = analytics_service.convert_query_results_to_time_series(&query_result, "cpu_usage")?;
    let memory_data = analytics_service.convert_query_results_to_time_series(&query_result, "memory_usage")?;

    // Detect anomalies
    let error_anomalies = analytics_service.detect_anomalies(&error_rate_data).await?;
    let response_anomalies = analytics_service.detect_anomalies(&response_time_data).await?;
    let cpu_anomalies = analytics_service.detect_anomalies(&cpu_data).await?;
    let memory_anomalies = analytics_service.detect_anomalies(&memory_data).await?;

    // Generate alerts
    let alerts = analytics_service.generate_alerts(
        &error_anomalies,
        &response_anomalies,
        &cpu_anomalies,
        &memory_anomalies,
        thresholds,
    )?;

    // Add threshold-based alerts
    let mut threshold_alerts = Vec::new();
    
    // Check error rate threshold
    let avg_error_rate = error_rate_data.points.iter().map(|p| p.value).sum::<f64>() / error_rate_data.points.len() as f64;
    if let Some(error_threshold) = thresholds.get("error_rate").and_then(|v| v.as_f64()) {
        if avg_error_rate > error_threshold {
            threshold_alerts.push(serde_json::json!({
                "type": "threshold_exceeded",
                "metric": "error_rate",
                "severity": "high",
                "description": format!("Error rate {:.2}% exceeds threshold {:.2}%", avg_error_rate * 100.0, error_threshold * 100.0),
                "current_value": avg_error_rate,
                "threshold": error_threshold,
                "timestamp": chrono::Utc::now(),
            }));
        }
    }

    // Check response time threshold
    let avg_response_time = response_time_data.points.iter().map(|p| p.value).sum::<f64>() / response_time_data.points.len() as f64;
    if let Some(response_threshold) = thresholds.get("response_time").and_then(|v| v.as_f64()) {
        if avg_response_time > response_threshold {
            threshold_alerts.push(serde_json::json!({
                "type": "threshold_exceeded",
                "metric": "response_time",
                "severity": "medium",
                "description": format!("Response time {:.2}ms exceeds threshold {:.2}ms", avg_response_time, response_threshold),
                "current_value": avg_response_time,
                "threshold": response_threshold,
                "timestamp": chrono::Utc::now(),
            }));
        }
    }

    // Combine all alerts
    let mut all_alerts = alerts;
    all_alerts.extend(threshold_alerts);

    Ok(serde_json::json!({
        "alerts": all_alerts,
        "timestamp": chrono::Utc::now(),
        "data_points": error_rate_data.points.len(),
        "summary": {
            "total_alerts": all_alerts.len(),
            "high_severity": all_alerts.iter().filter(|a| a.get("severity").and_then(|s| s.as_str()) == Some("high")).count(),
            "medium_severity": all_alerts.iter().filter(|a| a.get("severity").and_then(|s| s.as_str()) == Some("medium")).count(),
            "low_severity": all_alerts.iter().filter(|a| a.get("severity").and_then(|s| s.as_str()) == Some("low")).count(),
        },
        "current_metrics": {
            "avg_error_rate": avg_error_rate,
            "avg_response_time": avg_response_time,
            "avg_cpu_usage": cpu_data.points.iter().map(|p| p.value).sum::<f64>() / cpu_data.points.len() as f64,
            "avg_memory_usage": memory_data.points.iter().map(|p| p.value).sum::<f64>() / memory_data.points.len() as f64,
        }
    }))
}
