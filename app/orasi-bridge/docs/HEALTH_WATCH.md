# Health Watch Streaming Implementation

## Overview

The health watch streaming functionality allows clients to monitor the health status of services in real-time using gRPC streaming. This implementation provides continuous health status updates for the bridge-api service.

## Features

- **Real-time health monitoring**: Clients can subscribe to health status updates
- **Automatic health checks**: The service performs periodic health checks every 30 seconds
- **Failure detection**: Implements a failure threshold (3 consecutive failures) before marking a service as unhealthy
- **Graceful disconnection**: Handles client disconnections gracefully
- **Service validation**: Validates service names and returns appropriate errors for unknown services

## Implementation Details

### Proto Definition

The health watch streaming is defined in `proto/health.proto`:

```protobuf
service Health {
  // Check health status
  rpc Check(HealthCheckRequest) returns (HealthCheckResponse);
  
  // Watch health status changes
  rpc Watch(HealthCheckRequest) returns (stream HealthCheckResponse);
}
```

### Service Implementation

The `GrpcHealthService` implements the health watch streaming with the following key components:

1. **Stream Creation**: Uses `tokio::sync::mpsc` channels to create a streaming response
2. **Health Monitoring Task**: Spawns an async task that continuously monitors service health
3. **Interval-based Checks**: Performs health checks every 30 seconds
4. **Failure Tracking**: Tracks consecutive failures and marks service as unhealthy after 3 failures
5. **Status Updates**: Sends health status updates through the stream

### Health Check Logic

The service implements a simple health check mechanism:

- **Known Services**: `bridge-api` and empty service names are considered healthy
- **Unknown Services**: Return `ServiceUnknown` status
- **Simulated Failures**: Includes a 1% chance of failure for testing purposes
- **Failure Threshold**: Requires 3 consecutive failures before marking as unhealthy

## Usage

### Client Example

```rust
use tonic::{Request, Status};
use bridge_api::proto::{
    health_check_response::ServingStatus,
    health_client::HealthClient,
    HealthCheckRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = HealthClient::connect("http://[::1]:50051").await?;
    
    let request = Request::new(HealthCheckRequest {
        service: "bridge-api".to_string(),
    });
    
    let mut stream = client.watch(request).await?.into_inner();
    
    while let Some(result) = stream.message().await? {
        match result {
            Ok(response) => {
                let status = ServingStatus::from_i32(response.status)
                    .unwrap_or(ServingStatus::Unknown);
                println!("Health status: {:?}", status);
            }
            Err(status) => {
                eprintln!("Health watch error: {}", status);
                break;
            }
        }
    }
    
    Ok(())
}
```

### Server Integration

The health service is automatically integrated into the gRPC server when the `GrpcHealthService` is registered:

```rust
use bridge_api::services::grpc::health::GrpcHealthService;

let health_service = GrpcHealthService::new();
// Register with tonic server...
```

## Configuration

### Health Check Interval

The health check interval is configurable in the `watch` method:

```rust
let mut interval = interval(Duration::from_secs(30)); // 30 seconds
```

### Failure Threshold

The maximum number of consecutive failures before marking a service as unhealthy:

```rust
const MAX_FAILURES: u32 = 3;
```

### Stream Buffer Size

The size of the message channel buffer:

```rust
let (tx, rx) = mpsc::channel(100); // Buffer size of 100
```

## Error Handling

The implementation handles various error scenarios:

1. **Unknown Services**: Returns `NotFound` status for unknown service names
2. **Client Disconnection**: Gracefully stops the monitoring task when client disconnects
3. **Stream Errors**: Propagates errors through the stream for client handling

## Testing

The implementation includes comprehensive tests:

- `test_health_check`: Tests basic health check functionality
- `test_health_check_unknown_service`: Tests handling of unknown services
- `test_health_watch_stream_creation`: Tests stream creation
- `test_health_watch_unknown_service`: Tests watch for unknown services
- `test_is_service_healthy`: Tests the health check logic

Run tests with:

```bash
cargo test --package bridge-api health --lib
```

## Future Enhancements

Potential improvements for the health watch implementation:

1. **Configurable Health Checks**: Allow custom health check logic per service
2. **Metrics Integration**: Integrate with Prometheus metrics for health monitoring
3. **Health Check Plugins**: Support for external health check providers
4. **Service Discovery**: Automatic discovery of services to monitor
5. **Health Check Dependencies**: Support for service dependency health checks
6. **Health Check Timeouts**: Configurable timeouts for health checks
7. **Health Check Retries**: Configurable retry logic for failed health checks

## Dependencies

The implementation uses the following key dependencies:

- `tonic`: gRPC framework
- `tokio`: Async runtime
- `tracing`: Logging and observability
- `fastrand`: Random number generation for simulated failures
