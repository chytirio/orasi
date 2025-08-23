//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query optimizers for the bridge
//!
//! This module provides optimizer implementations for query optimization
//! including optimization rules and cost-based optimization.

pub mod sql_optimizer;

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::parsers::{ParsedQuery, QueryAst};

// Re-export SQL optimizer
pub use sql_optimizer::{SqlOptimizer, SqlOptimizerConfig};

/// Basic query optimizer implementation
pub struct BasicQueryOptimizer {
    name: String,
    version: String,
    stats: Arc<RwLock<OptimizerStats>>,
}

impl BasicQueryOptimizer {
    /// Create a new basic query optimizer
    pub fn new() -> Self {
        Self {
            name: "basic_optimizer".to_string(),
            version: "1.0.0".to_string(),
            stats: Arc::new(RwLock::new(OptimizerStats {
                optimizer: "basic_optimizer".to_string(),
                total_queries: 0,
                queries_per_minute: 0,
                total_optimization_time_ms: 0,
                avg_optimization_time_ms: 0.0,
                error_count: 0,
                last_optimization_time: None,
                is_optimizing: false,
            })),
        }
    }
}

#[async_trait]
impl QueryOptimizer for BasicQueryOptimizer {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing Basic Query Optimizer");
        Ok(())
    }

    async fn optimize(&self, query: ParsedQuery) -> BridgeResult<ParsedQuery> {
        let start_time = std::time::Instant::now();

        // For now, just return the original query
        // TODO: Implement actual optimization logic
        let optimized_query = query;

        let optimization_time = start_time.elapsed().as_millis() as u64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_queries += 1;
            stats.total_optimization_time_ms += optimization_time;
            stats.avg_optimization_time_ms =
                stats.total_optimization_time_ms as f64 / stats.total_queries as f64;
            stats.last_optimization_time = Some(Utc::now());
        }

        info!("Query optimized in {}ms", optimization_time);

        Ok(optimized_query)
    }

    async fn get_stats(&self) -> BridgeResult<OptimizerStats> {
        Ok(self.stats.read().await.clone())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }
}

/// Optimizer configuration trait
#[async_trait]
pub trait OptimizerConfig: Send + Sync {
    /// Get optimizer name
    fn name(&self) -> &str;

    /// Get optimizer version
    fn version(&self) -> &str;

    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;

    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Query optimizer trait for optimizing queries
#[async_trait]
pub trait QueryOptimizer: Send + Sync {
    /// Initialize the optimizer
    async fn init(&mut self) -> BridgeResult<()>;

    /// Optimize query
    async fn optimize(&self, query: ParsedQuery) -> BridgeResult<ParsedQuery>;

    /// Get optimizer statistics
    async fn get_stats(&self) -> BridgeResult<OptimizerStats>;

    /// Get optimizer name
    fn name(&self) -> &str;

    /// Get optimizer version
    fn version(&self) -> &str;
}

/// Optimization rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// Rule type
    pub rule_type: OptimizationRuleType,

    /// Rule enabled
    pub enabled: bool,

    /// Rule priority
    pub priority: u32,
}

/// Optimization rule type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationRuleType {
    PredicatePushdown,
    ProjectionPushdown,
    JoinReordering,
    FilterPushdown,
    LimitPushdown,
    AggregationPushdown,
    Custom(String),
}

/// Optimization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    /// Original query
    pub original_query: ParsedQuery,

    /// Optimized query
    pub optimized_query: ParsedQuery,

    /// Optimization rules applied
    pub rules_applied: Vec<String>,

    /// Optimization statistics
    pub stats: OptimizerStats,

    /// Optimization timestamp
    pub timestamp: DateTime<Utc>,
}

/// Optimizer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerStats {
    /// Optimizer name
    pub optimizer: String,

    /// Total queries optimized
    pub total_queries: u64,

    /// Queries optimized in last minute
    pub queries_per_minute: u64,

    /// Total optimization time in milliseconds
    pub total_optimization_time_ms: u64,

    /// Average optimization time per query in milliseconds
    pub avg_optimization_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last optimization timestamp
    pub last_optimization_time: Option<DateTime<Utc>>,

    /// Optimizer status
    pub is_optimizing: bool,
}

