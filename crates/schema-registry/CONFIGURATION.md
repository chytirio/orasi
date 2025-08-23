# Schema Registry Configuration Guide

This document provides comprehensive guidance for configuring the Orasi Schema Registry.

## Quick Start

### 1. Generate Configuration File

```bash
# Generate a basic configuration file
cargo run -- config generate --output schema-registry.toml

# Generate YAML configuration
cargo run -- config generate --output schema-registry.yaml --format yaml

# Generate JSON configuration
cargo run -- config generate --output schema-registry.json --format json
```

### 2. Validate Configuration

```bash
# Validate your configuration file
cargo run -- config validate --config schema-registry.toml
```

### 3. Start the Server

```bash
# Start with configuration file
cargo run -- serve --config schema-registry.toml

# Start with environment variables only
SCHEMA_REGISTRY__API__PORT=8080 cargo run -- serve
```

## Configuration Sources

The Schema Registry supports multiple configuration sources with the following precedence (highest to lowest):

1. **Environment Variables** (prefixed with `SCHEMA_REGISTRY__`)
2. **Configuration File** (TOML, YAML, or JSON)
3. **Default Values**

### Environment Variables

All configuration options can be set using environment variables with the `SCHEMA_REGISTRY__` prefix and double underscores (`__`) as separators:

```bash
# API Configuration
export SCHEMA_REGISTRY__API__HOST=0.0.0.0
export SCHEMA_REGISTRY__API__PORT=8080

# Storage Configuration
export SCHEMA_REGISTRY__STORAGE__BACKEND=Postgres
export SCHEMA_REGISTRY__STORAGE__POSTGRES__URL="postgresql://user:pass@localhost:5432/schema_registry"

# Security Configuration
export SCHEMA_REGISTRY__SECURITY__ENABLE_AUTH=true
export SCHEMA_REGISTRY__SECURITY__AUTH_TYPE=ApiKey
export SCHEMA_REGISTRY__SECURITY__ALLOWED_API_KEYS="key1,key2,key3"
```

### Configuration Files

The Schema Registry supports multiple configuration file formats:

- **TOML** (recommended): `schema-registry.toml`
- **YAML**: `schema-registry.yaml`
- **JSON**: `schema-registry.json`

## Configuration Options

### Storage Configuration

#### Backend Types

- **Memory**: In-memory storage (for testing)
- **Sqlite**: SQLite database storage
- **Postgres**: PostgreSQL database storage
- **Redis**: Redis key-value storage

#### Example Configurations

**Memory Storage (Development)**
```toml
[storage]
backend = "Memory"
connection_string = "memory://"
```

**SQLite Storage (Development/Testing)**
```toml
[storage]
backend = "Sqlite"
connection_string = "sqlite:///schema_registry.db"

[storage.sqlite]
database_path = "schema_registry.db"
enable_wal = true
connection_timeout = 30
journal_mode = "WAL"
```

**PostgreSQL Storage (Production)**
```toml
[storage]
backend = "Postgres"
connection_string = "postgresql://user:pass@localhost:5432/schema_registry"

[storage.postgres]
url = "postgresql://user:pass@localhost:5432/schema_registry"
database = "schema_registry"
username = "schema_registry_user"
password = "secure_password_here"
host = "localhost"
port = 5432
ssl_mode = "require"
pool_size = 20
```

**Redis Storage (Production)**
```toml
[storage]
backend = "Redis"
connection_string = "redis://localhost:6379"

[storage.redis]
url = "redis://localhost:6379"
host = "localhost"
port = 6379
password = ""
database = 0
pool_size = 10
connection_timeout = 30
read_timeout = 10
write_timeout = 10
key_prefix = "schema_registry:"
```

### API Configuration

```toml
[api]
host = "0.0.0.0"           # API server host
port = 8080                # API server port
base_path = "/api/v1"      # API base path
max_request_size = 52428800  # 50MB max request size
request_timeout = 60       # Request timeout in seconds
enable_cors = true         # Enable CORS
cors_origins = ["*"]       # Allowed CORS origins
```

### Security Configuration

```toml
[security]
enable_auth = true                    # Enable authentication
auth_type = "ApiKey"                  # Authentication type: None, ApiKey, Jwt, OAuth2
api_key_header = "X-API-Key"          # API key header name
allowed_api_keys = [                  # List of valid API keys
    "prod-api-key-1",
    "prod-api-key-2",
    "prod-api-key-3"
]
rate_limiting = true                  # Enable rate limiting
rate_limit_per_minute = 100           # Rate limit requests per minute
```

### Validation Configuration

```toml
[validation]
enable_validation = true              # Enable schema validation
strict_mode = true                    # Enable strict validation mode
max_schema_size = 5242880             # 5MB max schema size
allowed_formats = [                   # Allowed schema formats
    "Json",
    "Yaml", 
    "Avro",
    "Protobuf"
]
custom_rules = {}                     # Custom validation rules
```

### Monitoring Configuration

```toml
[monitoring]
enable_metrics = true                 # Enable Prometheus metrics
metrics_endpoint = "/metrics"         # Metrics endpoint
enable_health_checks = true           # Enable health checks
health_check_endpoint = "/health"     # Health check endpoint
enable_structured_logging = true      # Enable structured logging
log_level = "info"                    # Log level: debug, info, warn, error
log_format = "Json"                   # Log format: Json, Text
```

## Configuration Management CLI

The Schema Registry provides a comprehensive CLI for configuration management:

### Generate Configuration

```bash
# Generate basic configuration
cargo run -- config generate --output schema-registry.toml

# Generate in different formats
cargo run -- config generate --output schema-registry.yaml --format yaml
cargo run -- config generate --output schema-registry.json --format json
```

