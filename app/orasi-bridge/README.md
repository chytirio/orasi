# Orasi Bridge API Server

The main REST/gRPC API server for the Orasi OpenTelemetry Data Lake Bridge, providing comprehensive telemetry data ingestion, processing, and querying capabilities.

## Overview

Orasi Bridge is the central API server that orchestrates the entire OpenTelemetry data lake bridge ecosystem, offering:

- **REST API**: HTTP-based API for telemetry data ingestion and management
- **gRPC API**: High-performance gRPC interface for real-time data processing
- **Data Ingestion**: Multi-protocol telemetry data ingestion (OTLP, Kafka, etc.)
- **Data Processing**: Real-time streaming data processing and transformation
- **Query Interface**: SQL-based querying of telemetry data
- **Schema Management**: OpenTelemetry schema registry integration
- **Authentication**: Secure access control and user management
- **Monitoring**: Built-in health monitoring and metrics collection

## Key Features

### API Interfaces
- **REST API**: Standard HTTP API with OpenAPI documentation
- **gRPC API**: High-performance gRPC interface with protocol buffers
- **WebSocket API**: Real-time streaming data interface
- **GraphQL API**: Flexible query interface for complex data relationships

### Data Ingestion
- **OTLP Protocol**: OpenTelemetry protocol support (HTTP/gRPC)
- **Kafka Integration**: High-throughput Kafka data ingestion
- **Batch Processing**: Efficient batch data processing
- **Real-time Streaming**: Low-latency streaming data processing
- **Data Validation**: Schema validation and data quality checks

### Data Processing
- **Streaming Pipeline**: Real-time data transformation and enrichment
- **Aggregation**: Time-window based data aggregation
- **Filtering**: Configurable data filtering and routing
- **Transformation**: Data format conversion and enrichment
- **Compression**: Data compression and optimization

### Query Capabilities
- **SQL Interface**: Full SQL query support for telemetry data
- **Time Series Queries**: Specialized time-series analysis functions
- **Real-time Queries**: Live data querying capabilities
- **Query Optimization**: Automatic query optimization and caching
- **Multiple Formats**: JSON, CSV, Parquet, and Arrow output formats

### Schema Management
- **Schema Registry**: OpenTelemetry schema registration and management
- **Schema Validation**: Automatic schema validation and evolution
- **Semantic Conventions**: Built-in OpenTelemetry semantic conventions
- **Schema Evolution**: Backward-compatible schema evolution

## Quick Start

### Running the Server

```bash
# Build the server
cargo build --release

# Run with default configuration
./target/release/bridge-api

# Run with custom configuration
./target/release/bridge-api --config config/bridge.toml

# Run in development mode
cargo run --bin bridge-api
```

### Docker Deployment

```bash
# Build Docker image
docker build -t orasi-bridge .

# Run with Docker Compose
docker-compose up -d

# Run standalone container
docker run -p 8080:8080 -p 9090:9090 orasi-bridge
```

### Basic API Usage

```bash
# Health check
curl http://localhost:8080/health

# Ingest OTLP data
curl -X POST http://localhost:8080/v1/ingest/otlp \
  -H "Content-Type: application/json" \
  -d '{
    "resource": {
      "attributes": {
        "service.name": "my-service"
      }
    },
    "spans": [
      {
        "name": "http_request",
        "start_time": "2024-01-01T00:00:00Z",
        "end_time": "2024-01-01T00:00:01Z"
      }
    ]
  }'

# Query telemetry data
curl -X POST http://localhost:8080/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "SELECT service.name, COUNT(*) FROM telemetry_data GROUP BY service.name"
  }'
```

### gRPC Client Example

```rust
use tonic::{transport::Channel, Request};
use bridge_api::grpc::{
    telemetry_service_client::TelemetryServiceClient,
    IngestRequest, QueryRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to gRPC server
    let channel = Channel::from_static("http://localhost:9090")
        .connect()
        .await?;
    
    let mut client = TelemetryServiceClient::new(channel);
    
    // Ingest telemetry data
    let ingest_request = Request::new(IngestRequest {
        data: serde_json::json!({
            "resource": {
                "attributes": {
                    "service.name": "my-service"
                }
            },
            "spans": [
                {
                    "name": "http_request",
                    "start_time": "2024-01-01T00:00:00Z",
                    "end_time": "2024-01-01T00:00:01Z"
                }
            ]
        }).to_string(),
    });
    
    let response = client.ingest(ingest_request).await?;
    println!("Ingest response: {:?}", response);
    
    // Query telemetry data
    let query_request = Request::new(QueryRequest {
        sql: "SELECT service.name, COUNT(*) FROM telemetry_data GROUP BY service.name".to_string(),
    });
    
    let response = client.query(query_request).await?;
    println!("Query response: {:?}", response);
    
    Ok(())
}
```

## Configuration

### Server Configuration

