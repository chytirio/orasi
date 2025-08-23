//! Enhanced Query Engine Example
//!
//! This example demonstrates the enhanced query engine capabilities including:
//! - Time series analysis
//! - Query execution with caching
//! - Query optimization
//! - Advanced telemetry functions
//! - Performance monitoring

use bridge_core::BridgeResult;
use chrono::{Duration, Utc};
use query_engine::{
    functions::telemetry_functions::TimeSeriesPoint,
    functions::{TelemetryFunctionConfig, TelemetryFunctions},
    get_query_engine, init_query_engine, shutdown_query_engine,
    sources::DeltaLakeDataSourceConfig,
    QueryEngine, QueryEngineConfig,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    println!("🚀 Enhanced Query Engine Example");
    println!("================================\n");

    // Initialize query engine
    let config = QueryEngineConfig {
        enable_caching: true,
        cache_size_limit: 1000,
        enable_optimization: true,
        max_execution_time_seconds: 300,
        enable_streaming: false,
        max_result_set_size: 10000,
        enable_plan_visualization: true,
        enable_performance_monitoring: true,
        data_sources: HashMap::new(),
    };

    // Initialize the global query engine
    init_query_engine(config).await?;
    let query_engine = get_query_engine().expect("Query engine should be initialized");

    // Initialize telemetry functions
    let telemetry_config = TelemetryFunctionConfig::default();
    let telemetry_functions = TelemetryFunctions::new(telemetry_config);
    telemetry_functions.init().await?;

    println!("✅ Query Engine initialized successfully\n");

    // Run demonstrations
    println!("📊 Running Time Series Analysis Demo...");
    run_time_series_analysis(&telemetry_functions).await?;

    println!("\n🔍 Running Query Execution Demo...");
    run_query_execution_demo(&query_engine).await?;

    println!("\n⚡ Running Query Optimization Demo...");
    run_query_optimization_demo(&query_engine).await?;

    println!("\n📋 Running Query Plan Demo...");
    run_query_plan_demo(&query_engine).await?;

    println!("\n🧮 Running Advanced Functions Demo...");
    run_advanced_functions_demo(&query_engine).await?;

    println!("\n📈 Running Performance Monitoring Demo...");
    run_performance_monitoring_demo(&query_engine).await?;

    // Shutdown
    shutdown_query_engine().await?;
    println!("\n✅ Enhanced Query Engine Example completed successfully!");

    Ok(())
}

async fn run_time_series_analysis(telemetry_functions: &TelemetryFunctions) -> BridgeResult<()> {
    println!("  Adding time series data points...");

    // Add some sample time series data
    for i in 0..20 {
        let point = TimeSeriesPoint {
            timestamp: Utc::now() + Duration::minutes(i),
            value: 100.0 + i as f64 * 2.0 + (i % 3) as f64 * 10.0, // Some variation
            metadata: HashMap::new(),
        };
        telemetry_functions
            .add_time_series_point("cpu_usage".to_string(), point)
            .await?;
    }

    println!("  Analyzing time series...");
    let analysis = telemetry_functions.analyze_time_series("cpu_usage").await?;

    println!("  Analysis Results:");
    println!("    Mean: {:.2}", analysis.mean);
    println!("    Std Dev: {:.2}", analysis.std_dev);
    println!("    Min: {:.2}", analysis.min);
    println!("    Max: {:.2}", analysis.max);
    println!("    Trend: {:?}", analysis.trend);
    println!("    Trend Strength: {:.2}", analysis.trend_strength);
    println!("    Anomaly Score: {:.2}", analysis.anomaly_score);
    println!("    Is Anomaly: {}", analysis.is_anomaly);

    // Test forecasting
    println!("  Testing forecasting...");
    let forecast = telemetry_functions.forecast("cpu_usage", 5).await?;
    println!("    Forecast (next 5 points): {:?}", forecast);

    // Test moving average
    println!("  Testing moving average...");
    let moving_avg = telemetry_functions.moving_average("cpu_usage", 3).await?;
    println!("    Moving Average (window=3): {:?}", moving_avg);

    // Test percentile
    println!("  Testing percentile calculation...");
    let percentile_95 = telemetry_functions.percentile("cpu_usage", 95.0).await?;
    println!("    95th Percentile: {:?}", percentile_95);

    Ok(())
}

async fn run_query_execution_demo(query_engine: &QueryEngine) -> BridgeResult<()> {
    println!("  Executing sample queries...");

    // Execute a simple query
    let query1 = "SELECT * FROM telemetry WHERE service = 'web'";
    let result1 = query_engine.execute_query(query1).await?;
    println!("  Query 1 Result: {} rows", result1.data.len());

    // Execute another query to test caching
    let result2 = query_engine.execute_query(query1).await?;
    println!("  Query 1 (cached) Result: {} rows", result2.data.len());

    // Get cache statistics
    let cache_stats = query_engine.get_cache_stats().await;
    println!("  Cache Stats:");
    println!("    Total Entries: {}", cache_stats.total_entries);
    println!("    Hit Rate: {:.2}%", cache_stats.hit_rate * 100.0);

    Ok(())
}

async fn run_query_optimization_demo(query_engine: &QueryEngine) -> BridgeResult<()> {
    println!("  Testing query optimization...");

    let complex_query = "SELECT service, AVG(response_time) FROM telemetry WHERE timestamp > '2024-01-01' GROUP BY service ORDER BY AVG(response_time) DESC LIMIT 10";

    // Get query plan
    let plan = query_engine.get_query_plan(complex_query).await?;
    println!("  Query Plan:");
    println!("    {}", plan);

    // Execute optimized query
    let result = query_engine.execute_query(complex_query).await?;
    println!("  Optimized Query Result: {} rows", result.data.len());

    Ok(())
}

async fn run_query_plan_demo(query_engine: &QueryEngine) -> BridgeResult<()> {
    println!("  Generating query plans...");

    let queries = vec![
        "SELECT * FROM logs WHERE level = 'ERROR'",
        "SELECT service, COUNT(*) FROM metrics GROUP BY service",
        "SELECT * FROM traces WHERE duration > 1000",
    ];

    for (i, query) in queries.iter().enumerate() {
        let plan = query_engine.get_query_plan(query).await?;
        println!("  Query {} Plan:", i + 1);
        println!("    {}", plan);
    }

    Ok(())
}

async fn run_advanced_functions_demo(query_engine: &QueryEngine) -> BridgeResult<()> {
    println!("  Testing advanced telemetry functions...");

    // This would test the individual QueryFunction implementations
    // For now, just demonstrate the concept
    println!("  Advanced functions available:");
    println!("    - Time Series Aggregation");
    println!("    - Anomaly Detection");
    println!("    - Trend Analysis");
    println!("    - Forecasting");

    Ok(())
}

async fn run_performance_monitoring_demo(query_engine: &QueryEngine) -> BridgeResult<()> {
    println!("  Monitoring query engine performance...");

    // Get engine statistics
    let stats = query_engine.get_stats().await;
    println!("  Engine Statistics:");
    println!("    Total Queries: {}", stats.total_queries);
    println!("    Cached Queries: {}", stats.cached_queries);
    println!("    Optimized Queries: {}", stats.optimized_queries);
    println!(
        "    Average Execution Time: {:.2}ms",
        stats.avg_execution_time_ms
    );
    println!("    Cache Hit Rate: {:.2}%", stats.cache_hit_rate * 100.0);
    println!(
        "    Optimization Success Rate: {:.2}%",
        stats.optimization_success_rate * 100.0
    );

    Ok(())
}