### Validate Configuration

```bash
# Validate configuration file
cargo run -- config validate --config schema-registry.toml
```

### Show Current Configuration

```bash
# Show configuration from file
cargo run -- config show --config schema-registry.toml

# Show configuration with environment variables
SCHEMA_REGISTRY__API__PORT=9090 cargo run -- config show
```

### View Documentation

```bash
# Show configuration documentation
cargo run -- config docs --format text

# Show documentation in markdown
cargo run -- config docs --format markdown
```

## Production Configuration Examples

### Basic Production Setup

```toml
version = "1.0.0"

[storage]
backend = "Postgres"
connection_string = "postgresql://user:pass@localhost:5432/schema_registry"

[storage.postgres]
url = "postgresql://user:pass@localhost:5432/schema_registry"
database = "schema_registry"
username = "schema_registry_user"
password = "secure_password_here"
host = "localhost"
port = 5432
ssl_mode = "require"
pool_size = 20

[api]
host = "0.0.0.0"
port = 8080
base_path = "/api/v1"
max_request_size = 52428800  # 50MB
request_timeout = 60
enable_cors = true
cors_origins = ["https://your-domain.com"]

[security]
enable_auth = true
auth_type = "ApiKey"
api_key_header = "X-API-Key"
allowed_api_keys = ["prod-api-key-1", "prod-api-key-2"]
rate_limiting = true
rate_limit_per_minute = 100

[validation]
enable_validation = true
strict_mode = true
max_schema_size = 5242880  # 5MB
allowed_formats = ["Json", "Yaml", "Avro", "Protobuf"]

[monitoring]
enable_metrics = true
metrics_endpoint = "/metrics"
enable_health_checks = true
health_check_endpoint = "/health"
enable_structured_logging = true
log_level = "info"
log_format = "Json"
```

### Kubernetes Deployment

For Kubernetes deployments, use environment variables:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: schema-registry
spec:
  template:
    spec:
      containers:
      - name: schema-registry
        image: orasi/schema-registry:latest
        env:
        - name: SCHEMA_REGISTRY__API__HOST
          value: "0.0.0.0"
        - name: SCHEMA_REGISTRY__API__PORT
          value: "8080"
        - name: SCHEMA_REGISTRY__STORAGE__BACKEND
          value: "Postgres"
        - name: SCHEMA_REGISTRY__STORAGE__POSTGRES__URL
          valueFrom:
            secretKeyRef:
              name: schema-registry-secret
              key: database-url
        - name: SCHEMA_REGISTRY__SECURITY__ENABLE_AUTH
          value: "true"
        - name: SCHEMA_REGISTRY__SECURITY__AUTH_TYPE
          value: "ApiKey"
        - name: SCHEMA_REGISTRY__SECURITY__ALLOWED_API_KEYS
          valueFrom:
            secretKeyRef:
              name: schema-registry-secret
              key: api-keys
        ports:
        - containerPort: 8080
```

### Docker Compose

```yaml
version: '3.8'
services:
  schema-registry:
    image: orasi/schema-registry:latest
    ports:
      - "8080:8080"
    environment:
      - SCHEMA_REGISTRY__API__HOST=0.0.0.0
      - SCHEMA_REGISTRY__API__PORT=8080
      - SCHEMA_REGISTRY__STORAGE__BACKEND=Postgres
      - SCHEMA_REGISTRY__STORAGE__POSTGRES__URL=postgresql://user:pass@postgres:5432/schema_registry
      - SCHEMA_REGISTRY__SECURITY__ENABLE_AUTH=true
      - SCHEMA_REGISTRY__SECURITY__AUTH_TYPE=ApiKey
      - SCHEMA_REGISTRY__SECURITY__ALLOWED_API_KEYS=key1,key2,key3
    depends_on:
      - postgres

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=schema_registry
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

## Troubleshooting

### Common Issues

1. **Configuration not loading**: Ensure the configuration file path is correct and the file exists
2. **Environment variables not working**: Use double underscores (`__`) as separators, not single underscores
3. **Validation errors**: Check that all required fields are present and have valid values
4. **Database connection issues**: Verify database credentials and network connectivity

### Debug Configuration

```bash
# Show current configuration with debug logging
RUST_LOG=debug cargo run -- config show --config schema-registry.toml

# Validate configuration with detailed output
cargo run -- config validate --config schema-registry.toml
```

### Configuration Validation

The Schema Registry validates configuration at startup and will fail to start if:

- Required fields are missing
- Invalid values are provided
- Database connections cannot be established
- Security configuration is invalid

## Best Practices

1. **Use environment variables for secrets**: Never store passwords or API keys in configuration files
2. **Validate configuration**: Always validate configuration before deployment
3. **Use appropriate storage backends**: Use Memory for development, PostgreSQL for production
4. **Enable security features**: Always enable authentication and rate limiting in production
5. **Monitor and log**: Enable metrics and structured logging for production deployments
6. **Backup configuration**: Keep configuration files in version control (excluding secrets)
7. **Test configuration**: Test configuration changes in a staging environment first

## Migration Guide

### Upgrading Configuration

When upgrading the Schema Registry, check for:

1. **New required fields**: Add any new required configuration options
2. **Deprecated options**: Remove or update deprecated configuration options
3. **Breaking changes**: Review release notes for any breaking changes
4. **New features**: Consider enabling new features for better functionality

### Configuration Migration Tools

```bash
# Validate existing configuration
cargo run -- config validate --config old-config.toml

# Generate new configuration with current defaults
cargo run -- config generate --output new-config.toml

# Compare configurations
diff old-config.toml new-config.toml
```
