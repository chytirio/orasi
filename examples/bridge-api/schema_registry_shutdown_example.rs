//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating proper shutdown of the Schema Registry
//!
//! This example shows how to properly initialize and shutdown the schema registry
//! with all its components.

use schema_registry::{
    SchemaRegistryConfig, init_schema_registry, shutdown_schema_registry,
    schema::{Schema, SchemaFormat, SchemaType, SchemaVersion},
};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Schema Registry shutdown example");

    // Create configuration
    let config = SchemaRegistryConfig::default();
    info!("Created default configuration");

    // Initialize the schema registry
    let registry = match init_schema_registry(config).await {
        Ok(registry) => {
            info!("Schema registry initialized successfully");
            registry
        }
        Err(e) => {
            error!("Failed to initialize schema registry: {}", e);
            return Err(e.into());
        }
    };

    // Register a test schema
    let schema = Schema::new(
        "example-schema".to_string(),
        SchemaVersion::new(1, 0, 0),
        SchemaType::Metric,
        r#"{"type": "object", "properties": {"value": {"type": "number"}}}"#.to_string(),
        SchemaFormat::Json,
    );

    match registry.register_schema(schema).await {
        Ok(version) => {
            info!("Schema registered successfully with version: {}", version);
        }
        Err(e) => {
            error!("Failed to register schema: {}", e);
        }
    }

    // Perform a health check
    match registry.health_check().await {
        Ok(healthy) => {
            info!("Health check result: {}", healthy);
        }
        Err(e) => {
            error!("Health check failed: {}", e);
        }
    }

    // Get statistics
    match registry.get_stats().await {
        Ok(stats) => {
            info!("Registry stats: {} schemas, {} versions", 
                  stats.total_schemas, stats.total_versions);
        }
        Err(e) => {
            error!("Failed to get stats: {}", e);
        }
    }

    // Proper shutdown
    info!("Initiating graceful shutdown...");
    match shutdown_schema_registry(registry).await {
        Ok(()) => {
            info!("Schema registry shutdown completed successfully");
        }
        Err(e) => {
            error!("Failed to shutdown schema registry: {}", e);
            return Err(e.into());
        }
    }

    info!("Example completed successfully");
    Ok(())
}
