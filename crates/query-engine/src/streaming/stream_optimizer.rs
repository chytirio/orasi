//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Stream optimizer for streaming queries
//!
//! This module provides optimization capabilities for streaming queries.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    StreamingQueryConfig, StreamingQueryResult, StreamingQueryRow, StreamingQueryStats,
    StreamingQueryValue, WindowInfo, WindowType,
};
use crate::parsers::{AstNode, NodeType, ParsedQuery, QueryAst, QueryType, ValidationResult};
use crate::streaming::stream_parser::{
    BackpressureConfig, ContinuousQueryConfig, StreamParsingResult, WindowConfig,
};

/// Stream optimizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptimizerConfig {
    /// Optimizer name
    pub name: String,

    /// Optimizer version
    pub version: String,

    /// Enable optimization
    pub enable_optimization: bool,

    /// Enable window optimization
    pub enable_window_optimization: bool,

    /// Enable predicate pushdown
    pub enable_predicate_pushdown: bool,

    /// Enable projection pushdown
    pub enable_projection_pushdown: bool,

    /// Enable join optimization
    pub enable_join_optimization: bool,

    /// Enable aggregation optimization
    pub enable_aggregation_optimization: bool,

    /// Enable watermark optimization
    pub enable_watermark_optimization: bool,

    /// Enable backpressure optimization
    pub enable_backpressure_optimization: bool,

    /// Enable continuous query optimization
    pub enable_continuous_optimization: bool,

    /// Maximum optimization time in milliseconds
    pub max_optimization_time_ms: u64,

    /// Enable cost-based optimization
    pub enable_cost_based_optimization: bool,

    /// Enable rule-based optimization
    pub enable_rule_based_optimization: bool,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for StreamOptimizerConfig {
    fn default() -> Self {
        Self {
            name: "stream_optimizer".to_string(),
            version: "1.0.0".to_string(),
            enable_optimization: true,
            enable_window_optimization: true,
            enable_predicate_pushdown: true,
            enable_projection_pushdown: true,
            enable_join_optimization: true,
            enable_aggregation_optimization: true,
            enable_watermark_optimization: true,
            enable_backpressure_optimization: true,
            enable_continuous_optimization: true,
            max_optimization_time_ms: 5000,
            enable_cost_based_optimization: true,
            enable_rule_based_optimization: true,
            additional_config: HashMap::new(),
        }
    }
}

/// Stream optimization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptimizationStats {
    /// Total queries optimized
    pub total_queries: u64,

    /// Successful optimizations
    pub successful_optimizations: u64,

    /// Failed optimizations
    pub failed_optimizations: u64,

    /// Average optimization time in milliseconds
    pub avg_optimization_time_ms: f64,

    /// Total optimization time in milliseconds
    pub total_optimization_time_ms: u64,

    /// Window optimizations
    pub window_optimizations: u64,

    /// Predicate pushdown optimizations
    pub predicate_pushdown_optimizations: u64,

    /// Projection pushdown optimizations
    pub projection_pushdown_optimizations: u64,

    /// Join optimizations
    pub join_optimizations: u64,

    /// Aggregation optimizations
    pub aggregation_optimizations: u64,

    /// Watermark optimizations
    pub watermark_optimizations: u64,

    /// Backpressure optimizations
    pub backpressure_optimizations: u64,

    /// Continuous query optimizations
    pub continuous_optimizations: u64,

    /// Last optimization time
    pub last_optimization_time: Option<DateTime<Utc>>,
}

/// Stream optimization result
#[derive(Debug, Clone)]
pub struct StreamOptimizationResult {
    /// Original parsed query
    pub original_query: StreamParsingResult,

    /// Optimized parsed query
    pub optimized_query: StreamParsingResult,

    /// Applied optimization rules
    pub applied_rules: Vec<StreamOptimizationRule>,

    /// Optimization statistics
    pub stats: StreamOptimizationStats,

    /// Optimization metadata
    pub metadata: HashMap<String, String>,

    /// Optimization timestamp
    pub timestamp: DateTime<Utc>,
}

/// Stream optimization rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptimizationRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// Rule type
    pub rule_type: StreamOptimizationRuleType,

    /// Rule enabled
    pub enabled: bool,

    /// Rule priority
    pub priority: u32,

    /// Rule cost estimate
    pub cost_estimate: f64,

    /// Rule benefit estimate
    pub benefit_estimate: f64,
}

/// Stream optimization rule type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamOptimizationRuleType {
    WindowOptimization,
    PredicatePushdown,
    ProjectionPushdown,
    JoinOptimization,
    AggregationOptimization,
    WatermarkOptimization,
    BackpressureOptimization,
    ContinuousOptimization,
    MemoryOptimization,
    LatencyOptimization,
    ThroughputOptimization,
    ResourceOptimization,
    Custom(String),
}

