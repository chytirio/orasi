//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Phase 4: Production-Ready Telemetry Ingestion System
//!
//! This example demonstrates the complete production-ready telemetry ingestion
//! system with performance optimization, monitoring, configuration management,
//! and security features.

use bridge_core::{BridgeResult, TelemetryBatch, types::TelemetryRecord, TelemetryType, TelemetryData, MetricData, MetricValue, MetricType};
use ingestion::{
    config::{ConfigurationManager, ConfigurationBuilder, IngestionConfig, SystemConfig, PerformanceConfig, MonitoringConfig, SecurityConfig},
    performance::{PerformanceManager, ConnectionPoolConfig, BatchProcessorConfig},
    monitoring::{MonitoringManager, MetricsConfig, AlertRule, AlertCondition, AlertSeverity},
    security::{SecurityManager, AuthConfig, AuthMethod, JwtConfig, OAuthConfig, SessionConfig, RateLimitingConfig},
    conversion::{OtlpConverter, ArrowConverter, DataValidator},
    receivers::{ReceiverFactory, OtlpHttpReceiver, OtlpHttpReceiverConfig},
    processors::{ProcessorFactory, EnrichmentProcessor, EnrichmentProcessorConfig},
    IngestionSystem,
};
use std::collections::HashMap;
use std::time::Duration;
use chrono::Utc;
use uuid::Uuid;
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("Starting Phase 4: Production-Ready Telemetry Ingestion System");

    // Example 1: Configuration Management
    example_configuration_management().await?;

    // Example 2: Performance Optimization
    example_performance_optimization().await?;

    // Example 3: Monitoring & Observability
    example_monitoring_observability().await?;

    // Example 4: Security & Reliability
    example_security_reliability().await?;

    // Example 5: Complete System Integration
    example_complete_system_integration().await?;

    info!("Phase 4: Production-Ready Telemetry Ingestion System completed successfully");
    Ok(())
}

