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

/// Generate query plan for the request
fn generate_query_plan(request: &QueryRequest) -> Option<String> {
    // TODO: Implement actual query plan generation
    // This would typically involve:
    // 1. Parsing the query parameters
    // 2. Determining the execution strategy
    // 3. Estimating resource usage
    // 4. Formatting the plan as a string

    // For now, return a basic plan as placeholder
    Some(format!(
        "Query Plan: {} query over time range {} to {}",
        match request.query_type {
            QueryType::Metrics => "Metrics",
            QueryType::Traces => "Traces",
            QueryType::Logs => "Logs",
            QueryType::Analytics => "Analytics",
        },
        request.parameters.time_range.start,
        request.parameters.time_range.end
    ))
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
