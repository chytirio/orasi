# Orasi CLI

Command-line interface for the Orasi OpenTelemetry Data Lake Bridge, providing comprehensive management, monitoring, and administration capabilities.

## Overview

Orasi CLI is a powerful command-line tool for managing and interacting with the Orasi bridge ecosystem, offering:

- **Bridge Management**: Start, stop, and manage bridge services
- **Configuration Management**: Create, validate, and update configurations
- **Data Operations**: Query, export, and manage telemetry data
- **Schema Management**: Register, validate, and manage schemas
- **Monitoring**: Real-time monitoring and health checks
- **Development Tools**: Development and debugging utilities
- **Administration**: User management and system administration

## Key Features

### Bridge Management
- **Service Control**: Start, stop, restart, and status check services
- **Cluster Management**: Manage multi-node bridge clusters
- **Configuration Deployment**: Deploy configurations across services
- **Health Monitoring**: Real-time health monitoring and alerts
- **Log Management**: View and manage service logs

### Configuration Management
- **Configuration Creation**: Interactive configuration creation
- **Configuration Validation**: Validate configuration files
- **Configuration Deployment**: Deploy configurations to services
- **Environment Management**: Manage different environments
- **Secret Management**: Secure secret and credential management

### Data Operations
- **Data Querying**: SQL-based data querying and analysis
- **Data Export**: Export data in various formats (JSON, CSV, Parquet)
- **Data Import**: Import data from external sources
- **Data Validation**: Validate data quality and schema compliance
- **Data Migration**: Migrate data between storage systems

### Schema Management
- **Schema Registration**: Register new schemas
- **Schema Validation**: Validate schema definitions
- **Schema Evolution**: Manage schema evolution and migrations
- **Schema Discovery**: Discover and list available schemas
- **Schema Export**: Export schemas in various formats

### Monitoring & Observability
- **Real-time Monitoring**: Real-time system monitoring
- **Metrics Collection**: Collect and display metrics
- **Health Checks**: Comprehensive health checking
- **Alert Management**: Configure and manage alerts
- **Performance Analysis**: Performance analysis and optimization

### Development Tools
- **Debugging**: Debug and troubleshoot issues
- **Testing**: Run tests and validate functionality
- **Profiling**: Performance profiling and analysis
- **Documentation**: Generate and manage documentation
- **Code Generation**: Generate code from schemas

## Quick Start

### Installation

```bash
# Install from source
cargo install --path app/orasi-cli

# Install from crates.io (when available)
cargo install orasi-cli

# Build from source
cargo build --release --bin orasi-cli
```

### Basic Usage

```bash
# Check CLI version
orasi --version

# Get help
orasi --help

# Check bridge status
orasi status

# List all services
orasi services list

# Check service health
orasi health check
```

### Configuration Management

```bash
# Create new configuration
orasi config create

# Validate configuration
orasi config validate config/bridge.toml

# Deploy configuration
orasi config deploy config/bridge.toml

# List configurations
orasi config list
```

### Data Operations

```bash
# Query telemetry data
orasi query "SELECT service.name, COUNT(*) FROM telemetry_data GROUP BY service.name"

# Export data
orasi export --sql "SELECT * FROM telemetry_data WHERE timestamp >= NOW() - INTERVAL '1 hour'" --format json

# Import data
orasi import --file data.json --format json

# Validate data
orasi validate data --file data.json --schema my-schema
```

## Command Reference

### Bridge Management Commands

```bash
# Service management
orasi services start [service]     # Start a service
orasi services stop [service]      # Stop a service
orasi services restart [service]   # Restart a service
orasi services status [service]    # Check service status
orasi services list               # List all services
orasi services logs [service]      # View service logs

# Cluster management
orasi cluster status              # Check cluster status
orasi cluster nodes               # List cluster nodes
orasi cluster join [node]         # Join a node to cluster
orasi cluster leave [node]        # Remove node from cluster

# Configuration management
orasi config create               # Create new configuration
orasi config validate [file]      # Validate configuration
orasi config deploy [file]        # Deploy configuration
orasi config list                 # List configurations
orasi config show [name]          # Show configuration
orasi config update [name]        # Update configuration
orasi config delete [name]        # Delete configuration
```

### Data Operations Commands

```bash
# Query operations
orasi query [sql]                 # Execute SQL query
orasi query --file [file]         # Execute query from file
orasi query --interactive         # Interactive query mode
orasi query --explain [sql]       # Explain query plan

# Export operations
orasi export --sql [sql] --format [format]  # Export query results
orasi export --table [table] --format [format]  # Export table
orasi export --schema [schema] --format [format]  # Export schema

# Import operations
orasi import --file [file] --format [format]  # Import data
orasi import --table [table] --file [file]    # Import to table
orasi import --validate                         # Validate import

# Data management
orasi data list                   # List available data sources
orasi data info [source]          # Get data source info
orasi data validate [source]      # Validate data source
orasi data migrate [source] [target]  # Migrate data
```

### Schema Management Commands

```bash
# Schema operations
orasi schema register [file]      # Register schema
orasi schema validate [file]      # Validate schema
orasi schema list                 # List schemas
orasi schema show [name]          # Show schema
orasi schema update [name] [file] # Update schema
orasi schema delete [name]        # Delete schema
orasi schema export [name] --format [format]  # Export schema

# Schema evolution
orasi schema evolve [name] [file] # Evolve schema
orasi schema migrate [name]       # Migrate schema
orasi schema history [name]       # Show schema history
orasi schema diff [schema1] [schema2]  # Compare schemas
```