/// Example 1: Configuration Management
async fn example_configuration_management() -> BridgeResult<()> {
    info!("=== Example 1: Configuration Management ===");

    // Create production configuration using builder pattern
    let config = ConfigurationBuilder::new()
        .system(SystemConfig {
            service_name: "orasi-ingestion-prod".to_string(),
            service_version: "1.0.0".to_string(),
            environment: "production".to_string(),
            log_level: "info".to_string(),
            data_dir: std::path::PathBuf::from("/var/lib/orasi/data"),
            temp_dir: std::path::PathBuf::from("/var/lib/orasi/temp"),
            max_concurrent_operations: 1000,
            graceful_shutdown_timeout: Duration::from_secs(60),
        })
        .performance(PerformanceConfig {
            connection_pool: ingestion::config::ConnectionPoolConfig {
                max_connections: 500,
                max_idle_connections: 100,
                connection_timeout: Duration::from_secs(30),
                idle_timeout: Duration::from_secs(300),
                keep_alive_timeout: Duration::from_secs(60),
            },
            batch_processing: ingestion::config::BatchProcessorConfig {
                max_batch_size: 2000,
                max_batch_bytes: 20 * 1024 * 1024,
                batch_timeout: Duration::from_secs(5),
                max_memory_bytes: 200 * 1024 * 1024,
                backpressure_threshold: 0.8,
            },
            memory_limits: ingestion::config::MemoryLimits {
                max_heap_size: 2 * 1024 * 1024 * 1024,
                max_stack_size: 16 * 1024 * 1024,
                warning_threshold: 0.8,
                critical_threshold: 0.95,
            },
            cpu_limits: ingestion::config::CpuLimits {
                max_cpu_percent: 80.0,
                warning_threshold: 70.0,
                critical_threshold: 90.0,
            },
        })
        .monitoring(MonitoringConfig {
            metrics: MetricsConfig {
                enable_prometheus: true,
                enable_custom_metrics: true,
                collection_interval: Duration::from_secs(15),
                retention_period: Duration::from_secs(3600),
                enable_detailed_metrics: true,
                metrics_port: 9090,
                metrics_path: "/metrics".to_string(),
            },
            health_check: ingestion::config::HealthCheckConfig {
                check_interval: Duration::from_secs(30),
                timeout: Duration::from_secs(5),
                endpoints: vec!["/health".to_string(), "/ready".to_string()],
            },
            alerting: ingestion::config::AlertingConfig {
                rules: vec![
                    ingestion::config::AlertRuleConfig {
                        name: "high_error_rate".to_string(),
                        condition: "error_rate > 0.05".to_string(),
                        severity: "critical".to_string(),
                        message: "Error rate is too high".to_string(),
                        enabled: true,
                    },
                ],
                notification_channels: vec![
                    ingestion::config::NotificationChannel {
                        name: "slack".to_string(),
                        channel_type: "slack".to_string(),
                        config: HashMap::new(),
                    },
                ],
            },
        })
        .security(SecurityConfig {
            authentication: ingestion::config::AuthenticationConfig {
                enabled: true,
                method: "jwt".to_string(),
                jwt_secret: Some("production-jwt-secret".to_string()),
                oauth: None,
            },
            authorization: ingestion::config::AuthorizationConfig {
                enabled: true,
                rbac: ingestion::config::RbacConfig {
                    roles: HashMap::new(),
                    default_role: "user".to_string(),
                },
                permissions: HashMap::new(),
            },
            tls: ingestion::config::TlsConfig {
                enabled: true,
                cert_file: Some(std::path::PathBuf::from("/etc/orasi/certs/cert.pem")),
                key_file: Some(std::path::PathBuf::from("/etc/orasi/certs/key.pem")),
                ca_file: Some(std::path::PathBuf::from("/etc/orasi/certs/ca.pem")),
                version: "1.3".to_string(),
            },
            encryption: ingestion::config::EncryptionConfig {
                enabled: true,
                algorithm: "AES-256-GCM".to_string(),
                key: Some("production-encryption-key".to_string()),
                key_rotation_interval: Duration::from_secs(86400),
            },
        })
        .build();

    info!("Created production configuration with:");
    info!("  - Service: {} v{}", config.system.service_name, config.system.service_version);
    info!("  - Environment: {}", config.system.environment);
    info!("  - Max connections: {}", config.performance.connection_pool.max_connections);
    info!("  - Max batch size: {}", config.performance.batch_processing.max_batch_size);
    info!("  - Authentication: {}", config.security.authentication.enabled);
    info!("  - TLS: {}", config.security.tls.enabled);

    // Create configuration manager with hot reloading
    let config_path = std::path::PathBuf::from("/tmp/orasi-config.json");
    let mut config_manager = ConfigurationManager::new(config_path.clone()).await?;
    
    // Enable hot reloading
    config_manager.enable_hot_reload().await?;
    info!("Configuration hot reloading enabled");

    // Validate configuration
    config_manager.validate_config(&config)?;
    info!("Configuration validation passed");

    Ok(())
}

/// Example 2: Performance Optimization
async fn example_performance_optimization() -> BridgeResult<()> {
    info!("=== Example 2: Performance Optimization ===");

    // Create performance manager
    let pool_config = ConnectionPoolConfig {
        max_connections: 500,
        max_idle_connections: 100,
        connection_timeout: Duration::from_secs(30),
        idle_timeout: Duration::from_secs(300),
        keep_alive_timeout: Duration::from_secs(60),
        max_retries: 3,
        retry_backoff: Duration::from_millis(100),
    };

    let batch_config = BatchProcessorConfig {
        max_batch_size: 2000,
        max_batch_bytes: 20 * 1024 * 1024,
        batch_timeout: Duration::from_secs(5),
        max_memory_bytes: 200 * 1024 * 1024,
        backpressure_threshold: 0.8,
        enable_streaming: true,
        stream_buffer_size: 1000,
        enable_compression: true,
        compression_level: 6,
    };

    let performance_manager = PerformanceManager::new(pool_config, batch_config, 0.8);
    performance_manager.start().await?;
    info!("Performance manager started");

    // Test connection pooling
    let http_pool = performance_manager.connection_pool_manager().http_pool();
    let grpc_pool = performance_manager.connection_pool_manager().grpc_pool();

    info!("Connection pools created:");
    info!("  - HTTP pool: {} max connections", http_pool.config.max_connections);
    info!("  - gRPC pool: {} max connections", grpc_pool.config.max_connections);

    // Test batch processing
    let batch_processor = performance_manager.batch_processor();
    let test_batch = create_test_telemetry_batch(1000);
    
    let start_time = std::time::Instant::now();
    batch_processor.process_batch(test_batch).await?;
    let processing_time = start_time.elapsed();

    info!("Batch processing completed:");
    info!("  - Batch size: 1000 records");
    info!("  - Processing time: {:?}", processing_time);
    info!("  - Throughput: {:.2} records/sec", 1000.0 / processing_time.as_secs_f64());

    // Get performance statistics
    let stats = performance_manager.get_stats().await;
    info!("Performance statistics:");
    info!("  - Connection pools: {} total connections", stats.connection_pools.http.total_connections + stats.connection_pools.grpc.total_connections);
    info!("  - Batch processing: {} batches, {} records", stats.batch_processing.total_batches, stats.batch_processing.total_records);
    info!("  - Backpressure events: {}", stats.backpressure.backpressure_events);

    Ok(())
}

