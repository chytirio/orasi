+++
title = "Examples"
description = "Practical examples and tutorials for Orasi"
sort_by = "weight"
weight = 1
template = "docs/section.html"
+++

# Examples and Tutorials

This section provides practical examples and tutorials to help you get started with Orasi. Each example includes complete code, configuration, and step-by-step instructions.

## Quick Start Examples

### [Basic Setup](/examples/basic-setup/)
Get Orasi running locally with a simple data pipeline.

**What you'll learn:**
- Installing and configuring Orasi
- Starting the Bridge and Agent services
- Creating your first data pipeline
- Basic data ingestion and processing

### [Hello World Pipeline](/examples/hello-world/)
Create a simple "Hello World" data processing pipeline.

**What you'll learn:**
- Basic pipeline configuration
- Data transformation
- Output formatting
- Error handling

## Data Ingestion Examples

### [Kafka to Delta Lake](/examples/kafka-to-delta/)
Stream data from Kafka and write to Delta Lake tables.

**What you'll learn:**
- Kafka connector configuration
- Delta Lake integration
- Streaming data processing
- Schema evolution

### [S3 Parquet Processing](/examples/s3-parquet/)
Process Parquet files stored in Amazon S3.

**What you'll learn:**
- S3 connector setup
- Parquet file reading and writing
- Batch processing
- Data partitioning

### [Real-time Sensor Data](/examples/sensor-data/)
Process real-time sensor data with windowing and aggregation.

**What you'll learn:**
- Real-time data processing
- Time-based windowing
- Data aggregation
- State management

## Data Processing Examples

### [ETL Pipeline](/examples/etl-pipeline/)
Complete ETL (Extract, Transform, Load) pipeline example.

**What you'll learn:**
- Data extraction from multiple sources
- Complex transformations
- Data validation
- Error handling and retry logic

### [Data Quality Checks](/examples/data-quality/)
Implement data quality checks and validation.

**What you'll learn:**
- Schema validation
- Data quality rules
- Anomaly detection
- Quality metrics

### [Data Enrichment](/examples/data-enrichment/)
Enrich data with external sources and lookups.

**What you'll learn:**
- External API integration
- Data joining and enrichment
- Caching strategies
- Performance optimization

## Analytics Examples

### [SQL Analytics](/examples/sql-analytics/)
Use SQL to analyze data with the Query Engine.

**What you'll learn:**
- SQL query execution
- Data source integration
- Query optimization
- Result visualization

### [Streaming Analytics](/examples/streaming-analytics/)
Real-time analytics with streaming data.

**What you'll learn:**
- Stream processing
- Real-time aggregations
- Event-time processing
- Backpressure handling

### [Time Series Analysis](/examples/time-series/)
Analyze time series data with advanced windowing.

**What you'll learn:**
- Time series processing
- Sliding and tumbling windows
- Time-based aggregations
- Trend analysis

## Integration Examples

### [Kubernetes Deployment](/examples/kubernetes/)
Deploy Orasi on Kubernetes with Helm charts.

**What you'll learn:**
- Kubernetes deployment
- Service configuration
- Scaling and load balancing
- Monitoring and logging

### [Docker Compose](/examples/docker-compose/)
Run Orasi with Docker Compose for development.

**What you'll learn:**
- Docker containerization
- Service orchestration
- Development environment setup
- Local testing

### [CI/CD Pipeline](/examples/cicd/)
Set up continuous integration and deployment.

**What you'll learn:**
- Automated testing
- Deployment automation
- Configuration management
- Monitoring integration

## Advanced Examples

### [Multi-tenant Architecture](/examples/multi-tenant/)
Build a multi-tenant data platform with Orasi.

**What you'll learn:**
- Tenant isolation
- Resource management
- Security and access control
- Performance optimization

### [High Availability](/examples/high-availability/)
Deploy Orasi for high availability and fault tolerance.

**What you'll learn:**
- Load balancing
- Failover strategies
- Data replication
- Disaster recovery

### [Performance Tuning](/examples/performance-tuning/)
Optimize Orasi for high-performance workloads.

**What you'll learn:**
- Performance profiling
- Resource optimization
- Caching strategies
- Scaling techniques

## Example Categories

### [Data Connectors](/examples/connectors/)
Examples for each supported data connector:

- [Kafka Connector](/examples/connectors/kafka/)
- [Delta Lake Connector](/examples/connectors/delta-lake/)
- [Iceberg Connector](/examples/connectors/iceberg/)
- [Hudi Connector](/examples/connectors/hudi/)
- [S3 Parquet Connector](/examples/connectors/s3-parquet/)
- [Snowflake Connector](/examples/connectors/snowflake/)

### [Use Cases](/examples/use-cases/)
Industry-specific examples:

- [IoT Data Processing](/examples/use-cases/iot/)
- [Financial Data Analytics](/examples/use-cases/financial/)
- [E-commerce Analytics](/examples/use-cases/ecommerce/)
- [Log Analysis](/examples/use-cases/logs/)
- [Machine Learning Pipelines](/examples/use-cases/ml/)

### [Client Libraries](/examples/clients/)
Examples using different client libraries:

- [Rust Client](/examples/clients/rust/)
- [Python Client](/examples/clients/python/)
- [JavaScript Client](/examples/clients/javascript/)
- [Go Client](/examples/clients/go/)

## Running Examples

### Prerequisites

Before running any example, ensure you have:

1. **Orasi installed** - Follow the [Getting Started](/getting-started/) guide
2. **Required dependencies** - Check each example's prerequisites
3. **Data sources configured** - Set up any required external services
4. **Permissions** - Ensure proper access to data sources and sinks

### Example Structure

Each example follows this structure:

```
examples/example-name/
├── README.md              # Example description and instructions
├── config/                # Configuration files
│   ├── bridge.toml
│   └── agent.toml
├── data/                  # Sample data files
├── scripts/               # Setup and run scripts
└── src/                   # Source code (if applicable)
```

### Running an Example

```bash
# Navigate to the example directory
cd examples/example-name

# Read the README for specific instructions
cat README.md

# Run the setup script
./scripts/setup.sh

# Start the example
./scripts/run.sh

# Clean up when done
./scripts/cleanup.sh
```

## Contributing Examples

We welcome contributions! To add a new example:

1. Create a new directory in `examples/`
2. Follow the example structure above
3. Include a comprehensive README
4. Add setup and cleanup scripts
5. Test the example thoroughly
6. Submit a pull request

## Getting Help

If you encounter issues with examples:

1. Check the [troubleshooting guide](/troubleshooting/)
2. Review the example's README for known issues
3. Search existing [GitHub issues](https://github.com/your-org/orasi/issues)
4. Ask for help in our [Discord community](https://discord.gg/orasi)
