# Configuration Management System

The Bridge API includes a comprehensive configuration management system that provides hot reloading, component restart coordination, and validation capabilities.

## Overview

The configuration management system consists of:

- **ConfigService**: Main service for handling configuration updates
- **ComponentRestartHandler**: Trait for components that can be restarted
- **ConfigManager**: Hot reloading and file watching capabilities
- **Component Handlers**: Implementations for various bridge components

## Features

### 🔄 Hot Reloading
- Automatic configuration file watching
- Real-time configuration updates without service restart
- Configuration change detection and validation

### 🧩 Component Restart Coordination
- Graceful component shutdown and restart
- Component health monitoring
- Restart statistics and error tracking
- Parallel component restart for efficiency

### ✅ Validation
- JSON schema validation
- Bridge configuration validation
- Component-specific validation rules
- Detailed error reporting

### 💾 Persistence
- Configuration file persistence
- Configuration hash tracking
- Backup and rollback capabilities

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   gRPC/REST     │    │   ConfigService  │    │ BridgeConfig    │
│     Client      │───▶│                  │───▶│                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌──────────────────┐
                       │Component Handlers│
                       └──────────────────┘
                                │
                    ┌───────────┼───────────┐
                    ▼           ▼           ▼
            ┌─────────────┐ ┌─────────┐ ┌─────────┐
            │Query Engine │ │Ingestion│ │Streaming│
            └─────────────┘ └─────────┘ └─────────┘
```

## Usage

### Basic Configuration Service Setup

```rust
use bridge_api::{
    config_service::ConfigService,
    component_handlers::create_default_component_handlers,
    config::BridgeAPIConfig,
    metrics::ApiMetrics,
};
use bridge_core::BridgeConfig;

// Create configuration service
let config_service = ConfigService::new(
    BridgeAPIConfig::default(),
    BridgeConfig::default(),
    PathBuf::from("config/bridge-config.json"),
    ApiMetrics::new(),
);

// Register component handlers
let handlers = create_default_component_handlers();
for handler in handlers {
    config_service.register_component_handler(handler).await;
}

// Start configuration file watching
config_service.start_config_watching().await?;
```

### Configuration Update

```rust
use bridge_api::proto::UpdateConfigRequest;

let config_json = r#"{
    "api": {
        "host": "0.0.0.0",
        "port": 8080
    },
    "bridge": {
        "name": "my-bridge",
        "environment": "production",
        "ingestion": {
            "batch_size": 2000,
            "flush_interval_ms": 3000
        }
    }
}"#;

let request = UpdateConfigRequest {
    config_json: config_json.to_string(),
    validate_only: false,
    restart_components: true,
};

let response = config_service.update_config(&request).await?;
if response.success {
    println!("Configuration updated successfully");
    println!("Restarted components: {:?}", response.restarted_components);
} else {
    println!("Update failed: {}", response.error_message);
}
```

### Component Status Monitoring

```rust
let statuses = config_service.get_component_status().await?;
for status in statuses {
    println!("Component: {} - Status: {:?}", status.name, status.status);
    println!("  Restarts: {}", status.restart_count);
    if let Some(error) = &status.error_message {
        println!("  Error: {}", error);
    }
}
```

## Component Handlers

### Built-in Handlers

The system includes handlers for:

- **QueryEngineHandler**: Manages query engine configuration and restart
- **StreamingProcessorHandler**: Handles streaming processor configuration
- **IngestionHandler**: Manages ingestion pipeline configuration
- **SchemaRegistryHandler**: Handles schema registry configuration

### Creating Custom Handlers

```rust
use bridge_api::config_service::{ComponentRestartHandler, ComponentStatus, ComponentState};
use bridge_core::{BridgeResult, BridgeConfig};

pub struct MyCustomHandler {
    name: String,
    is_running: Arc<RwLock<bool>>,
    restart_count: Arc<RwLock<u32>>,
    last_restart: Arc<RwLock<Option<SystemTime>>>,
    error_message: Arc<RwLock<Option<String>>>,
}

#[async_trait::async_trait]
impl ComponentRestartHandler for MyCustomHandler {
    fn name(&self) -> &str {
        &self.name
    }

    async fn restart(&self, config: &BridgeConfig) -> BridgeResult<()> {
        // Stop current instance
        {
            let mut running = self.is_running.write().await;
            *running = false;
        }

        // Apply new configuration
        self.apply_config(config).await?;

        // Start new instance
        {
            let mut running = self.is_running.write().await;
            *running = true;
        }

        // Update statistics
        {
            let mut count = self.restart_count.write().await;
            *count += 1;
        }
        {
            let mut last = self.last_restart.write().await;
            *last = Some(SystemTime::now());
        }

        Ok(())
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let running = self.is_running.read().await;
        Ok(*running)
    }