/// Example 3: Monitoring & Observability
async fn example_monitoring_observability() -> BridgeResult<()> {
    info!("=== Example 3: Monitoring & Observability ===");

    // Create monitoring manager
    let metrics_config = MetricsConfig {
        enable_prometheus: true,
        enable_custom_metrics: true,
        collection_interval: Duration::from_secs(15),
        retention_period: Duration::from_secs(3600),
        enable_detailed_metrics: true,
        metrics_port: 9090,
        metrics_path: "/metrics".to_string(),
    };

    let monitoring_manager = MonitoringManager::new(metrics_config)?;
    monitoring_manager.start().await?;
    info!("Monitoring manager started");

    // Add alert rules
    let mut alert_manager = monitoring_manager.alert_manager();
    
    let high_error_rate_rule = AlertRule {
        name: "high_error_rate".to_string(),
        condition: AlertCondition::ErrorRateThreshold(0.05),
        severity: AlertSeverity::Critical,
        message: "Error rate is too high".to_string(),
        enabled: true,
    };

    let high_latency_rule = AlertRule {
        name: "high_latency".to_string(),
        condition: AlertCondition::LatencyThreshold(Duration::from_millis(1000)),
        severity: AlertSeverity::Warning,
        message: "Average latency is too high".to_string(),
        enabled: true,
    };

    let memory_usage_rule = AlertRule {
        name: "high_memory_usage".to_string(),
        condition: AlertCondition::MemoryUsageThreshold(0.9),
        severity: AlertSeverity::Warning,
        message: "Memory usage is too high".to_string(),
        enabled: true,
    };

    alert_manager.add_alert_rule(high_error_rate_rule);
    alert_manager.add_alert_rule(high_latency_rule);
    alert_manager.add_alert_rule(memory_usage_rule);

    info!("Alert rules configured:");
    info!("  - High error rate: >5% (Critical)");
    info!("  - High latency: >1000ms (Warning)");
    info!("  - High memory usage: >90% (Warning)");

    // Simulate some metrics collection
    let metrics_collector = monitoring_manager.metrics_collector();
    
    // Record some metrics
    metrics_collector.record_received(1000).await;
    metrics_collector.record_processed(950, Duration::from_millis(100)).await;
    metrics_collector.record_exported(950).await;
    metrics_collector.record_batch_received().await;
    metrics_collector.record_batch_processed(Duration::from_millis(50)).await;
    metrics_collector.record_batch_exported().await;

    // Record protocol metrics
    metrics_collector.record_protocol_request("otlp-http", true, 1024, Duration::from_millis(10)).await;
    metrics_collector.record_protocol_request("otlp-grpc", true, 2048, Duration::from_millis(5)).await;
    metrics_collector.record_protocol_request("arrow", false, 512, Duration::from_millis(100)).await;

    // Record some errors
    metrics_collector.record_error("validation_error", "converter").await;
    metrics_collector.record_error("network_error", "receiver").await;
    metrics_collector.record_warning().await;

    // Update performance metrics
    metrics_collector.update_performance_metrics(25.5, 1024 * 1024 * 100, 50.0).await;
    metrics_collector.update_queue_size(150).await;

    // Get comprehensive monitoring status
    let status = monitoring_manager.get_status().await;
    info!("Monitoring status:");
    info!("  - Uptime: {} seconds", status.metrics.uptime_seconds);
    info!("  - Total records: {}", status.metrics.ingestion.total_records_processed);
    info!("  - Error rate: {:.2}%", 
        (status.metrics.error.total_errors as f64 / status.metrics.ingestion.total_records_processed as f64) * 100.0);
    info!("  - Health status: {:?}", status.health.overall_status);
    info!("  - Active alerts: {}", status.alerts.len());

    // Check health status
    let health_checker = monitoring_manager.health_checker();
    let health_status = health_checker.get_status().await;
    info!("Health check results:");
    for (component, health) in &health_status.components {
        info!("  - {}: {:?} - {}", component, health.status, health.message);
    }

    Ok(())
}

