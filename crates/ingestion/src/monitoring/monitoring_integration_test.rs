//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Comprehensive integration tests for the monitoring system
//!
//! This module tests the integration of all monitoring components including
//! metrics, health checks, structured logging, and distributed tracing.

use super::*;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

/// Test monitoring system integration
#[tokio::test]
async fn test_monitoring_system_integration() {
    // Create monitoring configuration
    let config = MonitoringConfig::development();
    let manager = MonitoringManager::new(config);

    // Initialize monitoring
    manager.initialize().await.unwrap();

    // Test metrics recording
    test_metrics_integration(&manager).await;

    // Test health checks integration
    test_health_checks_integration(&manager).await;

    // Test structured logging integration
    test_structured_logging_integration(&manager).await;

    // Test distributed tracing integration
    test_distributed_tracing_integration(&manager).await;

    // Test comprehensive monitoring workflow
    test_comprehensive_monitoring_workflow(&manager).await;

    // Verify monitoring is healthy
    assert!(manager.is_healthy().await);

    // Get comprehensive statistics
    let stats = manager.get_statistics().await;
    assert!(stats.is_initialized);
    assert_eq!(stats.service_info.name, "ingestion-service");
    assert_eq!(stats.service_info.environment, "development");

    // Shutdown monitoring
    manager.shutdown().await.unwrap();
}

/// Test metrics integration
async fn test_metrics_integration(manager: &MonitoringManager) {
    // Record various metrics
    manager
        .record_ingestion_metrics("otlp", 100, 50, 150, true)
        .await
        .unwrap();

    manager
        .record_ingestion_metrics("kafka", 200, 100, 300, false)
        .await
        .unwrap();

    manager
        .record_performance_metrics("batch_processing", 200, true)
        .await
        .unwrap();

    manager
        .record_error_metrics("network", "Connection failed", "kafka")
        .await
        .unwrap();

    // Verify metrics are recorded
    let metrics_stats = manager.metrics().get_statistics().await;
    assert!(metrics_stats.total_metrics > 0);
    assert!(metrics_stats.ingestion_metrics.total_records > 0);
    assert!(metrics_stats.error_metrics.total_errors > 0);
}

/// Test health checks integration
async fn test_health_checks_integration(manager: &MonitoringManager) {
    // Add health checks
    let memory_checker = MemoryUsageHealthChecker::new("memory_check".to_string(), 90.0);
    manager
        .health_checks()
        .add_health_check(Box::new(memory_checker))
        .await
        .unwrap();

    // Create HTTP endpoint health check
    let endpoint = health_checks::HealthCheckEndpoint {
        name: "test_endpoint".to_string(),
        url: "http://localhost:8080/health".to_string(),
        expected_status: 200,
        timeout_secs: 5,
        auth: None,
        headers: HashMap::new(),
        required: false,
    };

    let http_checker = HttpEndpointHealthChecker::new("http_check".to_string(), endpoint);
    manager
        .health_checks()
        .add_health_check(Box::new(http_checker))
        .await
        .unwrap();

    // Wait for health checks to run
    sleep(Duration::from_secs(2)).await;

    // Verify health check results
    let health_stats = manager.health_checks().get_statistics().await;
    assert!(health_stats.total_checks > 0);

    let results = manager.health_checks().get_results().await;
    assert!(!results.is_empty());
}

/// Test structured logging integration
async fn test_structured_logging_integration(manager: &MonitoringManager) {
    // Test different log levels
    let mut fields = HashMap::new();
    fields.insert("test_field".to_string(), serde_json::json!("test_value"));
    fields.insert("numeric_field".to_string(), serde_json::json!(42));

    manager
        .logging()
        .info("Test info message", fields.clone())
        .await
        .unwrap();

    manager
        .logging()
        .warn("Test warning message", fields.clone())
        .await
        .unwrap();

    manager
        .logging()
        .error("Test error message", fields.clone())
        .await
        .unwrap();

    manager
        .logging()
        .debug("Test debug message", fields.clone())
        .await
        .unwrap();

    // Add custom field
    manager
        .logging()
        .add_custom_field("custom_key".to_string(), serde_json::json!("custom_value"))
        .await
        .unwrap();

    // Verify logging statistics
    let logging_stats = manager.logging().get_statistics().await;
    assert!(logging_stats.is_initialized);
}

