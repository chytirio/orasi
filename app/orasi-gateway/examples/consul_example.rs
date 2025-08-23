//! Example demonstrating Consul service discovery functionality

use orasi_gateway::{
    config::{GatewayConfig, ServiceDiscoveryBackend},
    discovery::{health::HealthCheckConfig, ServiceDiscovery},
    types::{EndpointHealthStatus, ServiceEndpoint, ServiceHealthStatus, ServiceInfo},
};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Consul service discovery example");

    // Create a gateway configuration with Consul backend
    let mut config = GatewayConfig::default();
    config.service_discovery.backend = ServiceDiscoveryBackend::Consul;
    config.service_discovery.endpoints = vec!["http://localhost:8500".to_string()]; // Default Consul port
    config.service_discovery.refresh_interval = Duration::from_secs(10);
    config.service_discovery.health_check_interval = Duration::from_secs(5);

    // Configure Consul-specific settings
    config.service_discovery.consul.datacenter = "dc1".to_string();
    config.service_discovery.consul.service_prefix = "orasi".to_string();
    config.service_discovery.consul.token = None; // No ACL token for this example

    info!("Configuration: {:?}", config.service_discovery);

    // Create service discovery (but don't start background tasks)
    let service_discovery = ServiceDiscovery::new(&config).await?;

    // Create some example services
    let service1 = ServiceInfo {
        name: "orasi-user-service".to_string(),
        endpoints: vec![
            ServiceEndpoint {
                url: "http://localhost:8081".to_string(),
                weight: 1,
                health_status: EndpointHealthStatus::Healthy,
                metadata: HashMap::new(),
            },
            ServiceEndpoint {
                url: "http://localhost:8082".to_string(),
                weight: 1,
                health_status: EndpointHealthStatus::Healthy,
                metadata: HashMap::new(),
            },
        ],
        health_status: ServiceHealthStatus::Healthy,
        metadata: HashMap::new(),
    };

    let service2 = ServiceInfo {
        name: "orasi-order-service".to_string(),
        endpoints: vec![ServiceEndpoint {
            url: "http://localhost:8083".to_string(),
            weight: 1,
            health_status: EndpointHealthStatus::Healthy,
            metadata: HashMap::new(),
        }],
        health_status: ServiceHealthStatus::Healthy,
        metadata: HashMap::new(),
    };

    // Register services with Consul
    info!("Registering services with Consul...");
    match service_discovery
        .register_service("orasi-user-service-1", &service1)
        .await
    {
        Ok(_) => info!("Successfully registered orasi-user-service-1"),
        Err(e) => error!("Failed to register orasi-user-service-1: {}", e),
    }

    match service_discovery
        .register_service("orasi-order-service-1", &service2)
        .await
    {
        Ok(_) => info!("Successfully registered orasi-order-service-1"),
        Err(e) => error!("Failed to register orasi-order-service-1: {}", e),
    }

    // Wait a bit for background processes
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Get all services from Consul
    info!("Discovering services from Consul...");
    let services = service_discovery.get_services().await;
    info!("Discovered {} services from Consul", services.len());

    for (service_id, service_info) in &services {
        info!(
            "Service: {} - Name: {} - Health: {:?}",
            service_id, service_info.name, service_info.health_status
        );

        for (i, endpoint) in service_info.endpoints.iter().enumerate() {
            info!(
                "  Endpoint {}: {} - Health: {:?}",
                i, endpoint.url, endpoint.health_status
            );
        }
    }

    // Get healthy services only
    let healthy_services = service_discovery.get_healthy_services().await;
    info!("Found {} healthy services", healthy_services.len());

    // Get a specific service
    if let Some(service) = service_discovery.get_service("orasi-user-service-1").await {
        info!("Retrieved orasi-user-service-1: {}", service.name);
    }

    // Manual refresh
    info!("Performing manual refresh from Consul");
    match service_discovery.refresh_services().await {
        Ok(refreshed_services) => {
            info!(
                "Refreshed {} services from Consul",
                refreshed_services.len()
            );
        }
        Err(e) => {
            warn!("Failed to refresh services from Consul: {}", e);
        }
    }

    // Wait a bit more to see health checks in action
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Deregister a service from Consul
    info!("Deregistering orasi-order-service-1 from Consul");
    match service_discovery
        .deregister_service("orasi-order-service-1")
        .await
    {
        Ok(_) => info!("Successfully deregistered orasi-order-service-1"),
        Err(e) => error!("Failed to deregister orasi-order-service-1: {}", e),
    }

    // Check final state
    let final_services = service_discovery.get_services().await;
    info!("Final service count: {}", final_services.len());

    info!("Consul example completed successfully");
    info!("Note: This example requires a running Consul server on localhost:8500");
    info!("To start Consul: consul agent -dev");

    Ok(())
}