/// Example 4: Security & Reliability
async fn example_security_reliability() -> BridgeResult<()> {
    info!("=== Example 4: Security & Reliability ===");

    // Create security manager
    let auth_config = AuthConfig {
        method: AuthMethod::Jwt,
        jwt: JwtConfig {
            secret: "production-jwt-secret-key".to_string(),
            algorithm: "HS256".to_string(),
            expiration: Duration::from_secs(3600),
            refresh_expiration: Duration::from_secs(86400),
            issuer: "orasi-ingestion".to_string(),
            audience: "telemetry-clients".to_string(),
        },
        oauth: OAuthConfig {
            providers: HashMap::new(),
            redirect_url: "https://orasi.example.com/oauth/callback".to_string(),
            state_secret: "oauth-state-secret".to_string(),
        },
        session: SessionConfig {
            timeout: Duration::from_secs(3600),
            max_sessions_per_user: 5,
            cleanup_interval: Duration::from_secs(300),
        },
        rate_limiting: RateLimitingConfig {
            window: Duration::from_secs(60),
            max_requests: 1000,
            by_ip: true,
            by_user: true,
        },
    };

    let security_manager = SecurityManager::new(auth_config)?;
    security_manager.initialize().await?;
    info!("Security manager initialized");

    // Test authentication
    let auth_manager = security_manager.auth_manager();

    // Create a test user
    let user = auth_manager.create_user(
        "testuser".to_string(),
        "test@example.com".to_string(),
        "password123".to_string(),
        vec!["admin".to_string(), "operator".to_string()],
    ).await?;

    info!("Created test user: {} with roles: {:?}", user.username, user.roles);

    // Authenticate user
    let auth_result = auth_manager.authenticate_user("testuser", "password123").await?;
    
    if auth_result.success {
        info!("User authentication successful");
        if let Some(token) = auth_result.token {
            info!("JWT token generated: {}...", &token[..20]);
        }
    } else {
        warn!("User authentication failed: {:?}", auth_result.error_message);
    }

    // Test authorization
    if let Some(user) = &auth_result.user {
        let authz_result = auth_manager.authorize(user, "read", "telemetry").await;
        info!("Authorization result for read:telemetry: {:?}", authz_result.allowed);

        let authz_result = auth_manager.authorize(user, "write", "telemetry").await;
        info!("Authorization result for write:telemetry: {:?}", authz_result.allowed);
    }

    // Test encryption
    let encryption_manager = security_manager.encryption_manager();
    let test_data = b"sensitive telemetry data";
    
    let encrypted_data = encryption_manager.encrypt(test_data).await?;
    info!("Data encrypted: {} bytes -> {} bytes", test_data.len(), encrypted_data.len());

    let decrypted_data = encryption_manager.decrypt(&encrypted_data).await?;
    info!("Data decrypted: {} bytes -> {} bytes", encrypted_data.len(), decrypted_data.len());

    assert_eq!(test_data, decrypted_data.as_slice());
    info!("Encryption/decryption test passed");

    // Test circuit breaker
    let circuit_breaker = security_manager.circuit_breaker();
    
    // Simulate successful operations
    for i in 0..3 {
        let result: Result<String, std::io::Error> = circuit_breaker.execute(|| {
            Ok(format!("Operation {}", i))
        }).await;
        
        match result {
            Ok(msg) => info!("Circuit breaker operation {}: {}", i, msg),
            Err(e) => error!("Circuit breaker operation {} failed: {}", i, e),
        }
    }

    // Simulate failing operations to trigger circuit breaker
    for i in 0..6 {
        let result: Result<String, std::io::Error> = circuit_breaker.execute(|| {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Simulated failure"))
        }).await;
        
        match result {
            Ok(msg) => info!("Circuit breaker operation {}: {}", i, msg),
            Err(e) => warn!("Circuit breaker operation {} failed: {}", i, e),
        }
    }

    let circuit_state = circuit_breaker.get_state().await;
    info!("Circuit breaker state: {:?}", circuit_state);

    Ok(())
}

