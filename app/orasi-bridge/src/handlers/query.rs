//! Query handlers

use axum::{extract::State, response::Json};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    rest::AppState,
    types::*,
};

use serde_json;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::RwLock;

/// Query execution plan
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub query_id: String,
    pub query_type: QueryType,
    pub execution_strategy: ExecutionStrategy,
    pub estimated_cost: QueryCost,
    pub optimizations: Vec<QueryOptimization>,
    pub execution_steps: Vec<ExecutionStep>,
    pub data_sources: Vec<DataSource>,
    pub parallelism: ParallelismInfo,
}

/// Execution strategy for the query
#[derive(Debug, Clone)]
pub enum ExecutionStrategy {
    /// Direct scan of time-series data
    TimeSeriesScan {
        index_usage: IndexUsage,
        scan_direction: ScanDirection,
    },
    /// Index-based lookup
    IndexLookup {
        index_type: IndexType,
        key_conditions: Vec<String>,
    },
    /// Aggregation-first strategy
    AggregationPush {
        aggregation_level: AggregationLevel,
        pre_aggregated: bool,
    },
    /// Multi-stage execution
    MultiStage {
        stages: Vec<ExecutionStage>,
    },
}

/// Query cost estimation
#[derive(Debug, Clone)]
pub struct QueryCost {
    pub estimated_rows_scanned: u64,
    pub estimated_rows_returned: u64,
    pub estimated_memory_mb: f64,
    pub estimated_execution_time_ms: u64,
    pub io_cost: f64,
    pub cpu_cost: f64,
    pub network_cost: f64,
}

/// Query optimization applied
#[derive(Debug, Clone)]
pub enum QueryOptimization {
    /// Filter pushdown to reduce data scanning
    FilterPushdown { filters: Vec<String> },
    /// Projection pushdown to reduce data transfer
    ProjectionPushdown { columns: Vec<String> },
    /// Index selection
    IndexSelection { index_name: String, selectivity: f64 },
    /// Aggregation pushdown
    AggregationPushdown { aggregations: Vec<String> },
    /// Time range optimization
    TimeRangeOptimization { original_range: String, optimized_range: String },
    /// Caching opportunity
    CachingStrategy { cache_key: String, ttl_seconds: u64 },
}

/// Execution step in the query plan
#[derive(Debug, Clone)]
pub struct ExecutionStep {
    pub step_id: usize,
    pub operation: Operation,
    pub estimated_cost: f64,
    pub estimated_rows: u64,
    pub dependencies: Vec<usize>,
    pub parallelizable: bool,
}

/// Operation types in execution steps
#[derive(Debug, Clone)]
pub enum Operation {
    /// Scan data source
    Scan { source: String, conditions: Vec<String> },
    /// Apply filters
    Filter { conditions: Vec<String> },
    /// Apply aggregations
    Aggregate { functions: Vec<String> },
    /// Sort data
    Sort { columns: Vec<String>, direction: SortDirection },
    /// Limit results
    Limit { count: usize, offset: Option<usize> },
}

/// Supporting data structures
#[derive(Debug, Clone)]
pub enum IndexUsage { Full, Partial, None }

#[derive(Debug, Clone)]
pub enum ScanDirection { Forward, Backward }

#[derive(Debug, Clone)]
pub enum IndexType { TimeIndex, FieldIndex, CompositeIndex }

#[derive(Debug, Clone)]
pub enum AggregationLevel { Row, Minute, Hour, Day }

#[derive(Debug, Clone)]
pub struct ExecutionStage {
    pub stage_id: usize,
    pub operations: Vec<Operation>,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone)]
pub struct DataSource {
    pub name: String,
    pub source_type: DataSourceType,
    pub estimated_size_mb: f64,
    pub partitions: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DataSourceType { TimeSeries, Logs, Metrics, Traces }

#[derive(Debug, Clone)]
pub struct ParallelismInfo {
    pub max_parallelism: usize,
    pub parallel_stages: Vec<usize>,
    pub sequential_dependencies: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub enum SortDirection { Ascending, Descending }

// Global query cache
static QUERY_CACHE: OnceLock<
    Arc<RwLock<HashMap<String, (QueryResults, chrono::DateTime<chrono::Utc>)>>>,
> = OnceLock::new();

/// Initialize global query cache
pub fn init_query_cache() {
    QUERY_CACHE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())));
}

/// Get result count from query results
fn get_result_count(results: &QueryResults) -> u64 {
    match results {
        QueryResults::Metrics(metrics_result) => metrics_result.data.len() as u64,
        QueryResults::Traces(traces_result) => traces_result.data.len() as u64,
        QueryResults::Logs(logs_result) => logs_result.data.len() as u64,
        QueryResults::Analytics(_) => 1, // Analytics results are typically single objects
    }
}

/// Generate cache key from query request
fn generate_cache_key(request: &QueryRequest) -> String {
    // Create a deterministic cache key based on query parameters
    let key_data = serde_json::json!({
        "query_type": request.query_type,
        "parameters": {
            "time_range": {
                "start": request.parameters.time_range.start,
                "end": request.parameters.time_range.end
            },
            "filters": request.parameters.filters,
            "aggregations": request.parameters.aggregations,
            "limit": request.parameters.limit,
            "offset": request.parameters.offset
        }
    });

    // Use SHA256 hash of the JSON for consistent cache keys
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key_data.to_string().as_bytes());
    format!("query_{:x}", hasher.finalize())
}

