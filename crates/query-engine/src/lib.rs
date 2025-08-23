//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query engine for OpenTelemetry Data Lake Bridge
//!
//! This module provides SQL and query language parsing, optimization,
//! and execution capabilities for telemetry data.

pub mod analytics;
pub mod cache;
pub mod executors;
pub mod functions;
pub mod optimizers;
pub mod parsers;
pub mod sources;
pub mod streaming;
pub mod visualization;

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

// Re-export main types
pub use analytics::{
    Analytics, AnalyticsConfig, AnomalyDetector, AnomalyDetectorConfig, AnomalyResult, AnomalyType,
    ClusterAnalyzer, ClusterAnalyzerConfig, ClusterResult, ClusterType, ForecastModel,
    ForecastResult, Forecaster, ForecasterConfig, MLModel, MLModelConfig, MLPredictor,
    MLPredictorConfig, PredictionResult, TimeSeriesAnalyzer, TimeSeriesAnalyzerConfig,
    TimeSeriesData, TimeSeriesPoint, TimeSeriesResult,
};
pub use cache::{Cache, CacheConfig, QueryCache};
pub use executors::{ExecutionEngine, ExecutorFactory, QueryExecutor, QueryResult};
pub use functions::{FunctionManager, FunctionValue, QueryFunction};
pub use optimizers::{OptimizationResult, QueryOptimizer};
pub use parsers::query_parser::QueryParserConfig;
pub use parsers::{ParsedQuery, QueryAst, QueryParser, QueryParserTrait};
pub use sources::{
    DataSource, DataSourceConfig, DataSourceManager, DataSourceResult, DataSourceStats,
    DeltaLakeDataSourceConfig, SourceManagerConfig, SourceManagerImpl,
};
pub use streaming::{
    windowing::DefaultWindowManager, ContinuousQuery, ContinuousQueryConfig,
    ContinuousQueryManager, StreamExecutor, StreamExecutorConfig, StreamQueryResult,
    StreamingQuery, StreamingQueryConfig, StreamingQueryManager, StreamingQueryResult,
    StreamingQueryStats, WindowConfig, WindowManager, WindowType,
};
pub use visualization::{
    MetricsCollector, MetricsCollectorConfig, PerformanceAnalyzer, PerformanceAnalyzerConfig,
    PerformanceReport, PlanVisualizer, PlanVisualizerConfig, QueryMetrics, QueryPerformanceMetrics,
    QueryPlanEdge, QueryPlanGraph, QueryPlanNode, ReportFormat, ReportGenerator,
    ReportGeneratorConfig, VisualizationConfig, VisualizationFormat, Visualizer,
};

/// Query engine version
pub const QUERY_ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Query engine configuration
#[derive(Debug, Clone)]
pub struct QueryEngineConfig {
    /// Enable query caching
    pub enable_caching: bool,

    /// Cache size limit
    pub cache_size_limit: usize,

    /// Enable query optimization
    pub enable_optimization: bool,

    /// Maximum query execution time in seconds
    pub max_execution_time_seconds: u64,

    /// Enable query result streaming
    pub enable_streaming: bool,

    /// Maximum result set size
    pub max_result_set_size: usize,

    /// Enable query plan visualization
    pub enable_plan_visualization: bool,

    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,

    /// Data source configurations
    pub data_sources: HashMap<String, String>, // Store as serialized strings
}

impl Default for QueryEngineConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_size_limit: 1000,
            enable_optimization: true,
            max_execution_time_seconds: 300,
            enable_streaming: false,
            max_result_set_size: 10000,
            enable_plan_visualization: false,
            enable_performance_monitoring: true,
            data_sources: HashMap::new(),
        }
    }
}

/// Query execution statistics
#[derive(Debug, Clone)]
pub struct QueryEngineStats {
    /// Total queries executed
    pub total_queries: u64,

    /// Total queries cached
    pub cached_queries: u64,

    /// Total queries optimized
    pub optimized_queries: u64,

    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,

    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,

    /// Cache hit rate
    pub cache_hit_rate: f64,

    /// Optimization success rate
    pub optimization_success_rate: f64,

