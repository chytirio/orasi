+++
title = "Getting Started"
description = "Learn how to set up and run Orasi"
sort_by = "weight"
weight = 1
template = "docs/section.html"
+++

# Getting Started with Orasi

Welcome to Orasi! This guide will help you get up and running with the Orasi data platform.

## Prerequisites

Before you begin, make sure you have the following installed:

- **Rust** (1.70 or later) - [Install Rust](https://rustup.rs/)
- **Cargo** (comes with Rust)
- **Git** - [Install Git](https://git-scm.com/)
- **Docker** (optional, for containerized deployment)

## Installation

### Option 1: From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/your-org/orasi.git
cd orasi

# Build all components
cargo build --release

# Install the CLI tool
cargo install --path app/orasi-cli
```

### Option 2: Using Docker

```bash
# Pull the official image
docker pull orasi/orasi:latest

# Run the bridge service
docker run -p 8080:8080 orasi/orasi-bridge:latest

# Run the agent
docker run -p 8081:8081 orasi/orasi-agent:latest
```

## Quick Start

### 1. Start the Bridge Service

The Bridge service acts as the API gateway for Orasi:

```bash
# From the project root
cargo run -p orasi-bridge

# Or using the CLI
orasi bridge start
```

The Bridge service will start on `http://localhost:8080` by default.

### 2. Start the Agent

The Agent handles data processing and transformation:

```bash
# From the project root
cargo run -p orasi-agent

# Or using the CLI
orasi agent start
```

The Agent service will start on `http://localhost:8081` by default.

### 3. Verify Installation

Check that both services are running:

```bash
# Check Bridge health
curl http://localhost:8080/health

# Check Agent health
curl http://localhost:8081/health
```

You should see a JSON response indicating the services are healthy.

## Configuration

Orasi uses configuration files to manage settings. The main configuration files are:

- `config/bridge.toml` - Bridge service configuration
- `config/agent.toml` - Agent service configuration
- `config/schema-registry.toml` - Schema registry configuration

### Basic Configuration Example

```toml
# config/bridge.toml
[server]
host = "0.0.0.0"
port = 8080

[auth]
enabled = true
jwt_secret = "your-secret-key"

[logging]
level = "info"
```

## Next Steps

Now that you have Orasi running, you can:

1. **[Explore Components](/components/)** - Learn about each Orasi component
2. **[Try Examples](/examples/)** - Run through practical examples
3. **[Read API Docs](/api/)** - Understand the available APIs
4. **[Configure Connectors](/components/connectors/)** - Set up data source connections

## Troubleshooting

### Common Issues

**Service won't start:**
- Check if the required ports are available
- Verify your configuration files
- Check the logs for error messages

**Build errors:**
- Ensure you have the latest Rust toolchain
- Run `cargo clean` and try building again
- Check that all dependencies are available

**Connection issues:**
- Verify network connectivity
- Check firewall settings
- Ensure services are running on the expected ports

### Getting Help

If you encounter issues:

1. Check the [troubleshooting guide](/troubleshooting/)
2. Search existing [GitHub issues](https://github.com/your-org/orasi/issues)
3. Join our [Discord community](https://discord.gg/orasi)
4. Create a new issue with detailed information about your problem