/// Test distributed tracing integration
async fn test_distributed_tracing_integration(manager: &MonitoringManager) {
    // Start a trace span
    let mut attributes = HashMap::new();
    attributes.insert("operation".to_string(), serde_json::json!("test_operation"));
    attributes.insert("component".to_string(), serde_json::json!("test_component"));

    let span_id = manager
        .tracing()
        .start_span("test_span", attributes)
        .await
        .unwrap();

    // Add span attributes
    manager
        .tracing()
        .add_span_attribute(&span_id, "test_attr", serde_json::json!("test_value"))
        .await
        .unwrap();

    // Add span event
    let mut event_attributes = HashMap::new();
    event_attributes.insert("event_type".to_string(), serde_json::json!("test_event"));
    manager
        .tracing()
        .add_span_event(&span_id, "test_event", event_attributes)
        .await
        .unwrap();

    // End span successfully
    manager
        .tracing()
        .end_span(&span_id)
        .await
        .unwrap();

    // Test span with error
    let error_span_id = manager
        .tracing()
        .start_span("error_span", HashMap::new())
        .await
        .unwrap();

    let status = SpanStatus {
        code: distributed_tracing::SpanStatusCode::Error,
        message: Some("Test error".to_string()),
    };

    manager
        .tracing()
        .set_span_status(&error_span_id, status)
        .await
        .unwrap();

    manager
        .tracing()
        .end_span(&error_span_id)
        .await
        .unwrap();

    // Verify tracing statistics
    let tracing_stats = manager.tracing().get_statistics().await;
    assert!(tracing_stats.is_initialized);
}

/// Test comprehensive monitoring workflow
async fn test_comprehensive_monitoring_workflow(manager: &MonitoringManager) {
    // Simulate a complete ingestion workflow with monitoring

    // 1. Start ingestion span
    let span_id = manager
        .start_ingestion_span("otlp", 100)
        .await
        .unwrap();

    // 2. Record ingestion metrics
    manager
        .record_ingestion_metrics("otlp", 100, 50, 150, true)
        .await
        .unwrap();

    // 3. Record performance metrics
    manager
        .record_performance_metrics("data_processing", 75, true)
        .await
        .unwrap();

    // 4. Log structured message
    let mut fields = HashMap::new();
    fields.insert("batch_id".to_string(), serde_json::json!("batch_123"));
    fields.insert("protocol".to_string(), serde_json::json!("otlp"));
    fields.insert("record_count".to_string(), serde_json::json!(100));

    manager
        .logging()
        .info("Batch processed successfully", fields)
        .await
        .unwrap();

    // 5. End span successfully
    manager
        .end_ingestion_span(&span_id, true, None)
        .await
        .unwrap();

    // 6. Test error scenario
    let error_span_id = manager
        .start_ingestion_span("kafka", 50)
        .await
        .unwrap();

    // Record error
    manager
        .record_error_metrics("network", "Kafka connection failed", "kafka")
        .await
        .unwrap();

    // Log error
    let mut error_fields = HashMap::new();
    error_fields.insert("error_type".to_string(), serde_json::json!("network"));
    error_fields.insert("component".to_string(), serde_json::json!("kafka"));

    manager
        .logging()
        .error("Kafka connection failed", error_fields)
        .await
        .unwrap();

    // End span with error
    manager
        .end_ingestion_span(&error_span_id, false, Some("Kafka connection failed"))
        .await
        .unwrap();

    // Verify comprehensive statistics
    let stats = manager.get_statistics().await;
    assert!(stats.is_initialized);
    assert!(stats.metrics.total_metrics > 0);
    assert!(stats.health_checks.total_checks > 0);
    assert!(stats.structured_logging.is_initialized);
    assert!(stats.distributed_tracing.is_initialized);
}

/// Test monitoring trait implementation
#[tokio::test]
async fn test_monitorable_trait() {
    // Create a test struct that implements Monitorable
    struct TestComponent {
        monitoring: Arc<MonitoringManager>,
    }

    impl Monitorable for TestComponent {
        fn monitoring(&self) -> Arc<MonitoringManager> {
            Arc::clone(&self.monitoring)
        }
    }

    let config = MonitoringConfig::development();
    let monitoring_manager = Arc::new(MonitoringManager::new(config));
    monitoring_manager.initialize().await.unwrap();

    let component = TestComponent {
        monitoring: Arc::clone(&monitoring_manager),
    };

    // Test trait methods
    component
        .record_operation_metrics("test_operation", 100, true)
        .await
        .unwrap();

    component
        .record_error("test_error", "Test error message", "test_component")
        .await
        .unwrap();

    let span_id = component
        .start_span("test_span", HashMap::new())
        .await
        .unwrap();

    component
        .end_span(&span_id, true, None)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("test_field".to_string(), serde_json::json!("test_value"));
    component
        .log(LogLevel::Info, "Test message", fields)
        .await
        .unwrap();

    assert!(component.is_healthy().await);

    monitoring_manager.shutdown().await.unwrap();
}