/// Optimizer manager for managing multiple optimizers
pub struct OptimizerManager {
    optimizers: HashMap<String, Box<dyn QueryOptimizer>>,
    rules: Vec<OptimizationRule>,
    is_running: Arc<RwLock<bool>>,
}

impl OptimizerManager {
    /// Create new optimizer manager
    pub fn new() -> Self {
        Self {
            optimizers: HashMap::new(),
            rules: Vec::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add optimizer
    pub fn add_optimizer(&mut self, name: String, optimizer: Box<dyn QueryOptimizer>) {
        self.optimizers.insert(name, optimizer);
    }

    /// Remove optimizer
    pub fn remove_optimizer(&mut self, name: &str) -> Option<Box<dyn QueryOptimizer>> {
        self.optimizers.remove(name)
    }

    /// Get optimizer
    pub fn get_optimizer(&self, name: &str) -> Option<&dyn QueryOptimizer> {
        self.optimizers.get(name).map(|o| o.as_ref())
    }

    /// Get all optimizer names
    pub fn get_optimizer_names(&self) -> Vec<String> {
        self.optimizers.keys().cloned().collect()
    }

    /// Add optimization rule
    pub fn add_rule(&mut self, rule: OptimizationRule) {
        self.rules.push(rule);
    }

    /// Remove optimization rule
    pub fn remove_rule(&mut self, rule_name: &str) -> Option<OptimizationRule> {
        if let Some(index) = self.rules.iter().position(|r| r.name == rule_name) {
            Some(self.rules.remove(index))
        } else {
            None
        }
    }

    /// Get optimization rules
    pub fn get_rules(&self) -> &[OptimizationRule] {
        &self.rules
    }

    /// Optimize query with specified optimizer
    pub async fn optimize_query(
        &self,
        optimizer_name: &str,
        query: ParsedQuery,
    ) -> BridgeResult<ParsedQuery> {
        if let Some(optimizer) = self.get_optimizer(optimizer_name) {
            optimizer.optimize(query).await
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "Optimizer not found: {}",
                optimizer_name
            )))
        }
    }

    /// Get optimizer statistics
    pub async fn get_stats(&self) -> BridgeResult<HashMap<String, OptimizerStats>> {
        let mut stats = HashMap::new();

        for (name, optimizer) in &self.optimizers {
            match optimizer.get_stats().await {
                Ok(optimizer_stats) => {
                    stats.insert(name.clone(), optimizer_stats);
                }
                Err(e) => {
                    error!("Failed to get stats for optimizer {}: {}", name, e);
                    return Err(e);
                }
            }
        }

        Ok(stats)
    }
}

/// Optimizer factory for creating optimizers
pub struct OptimizerFactory;

impl OptimizerFactory {
    /// Create an optimizer based on configuration
    pub async fn create_optimizer(
        config: &dyn OptimizerConfig,
    ) -> BridgeResult<Box<dyn QueryOptimizer>> {
        match config.name() {
            "sql" => {
                // Create SQL optimizer
                if let Some(sql_config) = config.as_any().downcast_ref::<SqlOptimizerConfig>() {
                    let mut optimizer = SqlOptimizer::new(sql_config.clone());
                    optimizer.init().await?;
                    Ok(Box::new(optimizer))
                } else {
                    // Create with default config if not provided
                    let sql_config = SqlOptimizerConfig::new();
                    let mut optimizer = SqlOptimizer::new(sql_config);
                    optimizer.init().await?;
                    Ok(Box::new(optimizer))
                }
            }
            "query" => {
                // Use SQL optimizer as the generic query optimizer for now
                let sql_config = SqlOptimizerConfig::new();
                let mut optimizer = SqlOptimizer::new(sql_config);
                optimizer.init().await?;
                Ok(Box::new(optimizer))
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported optimizer: {}",
                config.name()
            ))),
        }
    }
}
