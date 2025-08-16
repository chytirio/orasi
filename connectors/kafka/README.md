# Apache Kafka Connector

A comprehensive Apache Kafka connector for the OpenTelemetry Data Lake Bridge, providing seamless integration with Kafka for streaming telemetry data.

## Features

- **Full Kafka Integration**: Complete producer and consumer implementations
- **Telemetry Data Support**: Native support for metrics, traces, and logs
- **Real-time Streaming**: High-performance streaming of telemetry data
- **Comprehensive Configuration**: Flexible configuration for all Kafka settings
- **Health Monitoring**: Built-in health checks and metrics collection
- **Error Handling**: Robust error handling and recovery mechanisms
- **Authentication Support**: SASL/SSL authentication support
- **Topic Management**: Automatic topic creation and management
- **Cluster Management**: Kafka cluster discovery and metadata management

## Architecture

The Kafka connector consists of several key components:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   KafkaConfig   │    │ KafkaConnector  │    │ RealKafkaExporter│
│                 │    │                 │    │                 │
│ - Cluster config│    │ - Main connector│    │ - Export batches│
│ - Producer config│   │ - Health checks │    │ - Process data  │
│ - Consumer config│   │ - Stats tracking│    │ - Error handling│
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  KafkaProducer  │    │  KafkaConsumer  │    │   KafkaMetrics  │
│                 │    │                 │    │                 │
│ - Write metrics │    │ - Read metrics  │    │ - Track stats   │
│ - Write traces  │    │ - Read traces   │    │ - Monitor perf  │
│ - Write logs    │    │ - Read logs     │    │ - Error counts  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  KafkaCluster   │    │   KafkaTopic    │    │   Error Types   │
│                 │    │                 │    │                 │
│ - Cluster mgmt  │    │ - Topic mgmt    │    │ - Error handling│
│ - Broker disc   │    │ - Partition mgmt│    │ - Recovery      │
│ - Metadata      │    │ - Config mgmt   │    │ - Logging       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Quick Start

### Basic Usage

```rust
use kafka_connector::{KafkaConnector, KafkaConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let config = KafkaConfig::new(
        "localhost:9092".to_string(),
        "telemetry-data".to_string(),
        "my-consumer-group".to_string(),
    );
    
    // Create and connect
    let connector = KafkaConnector::connect(config).await?;
    
    // Use the connector...
    
    // Shutdown
    connector.shutdown().await?;
    Ok(())
}
```

### Producer Example

```rust
use kafka_connector::{KafkaProducer, KafkaConfig};
use kafka_connector::bridge_core::types::{MetricsBatch, MetricData, MetricType, MetricValue};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = KafkaConfig::new(
        "localhost:9092".to_string(),
        "metrics".to_string(),
        "producer-group".to_string(),
    );
    
    let producer = KafkaProducer::new(config).await?;
    
    // Create metrics batch
    let metrics_batch = MetricsBatch {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        metrics: vec![
            MetricData {
                name: "cpu_usage".to_string(),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(75.5),
                timestamp: chrono::Utc::now(),
                labels: std::collections::HashMap::new(),
            }
        ],
        metadata: std::collections::HashMap::new(),
    };
    
    // Write to Kafka
    let result = producer.write_metrics(metrics_batch).await?;
    println!("Wrote {} metrics", result.records_written);
    
    producer.close().await?;
    Ok(())
}
```

### Consumer Example

```rust
use kafka_connector::{KafkaConsumer, KafkaConfig};
use kafka_connector::bridge_core::types::MetricsQuery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = KafkaConfig::new(
        "localhost:9092".to_string(),
        "metrics".to_string(),
        "consumer-group".to_string(),
    );
    
    let consumer = KafkaConsumer::new(config).await?;
    
    // Create query
    let query = MetricsQuery {
        filters: vec![],
        aggregations: vec![],
        time_range: None,
        limit: Some(100),
        offset: Some(0),
    };
    
    // Read from Kafka
    let result = consumer.query_metrics(query).await?;
    println!("Found {} metrics", result.metrics.len());
    
    consumer.close().await?;
    Ok(())
}
```

