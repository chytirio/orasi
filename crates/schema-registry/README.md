# Schema Registry

A comprehensive schema registry for OpenTelemetry telemetry schemas and lakehouse schema evolution, implementing the schema-first approach outlined in OTEP-0243.

## Overview

The Schema Registry provides a centralized service for managing, validating, and evolving telemetry schemas in OpenTelemetry environments. It supports both OpenTelemetry telemetry schemas (as defined in OTEP-0243) and lakehouse schema evolution features.

## Key Features

- **Component Telemetry Schema**: Support for developer-defined telemetry schemas
- **Semantic Convention Registry**: Management of standardized telemetry definitions
- **Resolved Telemetry Schema**: Self-contained, resolved schemas for distribution
- **Schema Evolution**: Backward compatibility checking and schema migration
- **Lakehouse Integration**: Schema management for data lakehouse systems
- **Validation**: Comprehensive schema validation and compatibility checking
- **Registry API**: HTTP API for schema registration and retrieval
- **Storage Backends**: Support for PostgreSQL, SQLite, and Redis storage
- **CLI Tools**: Command-line interface for schema management

## Architecture

The schema registry follows the OpenTelemetry schema-first approach outlined in OTEP-0243:

1. **Component Telemetry Schema**: Created by application/library developers
2. **Semantic Convention Registry**: Standardized telemetry definitions
3. **Resolved Telemetry Schema**: Self-contained schemas for distribution

## Quick Start

### Installation

```bash
cargo install schema-registry
```

### Start the Server

```bash
# Start with memory storage (default)
schema-registry serve

# Start with PostgreSQL
schema-registry serve --storage postgres --database-url "postgres://localhost/schema_registry"

# Start with SQLite
schema-registry serve --storage sqlite --database-url "schema_registry.db"

# Start with Redis
schema-registry serve --storage redis --database-url "redis://localhost:6379"
```

### Using the CLI

```bash
# Validate a schema file
schema-registry validate my-schema.yaml

# Register a schema
schema-registry register my-schema.yaml --url http://localhost:8081

# Get a schema by fingerprint
schema-registry get abc123 --url http://localhost:8081

# List all schemas
schema-registry list --url http://localhost:8081
```

## API Endpoints

### Health Check
```
GET /health
```

### Register Schema
```
POST /schemas
Content-Type: application/json

{
  "schema": {
    "metadata": {
      "name": "my-app-schema",
      "description": "Schema for my application"
    },
    "version": {
      "major": 1,
      "minor": 0,
      "patch": 0
    },
    "signals": [...]
  }
}
```

### Get Schema
```
GET /schemas/{fingerprint}
```

### List Schemas
```
GET /schemas
```

### Delete Schema
```
DELETE /schemas/{fingerprint}
```

## Configuration

The schema registry can be configured via configuration file or environment variables.

### Configuration File (TOML)

```toml
[api]
host = "0.0.0.0"
port = 8081
base_path = "/api/v1"
timeout_secs = 30

[storage]
backend = "postgres"

[storage.postgres]
url = "postgres://localhost/schema_registry"
pool_size = 10
connection_timeout_secs = 30

[validation]
enabled = true
timeout_secs = 30
strict_mode = false

[metrics]
enabled = true
host = "0.0.0.0"
port = 9091
```

### Environment Variables

```bash
export SCHEMA_REGISTRY_API_HOST=0.0.0.0
export SCHEMA_REGISTRY_API_PORT=8081
export SCHEMA_REGISTRY_STORAGE_BACKEND=postgres
export SCHEMA_REGISTRY_POSTGRES_URL=postgres://localhost/schema_registry
```

## Schema Format

### Component Telemetry Schema (YAML)

```yaml
metadata:
  name: my-app-schema
  description: Schema for my application telemetry
  author: "My Team"
  contact: "team@example.com"
  license: "Apache-2.0"

version:
  major: 1
  minor: 0
  patch: 0

imports:
  - name: opentelemetry-semantic-conventions
    version:
      major: 1
      minor: 0
      patch: 0
    uri: "https://opentelemetry.io/schemas/1.0.0"

resources:
  - name: service
    description: Service resource
    attributes:
      - name: service.name
        attribute_type: String
        required: true
        description: Service name

signals:
  - signal_type: Metric
    name: http_requests_total
    description: Total HTTP requests
    data:
      Metric:
        name: http_requests_total
        description: Total HTTP requests
        unit: "requests"
        metric_type: Counter
        attributes:
          - name: method
            attribute_type: String
            required: true
          - name: status_code
            attribute_type: Integer
            required: true
```

## Storage Backends

### PostgreSQL

PostgreSQL is the recommended storage backend for production use. It provides ACID compliance, complex queries, and excellent performance.

```bash
schema-registry serve --storage postgres --database-url "postgres://user:pass@localhost/schema_registry"
```

### SQLite

SQLite is perfect for development and small deployments. It requires no server setup and stores data in a single file.

```bash
schema-registry serve --storage sqlite --database-url "schema_registry.db"
```

### Redis

Redis provides high-performance caching and is suitable for read-heavy workloads. Note that Redis has limited query capabilities.

```bash
schema-registry serve --storage redis --database-url "redis://localhost:6379"
```

### Memory

Memory storage is useful for testing and development. Data is lost when the server restarts.

```bash
schema-registry serve --storage memory
```

## Development

### Building from Source

```bash
git clone https://github.com/chytirio/orasi.git
cd orasi/crates/schema-registry
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Running with Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/schema-registry /usr/local/bin/
EXPOSE 8081
CMD ["schema-registry", "serve"]
```

## Integration with Lakehouse Connectors

The schema registry integrates with the lakehouse connectors to provide schema evolution capabilities:

```rust
use schema_registry::{SchemaRegistry, ComponentSchema};

// Create schema registry
let registry = SchemaRegistry::new(config).await?;

// Register a component schema
let component_schema = ComponentSchema::from_file("my-schema.yaml")?;
let resolved_schema = registry.register_schema(component_schema).await?;

// Use the resolved schema with lakehouse connectors
lakehouse_connector.write_with_schema(&resolved_schema, data).await?;
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the Apache 2.0 License - see the LICENSE file for details.

## References

- [OTEP-0243: Application Telemetry Schema](https://github.com/open-telemetry/oteps/blob/main/text/0243-application-telemetry-schema.md)
- [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)
- [Schema-First Application Telemetry](https://research.facebook.com/publications/positional-paper-schema-first-application-telemetry/)
