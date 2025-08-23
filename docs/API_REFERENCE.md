# Orasi Ingestion API Reference

## Overview

The Orasi Ingestion API provides comprehensive telemetry data ingestion capabilities supporting multiple protocols and data formats. This document provides detailed API reference for all components.

## Table of Contents

1. [Core Types](#core-types)
2. [Ingestion System](#ingestion-system)
3. [Protocols](#protocols)
4. [Processors](#processors)
5. [Exporters](#exporters)
6. [Monitoring](#monitoring)
7. [Security](#security)
8. [Configuration](#configuration)

## Core Types

### TelemetryRecord

Represents a single telemetry data point.

```rust
pub struct TelemetryRecord {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub record_type: TelemetryType,
    pub data: TelemetryData,
    pub attributes: HashMap<String, String>,
    pub tags: HashMap<String, String>,
    pub resource: Option<Resource>,
    pub service: Option<Service>,
}
```

**Fields:**
- `id`: Unique identifier for the record
- `timestamp`: When the telemetry data was collected
- `record_type`: Type of telemetry data (Metric, Log, Trace)
- `data`: The actual telemetry data
- `attributes`: Key-value pairs for additional metadata
- `tags`: Labels for categorization
- `resource`: Resource information (optional)
- `service`: Service information (optional)

### TelemetryBatch

Represents a batch of telemetry records.

```rust
pub struct TelemetryBatch {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub size: usize,
    pub records: Vec<TelemetryRecord>,
    pub metadata: HashMap<String, String>,
}
```

**Fields:**
- `id`: Unique identifier for the batch
- `timestamp`: When the batch was created
- `source`: Source identifier
- `size`: Number of records in the batch
- `records`: Vector of telemetry records
- `metadata`: Additional batch metadata

## Ingestion System

### IngestionSystem

Main entry point for the ingestion system.

```rust
pub struct IngestionSystem {
    receivers: HashMap<String, Box<dyn TelemetryReceiver>>,
    processors: HashMap<String, Box<dyn TelemetryProcessor>>,
    exporters: HashMap<String, Box<dyn LakehouseExporter>>,
    monitoring: Arc<MonitoringManager>,
}
```

#### Methods

##### `new() -> Self`
Creates a new ingestion system with default monitoring configuration.

```rust
let system = IngestionSystem::new();
```

##### `with_monitoring(monitoring: Arc<MonitoringManager>) -> Self`
Creates a new ingestion system with custom monitoring.

```rust
let monitoring_config = MonitoringConfig::production();
let monitoring = Arc::new(MonitoringManager::new(monitoring_config));
let system = IngestionSystem::with_monitoring(monitoring);
```

##### `add_receiver(name: String, receiver: Box<dyn TelemetryReceiver>)`
Adds a receiver to the system.

```rust
let receiver = OtlpReceiver::new(config);
system.add_receiver("otlp".to_string(), Box::new(receiver));
```

##### `add_processor(name: String, processor: Box<dyn TelemetryProcessor>)`
Adds a processor to the system.

```rust
let processor = BatchProcessor::new(config);
system.add_processor("batch".to_string(), Box::new(processor));
```

##### `add_exporter(name: String, exporter: Box<dyn LakehouseExporter>)`
Adds an exporter to the system.

```rust
let exporter = BatchExporter::new(config);
system.add_exporter("lakehouse".to_string(), Box::new(exporter));
```

##### `process_batch(batch: TelemetryBatch) -> BridgeResult<()>`
Processes a telemetry batch through the pipeline.

```rust
let batch = create_telemetry_batch();
system.process_batch(batch).await?;
```

##### `initialize() -> BridgeResult<()>`
Initializes the ingestion system.

```rust
system.initialize().await?;
```

##### `health_check() -> BridgeResult<()>`
Performs health checks on all components.

```rust
system.health_check().await?;
```

##### `get_stats() -> BridgeResult<()>`
Gets statistics from all components.

```rust
system.get_stats().await?;
```

## Protocols

### OTLP Protocol

#### OtlpReceiver

Receives OpenTelemetry data over OTLP protocol.

```rust
pub struct OtlpReceiver {
    config: OtlpReceiverConfig,
    server: Option<Server>,
}
```

##### Configuration

```rust
pub struct OtlpReceiverConfig {
    pub host: String,
    pub port: u16,
    pub protocol: OtlpProtocol,
    pub tls_config: Option<TlsConfig>,
    pub auth_config: Option<AuthConfig>,
    pub max_concurrent_requests: usize,
    pub request_timeout: Duration,
}
```

##### Usage

```rust
let config = OtlpReceiverConfig::new_arrow("0.0.0.0".to_string(), 4318);
let receiver = OtlpReceiver::new(config);
```

#### OtlpProtocol

Supported OTLP protocols.

```rust
pub enum OtlpProtocol {
    Arrow,
    Grpc,
    Http,
}
```

### Kafka Protocol

#### KafkaReceiver

Receives telemetry data from Kafka topics.

```rust
pub struct KafkaReceiver {
    config: KafkaReceiverConfig,
    consumer: Option<StreamConsumer>,
}
```

##### Configuration

```rust
pub struct KafkaReceiverConfig {
    pub bootstrap_servers: Vec<String>,
    pub topic: String,
    pub group_id: String,
    pub auto_offset_reset: AutoOffsetReset,
    pub enable_auto_commit: bool,
    pub session_timeout_ms: i32,
    pub heartbeat_interval_ms: i32,
    pub max_poll_records: i32,
}
```

##### Usage

```rust
let config = KafkaReceiverConfig::new(
    vec!["localhost:9092".to_string()],
    "telemetry-data".to_string(),
    "ingestion-group".to_string(),
);
let receiver = KafkaReceiver::new(config);
```

### OTAP Protocol

#### OtapReceiver

Receives OpenTelemetry data in Arrow format.

```rust
pub struct OtapReceiver {
    config: OtapReceiverConfig,
    server: Option<OtapServer>,
}
```

##### Configuration

```rust
pub struct OtapReceiverConfig {
    pub host: String,
    pub port: u16,
    pub max_message_size: usize,
    pub enable_compression: bool,
    pub auth_config: Option<AuthConfig>,
}
```

## Processors

### BatchProcessor

Batches telemetry records for efficient processing.

```rust
pub struct BatchProcessor {
    config: BatchProcessorConfig,
    buffer: Vec<TelemetryRecord>,
}
```

##### Configuration

```rust
pub struct BatchProcessorConfig {
    pub max_batch_size: usize,
    pub max_wait_time: Duration,
    pub enable_compression: bool,
}
```

##### Usage

```rust
let config = BatchProcessorConfig::new(1000, Duration::from_secs(5));
let processor = BatchProcessor::new(config);
```

### FilterProcessor

Filters telemetry records based on configurable rules.

```rust
pub struct FilterProcessor {
    config: FilterProcessorConfig,
    rules: Vec<FilterRule>,
}
```

##### Configuration

```rust
pub struct FilterProcessorConfig {
    pub rules: Vec<FilterRule>,
    pub default_action: FilterAction,
    pub enable_metrics: bool,
}
```

##### Filter Rules

```rust
pub struct FilterRule {
    pub name: String,
    pub condition: FilterCondition,
    pub action: FilterAction,
    pub priority: u32,
}
```

##### Usage

```rust
let rule = FilterRule::new(
    "drop_errors".to_string(),
    FilterCondition::FieldEquals("level".to_string(), "error".to_string()),
    FilterAction::Drop,
    1,
);

let config = FilterProcessorConfig::new(vec![rule], FilterAction::Accept);
let processor = FilterProcessor::new(config);
```

### TransformProcessor

Transforms and enriches telemetry records.

```rust
pub struct TransformProcessor {
    config: TransformProcessorConfig,
    transformations: Vec<Transformation>,
}
```

##### Configuration

```rust
pub struct TransformProcessorConfig {
    pub transformations: Vec<Transformation>,
    pub enable_validation: bool,
    pub strict_mode: bool,
}
```

##### Transformations

```rust
pub struct Transformation {
    pub name: String,
    pub field_path: String,
    pub operation: TransformOperation,
    pub parameters: HashMap<String, String>,
}
```

##### Usage

```rust
let transformation = Transformation::new(
    "add_timestamp".to_string(),
    "timestamp".to_string(),
    TransformOperation::SetValue("{{now}}".to_string()),
    HashMap::new(),
);

let config = TransformProcessorConfig::new(vec![transformation], true);
let processor = TransformProcessor::new(config);
```

### AggregateProcessor

Aggregates telemetry records using various functions.

```rust
pub struct AggregateProcessor {
    config: AggregateProcessorConfig,
    state: HashMap<String, AggregationState>,
}
```

##### Configuration

```rust
pub struct AggregateProcessorConfig {
    pub rules: Vec<AggregationRule>,
    pub window_config: WindowConfig,
    pub enable_persistence: bool,
}
```

##### Usage

```rust
let rule = AggregationRule::new(
    "count_requests".to_string(),
    "request_count".to_string(),
    AggregationFunction::Count,
    WindowConfig::Tumbling(Duration::from_secs(60)),
);

let config = AggregateProcessorConfig::new(vec![rule], WindowConfig::Tumbling(Duration::from_secs(60)));
let processor = AggregateProcessor::new(config);
```

## Exporters

### BatchExporter

Exports telemetry batches to various destinations.

```rust
pub struct BatchExporter {
    config: BatchExporterConfig,
    client: Option<HttpClient>,
}
```

##### Configuration

```rust
pub struct BatchExporterConfig {
    pub endpoint: String,
    pub batch_size: usize,
    pub retry_config: RetryConfig,
    pub auth_config: Option<AuthConfig>,
}
```

##### Usage

```rust
let config = BatchExporterConfig::new(
    "http://localhost:8080/export".to_string(),
    1000,
    RetryConfig::default(),
);
let exporter = BatchExporter::new(config);
```

## Monitoring

### MonitoringManager

Manages all monitoring components.

```rust
pub struct MonitoringManager {
    config: MonitoringConfig,
    metrics_manager: Arc<MetricsManager>,
    health_check_manager: Arc<HealthCheckManager>,
    structured_logging_manager: Arc<StructuredLoggingManager>,
    distributed_tracing_manager: Arc<DistributedTracingManager>,
}
```

#### Methods

##### `initialize() -> BridgeResult<()>`
Initializes all monitoring components.

```rust
let monitoring = MonitoringManager::new(config);
monitoring.initialize().await?;
```

##### `record_ingestion_metrics(protocol, record_count, batch_size, processing_time_ms, success) -> BridgeResult<()>`
Records ingestion metrics.

```rust
monitoring.record_ingestion_metrics("otlp", 100, 50, 150, true).await?;
```

##### `record_error_metrics(error_type, error_message, component) -> BridgeResult<()>`
Records error metrics.

```rust
monitoring.record_error_metrics("network", "Connection failed", "kafka").await?;
```

##### `start_ingestion_span(protocol, batch_size) -> BridgeResult<String>`
Starts a distributed tracing span.

```rust
let span_id = monitoring.start_ingestion_span("otlp", 100).await?;
```

##### `end_ingestion_span(span_id, success, error_message) -> BridgeResult<()>`
Ends a distributed tracing span.

```rust
monitoring.end_ingestion_span(&span_id, true, None).await?;
```

### MetricsManager

Manages Prometheus metrics collection.

```rust
pub struct MetricsManager {
    config: MetricsConfig,
    registry: Registry,
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
}
```

#### Methods

##### `record_ingestion_metrics(protocol, record_count, batch_size, processing_time_ms, success) -> BridgeResult<()>`
Records ingestion-specific metrics.

##### `record_error_metrics(error_type, component) -> BridgeResult<()>`
Records error-specific metrics.

##### `record_performance_metrics(operation, duration_ms, success) -> BridgeResult<()>`
Records performance metrics.

### HealthCheckManager

Manages health checks for all components.

```rust
pub struct HealthCheckManager {
    config: HealthCheckConfig,
    health_checks: Arc<RwLock<HashMap<String, Box<dyn HealthChecker>>>>,
}
```

#### Methods

##### `add_health_check(health_check: Box<dyn HealthChecker>) -> BridgeResult<()>`
Adds a health check.

```rust
let memory_checker = MemoryUsageHealthChecker::new("memory".to_string(), 90.0);
manager.add_health_check(Box::new(memory_checker)).await?;
```

##### `get_overall_health() -> HealthStatus`
Gets the overall health status.

```rust
let health = manager.get_overall_health().await;
```

### StructuredLoggingManager

Manages structured logging.

```rust
pub struct StructuredLoggingManager {
    config: StructuredLoggingConfig,
    custom_fields: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}
```

#### Methods

##### `log(level, message, fields) -> BridgeResult<()>`
Logs a structured message.

```rust
let mut fields = HashMap::new();
fields.insert("user_id".to_string(), serde_json::json!("123"));
manager.log(LogLevel::Info, "User logged in", fields).await?;
```

##### `add_custom_field(key, value) -> BridgeResult<()>`
Adds a custom field to all log messages.

```rust
manager.add_custom_field("service".to_string(), serde_json::json!("ingestion")).await?;
```

### DistributedTracingManager

Manages distributed tracing.

```rust
pub struct DistributedTracingManager {
    config: DistributedTracingConfig,
    tracer: Arc<opentelemetry::trace::Tracer>,
}
```

#### Methods

##### `start_span(name, attributes) -> BridgeResult<String>`
Starts a new trace span.

```rust
let mut attributes = HashMap::new();
attributes.insert("operation".to_string(), serde_json::json!("ingestion"));
let span_id = manager.start_span("process_batch", attributes).await?;
```

##### `end_span(span_id) -> BridgeResult<()>`
Ends a trace span.

```rust
manager.end_span(&span_id).await?;
```

##### `add_span_attribute(span_id, key, value) -> BridgeResult<()>`
Adds an attribute to a span.

```rust
manager.add_span_attribute(&span_id, "batch_size", serde_json::json!(100)).await?;
```

## Security

### SecurityManager

Manages all security features.

```rust
pub struct SecurityManager {
    auth_manager: AuthManager,
    encryption_manager: EncryptionManager,
    circuit_breaker: CircuitBreaker,
}
```

#### Methods

##### `encrypt_data(data: &[u8]) -> BridgeResult<Vec<u8>>`
Encrypts data.

```rust
let encrypted = security.encrypt_data(&data).await?;
```

##### `decrypt_data(encrypted_data: &[u8]) -> BridgeResult<Vec<u8>>`
Decrypts data.

```rust
let decrypted = security.decrypt_data(&encrypted).await?;
```

### AuthorizationManager

Manages authorization and access control.

```rust
pub struct AuthorizationManager {
    rbac: Arc<RwLock<RbacSystem>>,
    audit_log: Arc<RwLock<Vec<AuditLogEntry>>>,
    config: AuthorizationConfig,
}
```

#### Methods

##### `check_permission(user_id, resource, action, context) -> BridgeResult<bool>`
Checks if a user has permission to perform an action.

```rust
let context = AuthorizationContext::new()
    .with_ip_address("192.168.1.1".to_string())
    .with_user_agent("curl/7.68.0".to_string());

let has_permission = manager
    .check_permission("user123", "ingestion", &Action::Write, Some(context))
    .await?;
```

##### `add_user_role(user_id, role_name) -> BridgeResult<()>`
Adds a role to a user.

```rust
manager.add_user_role("user123", "admin").await?;
```

##### `create_role(role: Role) -> BridgeResult<()>`
Creates a new role.

```rust
let role = Role {
    name: "analyst".to_string(),
    description: "Data analyst role".to_string(),
    permissions: HashSet::from([
        Permission {
            resource: "ingestion".to_string(),
            action: Action::Read,
            conditions: None,
        },
    ]),
    active: true,
    created_at: Utc::now(),
    modified_at: Utc::now(),
};

manager.create_role(role).await?;
```

## Configuration

### Configuration Files

The ingestion system uses TOML configuration files.

#### Example Configuration

```toml
[ingestion]
enabled = true
max_batch_size = 1000
processing_timeout = "30s"

[ingestion.protocols.otlp]
enabled = true
host = "0.0.0.0"
port = 4318
protocol = "arrow"

[ingestion.protocols.kafka]
enabled = true
bootstrap_servers = ["localhost:9092"]
topic = "telemetry-data"
group_id = "ingestion-group"

[ingestion.processors.batch]
enabled = true
max_batch_size = 1000
max_wait_time = "5s"

[ingestion.processors.filter]
enabled = true
rules = [
    { name = "drop_errors", condition = "level == 'error'", action = "drop" }
]

[ingestion.exporters.lakehouse]
enabled = true
endpoint = "http://localhost:8080/export"
batch_size = 1000

[monitoring]
enabled = true

[monitoring.metrics]
enabled = true
port = 9090

[monitoring.health_checks]
enabled = true
check_interval = "30s"

[monitoring.logging]
enabled = true
level = "info"
json_format = true

[monitoring.tracing]
enabled = true
service_name = "ingestion-service"

[security]
enabled = true

[security.auth]
enabled = true
method = "jwt"

[security.authorization]
enabled = true
default_role = "user"
enable_audit_log = true
```

### Environment Variables

The system also supports configuration via environment variables:

- `ORASI_INGESTION_ENABLED`: Enable/disable ingestion
- `ORASI_INGESTION_MAX_BATCH_SIZE`: Maximum batch size
- `ORASI_MONITORING_ENABLED`: Enable/disable monitoring
- `ORASI_SECURITY_ENABLED`: Enable/disable security
- `ORASI_LOG_LEVEL`: Logging level

## Error Handling

### BridgeResult

All operations return a `BridgeResult<T>` which is an alias for `Result<T, BridgeError>`.

### BridgeError

```rust
pub enum BridgeError {
    Configuration(String),
    Authentication(String),
    Authorization(String),
    Network(String),
    Serialization(String),
    Validation(String),
    HealthCheck(String),
    Internal(String),
}
```

### Error Handling Example

```rust
match system.process_batch(batch).await {
    Ok(_) => println!("Batch processed successfully"),
    Err(BridgeError::Configuration(msg)) => eprintln!("Configuration error: {}", msg),
    Err(BridgeError::Network(msg)) => eprintln!("Network error: {}", msg),
    Err(e) => eprintln!("Unexpected error: {:?}", e),
}
```

## Examples

### Basic Usage

```rust
use ingestion::{
    IngestionSystem, OtlpReceiver, BatchProcessor, BatchExporter,
    receivers::OtlpReceiverConfig,
    processors::BatchProcessorConfig,
    exporters::BatchExporterConfig,
};

#[tokio::main]
async fn main() -> bridge_core::BridgeResult<()> {
    // Create ingestion system
    let mut system = IngestionSystem::new();
    
    // Add OTLP receiver
    let otlp_config = OtlpReceiverConfig::new_arrow("0.0.0.0".to_string(), 4318);
    let otlp_receiver = OtlpReceiver::new(otlp_config);
    system.add_receiver("otlp".to_string(), Box::new(otlp_receiver));
    
    // Add batch processor
    let batch_config = BatchProcessorConfig::new(1000, Duration::from_secs(5));
    let batch_processor = BatchProcessor::new(batch_config);
    system.add_processor("batch".to_string(), Box::new(batch_processor));
    
    // Add exporter
    let exporter_config = BatchExporterConfig::new(
        "http://localhost:8080/export".to_string(),
        1000,
        RetryConfig::default(),
    );
    let exporter = BatchExporter::new(exporter_config);
    system.add_exporter("lakehouse".to_string(), Box::new(exporter));
    
    // Initialize system
    system.initialize().await?;
    
    // Start processing
    loop {
        // Process batches
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

### With Monitoring

```rust
use ingestion::{
    IngestionSystem, MonitoringConfig, MonitoringManager,
};

#[tokio::main]
async fn main() -> bridge_core::BridgeResult<()> {
    // Create monitoring configuration
    let monitoring_config = MonitoringConfig::production();
    let monitoring = Arc::new(MonitoringManager::new(monitoring_config));
    
    // Create ingestion system with monitoring
    let mut system = IngestionSystem::with_monitoring(monitoring);
    
    // Initialize system (includes monitoring)
    system.initialize().await?;
    
    // System is now ready with full monitoring
    Ok(())
}
```

### With Security

```rust
use ingestion::{
    IngestionSystem, SecurityManager, AuthConfig,
    security::{AuthorizationManager, AuthorizationConfig},
};

#[tokio::main]
async fn main() -> bridge_core::BridgeResult<()> {
    // Create security configuration
    let auth_config = AuthConfig::new();
    let security = SecurityManager::new(auth_config).await?;
    
    // Create authorization configuration
    let auth_config = AuthorizationConfig {
        enabled: true,
        default_role: "user".to_string(),
        enable_audit_log: true,
        max_audit_entries: 1000,
        audit_retention_days: 30,
    };
    let authorization = AuthorizationManager::new(auth_config);
    
    // Create ingestion system
    let mut system = IngestionSystem::new();
    
    // Initialize security
    security.initialize().await?;
    authorization.initialize().await?;
    
    // System is now ready with security
    Ok(())
}
```

## Best Practices

### Performance

1. **Batch Processing**: Use appropriate batch sizes for your workload
2. **Connection Pooling**: Reuse connections when possible
3. **Async Operations**: Use async/await throughout the stack
4. **Monitoring**: Monitor performance metrics and adjust accordingly

### Security

1. **Authentication**: Always enable authentication in production
2. **Authorization**: Use role-based access control
3. **Encryption**: Encrypt sensitive data in transit and at rest
4. **Audit Logging**: Enable audit logging for compliance

### Monitoring

1. **Metrics**: Collect comprehensive metrics
2. **Health Checks**: Implement health checks for all components
3. **Logging**: Use structured logging with appropriate levels
4. **Tracing**: Enable distributed tracing for debugging

### Configuration

1. **Environment Variables**: Use environment variables for sensitive configuration
2. **Validation**: Validate all configuration on startup
3. **Hot Reload**: Use configuration hot reload when possible
4. **Documentation**: Document all configuration options

## Troubleshooting

### Common Issues

1. **Connection Errors**: Check network connectivity and firewall settings
2. **Authentication Errors**: Verify credentials and token validity
3. **Performance Issues**: Monitor metrics and adjust batch sizes
4. **Memory Issues**: Check for memory leaks and adjust buffer sizes

### Debugging

1. **Enable Debug Logging**: Set log level to debug for detailed information
2. **Use Tracing**: Enable distributed tracing to trace request flow
3. **Check Metrics**: Monitor metrics for performance bottlenecks
4. **Health Checks**: Use health checks to identify failing components

### Support

For additional support:

1. **Documentation**: Check the full documentation
2. **Examples**: Review the example code
3. **Issues**: Report issues on GitHub
4. **Community**: Join the community discussions
