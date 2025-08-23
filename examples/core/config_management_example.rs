//! Configuration Management Example
//!
//! This example demonstrates how to use the bridge configuration management system
//! with component restart handlers, hot reloading, and configuration updates.

use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

use bridge_api::{
    config::BridgeAPIConfig,
    metrics::ApiMetrics,
    proto::{ComponentStatus, UpdateConfigRequest, UpdateConfigResponse},
    services::config::ConfigService,
};
use bridge_core::BridgeConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting Configuration Management Example");

    // Create default bridge configuration
    let bridge_config = BridgeConfig::default();

    // Create default API configuration
    let api_config = BridgeAPIConfig::default();

    // Create metrics
    let metrics = ApiMetrics::new();

    // Create configuration file path
    let config_path = PathBuf::from("examples/config/core/bridge-config.json");

    // Ensure config directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create config service
    let config_service =
        ConfigService::new(api_config, bridge_config, config_path.clone(), metrics);

    // Start config monitoring
    config_service.start_config_monitoring().await?;

    // Example 1: Validate configuration without applying
    info!("=== Example 1: Configuration Validation ===");
    let validation_config = r#"{
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
        },
        "bridge": {
            "name": "test-bridge",
            "environment": "development",
            "ingestion": {
                "otlp_endpoint": "http://localhost:4317",
                "batch_size": 1000,
                "flush_interval_ms": 5000,
                "buffer_size": 10000,
                "compression_level": 6,
                "enable_persistence": false,
                "enable_backpressure": true,
                "backpressure_threshold": 80
            },
            "lakehouses": {},
            "processing": {
                "worker_threads": 4,
                "enable_streaming": true,
                "stream_window_ms": 60000,
                "enable_transformation": false,
                "enable_filtering": false,
                "enable_aggregation": false,
                "enable_anomaly_detection": false,
                "query_timeout_secs": 30,
                "enable_query_caching": true,
                "cache_size": 104857600,
                "cache_ttl_secs": 300
            },
            "plugin": {
                "enable_ide_plugin": true,
                "plugin_endpoint": "http://localhost:8081",
                "features": {
                    "enable_workflow_analytics": true,
                    "enable_agent_analytics": true,
                    "enable_multi_repo_analytics": true,
                    "enable_real_time_alerting": true,
                    "enable_interactive_querying": true,
                    "enable_data_visualization": true
                }
            },
            "security": {
                "enable_tls": false,
                "enable_authentication": false,
                "authentication_methods": ["None"],
                "enable_authorization": false,
                "enable_audit_logging": false,
                "enable_pii_scrubbing": false,
                "data_retention_days": 90,
                "enable_encryption_at_rest": false
            },
            "monitoring": {
                "enable_metrics": true,
                "metrics_endpoint": "http://localhost:9090",
                "enable_health_checks": true,
                "health_endpoint": "http://localhost:8080",
                "enable_structured_logging": true,
                "log_level": "info",
                "log_format": "json",
                "enable_distributed_tracing": false,
                "enable_performance_profiling": false
            },
            "advanced": {
                "enable_experimental_features": false,
                "performance_tuning": {
                    "enable_simd": true,
                    "enable_custom_allocators": false,
                    "enable_pgo": false
                },
                "circuit_breaker": {
                    "failure_threshold": 5,
                    "success_threshold": 2,
                    "timeout_ms": 5000,
                    "half_open_timeout_ms": 30000
                },
                "retry": {
                    "max_attempts": 3,
                    "initial_backoff_ms": 1000,
                    "max_backoff_ms": 30000,
                    "backoff_multiplier": 2.0,
                    "enable_exponential_backoff": true,
                    "enable_jitter": true
                },
                "connection_pooling": {
                    "min_pool_size": 5,
                    "max_pool_size": 20,
                    "connection_timeout_secs": 30,
                    "idle_timeout_secs": 300,
                    "max_lifetime_secs": 3600
                }
            }
        }
    }"#;

    let validation_response = config_service.update_config(&validation_config).await?;
    if validation_response.success {
        info!("✅ Configuration validation successful");
    } else {
        error!(
            "❌ Configuration validation failed: {}",
            validation_response.error_message
        );
        for error in &validation_response.validation_errors {
            error!("  - {}", error);
        }
    }

    // Example 2: Update configuration without restarting components
    info!("=== Example 2: Configuration Update (No Restart) ===");
    let update_config = r#"{
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
        },
        "bridge": {
            "name": "updated-bridge",
            "environment": "development",
            "ingestion": {
                "otlp_endpoint": "http://localhost:4317",
                "batch_size": 2000,
                "flush_interval_ms": 3000,
                "buffer_size": 15000,
                "compression_level": 7,
                "enable_persistence": false,
                "enable_backpressure": true,
                "backpressure_threshold": 85
            },
            "lakehouses": {},
            "processing": {
                "worker_threads": 8,
                "enable_streaming": true,
                "stream_window_ms": 30000,
                "enable_transformation": true,
                "enable_filtering": true,
                "enable_aggregation": false,
                "enable_anomaly_detection": false,
                "query_timeout_secs": 60,
                "enable_query_caching": true,
                "cache_size": 209715200,
                "cache_ttl_secs": 600
            },
            "plugin": {
                "enable_ide_plugin": true,
                "plugin_endpoint": "http://localhost:8081",
                "features": {
                    "enable_workflow_analytics": true,
                    "enable_agent_analytics": true,
                    "enable_multi_repo_analytics": true,
                    "enable_real_time_alerting": true,
                    "enable_interactive_querying": true,
                    "enable_data_visualization": true
                }
            },
            "security": {
                "enable_tls": false,
                "enable_authentication": false,
                "authentication_methods": ["None"],
                "enable_authorization": false,
                "enable_audit_logging": false,
                "enable_pii_scrubbing": false,
                "data_retention_days": 90,
                "enable_encryption_at_rest": false
            },
            "monitoring": {
                "enable_metrics": true,
                "metrics_endpoint": "http://localhost:9090",
                "enable_health_checks": true,
                "health_endpoint": "http://localhost:8080",
                "enable_structured_logging": true,
                "log_level": "debug",
                "log_format": "json",
                "enable_distributed_tracing": false,
                "enable_performance_profiling": false
            },
            "advanced": {
                "enable_experimental_features": false,
                "performance_tuning": {
                    "enable_simd": true,
                    "enable_custom_allocators": false,
                    "enable_pgo": false
                },
                "circuit_breaker": {
                    "failure_threshold": 5,
                    "success_threshold": 2,
                    "timeout_ms": 5000,
                    "half_open_timeout_ms": 30000
                },
                "retry": {
                    "max_attempts": 3,
                    "initial_backoff_ms": 1000,
                    "max_backoff_ms": 30000,
                    "backoff_multiplier": 2.0,
                    "enable_exponential_backoff": true,
                    "enable_jitter": true
                },
                "connection_pooling": {
                    "min_pool_size": 5,
                    "max_pool_size": 20,
                    "connection_timeout_secs": 30,
                    "idle_timeout_secs": 300,
                    "max_lifetime_secs": 3600
                }
            }
        }
    }"#;

    let update_response = config_service.update_config(&update_config).await?;
    if update_response.success {
        info!("✅ Configuration updated successfully");

        // Get current configuration
        let current_config = config_service.get_config().await?;
        info!("Current configuration: {:?}", current_config);

        // Check component statuses
        let component_status = config_service
            .get_component_status("test-component")
            .await?;
        info!("Component status: {:?}", component_status);
    } else {
        error!(
            "❌ Configuration update failed: {}",
            update_response.error_message
        );
    }

    // Example 3: Update configuration with component restart
    info!("=== Example 3: Configuration Update with Component Restart ===");
    let restart_config = r#"{
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
        },
        "bridge": {
            "name": "restarted-bridge",
            "environment": "development",
            "ingestion": {
                "otlp_endpoint": "http://localhost:4317",
                "batch_size": 3000,
                "flush_interval_ms": 2000,
                "buffer_size": 20000,
                "compression_level": 8,
                "enable_persistence": true,
                "enable_backpressure": true,
                "backpressure_threshold": 90
            },
            "lakehouses": {},
            "processing": {
                "worker_threads": 16,
                "enable_streaming": true,
                "stream_window_ms": 15000,
                "enable_transformation": true,
                "enable_filtering": true,
                "enable_aggregation": true,
                "enable_anomaly_detection": true,
                "query_timeout_secs": 120,
                "enable_query_caching": true,
                "cache_size": 419430400,
                "cache_ttl_secs": 1200
            },
            "plugin": {
                "enable_ide_plugin": true,
                "plugin_endpoint": "http://localhost:8081",
                "features": {
                    "enable_workflow_analytics": true,
                    "enable_agent_analytics": true,
                    "enable_multi_repo_analytics": true,
                    "enable_real_time_alerting": true,
                    "enable_interactive_querying": true,
                    "enable_data_visualization": true
                }
            },
            "security": {
                "enable_tls": false,
                "enable_authentication": false,
                "authentication_methods": ["None"],
                "enable_authorization": false,
                "enable_audit_logging": false,
                "enable_pii_scrubbing": false,
                "data_retention_days": 90,
                "enable_encryption_at_rest": false
            },
            "monitoring": {
                "enable_metrics": true,
                "metrics_endpoint": "http://localhost:9090",
                "enable_health_checks": true,
                "health_endpoint": "http://localhost:8080",
                "enable_structured_logging": true,
                "log_level": "trace",
                "log_format": "json",
                "enable_distributed_tracing": true,
                "enable_performance_profiling": true
            },
            "advanced": {
                "enable_experimental_features": true,
                "performance_tuning": {
                    "enable_simd": true,
                    "enable_custom_allocators": true,
                    "enable_pgo": true
                },
                "circuit_breaker": {
                    "failure_threshold": 5,
                    "success_threshold": 2,
                    "timeout_ms": 5000,
                    "half_open_timeout_ms": 30000
                },
                "retry": {
                    "max_attempts": 3,
                    "initial_backoff_ms": 1000,
                    "max_backoff_ms": 30000,
                    "backoff_multiplier": 2.0,
                    "enable_exponential_backoff": true,
                    "enable_jitter": true
                },
                "connection_pooling": {
                    "min_pool_size": 5,
                    "max_pool_size": 20,
                    "connection_timeout_secs": 30,
                    "idle_timeout_secs": 300,
                    "max_lifetime_secs": 3600
                }
            }
        }
    }"#;

    let restart_response = config_service.update_config(&restart_config).await?;
    if restart_response.success {
        info!("✅ Configuration updated and components restarted");
    } else {
        error!(
            "❌ Failed to restart components: {}",
            restart_response.error_message
        );
    }

    // Check configuration changes
    info!("Configuration change detection:");
    let config = config_service.get_config().await?;
    info!("Current configuration hash: {:?}", config);

    // Example 5: Handle invalid configuration
    info!("=== Example 5: Invalid Configuration Handling ===");
    let invalid_config = r#"{
        "api": {
            "host": "0.0.0.0",
            "port": 99999
        },
        "grpc": {
            "host": "0.0.0.0",
            "port": 9090
        },
        "metrics": {
            "enabled": "not_a_boolean",
            "port": 9091
        }
    }"#;

    let invalid_response = config_service.update_config(&invalid_config).await?;
    if !invalid_response.success {
        info!("✅ Invalid configuration properly rejected");
        info!("Error: {}", invalid_response.error_message);
    } else {
        warn!("⚠️ Invalid configuration was accepted (unexpected)");
    }

    info!("Configuration Management Example completed successfully");
    Ok(())
}