## Configuration

### Basic Configuration

```rust
let config = KafkaConfig::new(
    "localhost:9092".to_string(),    // Bootstrap servers
    "telemetry-data".to_string(),    // Topic name
    "consumer-group".to_string(),    // Consumer group ID
);
```

### Advanced Configuration

```rust
use kafka_connector::config::{KafkaAuth, KafkaSSL};

let config = KafkaConfig {
    cluster: KafkaCluster {
        bootstrap_servers: "kafka1:9092,kafka2:9092".to_string(),
        client_id: "my-client".to_string(),
        request_timeout_ms: 30000,
        socket_timeout_ms: 30000,
        connection_timeout_ms: 60000,
        metadata_max_age_ms: 300000,
        reconnect_backoff_ms: 50,
        retry_backoff_ms: 100,
    },
    auth: Some(KafkaAuth {
        mechanism: "PLAIN".to_string(),
        username: Some("username".to_string()),
        password: Some("password".to_string()),
        ssl: Some(KafkaSSL {
            certificate_path: Some("/path/to/cert.pem".to_string()),
            key_path: Some("/path/to/key.pem".to_string()),
            ca_path: Some("/path/to/ca.pem".to_string()),
            password: Some("ssl_password".to_string()),
            verify: true,
        }),
    }),
    producer: KafkaProducerConfig {
        batch_size: 1000,
        flush_interval_ms: 1000,
        compression_type: "snappy".to_string(),
        acks: "all".to_string(),
        retries: 3,
        max_in_flight_requests: 5,
        enable_idempotence: true,
        max_request_size: 1048576,
        buffer_memory: 33554432,
        linger_ms: 5,
        batch_size_bytes: 16384,
    },
    consumer: KafkaConsumerConfig {
        group_id: "my-consumer-group".to_string(),
        auto_offset_reset: "latest".to_string(),
        enable_auto_commit: true,
        auto_commit_interval_ms: 5000,
        session_timeout_ms: 30000,
        heartbeat_interval_ms: 3000,
        max_poll_records: 500,
        max_poll_interval_ms: 300000,
        fetch_min_bytes: 1,
        fetch_max_wait_ms: 500,
        enable_partition_eof: false,
    },
    topic: KafkaTopic {
        name: "telemetry-data".to_string(),
        partitions: 3,
        replication_factor: 1,
        config: std::collections::HashMap::new(),
    },
    settings: std::collections::HashMap::new(),
};
```

## Features in Detail

### Producer Features

- **Batch Writing**: Efficient batch writing of telemetry data
- **Compression**: Support for various compression types (snappy, gzip, lz4, zstd)
- **Idempotence**: Exactly-once delivery semantics
- **Retry Logic**: Configurable retry policies
- **Performance Tuning**: Optimized for high-throughput scenarios

### Consumer Features

- **Group Management**: Consumer group coordination
- **Offset Management**: Automatic and manual offset management
- **Partition Assignment**: Automatic partition assignment
- **Rebalancing**: Graceful handling of consumer group rebalancing
- **Backpressure**: Built-in backpressure handling

### Exporter Features

- **Batch Processing**: Efficient batch export operations
- **Error Handling**: Comprehensive error handling and recovery
- **Statistics**: Detailed export statistics and metrics
- **Health Monitoring**: Real-time health status monitoring

### Metrics Collection

- **Performance Metrics**: Track producer and consumer performance
- **Error Tracking**: Monitor error rates and types
- **Throughput Metrics**: Measure data throughput
- **Latency Metrics**: Track operation latencies

## Error Handling

The connector provides comprehensive error handling:

