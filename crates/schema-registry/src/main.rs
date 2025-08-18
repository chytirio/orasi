//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Schema Registry main binary
//!
//! This binary provides the schema registry service for
//! OpenTelemetry Data Lake Bridge.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use schema_registry::{
    config::StorageBackendType, init_schema_registry, shutdown_schema_registry,
    SchemaRegistryConfig, SCHEMA_REGISTRY_NAME, SCHEMA_REGISTRY_VERSION,
};

#[derive(Parser)]
#[command(name = "schema-registry")]
#[command(about = "OpenTelemetry Schema Registry Server")]
#[command(version = SCHEMA_REGISTRY_VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the schema registry server
    Serve {
        /// Configuration file path
        #[arg(short, long, default_value = "config/schema-registry.toml")]
        config: PathBuf,

        /// API host
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// API port
        #[arg(long, default_value = "8081")]
        port: u16,

        /// Storage backend type
        #[arg(long, default_value = "postgres")]
        storage: Option<String>,

        /// Database URL
        #[arg(long)]
        database_url: Option<String>,
    },

    /// Register a schema
    Register {
        /// Schema file path
        #[arg(short, long)]
        schema: PathBuf,

        /// API endpoint
        #[arg(long, default_value = "http://localhost:8081")]
        endpoint: String,
    },

    /// Get a schema
    Get {
        /// Schema ID
        #[arg(short, long)]
        id: String,

        /// API endpoint
        #[arg(long, default_value = "http://localhost:8081")]
        endpoint: String,
    },

    /// List schemas
    List {
        /// API endpoint
        #[arg(long, default_value = "http://localhost:8081")]
        endpoint: String,
    },

    /// Validate a schema
    Validate {
        /// Schema file path
        #[arg(short, long)]
        schema: PathBuf,

        /// API endpoint
        #[arg(long, default_value = "http://localhost:8081")]
        endpoint: String,
    },

    /// Delete a schema
    Delete {
        /// Schema fingerprint
        #[arg(short, long)]
        fingerprint: String,

        /// API endpoint
        #[arg(long, default_value = "http://localhost:8081")]
        endpoint: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            config,
            host,
            port,
            storage,
            database_url,
        } => {
            // Load configuration
            let mut config = SchemaRegistryConfig::from_file(&config)?;

            // Override configuration with CLI arguments
            config.api.host = host.clone();
            config.api.port = port;

            // Override storage configuration if provided
            if let Some(storage) = storage {
                config.storage.backend = match storage.as_str() {
                    "postgres" => StorageBackendType::Postgres,
                    "sqlite" => StorageBackendType::Sqlite,
                    "redis" => StorageBackendType::Redis,
                    "memory" => StorageBackendType::Memory,
                    _ => {
                        error!("Unsupported storage backend: {}", storage);
                        std::process::exit(1);
                    }
                };
            }

            if let Some(url) = database_url {
                match config.storage.backend {
                    StorageBackendType::Postgres => {
                        config.storage.postgres.url = url;
                    }
                    StorageBackendType::Sqlite => {
                        config.storage.sqlite.database_path = PathBuf::from(url);
                    }
                    _ => {
                        warn!(
                            "Database URL ignored for storage backend: {:?}",
                            config.storage.backend
                        );
                    }
                }
            }

            // Initialize schema registry
            let registry = init_schema_registry(config.clone()).await?;

            // Create API server
            let manager =
                schema_registry::registry::SchemaRegistryManager::new_with_storage(config).await?;
            let api = schema_registry::api::SchemaRegistryApi::new(Arc::new(manager));
            let app = api.create_app();

            // Start server
            let addr = SocketAddr::from_str(&format!("{}:{}", host, port))?;
            let listener = tokio::net::TcpListener::bind(addr).await?;

            info!("Schema Registry server starting on {}", addr);

            let server = axum::serve(listener, app);
            server.await?;

            // Shutdown
            shutdown_schema_registry(registry).await?;
        }

        Commands::Register { schema, endpoint } => {
            // Read schema file
            let content = std::fs::read_to_string(&schema)?;
            let component_schema: schema_registry::types::ComponentSchema =
                serde_yaml::from_str(&content)?;

            // Create HTTP client
            let client = reqwest::Client::new();

            // Parse version string (assuming format like "1.0.0")
            let version_parts: Vec<&str> = component_schema.version.split('.').collect();
            let version = if version_parts.len() >= 3 {
                schema_registry::schema::SchemaVersion::new(
                    version_parts[0].parse()?,
                    version_parts[1].parse()?,
                    version_parts[2].parse()?,
                )
            } else {
                schema_registry::schema::SchemaVersion::new(1, 0, 0)
            };

            // Register schema
            let request = schema_registry::api::RegisterSchemaRequest {
                name: component_schema.name,
                version,
                schema_type: schema_registry::schema::SchemaType::Metric, // Default to Metric
                content: serde_json::to_string(&component_schema.schema)?,
                format: schema_registry::schema::SchemaFormat::Json, // Default to JSON
            };

            let response = client
                .post(&format!("{}/api/v1/schemas", endpoint))
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let result: schema_registry::api::responses::RegisterSchemaResponse = response.json().await?;
                println!(
                    "Schema registered successfully with fingerprint: {}",
                    result.fingerprint
                );
            } else {
                error!("Failed to register schema: {}", response.status());
                std::process::exit(1);
            }
        }

        Commands::Get { id, endpoint } => {
            // Create HTTP client
            let client = reqwest::Client::new();

            // Get schema
            let response = client
                .get(&format!("{}/api/v1/schemas/{}", endpoint, id))
                .send()
                .await?;

            if response.status().is_success() {
                let result: schema_registry::api::responses::GetSchemaResponse = response.json().await?;
                println!("Schema: {}", serde_json::to_string_pretty(&result.schema)?);
            } else {
                error!("Failed to get schema: {}", response.status());
                std::process::exit(1);
            }
        }

        Commands::List { endpoint } => {
            // Create HTTP client
            let client = reqwest::Client::new();

            // List schemas
            let response = client
                .get(&format!("{}/api/v1/schemas", endpoint))
                .send()
                .await?;

            if response.status().is_success() {
                let result: schema_registry::api::responses::ListSchemasResponse = response.json().await?;
                println!(
                    "Schemas: {}",
                    serde_json::to_string_pretty(&result.schemas)?
                );
            } else {
                error!("Failed to list schemas: {}", response.status());
                std::process::exit(1);
            }
        }

        Commands::Validate { schema, endpoint } => {
            // Read schema file
            let content = std::fs::read_to_string(&schema)?;
            let component_schema: schema_registry::types::ComponentSchema =
                serde_yaml::from_str(&content)?;

            // Create HTTP client
            let client = reqwest::Client::new();

            // Validate schema
            let request = schema_registry::api::ValidateDataRequest {
                data: bridge_core::types::TelemetryBatch::new("cli-validation".to_string(), vec![]), // Empty batch for schema validation
            };

            let response = client
                .post(&format!("{}/api/v1/schemas/validate", endpoint))
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let result: schema_registry::api::responses::ValidateDataResponse = response.json().await?;
                if result.valid {
                    println!("✅ Schema validation passed");
                    println!("Status: {}", result.status);
                    println!(
                        "Errors: {}, Warnings: {}",
                        result.error_count, result.warning_count
                    );
                } else {
                    println!("❌ Schema validation failed");
                    println!("Status: {}", result.status);
                    println!(
                        "Errors: {}, Warnings: {}",
                        result.error_count, result.warning_count
                    );
                    if !result.errors.is_empty() {
                        println!("Errors:");
                        for error in result.errors {
                            println!("  - {:?}", error);
                        }
                    }
                    if !result.warnings.is_empty() {
                        println!("Warnings:");
                        for warning in result.warnings {
                            println!("  - {:?}", warning);
                        }
                    }
                    std::process::exit(1);
                }
            } else {
                error!("Failed to validate schema: {}", response.status());
                std::process::exit(1);
            }
        }

        Commands::Delete {
            fingerprint,
            endpoint,
        } => {
            // Create HTTP client
            let client = reqwest::Client::new();

            // Delete schema
            let response = client
                .delete(&format!("{}/api/v1/schemas/{}", endpoint, fingerprint))
                .send()
                .await?;

            if response.status().is_success() {
                let result: schema_registry::api::responses::DeleteSchemaResponse = response.json().await?;
                println!("✅ Schema deleted successfully: {}", result.message);
            } else {
                error!("Failed to delete schema: {}", response.status());
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