```toml
# bridge.toml
[server]
host = "0.0.0.0"
port = 8080
grpc_port = 9090
workers = 4

[server.api]
enable_rest = true
enable_grpc = true
enable_graphql = true
enable_websocket = true

[server.auth]
enabled = true
jwt_secret = "your-secret-key"
token_expiry_seconds = 3600

[server.cors]
enabled = true
allowed_origins = ["http://localhost:3000", "https://your-domain.com"]
allowed_methods = ["GET", "POST", "PUT", "DELETE"]
allowed_headers = ["*"]

[ingestion]
batch_size = 1000
batch_timeout_ms = 5000
max_concurrent_ingestions = 10

[ingestion.otlp]
enabled = true
http_port = 4318
grpc_port = 4317

[ingestion.kafka]
enabled = true
bootstrap_servers = ["localhost:9092"]
topics = ["telemetry-data"]
group_id = "orasi-bridge"

[processing]
streaming_enabled = true
max_concurrent_processors = 8
buffer_size = 10000

[query]
engine_enabled = true
max_memory_mb = 2048
query_timeout_seconds = 300
cache_size_mb = 512

[storage]
type = "parquet"
path = "/data/telemetry"
partition_by = ["service_name", "date"]

[monitoring]
metrics_enabled = true
metrics_port = 9090
health_check_enabled = true
health_check_port = 8081
```

## API Reference

### REST API Endpoints

#### Health and Monitoring
```
GET  /health                    # Health check
GET  /metrics                   # Prometheus metrics
GET  /ready                     # Readiness check
GET  /live                      # Liveness check
```

#### Data Ingestion
```
POST /v1/ingest/otlp           # OTLP data ingestion
POST /v1/ingest/kafka          # Kafka data ingestion
POST /v1/ingest/batch          # Batch data ingestion
POST /v1/ingest/stream         # Streaming data ingestion
```

#### Data Querying
```
POST /v1/query                 # SQL query execution
GET  /v1/query/{query_id}      # Query status and results
POST /v1/query/stream          # Streaming query execution
GET  /v1/schemas               # List available schemas
```

#### Streaming Telemetry
```
POST /v1/telemetry/stream      # Stream telemetry data in batches
```

#### Schema Management
```
GET    /v1/schemas             # List schemas
POST   /v1/schemas             # Register schema
GET    /v1/schemas/{id}        # Get schema
PUT    /v1/schemas/{id}        # Update schema
DELETE /v1/schemas/{id}        # Delete schema
```

#### Authentication
```
POST /v1/auth/login            # User login
POST /v1/auth/logout           # User logout
POST /v1/auth/refresh          # Refresh token
GET  /v1/auth/profile          # User profile
```

### gRPC Services

#### TelemetryService
```protobuf
service TelemetryService {
  rpc Ingest(IngestRequest) returns (IngestResponse);
  rpc Query(QueryRequest) returns (QueryResponse);
  rpc StreamIngest(stream IngestRequest) returns (stream IngestResponse);
  rpc StreamQuery(QueryRequest) returns (stream QueryResponse);
}
```

#### SchemaService
```protobuf
service SchemaService {
  rpc RegisterSchema(RegisterSchemaRequest) returns (RegisterSchemaResponse);
  rpc GetSchema(GetSchemaRequest) returns (GetSchemaResponse);
  rpc ListSchemas(ListSchemasRequest) returns (ListSchemasResponse);
  rpc UpdateSchema(UpdateSchemaRequest) returns (UpdateSchemaResponse);
  rpc DeleteSchema(DeleteSchemaRequest) returns (DeleteSchemaResponse);
}
```

#### BridgeService (gRPC)
```protobuf
service BridgeService {
  rpc QueryTelemetry(QueryTelemetryRequest) returns (QueryTelemetryResponse);
  rpc StreamTelemetry(StreamTelemetryRequest) returns (stream StreamTelemetryResponse);
  rpc GetStatus(GetStatusRequest) returns (GetStatusResponse);
  rpc UpdateConfig(UpdateConfigRequest) returns (UpdateConfigResponse);
  rpc GetMetrics(GetMetricsRequest) returns (GetMetricsResponse);
}
```

## Architecture

The Orasi Bridge follows a microservices architecture with clear separation of concerns:

```
┌─────────────────┐
│   Orasi Bridge  │
├─────────────────┤
│  REST API       │
│  gRPC API       │
│  WebSocket API  │
│  GraphQL API    │
├─────────────────┤
│  Ingestion      │
│  Processing     │
│  Query Engine   │
│  Schema Registry│
├─────────────────┤
│  Storage        │
│  Monitoring     │
│  Auth           │
└─────────────────┘
```

### Core Components

1. **API Layer**: Multiple API interfaces (REST, gRPC, WebSocket, GraphQL)
2. **Ingestion Layer**: Multi-protocol data ingestion
3. **Processing Layer**: Real-time data processing and transformation
4. **Query Layer**: SQL query execution and optimization
5. **Schema Layer**: Schema management and validation
6. **Storage Layer**: Data persistence and retrieval
7. **Monitoring Layer**: Health checks and metrics collection
8. **Auth Layer**: Authentication and authorization

