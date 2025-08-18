+++
title = "Components"
description = "Learn about Orasi components and architecture"
sort_by = "weight"
weight = 1
template = "docs/section.html"
+++

# Orasi Components

Orasi is built as a modular system with several core components that work together to provide a comprehensive data platform. This section provides detailed documentation for each component.

## Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Orasi Web     │    │  Orasi Gateway  │    │  Orasi Bridge   │
│   (Frontend)    │◄──►│   (Load Bal.)   │◄──►│  (API Gateway)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
                       ┌─────────────────┐             │
                       │ Schema Registry │◄────────────┘
                       │   (Metadata)    │
                       └─────────────────┘
                                │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Orasi Agent   │    │ Query Engine    │    │  Streaming      │
│ (Processing)    │◄──►│ (SQL Engine)    │◄──►│  Processor      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                       │                       │
        ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Data Connectors                              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ │
│  │   Kafka     │ │ Delta Lake  │ │   Iceberg   │ │    Hudi     │ │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### [Orasi Bridge](/components/bridge/)
The API gateway and service mesh that handles all incoming requests, authentication, routing, and load balancing.

**Key Features:**
- REST and gRPC API endpoints
- Authentication and authorization
- Request routing and load balancing
- Rate limiting and throttling
- Request/response transformation
- Health monitoring

### [Orasi Agent](/components/agent/)
The data processing and transformation engine that handles data ingestion, processing, and transformation.

**Key Features:**
- Data ingestion from multiple sources
- Real-time and batch processing
- Data transformation and enrichment
- Schema validation and evolution
- Error handling and retry logic
- Performance monitoring

### [Orasi Gateway](/components/gateway/)
The load balancer and reverse proxy that distributes traffic across multiple instances.

**Key Features:**
- Load balancing algorithms
- Health checking
- SSL/TLS termination
- Request routing
- Connection pooling
- Circuit breaker patterns

### [Schema Registry](/components/schema-registry/)
Centralized schema management and validation service.

**Key Features:**
- Schema storage and versioning
- Schema validation and compatibility checking
- Schema evolution management
- REST API for schema operations
- Integration with data formats

### [Query Engine](/components/query-engine/)
SQL processing and optimization engine built on DataFusion.

**Key Features:**
- SQL query parsing and optimization
- Multiple data source support
- Query caching and optimization
- Distributed query execution
- Integration with streaming data

### [Streaming Processor](/components/streaming-processor/)
Real-time data processing and analytics engine.

**Key Features:**
- Stream processing pipelines
- Window-based aggregations
- State management
- Event-time processing
- Backpressure handling

## Data Connectors

### [Kafka Connector](/components/connectors/kafka/)
Connect to Apache Kafka for streaming data ingestion and publishing.

### [Delta Lake Connector](/components/connectors/delta-lake/)
Read from and write to Delta Lake tables with ACID transactions.

### [Iceberg Connector](/components/connectors/iceberg/)
Connect to Apache Iceberg tables for analytical workloads.

### [Hudi Connector](/components/connectors/hudi/)
Integrate with Apache Hudi for incremental data processing.

### [S3 Parquet Connector](/components/connectors/s3-parquet/)
Read and write Parquet files stored in Amazon S3.

### [Snowflake Connector](/components/connectors/snowflake/)
Connect to Snowflake data warehouse for data exchange.

## Supporting Components

### [Bridge Auth](/components/bridge-auth/)
Authentication and authorization library used by the Bridge service.

### [Bridge Core](/components/bridge-core/)
Core utilities and shared functionality for Bridge components.

### [Ingestion Framework](/components/ingestion/)
Data ingestion framework for handling various data sources and formats.

## Component Communication

Components communicate through:

1. **HTTP/gRPC APIs** - For synchronous request/response patterns
2. **Message Queues** - For asynchronous event-driven communication
3. **Shared Storage** - For configuration and metadata
4. **Health Checks** - For service discovery and monitoring

## Deployment Options

### Monolithic Deployment
All components run in a single process (good for development and testing).

### Microservices Deployment
Each component runs as a separate service (recommended for production).

### Kubernetes Deployment
Containerized deployment with Kubernetes orchestration.

## Configuration

Each component has its own configuration file:

- `bridge.toml` - Bridge service configuration
- `agent.toml` - Agent service configuration
- `gateway.toml` - Gateway service configuration
- `schema-registry.toml` - Schema registry configuration

## Monitoring and Observability

All components provide:

- **Health endpoints** - For service health monitoring
- **Metrics** - Prometheus-compatible metrics
- **Logging** - Structured logging with configurable levels
- **Tracing** - Distributed tracing for request flows
