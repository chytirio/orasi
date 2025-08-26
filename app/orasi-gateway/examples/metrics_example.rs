//! Example demonstrating the metrics collector functionality

use orasi_gateway::{config::GatewayConfig, metrics::MetricsCollector};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Starting Orasi Gateway Metrics Example");

    // Create gateway configuration
    let config = GatewayConfig::default();
    println!("📋 Gateway ID: {}", config.gateway_id);
    println!("📊 Metrics endpoint: {}", config.metrics_endpoint);

    // Create metrics collector
    let collector = MetricsCollector::new(&config).await?;
    println!("✅ Metrics collector created successfully");

    // Start metrics collector
    collector.start().await?;
    println!("✅ Metrics collector started successfully");

    // Simulate some metrics recording
    println!("📈 Recording sample metrics...");
    
    // Record some requests
    collector.record_request("GET", "/api/v1/health", 200);
    collector.record_request("POST", "/api/v1/users", 201);
    collector.record_request("GET", "/api/v1/users", 200);
    collector.record_request("DELETE", "/api/v1/users/123", 404);
    
    // Record response times
    collector.record_response_time("GET", "/api/v1/health", 15.5);
    collector.record_response_time("POST", "/api/v1/users", 45.2);
    collector.record_response_time("GET", "/api/v1/users", 23.1);
    collector.record_response_time("DELETE", "/api/v1/users/123", 8.7);
    
    // Record active connections
    collector.record_active_connections(42);
    
    // Record rate limit violations
    collector.record_rate_limit_violation("192.168.1.1");
    collector.record_rate_limit_violation("10.0.0.5");
    
    // Record circuit breaker trips
    collector.record_circuit_breaker_trip("user-service");
    collector.record_circuit_breaker_trip("payment-service");
    
    // Record service discovery events
    collector.record_service_discovery_event("service_added", "user-service");
    collector.record_service_discovery_event("service_removed", "old-service");
    
    // Record load balancer events
    collector.record_load_balancer_event("endpoint_selected", "user-service");
    collector.record_load_balancer_event("endpoint_failed", "payment-service");
    
    // Record health checks
    collector.record_health_check("user-service", true);
    collector.record_health_check("payment-service", false);
    collector.record_health_check("notification-service", true);

    println!("✅ Sample metrics recorded successfully");

    // Collect metrics
    let metrics = collector.collect_metrics().await?;
    println!("📊 Collected metrics:");
    println!("   - Total requests: {}", metrics.total_requests);
    println!("   - Successful requests: {}", metrics.successful_requests);
    println!("   - Failed requests: {}", metrics.failed_requests);
    println!("   - Average response time: {:.2} ms", metrics.avg_response_time_ms);
    println!("   - Active connections: {}", metrics.active_connections);
    println!("   - Rate limit violations: {}", metrics.rate_limit_violations);
    println!("   - Circuit breaker trips: {}", metrics.circuit_breaker_trips);

    // Check if collector is running
    let is_running = collector.is_running().await;
    println!("🔄 Metrics collector running: {}", is_running);

    // Wait a bit to see metrics in action
    println!("⏳ Waiting 5 seconds to demonstrate metrics collection...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Stop metrics collector
    collector.stop().await?;
    println!("✅ Metrics collector stopped successfully");

    // Check if collector is stopped
    let is_running = collector.is_running().await;
    println!("🔄 Metrics collector running: {}", is_running);

    println!("🎉 Metrics example completed successfully!");
    println!("💡 You can access Prometheus metrics at: http://{}", config.metrics_endpoint);

    Ok(())
}