## Development

### Building

```bash
# Build all components
cargo build

# Build with optimizations
cargo build --release

# Build specific binary
cargo build --bin bridge-api

# Build with features
cargo build --features openapi
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_ingestion

# Run integration tests
cargo test --test integration

# Run with logging
RUST_LOG=debug cargo test
```

### Development Server

```bash
# Run development server
cargo run --bin bridge-api

# Run with hot reload
cargo watch -x run --bin bridge-api

# Run with custom config
cargo run --bin bridge-api -- --config config/dev.toml
```

### API Documentation

```bash
# Generate OpenAPI spec
cargo run --bin bridge-api -- --generate-openapi

# Serve API documentation
cargo run --bin bridge-api -- --serve-docs
```

## Deployment

### Docker Deployment

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin bridge-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/bridge-api /usr/local/bin/
EXPOSE 8080 9090
CMD ["bridge-api"]
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: orasi-bridge
spec:
  replicas: 3
  selector:
    matchLabels:
      app: orasi-bridge
  template:
    metadata:
      labels:
        app: orasi-bridge
    spec:
      containers:
      - name: bridge-api
        image: orasi/bridge-api:latest
        ports:
        - containerPort: 8080
        - containerPort: 9090
        env:
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
```

## Monitoring

### Metrics

The bridge exposes Prometheus metrics at `/metrics`:

- **Request metrics**: Request count, duration, error rates
- **Ingestion metrics**: Data ingestion rates and volumes
- **Processing metrics**: Processing latency and throughput
- **Query metrics**: Query execution times and cache hit rates
- **System metrics**: CPU, memory, and disk usage

### Health Checks

Health check endpoints:

- `/health`: Overall system health
- `/ready`: Readiness for traffic
- `/live`: Liveness check

### Logging

Structured logging with configurable levels:

```bash
# Set log level
export RUST_LOG=info

# Enable JSON logging
export RUST_LOG_JSON=true
```

## Streaming Telemetry

The Orasi Bridge provides a powerful streaming telemetry API that allows you to efficiently retrieve large volumes of telemetry data in real-time batches.

### Features

- **Batch Processing**: Data is streamed in configurable batch sizes
- **Real-time Streaming**: Low-latency data delivery with gRPC streaming
- **Pagination Support**: Automatic pagination for large datasets
- **Error Handling**: Robust error handling with graceful degradation
- **Timeout Protection**: Configurable timeouts to prevent infinite streaming
- **Resource Limits**: Built-in limits to prevent resource exhaustion

### gRPC Streaming Example

```rust
use tonic::{Request, Response};
use bridge_api::proto::bridge::grpc::{
    bridge_service_client::BridgeServiceClient,
    StreamTelemetryRequest, TimeRange, Filter,
};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the bridge service
    let mut client = BridgeServiceClient::connect("http://[::1]:50051").await?;

    // Create a streaming telemetry request
    let request = StreamTelemetryRequest {
        query_type: "traces".to_string(),
        time_range: Some(TimeRange {
            start_time: chrono::Utc::now().timestamp() - 3600, // 1 hour ago
            end_time: chrono::Utc::now().timestamp(),
        }),
        filters: vec![
            Filter {
                field: "service.name".to_string(),
                operator: "eq".to_string(),
                value: "example-service".to_string(),
            },
        ],
        batch_size: 100,
    };

    // Make the streaming request
    let request = Request::new(request);
    let mut stream = client.stream_telemetry(request).await?.into_inner();

    // Process the streaming response
    let mut total_records = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                total_records += response.records.len();
                println!("Received batch: {} records (total: {})", 
                    response.records.len(), total_records);

                if response.is_last_batch {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error receiving stream: {}", e);
                break;
            }
        }
    }

    println!("Streaming completed. Total records: {}", total_records);
    Ok(())
}
```

### Request Parameters

- **query_type**: Type of telemetry data to stream (`traces`, `metrics`, `logs`)
- **time_range**: Optional time range filter (start_time, end_time)
- **filters**: Array of field filters for data filtering
- **batch_size**: Number of records per batch (1-1000, default: 100)

### Response Format

Each batch response contains:
- **records**: Array of telemetry records in this batch
- **is_last_batch**: Boolean indicating if this is the final batch
- **total_processed**: Total number of records processed so far

### Error Handling

The streaming API includes comprehensive error handling:

- **Validation Errors**: Invalid request parameters
- **Timeout Errors**: Stream timeout after 5 minutes
- **Resource Errors**: Maximum batch limit exceeded (1000 batches)
- **Connection Errors**: Client disconnection handling

### Performance Considerations

- **Batch Size**: Optimal batch size depends on your use case (100-500 records)
- **Timeout**: Default 5-minute timeout, adjustable via configuration
- **Memory Usage**: Each batch is processed independently to minimize memory usage
- **Network**: gRPC streaming provides efficient network utilization

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