```rust
use kafka_connector::error::{KafkaError, KafkaResult};

match producer.write_metrics(batch).await {
    Ok(result) => {
        println!("Successfully wrote {} records", result.records_written);
    }
    Err(KafkaError::Connection(msg)) => {
        eprintln!("Connection error: {}", msg);
        // Handle connection issues
    }
    Err(KafkaError::Producer(msg)) => {
        eprintln!("Producer error: {}", msg);
        // Handle producer issues
    }
    Err(KafkaError::Serialization(msg)) => {
        eprintln!("Serialization error: {}", msg);
        // Handle serialization issues
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
        // Handle other errors
    }
}
```

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
# Set up test environment
export KAFKA_TEST_BOOTSTRAP_SERVERS=localhost:9092
export KAFKA_TEST_TOPIC=test_telemetry
export KAFKA_TEST_GROUP_ID=test-consumer-group

# Run integration tests
cargo test --features integration
```

### Example Execution

```bash
# Run the comprehensive example
cargo run --example kafka_connector_example
```

## Performance Considerations

### Producer Optimization

- **Batch Size**: Increase `batch_size` for higher throughput
- **Compression**: Use snappy or lz4 for better compression ratios
- **Buffer Memory**: Adjust `buffer_memory` based on available RAM
- **Linger Time**: Increase `linger_ms` for better batching

### Consumer Optimization

- **Max Poll Records**: Increase `max_poll_records` for better throughput
- **Fetch Min Bytes**: Set `fetch_min_bytes` to reduce network overhead
- **Session Timeout**: Adjust `session_timeout_ms` based on processing time
- **Heartbeat Interval**: Optimize `heartbeat_interval_ms` for stability

## Monitoring and Observability

### Health Checks

```rust
let health = connector.health_check().await?;
if health {
    println!("Kafka connector is healthy");
} else {
    println!("Kafka connector has issues");
}
```

### Statistics

```rust
let stats = connector.get_stats().await?;
println!("Total writes: {}", stats.total_writes);
println!("Total reads: {}", stats.total_reads);
println!("Error count: {}", stats.error_count);
println!("Average write time: {:.2}ms", stats.avg_write_time_ms);
```

### Metrics Collection

```rust
use kafka_connector::KafkaMetrics;

let metrics = KafkaMetrics::new();
metrics.record_produce(100, 10240, 50); // 100 messages, 10KB, 50ms
metrics.record_consume(80, 8192, 40);   // 80 messages, 8KB, 40ms

let snapshot = metrics.get_snapshot();
println!("Total messages produced: {}", snapshot.total_messages_produced);
println!("Average produce time: {}ms", snapshot.avg_produce_time_ms);
```

## Security

### Authentication

```rust
let auth = KafkaAuth {
    mechanism: "PLAIN".to_string(),
    username: Some("username".to_string()),
    password: Some("password".to_string()),
    ssl: None,
};
```

### SSL/TLS

```rust
let ssl = KafkaSSL {
    certificate_path: Some("/path/to/cert.pem".to_string()),
    key_path: Some("/path/to/key.pem".to_string()),
    ca_path: Some("/path/to/ca.pem".to_string()),
    password: Some("ssl_password".to_string()),
    verify: true,
};
```

## Troubleshooting

### Common Issues

1. **Connection Timeouts**
   - Check network connectivity
   - Verify bootstrap servers
   - Adjust timeout settings

2. **Authentication Failures**
   - Verify credentials
   - Check SASL mechanism
   - Validate SSL certificates

3. **Performance Issues**
   - Monitor batch sizes
   - Check compression settings
   - Review buffer configurations

4. **Consumer Lag**
   - Increase consumer instances
   - Optimize processing logic
   - Check partition count

### Debug Logging

```rust
use tracing::Level;

// Enable debug logging
tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .init();
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run the test suite
6. Submit a pull request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](../../LICENSE) file for details.

## Support

For support and questions:

- Create an issue on GitHub
- Check the documentation
- Review the examples
- Join the community discussions