### Monitoring Commands

```bash
# Health monitoring
orasi health check                # Check overall health
orasi health check [service]      # Check service health
orasi health status               # Show health status
orasi health watch                # Watch health in real-time

# Metrics monitoring
orasi metrics list                # List available metrics
orasi metrics show [metric]       # Show metric values
orasi metrics watch [metric]      # Watch metric in real-time
orasi metrics export [metric]     # Export metrics

# Alert management
orasi alerts list                 # List alerts
orasi alerts show [alert]         # Show alert details
orasi alerts create [alert]       # Create alert
orasi alerts update [alert]       # Update alert
orasi alerts delete [alert]       # Delete alert
```

### Development Commands

```bash
# Debugging
orasi debug connect [service]     # Connect to service for debugging
orasi debug logs [service]        # View debug logs
orasi debug trace [request]       # Trace request flow
orasi debug profile [service]     # Profile service performance

# Testing
orasi test run                    # Run all tests
orasi test run [test]             # Run specific test
orasi test validate               # Validate test environment
orasi test report                 # Generate test report

# Code generation
orasi generate schema [schema]    # Generate code from schema
orasi generate client [api]       # Generate API client
orasi generate docs [service]     # Generate documentation
```

## Configuration

### CLI Configuration

```toml
# ~/.orasi/config.toml
[cli]
default_environment = "production"
output_format = "table"  # table, json, yaml
color_output = true
interactive_mode = true

[cli.connections]
default_bridge = "http://localhost:8080"
default_gateway = "http://localhost:80"
default_agent = "http://localhost:8081"

[cli.auth]
token_file = "~/.orasi/token"
api_key_file = "~/.orasi/api_key"

[cli.logging]
level = "info"
format = "text"  # text, json
file_path = "~/.orasi/cli.log"
```

### Environment Configuration

```bash
# Environment variables
export ORASI_BRIDGE_URL=http://localhost:8080
export ORASI_GATEWAY_URL=http://localhost:80
export ORASI_AGENT_URL=http://localhost:8081
export ORASI_API_KEY=your-api-key
export ORASI_TOKEN=your-jwt-token
export ORASI_OUTPUT_FORMAT=json
export ORASI_COLOR_OUTPUT=false
```

## Usage Examples

### Service Management

```bash
# Start all services
orasi services start

# Start specific service
orasi services start bridge-api

# Check service status
orasi services status

# View service logs
orasi services logs bridge-api --follow

# Restart service
orasi services restart query-engine
```

### Data Querying

```bash
# Simple query
orasi query "SELECT COUNT(*) FROM telemetry_data"

# Complex query with output
orasi query "
  SELECT 
    service.name,
    COUNT(*) as request_count,
    AVG(duration_ms) as avg_duration
  FROM telemetry_data 
  WHERE timestamp >= NOW() - INTERVAL '1 hour'
  GROUP BY service.name
  ORDER BY request_count DESC
" --format json

# Interactive query mode
orasi query --interactive
```

### Schema Management

```bash
# Register schema
orasi schema register schemas/my-app.yaml

# List schemas
orasi schema list

# Show schema details
orasi schema show my-app-schema

# Export schema
orasi schema export my-app-schema --format json

# Validate schema
orasi schema validate schemas/my-app.yaml
```

### Monitoring

```bash
# Check overall health
orasi health check

# Watch health in real-time
orasi health watch

# Show metrics
orasi metrics show request_count

# Watch metrics
orasi metrics watch cpu_usage --interval 5s

# List alerts
orasi alerts list
```

### Configuration Management

```bash
# Create configuration interactively
orasi config create

# Validate configuration
orasi config validate config/bridge.toml

# Deploy configuration
orasi config deploy config/bridge.toml

# List configurations
orasi config list

# Show configuration
orasi config show production
```

## Development

### Building

```bash
# Build CLI
cargo build --bin orasi-cli

# Build with optimizations
cargo build --release --bin orasi-cli

# Build with features
cargo build --features completions --bin orasi-cli
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_query_command

# Run integration tests
cargo test --test integration

# Run with logging
RUST_LOG=debug cargo test
```

### Development Tools

```bash
# Generate shell completions
cargo run --bin orasi-cli -- completions bash > completions/orasi.bash
cargo run --bin orasi-cli -- completions zsh > completions/_orasi
cargo run --bin orasi-cli -- completions fish > completions/orasi.fish

# Generate documentation
cargo run --bin orasi-cli -- generate-docs

# Run linter
cargo clippy --bin orasi-cli

# Run formatter
cargo fmt --bin orasi-cli
```

## Installation

### From Source

```bash
# Clone repository
git clone https://github.com/chytirio/orasi.git
cd orasi

# Build CLI
cargo build --release --bin orasi-cli

# Install CLI
cargo install --path app/orasi-cli

# Or copy binary
sudo cp target/release/orasi-cli /usr/local/bin/orasi
```

### Docker Installation

```bash
# Build Docker image
docker build -t orasi-cli .

# Run CLI in container
docker run --rm -it orasi-cli --help

# Mount configuration
docker run --rm -it -v ~/.orasi:/root/.orasi orasi-cli status
```

### Package Managers

```bash
# Homebrew (macOS)
brew install orasi-cli

# Cargo (Rust package manager)
cargo install orasi-cli

# Manual installation
# Download binary from releases and add to PATH
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