    async fn get_status(&self) -> BridgeResult<ComponentStatus> {
        let running = self.is_running.read().await;
        let count = self.restart_count.read().await;
        let last = self.last_restart.read().await;
        let error = self.error_message.read().await;

        let status = if *running {
            ComponentState::Running
        } else {
            ComponentState::Stopped
        };

        Ok(ComponentStatus {
            name: self.name.clone(),
            status,
            last_restart: *last,
            restart_count: *count,
            error_message: error.clone(),
        })
    }
}
```

## Configuration Format

### API Configuration

```json
{
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
}
```

### Bridge Configuration

```json
{
    "bridge": {
        "name": "my-bridge",
        "environment": "production",
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
        "processing": {
            "worker_threads": 8,
            "enable_streaming": true,
            "stream_window_ms": 30000,
            "enable_transformation": true,
            "enable_filtering": true,
            "enable_aggregation": false,
            "query_timeout_secs": 60,
            "enable_query_caching": true,
            "cache_size": 209715200,
            "cache_ttl_secs": 600
        },
        "security": {
            "enable_tls": false,
            "enable_authentication": false,
            "authentication_methods": ["None"],
            "enable_authorization": false,
            "data_retention_days": 90
        },
        "monitoring": {
            "enable_metrics": true,
            "metrics_endpoint": "http://localhost:9090",
            "enable_health_checks": true,
            "health_endpoint": "http://localhost:8080",
            "log_level": "info",
            "log_format": "json"
        }
    }
}
```

## Validation Rules

### API Configuration Validation
- Host must be non-empty
- Port must be between 1 and 65535
- Request timeout must be greater than 0
- Max request size must be greater than 0

### gRPC Configuration Validation
- Host must be non-empty
- Port must be between 1 and 65535
- Max message size must be greater than 0
- Max concurrent streams must be greater than 0

### Metrics Configuration Validation
- Port must be between 1 and 65535 (if provided)
- Enabled must be a boolean
- Path must be non-empty (if provided)

### Bridge Configuration Validation
- Name must be between 1 and 100 characters
- Environment must be between 1 and 20 characters
- Batch size must be between 1 and 10000
- Flush interval must be between 100 and 60000ms
- Buffer size must be between 1000 and 100000
- Compression level must be between 0 and 9
- Backpressure threshold must be between 50 and 95

## Error Handling

The configuration management system provides detailed error reporting:

```rust
let response = config_service.update_config(&request).await?;
if !response.success {
    println!("Configuration update failed: {}", response.error_message);
    for error in &response.validation_errors {
        println!("  - {}", error);
    }
}
```

## Monitoring and Observability

### Configuration Change Detection

```rust
let has_changed = config_service.has_config_changed().await?;
let config_hash = config_service.get_config_hash().await;
```

### Component Health Monitoring

```rust
let statuses = config_service.get_component_status().await?;
for status in statuses {
    match status.status {
        ComponentState::Running => println!("✅ {} is running", status.name),
        ComponentState::Stopped => println!("⏹️ {} is stopped", status.name),
        ComponentState::Error => println!("❌ {} has errors", status.name),
        _ => println!("🔄 {} is transitioning", status.name),
    }
}
```

## Best Practices

### 1. Configuration Validation
Always validate configurations before applying them:

```rust
let request = UpdateConfigRequest {
    config_json: config_json.to_string(),
    validate_only: true,  // Validate only
    restart_components: false,
};
```

### 2. Graceful Component Restart
Implement proper shutdown and startup sequences in component handlers:

```rust
async fn restart(&self, config: &BridgeConfig) -> BridgeResult<()> {
    // 1. Stop gracefully
    self.stop_gracefully().await?;
    
    // 2. Wait for cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // 3. Apply new configuration
    self.apply_config(config).await?;
    
    // 4. Start new instance
    self.start().await?;
    
    Ok(())
}
```

### 3. Error Recovery
Handle component restart failures gracefully:

```rust
for (component_name, handler) in handlers.iter() {
    match handler.restart(&bridge_config).await {
        Ok(_) => {
            restarted_components.push(component_name.clone());
            info!("Successfully restarted component: {}", component_name);
        }
        Err(e) => {
            error!("Failed to restart component {}: {}", component_name, e);
            // Continue with other components
        }
    }
}
```

### 4. Configuration Backup
Always backup configurations before major changes:

```rust
// Backup current configuration
let current_config = config_service.get_current_config().await?;
let backup_path = format!("config/backup-{}.json", chrono::Utc::now().timestamp());
std::fs::write(&backup_path, serde_json::to_string_pretty(&current_config)?)?;
```

## Troubleshooting

### Common Issues

1. **Configuration Validation Failures**
   - Check JSON syntax
   - Verify required fields are present
   - Ensure values are within valid ranges

2. **Component Restart Failures**
   - Check component logs for errors
   - Verify component dependencies
   - Ensure proper shutdown sequences

3. **File Watching Issues**
   - Check file permissions
   - Verify file path exists
   - Ensure sufficient file descriptors

### Debug Mode

Enable debug logging for detailed troubleshooting:

```rust
tracing_subscriber::fmt()
    .with_env_filter("debug")
    .init();
```

## Integration with Existing Systems

The configuration management system integrates seamlessly with:

- **gRPC API**: Configuration updates via gRPC calls
- **REST API**: Configuration management endpoints
- **Metrics**: Configuration change metrics
- **Health Checks**: Component health monitoring
- **Logging**: Structured logging for configuration changes

## Performance Considerations

- Configuration validation is performed in-memory
- Component restarts are performed in parallel
- File watching uses efficient file system events
- Configuration hashing uses fast hashing algorithms
- Component status queries are cached

## Security

- Configuration files should have appropriate permissions
- Sensitive configuration values should be encrypted
- Access to configuration endpoints should be authenticated
- Configuration changes should be audited