    /// Last query execution time
    pub last_query_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for QueryEngineStats {
    fn default() -> Self {
        Self {
            total_queries: 0,
            cached_queries: 0,
            optimized_queries: 0,
            avg_execution_time_ms: 0.0,
            total_execution_time_ms: 0,
            cache_hit_rate: 0.0,
            optimization_success_rate: 0.0,
            last_query_time: None,
        }
    }
}

/// Main query engine implementation
pub struct QueryEngine {
    /// Engine configuration
    config: QueryEngineConfig,

    /// Engine statistics
    stats: Arc<RwLock<QueryEngineStats>>,

    /// Query cache
    cache: Arc<QueryCache>,

    /// Query parser
    parser: Arc<QueryParser>,

    /// Query optimizer
    optimizer: Arc<dyn QueryOptimizer>,

    /// Function manager
    function_manager: Arc<FunctionManager>,

    /// Data source manager
    source_manager: Arc<SourceManagerImpl>,

    /// Execution engine
    execution_engine: Arc<ExecutionEngine>,
}

impl QueryEngine {
    /// Create a new query engine
    pub async fn new(config: QueryEngineConfig) -> BridgeResult<Self> {
        info!("Creating Query Engine v{}", QUERY_ENGINE_VERSION);

        let cache = Arc::new(QueryCache::new(CacheConfig {
            max_size_bytes: config.cache_size_limit as u64,
            default_ttl_seconds: 3600,
            enabled: true,
            debug_logging: false,
            name: "query_cache".to_string(),
            version: QUERY_ENGINE_VERSION.to_string(),
        }));

        let parser_config = QueryParserConfig::default();
        let mut parser = QueryParser::new(&parser_config).await?;
        parser.init().await?;
        let parser = Arc::new(parser);

        let optimizer = Arc::new(optimizers::BasicQueryOptimizer::new());
        let function_manager = Arc::new(FunctionManager::new());
        let source_manager = Arc::new(SourceManagerImpl::new(SourceManagerConfig::new()));

        // Create execution engine with default executor
        let mut execution_engine = ExecutionEngine::new();
        let default_executor = executors::MockExecutor::new();
        execution_engine.add_executor("default".to_string(), Box::new(default_executor));
        let execution_engine = Arc::new(execution_engine);

        Ok(Self {
            config,
            stats: Arc::new(RwLock::new(QueryEngineStats::default())),
            cache,
            parser,
            optimizer,
            function_manager,
            source_manager,
            execution_engine,
        })
    }

    /// Initialize the query engine
    pub async fn init(&self) -> BridgeResult<()> {
        info!("Initializing Query Engine");

        // Initialize data sources
        for (name, config_str) in &self.config.data_sources {
            self.source_manager
                .register_source(
                    name.clone(),
                    "unknown".to_string(),
                    config_str.clone(),
                    HashMap::new(),
                )
                .await?;
        }

        info!("Query Engine initialization completed");
        Ok(())
    }

    /// Execute a query
    pub async fn execute_query(&self, query_string: &str) -> BridgeResult<QueryResult> {
        let start_time = std::time::Instant::now();
        let query_id = Uuid::new_v4();

        info!("Executing query: {}", query_id);

        // Parse query first
        let parsed_query = self.parser.parse(query_string).await?;

        // Check cache first
        if self.config.enable_caching {
            if let Ok(Some(cached_result)) = self.cache.get(&parsed_query).await {
                info!("Query result found in cache");
                self.update_stats_cached().await;
                return Ok(cached_result);
            }
        }

        // Optimize query if enabled
        let optimized_query = if self.config.enable_optimization {
            match self.optimizer.optimize(parsed_query.clone()).await {
                Ok(optimized) => {
                    self.update_stats_optimized().await;
                    optimized
                }
                Err(e) => {
                    warn!("Query optimization failed: {}, using original query", e);
                    parsed_query.clone()
                }
            }
        } else {
            parsed_query.clone()
        };

        // Execute query
        let result = self
            .execution_engine
            .execute_query("default", optimized_query)
            .await?;

        // Cache result if enabled
        if self.config.enable_caching {
            let _ = self.cache.put(&parsed_query, result.clone()).await;
        }

        // Update statistics
        let execution_time = start_time.elapsed();
        self.update_stats_executed(execution_time.as_millis() as u64)
            .await;

        info!("Query executed in {}ms", execution_time.as_millis());

        Ok(result)
    }