/// Check if query result came from cache
fn check_cache_hit(request: &QueryRequest) -> bool {
    // Check if caching is enabled for this request
    if let Some(options) = &request.options {
        if !options.enable_cache {
            return false;
        }
    }

    // Get cache instance
    let cache = match QUERY_CACHE.get() {
        Some(cache) => cache,
        None => return false, // Cache not initialized
    };

    // Generate cache key
    let cache_key = generate_cache_key(request);

    // Check if we can acquire a read lock (non-blocking check)
    match cache.try_read() {
        Ok(cache_guard) => {
            if let Some((_, cached_at)) = cache_guard.get(&cache_key) {
                // Check if cache entry is still valid
                let now = chrono::Utc::now();
                let age = now.signed_duration_since(*cached_at).num_seconds() as u64;

                // Get TTL from request options or use default
                let ttl_seconds = request
                    .options
                    .as_ref()
                    .and_then(|opt| opt.cache_ttl_seconds)
                    .unwrap_or(300); // Default 5 minutes

                return age < ttl_seconds;
            }
        }
        Err(_) => {
            // Cache is locked, assume cache miss
            return false;
        }
    }

    false
}

/// Store query result in cache
async fn store_in_cache(request: &QueryRequest, results: QueryResults) {
    // Check if caching is enabled for this request
    if let Some(options) = &request.options {
        if !options.enable_cache {
            return;
        }
    }

    // Get cache instance
    let cache = match QUERY_CACHE.get() {
        Some(cache) => cache,
        None => return, // Cache not initialized
    };

    // Generate cache key
    let cache_key = generate_cache_key(request);
    let now = chrono::Utc::now();

    // Store in cache
    let mut cache_guard = cache.write().await;
    cache_guard.insert(cache_key, (results, now));
}

/// Get cached query result
async fn get_cached_result(request: &QueryRequest) -> Option<QueryResults> {
    // Check if caching is enabled for this request
    if let Some(options) = &request.options {
        if !options.enable_cache {
            return None;
        }
    }

    // Get cache instance
    let cache = match QUERY_CACHE.get() {
        Some(cache) => cache,
        None => return None, // Cache not initialized
    };

    // Generate cache key
    let cache_key = generate_cache_key(request);

    // Try to get cached result
    let cache_guard = cache.read().await;
    if let Some((results, cached_at)) = cache_guard.get(&cache_key) {
        // Check if cache entry is still valid
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(*cached_at).num_seconds() as u64;

        // Get TTL from request options or use default
        let ttl_seconds = request
            .options
            .as_ref()
            .and_then(|opt| opt.cache_ttl_seconds)
            .unwrap_or(300); // Default 5 minutes

        if age < ttl_seconds {
            return Some(results.clone());
        }
    }

    None
}

/// Generate comprehensive query plan for the request
pub fn generate_query_plan(request: &QueryRequest) -> Option<String> {
    let plan = create_execution_plan(request);
    Some(format_query_plan(&plan))
}

/// Create detailed execution plan
fn create_execution_plan(request: &QueryRequest) -> QueryPlan {
    let query_id = Uuid::new_v4().to_string();
    
    // Analyze query characteristics
    let time_range_duration = request.parameters.time_range.end
        .signed_duration_since(request.parameters.time_range.start)
        .num_seconds() as u64;
    
    let filter_count = request.parameters.filters.as_ref().map(|f| f.len()).unwrap_or(0);
    let aggregation_count = request.parameters.aggregations.as_ref().map(|a| a.len()).unwrap_or(0);
    
    // Determine execution strategy
    let execution_strategy = determine_execution_strategy(request, time_range_duration, filter_count, aggregation_count);
    
    // Estimate query cost
    let estimated_cost = estimate_query_cost(request, &execution_strategy, time_range_duration);
    
    // Identify optimization opportunities
    let optimizations = identify_optimizations(request, &execution_strategy);
    
    // Create execution steps
    let execution_steps = create_execution_steps(request, &execution_strategy);
    
    // Identify data sources
    let data_sources = identify_data_sources(request);
    
    // Determine parallelism strategy
    let parallelism = determine_parallelism(&execution_steps, request);
    
    QueryPlan {
        query_id,
        query_type: request.query_type.clone(),
        execution_strategy,
        estimated_cost,
        optimizations,
        execution_steps,
        data_sources,
        parallelism,
    }
}

