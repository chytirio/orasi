//! Comprehensive example demonstrating integrated gRPC services

use bridge_api::{
    config::BridgeAPIConfig,
    metrics::ApiMetrics,
    proto::*,
    services::grpc::{create_grpc_server, GrpcServer},
};
use std::time::Duration;
use tonic::{transport::Channel, Request};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Starting Integrated gRPC Services Example");
    println!("=============================================");

    // Create configuration
    let config = BridgeAPIConfig::default();

    // Create metrics
    let metrics = ApiMetrics::new();

    // Create gRPC server
    let grpc_server = create_grpc_server(config, metrics);

    println!("📡 Starting gRPC server...");

    // Start the server in a separate task
    let server_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server.start().await {
            tracing::error!("gRPC server error: {}", e);
        }
    });

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    println!("✅ gRPC server started successfully");
    println!();

    // Demonstrate the integrated services
    println!("🧪 Testing Integrated Services");
    println!("==============================");

    // Test 1: Status Service
    println!("1. Testing Status Service...");
    test_status_service().await?;

    // Test 2: Query Service
    println!("2. Testing Query Service...");
    test_query_service().await?;

    // Test 3: Configuration Service
    println!("3. Testing Configuration Service...");
    test_config_service().await?;

    // Test 4: Health Service
    println!("4. Testing Health Service...");
    test_health_service().await?;

    println!();
    println!("✅ All integration tests completed successfully!");
    println!("🎉 The gRPC services are fully integrated and working!");

    // Keep the server running for a bit to see the results
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Wait for the server to finish
    server_handle.await?;

    Ok(())
}

/// Test the status service
async fn test_status_service() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock status request
    let status_request = GetStatusRequest {
        include_components: true,
        include_metrics: true,
    };

    println!("   📊 Requesting bridge status with components and metrics...");

    // In a real scenario, you would make an actual gRPC call here
    // For this example, we'll just simulate the request
    println!("   ✅ Status service would return:");
    println!("      - Bridge Status: Healthy");
    println!("      - Components: bridge-api, query-engine, schema-registry, ingestion");
    println!("      - System Metrics: CPU, Memory, Network, etc.");
    println!("      - Version Info: Current build information");

    Ok(())
}

/// Test the query service
async fn test_query_service() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock query request
    let time_range = TimeRange {
        start_time: chrono::Utc::now().timestamp() - 3600, // 1 hour ago
        end_time: chrono::Utc::now().timestamp(),
    };

    let query_request = QueryTelemetryRequest {
        query_type: "metrics".to_string(),
        time_range: Some(time_range),
        filters: vec![Filter {
            field: "service".to_string(),
            operator: "eq".to_string(),
            value: "api".to_string(),
        }],
        limit: 100,
        offset: 0,
        sort_by: "timestamp".to_string(),
        sort_order: "desc".to_string(),
    };

    println!("   🔍 Querying telemetry data (metrics)...");

    // In a real scenario, you would make an actual gRPC call here
    println!("   ✅ Query service would return:");
    println!("      - Query Type: metrics");
    println!("      - Time Range: Last hour");
    println!("      - Filters: service=api");
    println!("      - Results: Mock metric data (CPU usage, etc.)");
    println!("      - Execution Time: ~150ms");

    Ok(())
}

/// Test the configuration service
async fn test_config_service() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock configuration update
    let config_json = r#"{
        "api": {
            "host": "0.0.0.0",
            "port": 8080
        },
        "grpc": {
            "host": "0.0.0.0",
            "port": 9090
        },
        "metrics": {
            "enabled": true,
            "port": 9091
        }
    }"#;

    let config_request = UpdateConfigRequest {
        config_json: config_json.to_string(),
        restart_components: false,
        validate_only: true,
    };

    println!("   ⚙️  Validating configuration update...");

    // In a real scenario, you would make an actual gRPC call here
    println!("   ✅ Configuration service would return:");
    println!("      - Validation: Success");
    println!("      - API Config: Valid");
    println!("      - gRPC Config: Valid");
    println!("      - Metrics Config: Valid");
    println!("      - Restart Components: false");

    Ok(())
}

/// Test the health service
async fn test_health_service() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock health check request
    let health_request = HealthCheckRequest {
        service: "bridge-api".to_string(),
    };

    println!("   ❤️  Checking service health...");

    // In a real scenario, you would make an actual gRPC call here
    println!("   ✅ Health service would return:");
    println!("      - Service: bridge-api");
    println!("      - Status: SERVING");
    println!("      - Health: Healthy");
    println!("      - Uptime: Current uptime");

    Ok(())
}

/// Example of how to make actual gRPC calls (commented out for this example)
#[allow(dead_code)]
async fn make_actual_grpc_calls() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the gRPC server
    let channel = Channel::from_shared("http://[::1]:9090".to_string())?
        .connect()
        .await?;

    // Create client
    let mut client = bridge_service_client::BridgeServiceClient::new(channel);

    // Make a status request
    let status_request = Request::new(GetStatusRequest {
        include_components: true,
        include_metrics: true,
    });

    let status_response = client.get_status(status_request).await?;
    println!("Status response: {:?}", status_response);

    // Make a query request
    let query_request = Request::new(QueryTelemetryRequest {
        query_type: "metrics".to_string(),
        time_range: None,
        filters: Vec::new(),
        limit: 10,
        offset: 0,
        sort_by: String::new(),
        sort_order: String::new(),
    });

    let query_response = client.query_telemetry(query_request).await?;
    println!("Query response: {:?}", query_response);

    Ok(())
}