    /// Execute a streaming query (placeholder)
    pub async fn execute_streaming_query(&self, _query_string: &str) -> BridgeResult<()> {
        // TODO: Implement streaming query execution
        warn!("Streaming queries not yet implemented");
        Ok(())
    }

    /// Get query execution plan
    pub async fn get_query_plan(&self, query_string: &str) -> BridgeResult<String> {
        let parsed_query = self.parser.parse(query_string).await?;

        // TODO: Generate actual query plan
        let plan = format!("Query Plan for: {}", parsed_query.query_text);

        Ok(plan)
    }

    /// Get engine statistics
    pub async fn get_stats(&self) -> QueryEngineStats {
        self.stats.read().await.clone()
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> cache::CacheStats {
        self.cache
            .get_stats()
            .await
            .expect("Failed to get cache stats")
    }

    /// Clear query cache
    pub async fn clear_cache(&self) -> BridgeResult<()> {
        // TODO: Implement cache clearing
        info!("Cache cleared");
        Ok(())
    }

    /// Add data source
    pub async fn add_data_source(&self, name: String, config_str: String) -> BridgeResult<()> {
        self.source_manager
            .register_source(name, "unknown".to_string(), config_str, HashMap::new())
            .await
    }

    /// Remove data source
    pub async fn remove_data_source(&self, name: String) -> BridgeResult<()> {
        // TODO: Implement remove_source method
        info!("Removing data source: {}", name);
        Ok(())
    }

    /// List data sources
    pub async fn list_data_sources(&self) -> BridgeResult<Vec<String>> {
        let sources = self.source_manager.list_sources().await?;
        Ok(sources.into_iter().map(|s| s.name).collect())
    }

    /// Register function
    pub async fn register_function(&self, function: Box<dyn QueryFunction>) -> BridgeResult<()> {
        // TODO: Implement register_function method
        info!("Registering function: {}", function.name());
        Ok(())
    }

    // Private helper methods for statistics updates
    async fn update_stats_cached(&self) {
        let mut stats = self.stats.write().await;
        stats.cached_queries += 1;
        stats.total_queries += 1;
    }

    async fn update_stats_optimized(&self) {
        let mut stats = self.stats.write().await;
        stats.optimized_queries += 1;
    }

    async fn update_stats_executed(&self, execution_time_ms: u64) {
        let mut stats = self.stats.write().await;
        stats.total_queries += 1;
        stats.total_execution_time_ms += execution_time_ms;
        stats.avg_execution_time_ms =
            stats.total_execution_time_ms as f64 / stats.total_queries as f64;
        stats.last_query_time = Some(chrono::Utc::now());
    }
}

// Global query engine instance
static QUERY_ENGINE: tokio::sync::OnceCell<Arc<QueryEngine>> = tokio::sync::OnceCell::const_new();

/// Initialize the global query engine
pub async fn init_query_engine(config: QueryEngineConfig) -> BridgeResult<()> {
    let engine = Arc::new(QueryEngine::new(config).await?);
    engine.init().await?;

    QUERY_ENGINE
        .set(engine)
        .map_err(|_| bridge_core::BridgeError::configuration("Query engine already initialized"))?;

    info!("Global query engine initialized");
    Ok(())
}

/// Get the global query engine instance
pub fn get_query_engine() -> Option<Arc<QueryEngine>> {
    QUERY_ENGINE.get().cloned()
}

/// Shutdown the global query engine
pub async fn shutdown_query_engine() -> BridgeResult<()> {
    if let Some(engine) = QUERY_ENGINE.get() {
        // TODO: Implement proper shutdown logic
        info!("Query engine shutdown completed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_engine_creation() {
        let config = QueryEngineConfig::default();
        let engine = QueryEngine::new(config).await;
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_query_engine_initialization() {
        let config = QueryEngineConfig::default();
        let engine = QueryEngine::new(config).await.unwrap();
        let result = engine.init().await;
        assert!(result.is_ok());
    }
}
