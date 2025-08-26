//! Response Time Calculation Demo
//! 
//! This example demonstrates the response time calculation functionality
//! in the Orasi Gateway proxy routing system.

use orasi_gateway::{
    config::GatewayConfig,
    error::GatewayError,
    gateway::state::GatewayState,
    routing::proxy::Proxy,
    types::{ServiceEndpoint, EndpointHealthStatus},
};
use axum::{
    body::Body,
    http::{Request, Method, HeaderValue},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Orasi Gateway Response Time Calculation Demo");
    println!("===============================================");

    // Create gateway configuration
    let config = GatewayConfig::default();
    
    // Create gateway state
    let state = Arc::new(RwLock::new(GatewayState::new(&config)));
    
    // Create proxy instance
    let proxy = Proxy::new(&config, state).await?;

    println!("\n📊 Creating test endpoints...");

    // Create test endpoints
    let endpoints = vec![
        ServiceEndpoint {
            url: "http://httpbin.org/delay/1".to_string(),
            weight: 1,
            health_status: EndpointHealthStatus::Healthy,
            metadata: HashMap::new(),
        },
        ServiceEndpoint {
            url: "http://httpbin.org/delay/2".to_string(),
            weight: 1,
            health_status: EndpointHealthStatus::Healthy,
            metadata: HashMap::new(),
        },
        ServiceEndpoint {
            url: "http://httpbin.org/status/200".to_string(),
            weight: 1,
            health_status: EndpointHealthStatus::Healthy,
            metadata: HashMap::new(),
        },
    ];

    println!("✅ Created {} test endpoints", endpoints.len());

    // Test each endpoint
    for (i, endpoint) in endpoints.iter().enumerate() {
        println!("\n📊 Test {}: {}", i + 1, endpoint.url);
        
        // Create a test request
        let request = Request::builder()
            .method(Method::GET)
            .uri(&endpoint.url)
            .body(Body::empty())
            .unwrap();

        println!("  • Sending request to: {}", endpoint.url);
        
        // Measure start time
        let start_time = std::time::Instant::now();
        
        // Proxy the request
        match proxy.proxy_request(request, endpoint).await {
            Ok(response) => {
                let total_time = start_time.elapsed();
                
                println!("  ✅ Request successful!");
                println!("  • Status: {}", response.status());
                println!("  • Total time: {:?}", total_time);
                
                // Extract response time headers
                let headers = response.headers();
                
                if let Some(response_time) = headers.get("X-Response-Time") {
                    println!("  • X-Response-Time: {}", response_time.to_str().unwrap());
                }
                
                if let Some(response_time_micros) = headers.get("X-Response-Time-Micros") {
                    println!("  • X-Response-Time-Micros: {}", response_time_micros.to_str().unwrap());
                }
                
                if let Some(response_time_secs) = headers.get("X-Response-Time-Seconds") {
                    println!("  • X-Response-Time-Seconds: {}", response_time_secs.to_str().unwrap());
                }
                
                if let Some(gateway_proxy) = headers.get("X-Gateway-Proxy") {
                    println!("  • X-Gateway-Proxy: {}", gateway_proxy.to_str().unwrap());
                }
                
                if let Some(gateway_endpoint) = headers.get("X-Gateway-Endpoint") {
                    println!("  • X-Gateway-Endpoint: {}", gateway_endpoint.to_str().unwrap());
                }
                
                if let Some(gateway_timestamp) = headers.get("X-Gateway-Timestamp") {
                    println!("  • X-Gateway-Timestamp: {}", gateway_timestamp.to_str().unwrap());
                }
            }
            Err(e) => {
                let total_time = start_time.elapsed();
                println!("  ❌ Request failed: {:?}", e);
                println!("  • Time to failure: {:?}", total_time);
            }
        }
    }

    println!("\n📊 Testing response time formatting...");
    
    // Test the transform_response function directly with different durations
    let test_durations = vec![
        Duration::from_millis(1),
        Duration::from_millis(100),
        Duration::from_millis(1500),
        Duration::from_micros(500),
    ];

    for duration in test_durations {
        let mock_response = axum::http::Response::builder()
            .status(200)
            .body(Body::empty())
            .unwrap();

        let test_endpoint = ServiceEndpoint {
            url: "http://test.example.com".to_string(),
            weight: 1,
            health_status: EndpointHealthStatus::Healthy,
            metadata: HashMap::new(),
        };

        match proxy.transform_response(mock_response, &test_endpoint, duration).await {
            Ok(response) => {
                let headers = response.headers();
                let response_time = headers.get("X-Response-Time").unwrap();
                let response_time_micros = headers.get("X-Response-Time-Micros").unwrap();
                let response_time_secs = headers.get("X-Response-Time-Seconds").unwrap();
                
                println!("  • Duration: {:?}", duration);
                println!("    - X-Response-Time: {}", response_time.to_str().unwrap());
                println!("    - X-Response-Time-Micros: {}", response_time_micros.to_str().unwrap());
                println!("    - X-Response-Time-Seconds: {}", response_time_secs.to_str().unwrap());
            }
            Err(e) => {
                println!("  ❌ Failed to transform response: {:?}", e);
            }
        }
    }

    println!("\n✅ Response time calculation demo completed!");
    println!("\nThe gateway now provides:");
    println!("  • Accurate response time measurement");
    println!("  • Multiple time format headers (ms, microseconds, seconds)");
    println!("  • Gateway identification headers");
    println!("  • Endpoint information for debugging");
    println!("  • Timestamp information");
    println!("  • Performance logging based on response times");

    Ok(())
}
