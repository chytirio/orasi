+++
title = "Orasi Documentation"
description = "Comprehensive documentation for the Orasi data platform"
template = "index.html"
page_template = "page.html"
+++

# Welcome to Orasi

**Orasi** is a modern, high-performance data platform designed for real-time streaming, batch processing, and analytics at scale. Built with Rust for performance and reliability, Orasi provides a comprehensive suite of tools for data ingestion, processing, storage, and querying.

## 🚀 Key Features

- **Real-time Streaming**: Process data streams with low latency and high throughput
- **Batch Processing**: Efficient batch data processing with support for multiple formats
- **Multi-format Support**: Native support for Parquet, Delta Lake, Iceberg, and Hudi
- **SQL Query Engine**: Powerful SQL querying with DataFusion integration
- **Schema Registry**: Centralized schema management and evolution
- **Authentication & Authorization**: Enterprise-grade security with JWT and role-based access
- **Monitoring & Observability**: Built-in metrics, health checks, and monitoring
- **Cloud Native**: Designed for Kubernetes and cloud environments

## 🏗️ Architecture

Orasi is built as a modular system with several core components:

- **Orasi Bridge**: API gateway and service mesh
- **Orasi Agent**: Data processing and transformation engine
- **Orasi Gateway**: Load balancing and routing
- **Schema Registry**: Schema management and validation
- **Query Engine**: SQL processing and optimization
- **Streaming Processor**: Real-time data processing pipelines

## 📚 Documentation Sections

### [Getting Started](/getting-started/)
Learn how to set up and run Orasi, from installation to your first data pipeline.

### [Components](/components/)
Detailed documentation for each Orasi component and how they work together.

### [API Reference](/api/)
Complete API documentation for all Orasi services and endpoints.

### [Examples](/examples/)
Practical examples and tutorials for common use cases and integrations.

## 🛠️ Quick Start

```bash
# Clone the repository
git clone https://github.com/your-org/orasi.git
cd orasi

# Build the project
cargo build

# Run the bridge service
cargo run -p orasi-bridge

# Run the agent
cargo run -p orasi-agent
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](https://github.com/your-org/orasi/blob/main/CONTRIBUTING.md) for details.

## 📄 License

Orasi is licensed under the MIT License. See the [LICENSE](https://github.com/your-org/orasi/blob/main/LICENSE) file for details.