/// Query handler
pub async fn query_handler(
    State(state): State<AppState>,
    Json(request): Json<QueryRequest>,
) -> ApiResult<Json<ApiResponse<QueryResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Validate query request
    if request.parameters.time_range.start >= request.parameters.time_range.end {
        return Err(ApiError::BadRequest(
            "Invalid time range: start must be before end".to_string(),
        ));
    }

    let query_start = Instant::now();

    // Check cache first
    let mut cache_hit = false;
    let results = if let Some(cached_results) = get_cached_result(&request).await {
        cache_hit = true;
        cached_results
    } else {
        // Execute query based on type
        let query_results = match request.query_type {
            QueryType::Metrics => {
                // Execute metrics query with placeholder data
                let query_id = Uuid::new_v4();
                let mut metadata = HashMap::new();
                metadata.insert("query_type".to_string(), "metrics".to_string());
                metadata.insert(
                    "time_range_start".to_string(),
                    request.parameters.time_range.start.to_rfc3339(),
                );
                metadata.insert(
                    "time_range_end".to_string(),
                    request.parameters.time_range.end.to_rfc3339(),
                );

                // Create sample metrics data
                let sample_metrics = vec![
                    bridge_core::types::MetricData {
                        name: "http_requests_total".to_string(),
                        description: Some("Total HTTP requests".to_string()),
                        unit: Some("requests".to_string()),
                        metric_type: bridge_core::types::MetricType::Counter,
                        value: bridge_core::types::MetricValue::Counter(1234.0),
                        labels: vec![
                            ("method".to_string(), "GET".to_string()),
                            ("status".to_string(), "200".to_string()),
                        ]
                        .into_iter()
                        .collect(),
                        timestamp: chrono::Utc::now(),
                    },
                    bridge_core::types::MetricData {
                        name: "http_request_duration_seconds".to_string(),
                        description: Some("HTTP request duration".to_string()),
                        unit: Some("seconds".to_string()),
                        metric_type: bridge_core::types::MetricType::Histogram,
                        value: bridge_core::types::MetricValue::Histogram {
                            buckets: vec![
                                bridge_core::types::HistogramBucket {
                                    upper_bound: 0.1,
                                    count: 100,
                                },
                                bridge_core::types::HistogramBucket {
                                    upper_bound: 0.5,
                                    count: 50,
                                },
                                bridge_core::types::HistogramBucket {
                                    upper_bound: 1.0,
                                    count: 25,
                                },
                            ],
                            sum: 45.5,
                            count: 175,
                        },
                        labels: vec![("method".to_string(), "GET".to_string())]
                            .into_iter()
                            .collect(),
                        timestamp: chrono::Utc::now(),
                    },
                ];

                QueryResults::Metrics(bridge_core::types::MetricsResult {
                    query_id,
                    timestamp: chrono::Utc::now(),
                    status: bridge_core::types::QueryStatus::Success,
                    data: sample_metrics,
                    metadata,
                    duration_ms: query_start.elapsed().as_millis() as u64,
                    errors: Vec::new(),
                })
            }
            QueryType::Traces => {
                // Execute traces query with placeholder data
                let query_id = Uuid::new_v4();
                let mut metadata = HashMap::new();
                metadata.insert("query_type".to_string(), "traces".to_string());
                metadata.insert(
                    "time_range_start".to_string(),
                    request.parameters.time_range.start.to_rfc3339(),
                );
                metadata.insert(
                    "time_range_end".to_string(),
                    request.parameters.time_range.end.to_rfc3339(),
                );

                // Create sample traces data
                let sample_traces = vec![bridge_core::types::TraceData {
                    trace_id: "trace_1234567890abcdef".to_string(),
                    span_id: "span_abcdef1234567890".to_string(),
                    parent_span_id: None,
                    name: "HTTP GET /api/users".to_string(),
                    kind: bridge_core::types::SpanKind::Server,
                    start_time: chrono::Utc::now() - chrono::Duration::seconds(10),
                    end_time: Some(chrono::Utc::now() - chrono::Duration::seconds(9)),
                    duration_ns: Some(1_000_000_000), // 1 second
                    status: bridge_core::types::SpanStatus {
                        code: bridge_core::types::StatusCode::Ok,
                        message: None,
                    },
                    attributes: vec![
                        ("http.method".to_string(), "GET".to_string()),
                        ("http.url".to_string(), "/api/users".to_string()),
                        ("http.status_code".to_string(), "200".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                    events: vec![
                        bridge_core::types::SpanEvent {
                            name: "http.request.start".to_string(),
                            timestamp: chrono::Utc::now() - chrono::Duration::seconds(10),
                            attributes: vec![("http.method".to_string(), "GET".to_string())]
                                .into_iter()
                                .collect(),
                        },
                        bridge_core::types::SpanEvent {
                            name: "http.request.end".to_string(),
                            timestamp: chrono::Utc::now() - chrono::Duration::seconds(9),
                            attributes: vec![("http.status_code".to_string(), "200".to_string())]
                                .into_iter()
                                .collect(),
                        },
                    ],
                    links: Vec::new(),
                }];

                QueryResults::Traces(bridge_core::types::TracesResult {
                    query_id,
                    timestamp: chrono::Utc::now(),
                    status: bridge_core::types::QueryStatus::Success,
                    data: sample_traces,
                    metadata,
                    duration_ms: query_start.elapsed().as_millis() as u64,
                    errors: Vec::new(),
                })
            }
            QueryType::Logs => {
                // Execute logs query with placeholder data
                let query_id = Uuid::new_v4();
                let mut metadata = HashMap::new();
                metadata.insert("query_type".to_string(), "logs".to_string());
                metadata.insert(
                    "time_range_start".to_string(),
                    request.parameters.time_range.start.to_rfc3339(),
                );
                metadata.insert(
                    "time_range_end".to_string(),
                    request.parameters.time_range.end.to_rfc3339(),
                );

                // Create sample logs data
                let sample_logs = vec![
                    bridge_core::types::LogData {
                        timestamp: chrono::Utc::now() - chrono::Duration::minutes(5),
                        level: bridge_core::types::LogLevel::Info,
                        message: "User login successful".to_string(),
                        attributes: vec![
                            ("user_id".to_string(), "user123".to_string()),
                            ("ip_address".to_string(), "192.168.1.100".to_string()),
                        ]
                        .into_iter()
                        .collect(),
                        body: None,
                        severity_number: Some(9),
                        severity_text: Some("INFO".to_string()),
                    },
                    bridge_core::types::LogData {
                        timestamp: chrono::Utc::now() - chrono::Duration::minutes(3),
                        level: bridge_core::types::LogLevel::Warn,
                        message: "Database connection slow".to_string(),
                        attributes: vec![
                            ("db_host".to_string(), "db.example.com".to_string()),
                            ("response_time_ms".to_string(), "1500".to_string()),
                        ]
                        .into_iter()
                        .collect(),
                        body: None,
                        severity_number: Some(13),
                        severity_text: Some("WARN".to_string()),
                    },
                ];

                QueryResults::Logs(bridge_core::types::LogsResult {
                    query_id,
                    timestamp: chrono::Utc::now(),
                    status: bridge_core::types::QueryStatus::Success,
                    data: sample_logs,
                    metadata,
                    duration_ms: query_start.elapsed().as_millis() as u64,
                    errors: Vec::new(),
                })
            }
            QueryType::Analytics => {
                // Execute analytics query with placeholder data
                let request_id = Uuid::new_v4();
                let mut metadata = HashMap::new();
                metadata.insert("query_type".to_string(), "analytics".to_string());
                metadata.insert(
                    "time_range_start".to_string(),
                    request.parameters.time_range.start.to_rfc3339(),
                );
                metadata.insert(
                    "time_range_end".to_string(),
                    request.parameters.time_range.end.to_rfc3339(),
                );

                // Create sample analytics data
                let analytics_data = serde_json::json!({
                    "total_requests": 1234,
                    "average_response_time": 0.045,
                    "error_rate": 0.02,
                    "top_endpoints": [
                        {"endpoint": "/api/users", "count": 456},
                        {"endpoint": "/api/products", "count": 234},
                        {"endpoint": "/api/orders", "count": 123}
                    ],
                    "response_time_distribution": {
                        "0-100ms": 800,
                        "100-500ms": 300,
                        "500ms-1s": 100,
                        "1s+": 34
                    }
                });

                QueryResults::Analytics(bridge_core::types::AnalyticsResponse {
                    request_id,
                    timestamp: chrono::Utc::now(),
                    status: bridge_core::types::analytics::AnalyticsStatus::Success,
                    data: analytics_data,
                    metadata,
                    errors: Vec::new(),
                })
            }
        };

        // Store the result in cache if it wasn't a cache hit
        if !cache_hit {
            store_in_cache(&request, query_results.clone()).await;
        }

        query_results
    };

    let query_time = query_start.elapsed();

    // Record query metrics
    let query_type_str = match request.query_type {
        QueryType::Metrics => "metrics",
        QueryType::Traces => "traces",
        QueryType::Logs => "logs",
        QueryType::Analytics => "analytics",
    };

    state
        .metrics
        .record_processing(query_type_str, query_time, true);

    let metadata = QueryMetadata {
        query_id: Uuid::new_v4(),
        execution_time_ms: query_time.as_millis() as u64,
        result_count: get_result_count(&results) as usize,
        cache_hit,
        query_plan: generate_query_plan(&request),
    };

    let response = QueryResponse { results, metadata };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Determine the best execution strategy
fn determine_execution_strategy(
    request: &QueryRequest,
    time_range_duration: u64,
    filter_count: usize,
    aggregation_count: usize,
) -> ExecutionStrategy {
    // Strategy selection based on query characteristics
    match (&request.query_type, time_range_duration, filter_count, aggregation_count) {
        // Large time range with aggregations - prefer aggregation push
        (_, duration, _, agg_count) if duration > 86400 && agg_count > 0 => {
            ExecutionStrategy::AggregationPush {
                aggregation_level: if duration > 2592000 { // > 30 days
                    AggregationLevel::Day
                } else if duration > 86400 { // > 1 day
                    AggregationLevel::Hour
                } else {
                    AggregationLevel::Minute
                },
                pre_aggregated: duration > 604800, // > 1 week
            }
        }
        
        // High selectivity filters - use index lookup
        (_, _, filter_count, _) if filter_count > 2 => {
            ExecutionStrategy::IndexLookup {
                index_type: determine_best_index(request),
                key_conditions: extract_key_conditions(request),
            }
        }
        
        // Complex queries with multiple stages
        (QueryType::Analytics, _, _, _) => {
            ExecutionStrategy::MultiStage {
                stages: create_execution_stages(request),
            }
        }
        
        // Default to time series scan
        _ => {
            ExecutionStrategy::TimeSeriesScan {
                index_usage: if filter_count > 0 { IndexUsage::Partial } else { IndexUsage::Full },
                scan_direction: determine_scan_direction(request),
            }
        }
    }
}

/// Estimate query execution cost
fn estimate_query_cost(
    request: &QueryRequest,
    _strategy: &ExecutionStrategy,
    time_range_duration: u64,
) -> QueryCost {
    // Base cost calculation factors
    let time_factor = (time_range_duration as f64 / 3600.0).max(1.0); // Hours
    let filter_selectivity = estimate_filter_selectivity(&request.parameters.filters);
    let aggregation_factor = request.parameters.aggregations.as_ref().map(|a| a.len() as f64).unwrap_or(1.0);
    
    // Data size estimation based on query type
    let base_rows = match request.query_type {
        QueryType::Metrics => (time_factor * 1000.0) as u64, // 1K metrics per hour
        QueryType::Traces => (time_factor * 5000.0) as u64,  // 5K traces per hour
        QueryType::Logs => (time_factor * 10000.0) as u64,   // 10K logs per hour
        QueryType::Analytics => (time_factor * 500.0) as u64, // 500 analytics per hour
    };
    
    let estimated_rows_scanned = (base_rows as f64 / filter_selectivity) as u64;
    let estimated_rows_returned = std::cmp::min(
        estimated_rows_scanned,
        request.parameters.limit.unwrap_or(1000) as u64
    );
    
    // Memory estimation
    let row_size_bytes = match request.query_type {
        QueryType::Metrics => 200.0,
        QueryType::Traces => 1000.0,
        QueryType::Logs => 500.0,
        QueryType::Analytics => 2000.0,
    };
    
    let estimated_memory_mb = (estimated_rows_scanned as f64 * row_size_bytes) / (1024.0 * 1024.0);
    
    // Execution time estimation
    let base_scan_time = estimated_rows_scanned / 10000; // 10K rows per ms
    let aggregation_time = (aggregation_factor * estimated_rows_scanned as f64 / 50000.0) as u64; // Aggregation overhead
    let estimated_execution_time_ms = base_scan_time + aggregation_time;
    
    // Cost components
    let io_cost = estimated_rows_scanned as f64 * 0.001; // Cost per row scanned
    let cpu_cost = aggregation_factor * estimated_rows_returned as f64 * 0.0001; // CPU cost for aggregations
    let network_cost = estimated_rows_returned as f64 * row_size_bytes * 0.000001; // Network transfer cost
    
    QueryCost {
        estimated_rows_scanned,
        estimated_rows_returned,
        estimated_memory_mb,
        estimated_execution_time_ms,
        io_cost,
        cpu_cost,
        network_cost,
    }
}

// Helper functions for query planning

fn determine_best_index(request: &QueryRequest) -> IndexType {
    if request.parameters.filters.as_ref().map_or(false, |f| 
        f.iter().any(|filter| filter.field == "timestamp")) {
        IndexType::TimeIndex
    } else if request.parameters.filters.as_ref().map_or(false, |f| f.len() > 1) {
        IndexType::CompositeIndex
    } else {
        IndexType::FieldIndex
    }
}

fn extract_key_conditions(request: &QueryRequest) -> Vec<String> {
    request.parameters.filters.as_ref().map_or(vec![], |filters| {
        filters.iter()
            .map(|f| format!("{} {} {:?}", f.field, filter_operator_to_string(&f.operator), f.value))
            .collect()
    })
}

fn create_execution_stages(request: &QueryRequest) -> Vec<ExecutionStage> {
    let mut stages = Vec::new();
    
    // Stage 1: Data collection
    stages.push(ExecutionStage {
        stage_id: 0,
        operations: vec![
            Operation::Scan {
                source: format!("{:?}_data", request.query_type),
                conditions: extract_scan_conditions(request),
            }
        ],
        estimated_cost: 100.0,
    });
    
    // Stage 2: Processing
    if request.parameters.aggregations.is_some() {
        stages.push(ExecutionStage {
            stage_id: 1,
            operations: vec![
                Operation::Aggregate {
                    functions: extract_aggregation_functions(request),
                }
            ],
            estimated_cost: 200.0,
        });
    }
    
    stages
}

fn determine_scan_direction(request: &QueryRequest) -> ScanDirection {
    // Default to backward for time series data (most recent first)
    if request.parameters.limit.is_some() && request.parameters.offset.is_none() {
        ScanDirection::Backward
    } else {
        ScanDirection::Forward
    }
}

fn estimate_filter_selectivity(filters: &Option<Vec<bridge_core::types::Filter>>) -> f64 {
    filters.as_ref().map_or(1.0, |f| {
        // Rough selectivity estimation based on filter types
        let selectivity_product = f.iter().map(|filter| {
            match filter.operator {
                bridge_core::types::FilterOperator::Equals => 0.1,
                bridge_core::types::FilterOperator::Contains => 0.3,
                bridge_core::types::FilterOperator::GreaterThan | 
                bridge_core::types::FilterOperator::LessThan => 0.5,
                bridge_core::types::FilterOperator::In => 0.2,
                _ => 0.4,
            }
        }).product::<f64>();
        
        selectivity_product.max(0.001) // Minimum selectivity
    })
}

fn extract_scan_conditions(request: &QueryRequest) -> Vec<String> {
    let mut conditions = vec![
        format!("timestamp >= {}", request.parameters.time_range.start),
        format!("timestamp <= {}", request.parameters.time_range.end),
    ];
    
    if let Some(filters) = &request.parameters.filters {
        for filter in filters {
            conditions.push(format!("{} {} {:?}", 
                filter.field, 
                filter_operator_to_string(&filter.operator), 
                filter.value));
        }
    }
    
    conditions
}

fn extract_aggregation_functions(request: &QueryRequest) -> Vec<String> {
    request.parameters.aggregations.as_ref().map_or(vec![], |aggregations| {
        aggregations.iter()
            .map(|a| format!("{:?}({})", a.function, a.field))
            .collect()
    })
}

fn filter_operator_to_string(op: &bridge_core::types::FilterOperator) -> &'static str {
    match op {
        bridge_core::types::FilterOperator::Equals => "=",
        bridge_core::types::FilterOperator::NotEquals => "!=",
        bridge_core::types::FilterOperator::Contains => "CONTAINS",
        bridge_core::types::FilterOperator::NotContains => "NOT CONTAINS",
        bridge_core::types::FilterOperator::GreaterThan => ">",
        bridge_core::types::FilterOperator::GreaterThanOrEqual => ">=",
        bridge_core::types::FilterOperator::LessThan => "<",
        bridge_core::types::FilterOperator::LessThanOrEqual => "<=",
        bridge_core::types::FilterOperator::In => "IN",
        bridge_core::types::FilterOperator::NotIn => "NOT IN",
        bridge_core::types::FilterOperator::Exists => "EXISTS",
        bridge_core::types::FilterOperator::NotExists => "NOT EXISTS",
        bridge_core::types::FilterOperator::StartsWith => "STARTS WITH",
        bridge_core::types::FilterOperator::EndsWith => "ENDS WITH",
        bridge_core::types::FilterOperator::Regex => "MATCHES",
    }
}

// Additional helper functions for query planning

fn identify_optimizations(request: &QueryRequest, _strategy: &ExecutionStrategy) -> Vec<QueryOptimization> {
    let mut optimizations = Vec::new();
    
    // Filter pushdown optimization
    if let Some(filters) = &request.parameters.filters {
        let filter_descriptions: Vec<String> = filters.iter()
            .map(|f| format!("{} {} {:?}", f.field, filter_operator_to_string(&f.operator), f.value))
            .collect();
        
        optimizations.push(QueryOptimization::FilterPushdown {
            filters: filter_descriptions,
        });
    }
    
    // Aggregation pushdown for time series data
    if let Some(aggregations) = &request.parameters.aggregations {
        let agg_descriptions: Vec<String> = aggregations.iter()
            .map(|a| format!("{:?}({})", a.function, a.field))
            .collect();
        
        optimizations.push(QueryOptimization::AggregationPushdown {
            aggregations: agg_descriptions,
        });
    }
    
    // Time range optimization
    let time_range_hours = request.parameters.time_range.end
        .signed_duration_since(request.parameters.time_range.start)
        .num_hours();
    
    if time_range_hours > 24 {
        optimizations.push(QueryOptimization::TimeRangeOptimization {
            original_range: format!("{} to {}", request.parameters.time_range.start, request.parameters.time_range.end),
            optimized_range: "Consider using pre-aggregated data for large time ranges".to_string(),
        });
    }
    
    // Caching strategy
    if should_cache_query(request) {
        let cache_key = generate_cache_key(request);
        let ttl = determine_cache_ttl(request);
        
        optimizations.push(QueryOptimization::CachingStrategy {
            cache_key,
            ttl_seconds: ttl,
        });
    }
    
    optimizations
}

fn create_execution_steps(request: &QueryRequest, _strategy: &ExecutionStrategy) -> Vec<ExecutionStep> {
    let mut steps = Vec::new();
    let mut step_id = 0;
    
    // Step 1: Data source scan
    let scan_step = ExecutionStep {
        step_id,
        operation: Operation::Scan {
            source: format!("{:?}_data", request.query_type),
            conditions: extract_scan_conditions(request),
        },
        estimated_cost: 100.0,
        estimated_rows: estimate_scan_rows(request),
        dependencies: vec![],
        parallelizable: true,
    };
    steps.push(scan_step);
    step_id += 1;
    
    // Step 2: Apply filters
    if request.parameters.filters.is_some() {
        let filter_step = ExecutionStep {
            step_id,
            operation: Operation::Filter {
                conditions: extract_filter_conditions(request),
            },
            estimated_cost: 50.0,
            estimated_rows: (estimate_scan_rows(request) as f64 * estimate_filter_selectivity(&request.parameters.filters)) as u64,
            dependencies: vec![step_id - 1],
            parallelizable: true,
        };
        steps.push(filter_step);
        step_id += 1;
    }
    
    // Step 3: Apply aggregations
    if request.parameters.aggregations.is_some() {
        let agg_step = ExecutionStep {
            step_id,
            operation: Operation::Aggregate {
                functions: extract_aggregation_functions(request),
            },
            estimated_cost: 200.0,
            estimated_rows: estimate_aggregation_output_rows(request),
            dependencies: vec![step_id - 1],
            parallelizable: false,
        };
        steps.push(agg_step);
        step_id += 1;
    }
    
    // Step 4: Sort if needed
    if should_sort_results(request) {
        let sort_step = ExecutionStep {
            step_id,
            operation: Operation::Sort {
                columns: vec!["timestamp".to_string()],
                direction: SortDirection::Descending,
            },
            estimated_cost: 75.0,
            estimated_rows: steps.last().unwrap().estimated_rows,
            dependencies: vec![step_id - 1],
            parallelizable: false,
        };
        steps.push(sort_step);
        step_id += 1;
    }
    
    // Step 5: Apply limit/offset
    if request.parameters.limit.is_some() || request.parameters.offset.is_some() {
        let limit_step = ExecutionStep {
            step_id,
            operation: Operation::Limit {
                count: request.parameters.limit.unwrap_or(1000),
                offset: request.parameters.offset,
            },
            estimated_cost: 10.0,
            estimated_rows: std::cmp::min(
                steps.last().unwrap().estimated_rows,
                request.parameters.limit.unwrap_or(1000) as u64
            ),
            dependencies: vec![step_id - 1],
            parallelizable: true,
        };
        steps.push(limit_step);
    }
    
    steps
}

fn identify_data_sources(request: &QueryRequest) -> Vec<DataSource> {
    let source_name = match request.query_type {
        QueryType::Metrics => "metrics_timeseries",
        QueryType::Traces => "traces_spans",
        QueryType::Logs => "logs_entries",
        QueryType::Analytics => "analytics_aggregates",
    };
    
    let time_range_days = request.parameters.time_range.end
        .signed_duration_since(request.parameters.time_range.start)
        .num_days();
    
    let partitions = (0..std::cmp::max(1, time_range_days))
        .map(|i| format!("partition_{}", i))
        .collect();
    
    vec![DataSource {
        name: source_name.to_string(),
        source_type: match request.query_type {
            QueryType::Metrics => DataSourceType::Metrics,
            QueryType::Traces => DataSourceType::Traces,
            QueryType::Logs => DataSourceType::Logs,
            QueryType::Analytics => DataSourceType::TimeSeries,
        },
        estimated_size_mb: time_range_days as f64 * 1000.0,
        partitions,
    }]
}

fn determine_parallelism(steps: &[ExecutionStep], request: &QueryRequest) -> ParallelismInfo {
    let max_parallelism = std::cmp::min(8, std::cmp::max(1, 
        request.parameters.time_range.end
            .signed_duration_since(request.parameters.time_range.start)
            .num_days() as usize));
    
    let parallel_stages: Vec<usize> = steps.iter()
        .filter(|step| step.parallelizable)
        .map(|step| step.step_id)
        .collect();
    
    let sequential_dependencies: Vec<(usize, usize)> = steps.iter()
        .flat_map(|step| {
            step.dependencies.iter().map(move |dep| (*dep, step.step_id))
        })
        .collect();
    
    ParallelismInfo {
        max_parallelism,
        parallel_stages,
        sequential_dependencies,
    }
}

fn format_query_plan(plan: &QueryPlan) -> String {
    let mut formatted = String::new();
    
    formatted.push_str(&format!("=== Query Execution Plan (ID: {}) ===\n", plan.query_id));
    formatted.push_str(&format!("Query Type: {:?}\n", plan.query_type));
    
    // Execution Strategy
    formatted.push_str(&format!("\n📋 Execution Strategy: {}\n", format_execution_strategy(&plan.execution_strategy)));
    
    // Cost Estimation
    formatted.push_str(&format!("\n💰 Cost Estimation:\n"));
    formatted.push_str(&format!("  • Estimated rows scanned: {}\n", plan.estimated_cost.estimated_rows_scanned));
    formatted.push_str(&format!("  • Estimated rows returned: {}\n", plan.estimated_cost.estimated_rows_returned));
    formatted.push_str(&format!("  • Estimated memory usage: {:.2} MB\n", plan.estimated_cost.estimated_memory_mb));
    formatted.push_str(&format!("  • Estimated execution time: {} ms\n", plan.estimated_cost.estimated_execution_time_ms));
    formatted.push_str(&format!("  • Total cost: {:.4} (IO: {:.4}, CPU: {:.4}, Network: {:.4})\n", 
        plan.estimated_cost.io_cost + plan.estimated_cost.cpu_cost + plan.estimated_cost.network_cost,
        plan.estimated_cost.io_cost, plan.estimated_cost.cpu_cost, plan.estimated_cost.network_cost));
    
    // Optimizations
    if !plan.optimizations.is_empty() {
        formatted.push_str(&format!("\n🚀 Applied Optimizations:\n"));
        for (i, opt) in plan.optimizations.iter().enumerate() {
            formatted.push_str(&format!("  {}. {}\n", i + 1, format_optimization(opt)));
        }
    }
    
    // Execution Steps
    formatted.push_str(&format!("\n⚡ Execution Steps:\n"));
    for step in &plan.execution_steps {
        let parallelism = if step.parallelizable { "||" } else { "--" };
        formatted.push_str(&format!("  {} Step {}: {} (Cost: {:.2}, Rows: {})\n",
            parallelism, step.step_id, format_operation(&step.operation),
            step.estimated_cost, step.estimated_rows));
        
        if !step.dependencies.is_empty() {
            formatted.push_str(&format!("     Dependencies: {:?}\n", step.dependencies));
        }
    }
    
    // Data Sources
    formatted.push_str(&format!("\n💾 Data Sources:\n"));
    for source in &plan.data_sources {
        formatted.push_str(&format!("  • {} ({:?}): {:.2} MB across {} partitions\n",
            source.name, source.source_type, source.estimated_size_mb, source.partitions.len()));
    }
    
    // Parallelism Info
    formatted.push_str(&format!("\n🔀 Parallelism:\n"));
    formatted.push_str(&format!("  • Max parallelism: {}\n", plan.parallelism.max_parallelism));
    formatted.push_str(&format!("  • Parallel stages: {:?}\n", plan.parallelism.parallel_stages));
    if !plan.parallelism.sequential_dependencies.is_empty() {
        formatted.push_str(&format!("  • Sequential dependencies: {:?}\n", plan.parallelism.sequential_dependencies));
    }
    
    formatted.push_str("\n=== End Query Plan ===");
    formatted
}

// Additional helper functions

fn estimate_scan_rows(request: &QueryRequest) -> u64 {
    let time_range_hours = request.parameters.time_range.end
        .signed_duration_since(request.parameters.time_range.start)
        .num_hours() as u64;
    
    match request.query_type {
        QueryType::Metrics => time_range_hours * 1000,
        QueryType::Traces => time_range_hours * 5000,
        QueryType::Logs => time_range_hours * 10000,
        QueryType::Analytics => time_range_hours * 500,
    }
}

fn extract_filter_conditions(request: &QueryRequest) -> Vec<String> {
    request.parameters.filters.as_ref().map_or(vec![], |filters| {
        filters.iter()
            .map(|f| format!("{} {} {:?}", f.field, filter_operator_to_string(&f.operator), f.value))
            .collect()
    })
}

fn estimate_aggregation_output_rows(request: &QueryRequest) -> u64 {
    let base_rows = estimate_scan_rows(request);
    let selectivity = estimate_filter_selectivity(&request.parameters.filters);
    let filtered_rows = (base_rows as f64 * selectivity) as u64;
    
    request.parameters.aggregations.as_ref().map_or(filtered_rows, |aggs| {
        std::cmp::max(1, filtered_rows / (aggs.len() as u64 * 10))
    })
}

fn should_sort_results(request: &QueryRequest) -> bool {
    matches!(request.query_type, QueryType::Metrics | QueryType::Traces | QueryType::Logs) ||
    request.parameters.limit.is_some()
}

fn should_cache_query(request: &QueryRequest) -> bool {
    request.options.as_ref().map_or(false, |opts| opts.enable_cache)
}

fn determine_cache_ttl(request: &QueryRequest) -> u64 {
    request.options.as_ref()
        .and_then(|opts| opts.cache_ttl_seconds)
        .unwrap_or(300)
}

fn format_execution_strategy(strategy: &ExecutionStrategy) -> String {
    match strategy {
        ExecutionStrategy::TimeSeriesScan { index_usage, scan_direction } => {
            format!("Time Series Scan (Index: {:?}, Direction: {:?})", index_usage, scan_direction)
        }
        ExecutionStrategy::IndexLookup { index_type, key_conditions } => {
            format!("Index Lookup ({:?}) with {} conditions", index_type, key_conditions.len())
        }
        ExecutionStrategy::AggregationPush { aggregation_level, pre_aggregated } => {
            format!("Aggregation Push ({:?}{})", aggregation_level, 
                if *pre_aggregated { ", Pre-aggregated" } else { "" })
        }
        ExecutionStrategy::MultiStage { stages } => {
            format!("Multi-Stage Execution ({} stages)", stages.len())
        }
    }
}

fn format_optimization(opt: &QueryOptimization) -> String {
    match opt {
        QueryOptimization::FilterPushdown { filters } => {
            format!("Filter Pushdown: {} filters", filters.len())
        }
        QueryOptimization::ProjectionPushdown { columns } => {
            format!("Projection Pushdown: {} columns", columns.len())
        }
        QueryOptimization::IndexSelection { index_name, selectivity } => {
            format!("Index Selection: {} (selectivity: {:.3})", index_name, selectivity)
        }
        QueryOptimization::AggregationPushdown { aggregations } => {
            format!("Aggregation Pushdown: {} aggregations", aggregations.len())
        }
        QueryOptimization::TimeRangeOptimization { original_range, optimized_range } => {
            format!("Time Range Optimization: {} → {}", original_range, optimized_range)
        }
        QueryOptimization::CachingStrategy { cache_key, ttl_seconds } => {
            format!("Caching Strategy: {} (TTL: {}s)", &cache_key[..std::cmp::min(16, cache_key.len())], ttl_seconds)
        }
    }
}

fn format_operation(op: &Operation) -> String {
    match op {
        Operation::Scan { source, conditions } => {
            format!("Scan {} ({} conditions)", source, conditions.len())
        }
        Operation::Filter { conditions } => {
            format!("Filter ({} conditions)", conditions.len())
        }
        Operation::Aggregate { functions } => {
            format!("Aggregate ({} functions)", functions.len())
        }
        Operation::Sort { columns, direction } => {
            format!("Sort {} ({:?})", columns.join(", "), direction)
        }
        Operation::Limit { count, offset } => {
            format!("Limit {} offset {}", count, offset.unwrap_or(0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::types::TimeRange;

    #[tokio::test]
    async fn test_cache_functionality() {
        // Initialize cache
        init_query_cache();

        // Create a test query request
        let request = QueryRequest {
            query_type: QueryType::Metrics,
            parameters: QueryParameters {
                time_range: TimeRange {
                    start: chrono::Utc::now() - chrono::Duration::hours(1),
                    end: chrono::Utc::now(),
                },
                filters: None,
                aggregations: None,
                limit: Some(100),
                offset: Some(0),
            },
            options: Some(QueryOptions {
                enable_cache: true,
                cache_ttl_seconds: Some(300),
                timeout_seconds: None,
                enable_streaming: false,
            }),
        };

        // Test cache miss initially
        assert!(get_cached_result(&request).await.is_none());

        // Create a test result
        let test_result = QueryResults::Metrics(bridge_core::types::MetricsResult {
            query_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            status: bridge_core::types::QueryStatus::Success,
            data: vec![],
            metadata: HashMap::new(),
            duration_ms: 100,
            errors: vec![],
        });

        // Store in cache
        store_in_cache(&request, test_result.clone()).await;

        // Test cache hit
        let cached_result = get_cached_result(&request).await;
        assert!(cached_result.is_some());

        // Verify the cached result matches
        if let Some(cached) = cached_result {
            assert_eq!(format!("{:?}", cached), format!("{:?}", test_result));
        }
    }
}
