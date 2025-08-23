//! Example demonstrating service discovery functionality

use orasi_gateway::{
    config::GatewayConfig,
    discovery::{
        backends::{ServiceDiscoveryBackend, StaticBackend},
        health::HealthCheckConfig,
        ServiceDiscovery,
    },
    types::{EndpointHealthStatus, ServiceEndpoint, ServiceHealthStatus, ServiceInfo},
};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting service discovery example");

    // Create a basic gateway configuration
    let mut config = GatewayConfig::default();
    config.service_discovery.backend = orasi_gateway::config::ServiceDiscoveryBackend::Static;
    config.service_discovery.refresh_interval = Duration::from_secs(10);
    config.service_discovery.health_check_interval = Duration::from_secs(5);

    // Create service discovery
    let mut service_discovery = ServiceDiscovery::new(&config).await?;

    // Start service discovery
    service_discovery.start().await?;

    // Create some example services
    let service1 = ServiceInfo {
        name: "user-service".to_string(),
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
        name: "order-service".to_string(),
        endpoints: vec![ServiceEndpoint {
            url: "http://localhost:8083".to_string(),
            weight: 1,
            health_status: EndpointHealthStatus::Healthy,
            metadata: HashMap::new(),
        }],
        health_status: ServiceHealthStatus::Healthy,
        metadata: HashMap::new(),
    };

    // Register services
    service_discovery
        .register_service("user-service-1", &service1)
        .await?;
    service_discovery
        .register_service("order-service-1", &service2)
        .await?;

    info!("Registered services");

    // Wait a bit for background processes
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get all services
    let services = service_discovery.get_services().await;
    info!("Discovered {} services", services.len());

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
    if let Some(service) = service_discovery.get_service("user-service-1").await {
        info!("Retrieved user-service-1: {}", service.name);
    }

    // Manual refresh
    info!("Performing manual refresh");
    let refreshed_services = service_discovery.refresh_services().await?;
    info!("Refreshed {} services", refreshed_services.len());

    // Wait a bit more to see health checks in action
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Deregister a service
    service_discovery
        .deregister_service("order-service-1")
        .await?;
    info!("Deregistered order-service-1");

    // Check final state
    let final_services = service_discovery.get_services().await;
    info!("Final service count: {}", final_services.len());

    // Stop service discovery
    service_discovery.stop().await?;
    info!("Service discovery stopped");

    Ok(())
}