/// Stream optimization context
#[derive(Debug, Clone)]
pub struct StreamOptimizationContext {
    /// Query complexity estimate
    pub complexity_estimate: f64,

    /// Expected data volume
    pub expected_data_volume: u64,

    /// Available memory
    pub available_memory: u64,

    /// Target latency in milliseconds
    pub target_latency_ms: u64,

    /// Target throughput (records per second)
    pub target_throughput: u64,

    /// Resource constraints
    pub resource_constraints: HashMap<String, String>,
}

/// Stream optimizer trait
#[async_trait]
pub trait StreamOptimizer: Send + Sync {
    /// Optimize a streaming query
    async fn optimize(&self, query: StreamParsingResult) -> BridgeResult<StreamOptimizationResult>;

    /// Optimize with context
    async fn optimize_with_context(
        &self,
        query: StreamParsingResult,
        context: &StreamOptimizationContext,
    ) -> BridgeResult<StreamOptimizationResult>;

    /// Get optimizer statistics
    async fn get_stats(&self) -> BridgeResult<StreamOptimizationStats>;

    /// Start optimizer
    async fn start(&mut self) -> BridgeResult<()>;

    /// Stop optimizer
    async fn stop(&mut self) -> BridgeResult<()>;

    /// Check if optimizer is running
    fn is_running(&self) -> bool;

    /// Add optimization rule
    async fn add_rule(&mut self, rule: StreamOptimizationRule) -> BridgeResult<()>;

    /// Remove optimization rule
    async fn remove_rule(&mut self, rule_name: &str) -> BridgeResult<()>;

    /// Get available rules
    async fn get_rules(&self) -> BridgeResult<Vec<StreamOptimizationRule>>;
}

/// Default stream optimizer implementation
pub struct DefaultStreamOptimizer {
    config: StreamOptimizerConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<StreamOptimizationStats>>,
    rules: Arc<RwLock<Vec<StreamOptimizationRule>>>,
    start_time: Option<DateTime<Utc>>,
}

impl DefaultStreamOptimizer {
    /// Create a new stream optimizer
    pub fn new(config: StreamOptimizerConfig) -> Self {
        Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(StreamOptimizationStats {
                total_queries: 0,
                successful_optimizations: 0,
                failed_optimizations: 0,
                avg_optimization_time_ms: 0.0,
                total_optimization_time_ms: 0,
                window_optimizations: 0,
                predicate_pushdown_optimizations: 0,
                projection_pushdown_optimizations: 0,
                join_optimizations: 0,
                aggregation_optimizations: 0,
                watermark_optimizations: 0,
                backpressure_optimizations: 0,
                continuous_optimizations: 0,
                last_optimization_time: None,
            })),
            rules: Arc::new(RwLock::new(Self::default_rules())),
            start_time: None,
        }
    }

    /// Get default optimization rules
    fn default_rules() -> Vec<StreamOptimizationRule> {
        vec![
            StreamOptimizationRule {
                name: "window_size_optimization".to_string(),
                description: "Optimize window sizes for better performance".to_string(),
                rule_type: StreamOptimizationRuleType::WindowOptimization,
                enabled: true,
                priority: 10,
                cost_estimate: 0.1,
                benefit_estimate: 0.8,
            },
            StreamOptimizationRule {
                name: "predicate_pushdown".to_string(),
                description: "Push predicates down to reduce data volume".to_string(),
                rule_type: StreamOptimizationRuleType::PredicatePushdown,
                enabled: true,
                priority: 20,
                cost_estimate: 0.2,
                benefit_estimate: 0.9,
            },
            StreamOptimizationRule {
                name: "projection_pushdown".to_string(),
                description: "Push projections down to reduce data size".to_string(),
                rule_type: StreamOptimizationRuleType::ProjectionPushdown,
                enabled: true,
                priority: 15,
                cost_estimate: 0.1,
                benefit_estimate: 0.7,
            },
            StreamOptimizationRule {
                name: "aggregation_optimization".to_string(),
                description: "Optimize aggregation operations for streaming".to_string(),
                rule_type: StreamOptimizationRuleType::AggregationOptimization,
                enabled: true,
                priority: 25,
                cost_estimate: 0.3,
                benefit_estimate: 0.8,
            },
            StreamOptimizationRule {
                name: "watermark_optimization".to_string(),
                description: "Optimize watermark handling for latency".to_string(),
                rule_type: StreamOptimizationRuleType::WatermarkOptimization,
                enabled: true,
                priority: 30,
                cost_estimate: 0.2,
                benefit_estimate: 0.6,
            },
            StreamOptimizationRule {
                name: "backpressure_optimization".to_string(),
                description: "Optimize backpressure handling strategies".to_string(),
                rule_type: StreamOptimizationRuleType::BackpressureOptimization,
                enabled: true,
                priority: 35,
                cost_estimate: 0.4,
                benefit_estimate: 0.7,
            },
        ]
    }
}