/// Test monitoring configuration variants
#[tokio::test]
async fn test_monitoring_config_variants() {
    // Test production configuration
    let production_config = MonitoringConfig::production();
    assert_eq!(production_config.service_info.environment, "production");
    assert!(production_config.structured_logging.json_format);
    assert!(!production_config.structured_logging.console_output);

    // Test development configuration
    let development_config = MonitoringConfig::development();
    assert_eq!(development_config.service_info.environment, "development");
    assert!(!development_config.structured_logging.json_format);
    assert!(development_config.structured_logging.console_output);

    // Test custom service info
    let mut custom_tags = HashMap::new();
    custom_tags.insert("team".to_string(), "platform".to_string());
    custom_tags.insert("project".to_string(), "orasi".to_string());

    let service_info = ServiceInfo {
        name: "custom-service".to_string(),
        version: "2.0.0".to_string(),
        environment: "staging".to_string(),
        instance_id: "instance-123".to_string(),
        tags: custom_tags,
    };

    let custom_config = MonitoringConfig::with_service_info(service_info);
    assert_eq!(custom_config.service_info.name, "custom-service");
    assert_eq!(custom_config.service_info.version, "2.0.0");
    assert_eq!(custom_config.service_info.environment, "staging");
    assert_eq!(custom_config.service_info.instance_id, "instance-123");
    assert_eq!(custom_config.service_info.tags.len(), 2);
}

/// Test monitoring error handling
#[tokio::test]
async fn test_monitoring_error_handling() {
    let config = MonitoringConfig::development();
    let manager = MonitoringManager::new(config);

    // Test with monitoring disabled
    let disabled_config = {
        let mut config = MonitoringConfig::new();
        config.enabled = false;
        config
    };
    let disabled_manager = MonitoringManager::new(disabled_config);
    disabled_manager.initialize().await.unwrap();
    assert!(disabled_manager.is_healthy().await);

    // Test error recording
    manager
        .record_error_metrics("test_error", "Test error message", "test_component")
        .await
        .unwrap();

    // Test span with error
    let span_id = manager
        .tracing()
        .start_span("error_span", HashMap::new())
        .await
        .unwrap();

    manager
        .end_ingestion_span(&span_id, false, Some("Test error occurred"))
        .await
        .unwrap();

    // Verify error metrics are recorded
    let stats = manager.get_statistics().await;
    assert!(stats.metrics.error_metrics.total_errors > 0);
}

/// Test monitoring performance
#[tokio::test]
async fn test_monitoring_performance() {
    let config = MonitoringConfig::development();
    let manager = MonitoringManager::new(config);
    manager.initialize().await.unwrap();

    let start_time = std::time::Instant::now();

    // Perform many monitoring operations
    for i in 0..100 {
        let span_id = manager
            .start_ingestion_span("otlp", i)
            .await
            .unwrap();

        manager
            .record_ingestion_metrics("otlp", i, i / 2, i * 2, i % 2 == 0)
            .await
            .unwrap();

        manager
            .record_performance_metrics("test_operation", i, i % 2 == 0)
            .await
            .unwrap();

        let mut fields = HashMap::new();
        fields.insert("iteration".to_string(), serde_json::json!(i));
        manager
            .logging()
            .info("Performance test iteration", fields)
            .await
            .unwrap();

        manager
            .end_ingestion_span(&span_id, i % 2 == 0, None)
            .await
            .unwrap();
    }

    let duration = start_time.elapsed();
    println!("Performed 100 monitoring operations in {:?}", duration);

    // Verify all operations were recorded
    let stats = manager.get_statistics().await;
    assert!(stats.metrics.total_metrics > 0);
    assert!(stats.structured_logging.is_initialized);
    assert!(stats.distributed_tracing.is_initialized);

    manager.shutdown().await.unwrap();
}
