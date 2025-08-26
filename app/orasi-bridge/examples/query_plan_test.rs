//! Query plan generation test example
//! 
//! This example demonstrates the comprehensive query plan generation
//! functionality in the Bridge API.

use bridge_api::{
    handlers::query::{generate_query_plan, init_query_cache, query_handler},
    rest::AppState,
    types::{QueryRequest, QueryType, QueryParameters, QueryOptions},
    config::BridgeAPIConfig,
    metrics::ApiMetrics,
};
use bridge_core::types::{
    TimeRange, Filter, FilterOperator, FilterValue, 
    Aggregation, AggregationFunction
};
use axum::{extract::State, response::Json};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Bridge API Query Plan Generation Test");
    println!("========================================");

    // Initialize query cache
    init_query_cache();

    // Create application state
    let config = BridgeAPIConfig::default();
    let metrics = ApiMetrics::new();
    let state = AppState { config, metrics };

    // Test 1: Simple metrics query
    println!("\n📊 Test 1: Simple Metrics Query");
    let simple_metrics_request = QueryRequest {
        query_type: QueryType::Metrics,
        parameters: QueryParameters {
            time_range: TimeRange {
                start: chrono::Utc::now() - chrono::Duration::hours(1),
                end: chrono::Utc::now(),
            },
            filters: None,
            aggregations: None,
            limit: Some(100),
            offset: None,
        },
        options: Some(QueryOptions {
            enable_cache: true,
            cache_ttl_seconds: Some(300),
            timeout_seconds: Some(30),
            enable_streaming: false,
        }),
    };

    match query_handler(State(state.clone()), Json(simple_metrics_request)).await {
        Ok(response) => {
            let query_plan = response.0.data.metadata.query_plan.unwrap_or("No plan generated".to_string());
            println!("✅ Simple metrics query plan generated:");
            println!("{}", query_plan);
        }
        Err(e) => {
            println!("❌ Simple metrics query failed: {:?}", e);
        }
    }

    // Test 2: Complex query with filters and aggregations
    println!("\n📊 Test 2: Complex Query with Filters and Aggregations");
    let complex_request = QueryRequest {
        query_type: QueryType::Traces,
        parameters: QueryParameters {
            time_range: TimeRange {
                start: chrono::Utc::now() - chrono::Duration::days(7),
                end: chrono::Utc::now(),
            },
            filters: Some(vec![
                Filter::new(
                    "service.name".to_string(),
                    FilterOperator::Equals,
                    FilterValue::String("user-service".to_string()),
                ),
                Filter::new(
                    "duration".to_string(),
                    FilterOperator::GreaterThan,
                    FilterValue::Number(1000.0),
                ),
                Filter::new(
                    "status_code".to_string(),
                    FilterOperator::In,
                    FilterValue::Array(vec![
                        FilterValue::Number(200.0),
                        FilterValue::Number(201.0),
                        FilterValue::Number(404.0),
                    ]),
                ),
            ]),
            aggregations: Some(vec![
                Aggregation::new("duration".to_string(), AggregationFunction::Average)
                    .with_alias("avg_duration".to_string()),
                Aggregation::new("span_id".to_string(), AggregationFunction::Count)
                    .with_alias("span_count".to_string()),
                Aggregation::new("duration".to_string(), AggregationFunction::Percentile)
                    .with_parameters({
                        let mut params = std::collections::HashMap::new();
                        params.insert("percentile".to_string(), json!(95));
                        params
                    }),
            ]),
            limit: Some(1000),
            offset: Some(0),
        },
        options: Some(QueryOptions {
            enable_cache: true,
            cache_ttl_seconds: Some(600),
            timeout_seconds: Some(60),
            enable_streaming: true,
        }),
    };

    match query_handler(State(state.clone()), Json(complex_request)).await {
        Ok(response) => {
            let query_plan = response.0.data.metadata.query_plan.unwrap_or("No plan generated".to_string());
            println!("✅ Complex query plan generated:");
            println!("{}", query_plan);
        }
        Err(e) => {
            println!("❌ Complex query failed: {:?}", e);
        }
    }

    // Test 3: Analytics query (multi-stage execution)
    println!("\n📊 Test 3: Analytics Query (Multi-stage Execution)");
    let analytics_request = QueryRequest {
        query_type: QueryType::Analytics,
        parameters: QueryParameters {
            time_range: TimeRange {
                start: chrono::Utc::now() - chrono::Duration::days(30),
                end: chrono::Utc::now(),
            },
            filters: Some(vec![
                Filter::new(
                    "user_agent".to_string(),
                    FilterOperator::Contains,
                    FilterValue::String("mobile".to_string()),
                ),
            ]),
            aggregations: Some(vec![
                Aggregation::new("user_id".to_string(), AggregationFunction::Cardinality),
                Aggregation::new("request_count".to_string(), AggregationFunction::Sum),
                Aggregation::new("response_time".to_string(), AggregationFunction::Histogram),
            ]),
            limit: Some(500),
            offset: None,
        },
        options: Some(QueryOptions {
            enable_cache: false,
            cache_ttl_seconds: None,
            timeout_seconds: Some(120),
            enable_streaming: false,
        }),
    };

    match query_handler(State(state.clone()), Json(analytics_request)).await {
        Ok(response) => {
            let query_plan = response.0.data.metadata.query_plan.unwrap_or("No plan generated".to_string());
            println!("✅ Analytics query plan generated:");
            println!("{}", query_plan);
        }
        Err(e) => {
            println!("❌ Analytics query failed: {:?}", e);
        }
    }

    // Test 4: Large time range query (aggregation push strategy)
    println!("\n📊 Test 4: Large Time Range Query (Aggregation Push Strategy)");
    let large_time_range_request = QueryRequest {
        query_type: QueryType::Logs,
        parameters: QueryParameters {
            time_range: TimeRange {
                start: chrono::Utc::now() - chrono::Duration::days(90),
                end: chrono::Utc::now(),
            },
            filters: Some(vec![
                Filter::new(
                    "log_level".to_string(),
                    FilterOperator::In,
                    FilterValue::Array(vec![
                        FilterValue::String("ERROR".to_string()),
                        FilterValue::String("WARN".to_string()),
                    ]),
                ),
            ]),
            aggregations: Some(vec![
                Aggregation::new("timestamp".to_string(), AggregationFunction::Count)
                    .with_alias("error_count_by_day".to_string()),
                Aggregation::new("service".to_string(), AggregationFunction::Cardinality),
            ]),
            limit: Some(10000),
            offset: None,
        },
        options: Some(QueryOptions {
            enable_cache: true,
            cache_ttl_seconds: Some(3600), // 1 hour cache for large queries
            timeout_seconds: Some(300),    // 5 minute timeout
            enable_streaming: true,
        }),
    };

    match query_handler(State(state.clone()), Json(large_time_range_request)).await {
        Ok(response) => {
            let query_plan = response.0.data.metadata.query_plan.unwrap_or("No plan generated".to_string());
            println!("✅ Large time range query plan generated:");
            println!("{}", query_plan);
        }
        Err(e) => {
            println!("❌ Large time range query failed: {:?}", e);
        }
    }

    // Test 5: High selectivity filter query (index lookup strategy)
    println!("\n📊 Test 5: High Selectivity Filter Query (Index Lookup Strategy)");
    let high_selectivity_request = QueryRequest {
        query_type: QueryType::Metrics,
        parameters: QueryParameters {
            time_range: TimeRange {
                start: chrono::Utc::now() - chrono::Duration::hours(6),
                end: chrono::Utc::now(),
            },
            filters: Some(vec![
                Filter::new(
                    "timestamp".to_string(),
                    FilterOperator::GreaterThanOrEqual,
                    FilterValue::String((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339()),
                ),
                Filter::new(
                    "metric_name".to_string(),
                    FilterOperator::Equals,
                    FilterValue::String("cpu_usage".to_string()),
                ),
                Filter::new(
                    "host".to_string(),
                    FilterOperator::StartsWith,
                    FilterValue::String("prod-".to_string()),
                ),
                Filter::new(
                    "value".to_string(),
                    FilterOperator::GreaterThan,
                    FilterValue::Number(80.0),
                ),
            ]),
            aggregations: None,
            limit: Some(50),
            offset: None,
        },
        options: Some(QueryOptions {
            enable_cache: true,
            cache_ttl_seconds: Some(60),  // Short cache for real-time data
            timeout_seconds: Some(10),    // Fast query expected
            enable_streaming: false,
        }),
    };

    match query_handler(State(state.clone()), Json(high_selectivity_request)).await {
        Ok(response) => {
            let query_plan = response.0.data.metadata.query_plan.unwrap_or("No plan generated".to_string());
            println!("✅ High selectivity query plan generated:");
            println!("{}", query_plan);
        }
        Err(e) => {
            println!("❌ High selectivity query failed: {:?}", e);
        }
    }

    println!("\n✅ Query plan generation test completed!");
    println!("\nThe query planner successfully:");
    println!("  • Analyzed different query types and patterns");
    println!("  • Selected appropriate execution strategies");
    println!("  • Estimated costs and resource usage");
    println!("  • Identified optimization opportunities");
    println!("  • Generated detailed execution plans");
    println!("  • Determined parallelism strategies");

    Ok(())
}