#[async_trait]
impl StreamOptimizer for DefaultStreamOptimizer {
    async fn optimize(&self, query: StreamParsingResult) -> BridgeResult<StreamOptimizationResult> {
        let context = StreamOptimizationContext {
            complexity_estimate: 1.0,
            expected_data_volume: 1000000,
            available_memory: 1000000000, // 1GB
            target_latency_ms: 1000,
            target_throughput: 1000,
            resource_constraints: HashMap::new(),
        };

        self.optimize_with_context(query, &context).await
    }

    async fn optimize_with_context(
        &self,
        query: StreamParsingResult,
        context: &StreamOptimizationContext,
    ) -> BridgeResult<StreamOptimizationResult> {
        if !self.is_running() {
            return Err(bridge_core::BridgeError::query(
                "Stream optimizer is not running".to_string(),
            ));
        }

        let start_time = std::time::Instant::now();
        let original_query = query.clone();
        let mut optimized_query = query;
        let applied_rules = Vec::new();

        info!("Starting stream query optimization");

        // Basic optimization - in a real implementation, this would apply various rules
        if let Some(window_config) = &mut optimized_query.window_config {
            // Optimize window size based on query patterns
            if window_config.window_size_ms > 300000 {
                // > 5 minutes
                window_config.window_size_ms = 300000; // Cap at 5 minutes
                info!("Optimized window size to 5 minutes for better performance");
            }

            // Enable watermarking if not already enabled for large windows
            if window_config.window_size_ms > 60000 && !window_config.enable_watermarking {
                window_config.enable_watermarking = true;
                window_config.watermark_delay_ms = Some(5000);
                info!("Enabled watermarking for large window");
            }
        }

        let optimization_time = start_time.elapsed().as_millis() as u64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_queries += 1;
            stats.successful_optimizations += 1;
            stats.total_optimization_time_ms += optimization_time;
            stats.last_optimization_time = Some(Utc::now());

            // Calculate average optimization time
            if stats.successful_optimizations > 0 {
                stats.avg_optimization_time_ms =
                    stats.total_optimization_time_ms as f64 / stats.successful_optimizations as f64;
            }
        }

        // Create optimization result
        let result = StreamOptimizationResult {
            original_query,
            optimized_query,
            applied_rules,
            stats: self.stats.read().await.clone(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert(
                    "optimization_time_ms".to_string(),
                    optimization_time.to_string(),
                );
                metadata.insert(
                    "target_latency_ms".to_string(),
                    context.target_latency_ms.to_string(),
                );
                metadata.insert(
                    "target_throughput".to_string(),
                    context.target_throughput.to_string(),
                );
                metadata.insert(
                    "complexity_estimate".to_string(),
                    context.complexity_estimate.to_string(),
                );
                metadata
            },
            timestamp: Utc::now(),
        };

        info!(
            "Stream query optimization completed in {}ms",
            optimization_time
        );
        Ok(result)
    }

    async fn get_stats(&self) -> BridgeResult<StreamOptimizationStats> {
        Ok(self.stats.read().await.clone())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        if *self.is_running.read().await {
            warn!("Stream optimizer is already running");
            return Ok(());
        }

        info!("Starting stream optimizer: {}", self.config.name);

        // Set running state
        {
            let mut running = self.is_running.write().await;
            *running = true;
        }

        self.start_time = Some(Utc::now());

        info!("Stream optimizer started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        if !*self.is_running.read().await {
            warn!("Stream optimizer is not running");
            return Ok(());
        }

        info!("Stopping stream optimizer: {}", self.config.name);

        // Set running state to false
        {
            let mut running = self.is_running.write().await;
            *running = false;
        }

        info!("Stream optimizer stopped successfully");
        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.is_running.blocking_read()
    }

    async fn add_rule(&mut self, rule: StreamOptimizationRule) -> BridgeResult<()> {
        let mut rules = self.rules.write().await;

        // Check if rule already exists
        if rules.iter().any(|r| r.name == rule.name) {
            return Err(bridge_core::BridgeError::validation(format!(
                "Rule '{}' already exists",
                rule.name
            )));
        }

        rules.push(rule.clone());
        info!("Added optimization rule: {}", rule.name);
        Ok(())
    }

    async fn remove_rule(&mut self, rule_name: &str) -> BridgeResult<()> {
        let mut rules = self.rules.write().await;

        let original_len = rules.len();
        rules.retain(|r| r.name != rule_name);

        if rules.len() == original_len {
            return Err(bridge_core::BridgeError::validation(format!(
                "Rule '{}' not found",
                rule_name
            )));
        }

        info!("Removed optimization rule: {}", rule_name);
        Ok(())
    }

    async fn get_rules(&self) -> BridgeResult<Vec<StreamOptimizationRule>> {
        Ok(self.rules.read().await.clone())
    }
}
