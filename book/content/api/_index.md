+++
title = "API Reference"
description = "Complete API documentation for Orasi services"
sort_by = "weight"
weight = 1
template = "docs/section.html"
+++

# API Reference

This section provides comprehensive documentation for all Orasi APIs, including REST endpoints, gRPC services, and client libraries.

## API Overview

Orasi provides several APIs for different use cases:

- **Bridge API** - Main API gateway for all Orasi operations
- **Agent API** - Data processing and transformation operations
- **Schema Registry API** - Schema management and validation
- **Query Engine API** - SQL query execution and optimization
- **Streaming Processor API** - Real-time data processing

## Authentication

Most Orasi APIs require authentication. The following authentication methods are supported:

### JWT Authentication

```bash
# Get a JWT token
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "user", "password": "password"}'

# Use the token in subsequent requests
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/endpoint
```

### API Key Authentication

```bash
# Use API key in header
curl -H "X-API-Key: your-api-key" \
  http://localhost:8080/api/v1/endpoint
```

## Common Response Format

All APIs return responses in a consistent format:

```json
{
  "success": true,
  "data": {
    // Response data
  },
  "error": null,
  "timestamp": "2024-01-01T00:00:00Z"
}
```

Error responses:

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid input parameters",
    "details": {
      "field": "Field validation failed"
    }
  },
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## API Versions

Orasi APIs are versioned to ensure backward compatibility:

- **v1** - Current stable API version
- **v2** - Beta API version (subject to change)

To use a specific version, include it in the URL path:

```
http://localhost:8080/api/v1/endpoint
http://localhost:8080/api/v2/endpoint
```

## Rate Limiting

APIs are rate-limited to prevent abuse:

- **Authenticated requests**: 1000 requests per minute
- **Unauthenticated requests**: 100 requests per minute

Rate limit headers are included in responses:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995200
```

## API Documentation

### [Bridge API](/api/bridge/)
Complete documentation for the Bridge service API, including:

- Authentication endpoints
- Health check endpoints
- Request routing and transformation
- Load balancing configuration
- Monitoring and metrics

### [Agent API](/api/agent/)
Documentation for the Agent service API:

- Data ingestion endpoints
- Processing pipeline management
- Transformation configuration
- Error handling and retry logic
- Performance monitoring

### [Schema Registry API](/api/schema-registry/)
Schema management API documentation:

- Schema registration and retrieval
- Version management
- Compatibility checking
- Schema evolution
- Validation endpoints

### [Query Engine API](/api/query-engine/)
SQL query execution API:

- Query submission and execution
- Result streaming
- Query optimization
- Data source management
- Query history and caching

### [Streaming Processor API](/api/streaming-processor/)
Real-time processing API:

- Pipeline creation and management
- Stream processing configuration
- Window-based aggregations
- State management
- Event processing

## Client Libraries

### Rust Client

```rust
use orasi_client::{OrasiClient, Config};

let config = Config::new()
    .with_base_url("http://localhost:8080")
    .with_auth_token("your-token");

let client = OrasiClient::new(config);

// Use the client
let result = client.query_engine()
    .execute_sql("SELECT * FROM table")
    .await?;
```

### Python Client

```python
from orasi_client import OrasiClient

client = OrasiClient(
    base_url="http://localhost:8080",
    auth_token="your-token"
)

# Execute a query
result = client.query_engine.execute_sql("SELECT * FROM table")
```

### JavaScript/TypeScript Client

```typescript
import { OrasiClient } from '@orasi/client';

const client = new OrasiClient({
  baseUrl: 'http://localhost:8080',
  authToken: 'your-token'
});

// Execute a query
const result = await client.queryEngine.executeSql('SELECT * FROM table');
```

## WebSocket APIs

For real-time data streaming, Orasi provides WebSocket endpoints:

```javascript
const ws = new WebSocket('ws://localhost:8080/ws/stream');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};

// Subscribe to a data stream
ws.send(JSON.stringify({
  action: 'subscribe',
  stream: 'sensor-data'
}));
```

## gRPC Services

Orasi also provides gRPC services for high-performance communication:

```protobuf
service OrasiBridge {
  rpc HealthCheck(HealthRequest) returns (HealthResponse);
  rpc ProcessRequest(ProcessRequest) returns (ProcessResponse);
  rpc StreamData(stream DataChunk) returns (stream DataChunk);
}
```

## SDK Documentation

### [Rust SDK](/api/sdk/rust/)
Complete Rust client library documentation with examples.

### [Python SDK](/api/sdk/python/)
Python client library documentation and usage examples.

### [JavaScript SDK](/api/sdk/javascript/)
JavaScript/TypeScript client library documentation.

## OpenAPI/Swagger

Interactive API documentation is available at:

- **Bridge API**: `http://localhost:8080/docs`
- **Agent API**: `http://localhost:8081/docs`
- **Schema Registry**: `http://localhost:8082/docs`

## Testing APIs

### Using curl

```bash
# Health check
curl http://localhost:8080/health

# Authenticated request
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/endpoint

# POST request with JSON
curl -X POST http://localhost:8080/api/v1/endpoint \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"key": "value"}'
```

### Using Postman

Import the Orasi API collection from:
`https://github.com/your-org/orasi/blob/main/docs/postman/Orasi-API.postman_collection.json`