/// Example 5: Complete System Integration
async fn example_complete_system_integration() -> BridgeResult<()> {
    info!("=== Example 5: Complete System Integration ===");

    // Create complete ingestion system with all components
    let mut ingestion_system = IngestionSystem::new();

    // Add OTLP HTTP receiver
    let otlp_config = OtlpHttpReceiverConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        batching: true,
        compression: true,
        authentication: true,
        cors: true,
        request_size_limit: 10 * 1024 * 1024,
    };

    let otlp_receiver = OtlpHttpReceiver::new(&otlp_config).await?;
    ingestion_system.add_receiver("otlp-http".to_string(), Box::new(otlp_receiver));

    // Add enrichment processor
    let enrichment_config = EnrichmentProcessorConfig {
        enable_timestamp_enrichment: true,
        enable_service_enrichment: true,
        enable_environment_enrichment: true,
        enable_host_enrichment: true,
        service_mapping: HashMap::from([
            ("web-service".to_string(), "frontend".to_string()),
            ("api-service".to_string(), "backend".to_string()),
            ("db-service".to_string(), "database".to_string()),
        ]),
        environment_mapping: HashMap::from([
            ("dev".to_string(), "development".to_string()),
            ("staging".to_string(), "staging".to_string()),
            ("prod".to_string(), "production".to_string()),
        ]),
        sampling_rate: 1.0,
        enable_validation: true,
        custom_rules: vec![],
        validation_rules: vec![],
    };

    let enrichment_processor = EnrichmentProcessor::new(&enrichment_config).await?;
    ingestion_system.add_processor("enrichment".to_string(), Box::new(enrichment_processor));

    info!("Ingestion system configured with:");
    info!("  - OTLP HTTP receiver on port 8080");
    info!("  - Enrichment processor with service mapping");

    // Process test batches
    for i in 0..5 {
        let batch = create_test_telemetry_batch(100);
        let start_time = std::time::Instant::now();
        
        ingestion_system.process_batch(batch).await?;
        
        let processing_time = start_time.elapsed();
        info!("Batch {} processed in {:?}", i, processing_time);
        
        // Simulate some delay between batches
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Perform health check
    ingestion_system.health_check().await?;
    info!("System health check completed");

    info!("Complete system integration test passed");
    Ok(())
}

/// Create test telemetry batch
fn create_test_telemetry_batch(size: usize) -> TelemetryBatch {
    let records = (0..size).map(|i| TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(MetricData {
            name: format!("test_metric_{}", i % 10),
            description: Some(format!("Test metric {}", i)),
            unit: Some("count".to_string()),
            metric_type: MetricType::Gauge,
            value: MetricValue::Gauge(i as f64),
            labels: HashMap::from([
                ("service".to_string(), "test-service".to_string()),
                ("instance".to_string(), format!("instance-{}", i % 3)),
                ("version".to_string(), "1.0.0".to_string()),
            ]),
            timestamp: Utc::now(),
        }),
        attributes: HashMap::from([
            ("source".to_string(), "test".to_string()),
            ("environment".to_string(), "development".to_string()),
        ]),
        tags: HashMap::new(),
        resource: None,
        service: Some("test-service".to_string()),
    }).collect();

    TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "test".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    }
}
