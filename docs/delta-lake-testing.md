# Delta Lake Testing Platform

This document describes the Docker Compose-based testing platform for the Orasi Delta Lake integration.

## Overview

The testing platform consists of:
- **MinIO**: S3-compatible object storage for Delta Lake data
- **Apache Spark**: Distributed computing engine with Delta Lake support
- **Spark Worker**: Worker node for Spark cluster
- **Spark History Server**: Job history and monitoring
- **Jupyter Notebook**: Interactive development and testing environment

## Quick Start

### 1. Start the Testing Platform

```bash
# Quick setup and test (recommended)
just delta-quickstart

# Or step by step:
# Start all services
just delta-up

# Check service status
just delta-status

# View logs
just delta-logs-follow
```

### 2. Access Services

| Service | URL | Description |
|---------|-----|-------------|
| MinIO Console | http://localhost:9001 | S3-compatible storage management |
| Spark Master | http://localhost:8080 | Spark cluster management |
| Spark Worker | http://localhost:8081 | Spark worker monitoring |
| Spark History | http://localhost:18080 | Job history and monitoring |
| Jupyter Notebook | http://localhost:8888 | Interactive development |

**Default Credentials:**
- MinIO: `minioadmin` / `minioadmin`

### 3. Initialize MinIO Buckets

```bash
# Initialize buckets automatically
just delta-init-buckets

# Or manually:
docker exec orasi-minio mc alias set myminio http://localhost:9000 minioadmin minioadmin
docker exec orasi-minio mc mb myminio/test-bucket
docker exec orasi-minio mc mb myminio/spark-logs
```

## Testing Delta Lake Integration

### 1. Using Jupyter Notebook

1. Open Jupyter Notebook at http://localhost:8888
2. Navigate to `notebooks/delta_lake_testing.ipynb`
3. Run the cells to test Delta Lake functionality

### 2. Using Python Scripts

```python
from pyspark.sql import SparkSession
from delta.tables import DeltaTable

# Initialize Spark with Delta Lake support
spark = SparkSession.builder \
    .appName("DeltaLakeTest") \
    .master("spark://spark-master:7077") \
    .config("spark.sql.extensions", "io.delta.sql.DeltaSparkSessionExtension") \
    .config("spark.sql.catalog.spark_catalog", "org.apache.spark.sql.delta.catalog.DeltaCatalog") \
    .config("spark.hadoop.fs.s3a.endpoint", "http://minio:9000") \
    .config("spark.hadoop.fs.s3a.access.key", "minioadmin") \
    .config("spark.hadoop.fs.s3a.secret.key", "minioadmin") \
    .config("spark.hadoop.fs.s3a.path.style.access", "true") \
    .config("spark.hadoop.fs.s3a.impl", "org.apache.hadoop.fs.s3a.S3AFileSystem") \
    .getOrCreate()

# Test Delta Lake operations
data = [("test1", 1), ("test2", 2)]
df = spark.createDataFrame(data, ["name", "value"])

# Write to Delta Lake
df.write.format("delta").mode("overwrite").save("s3a://test-bucket/delta-table")

# Read from Delta Lake
result = spark.read.format("delta").load("s3a://test-bucket/delta-table")
result.show()
```

### 3. Testing with Orasi Connector

```bash
# Run the Delta Lake test service
just delta-run-test

# Or run specific tests
just delta-run-test-cmd "python -c 'from pyspark.sql import SparkSession; print(\"Test completed\")'"

# Execute Python script
just delta-run-python your_script.py

# Execute shell command
just delta-run-shell "pip install pandas && python -c 'import pandas; print(\"Pandas installed\")'"
```

## Configuration

### Spark Configuration

The Spark configuration is defined in `config/spark-defaults.conf`:

```properties
# Spark configuration for Delta Lake testing
spark.master                     spark://spark-master:7077
spark.driver.memory              2g
spark.executor.memory            2g
spark.executor.cores             2
spark.sql.extensions             io.delta.sql.DeltaSparkSessionExtension
spark.sql.catalog.spark_catalog  org.apache.spark.sql.delta.catalog.DeltaCatalog

# S3/MinIO configuration
spark.hadoop.fs.s3a.endpoint                http://minio:9000
spark.hadoop.fs.s3a.access.key              minioadmin
spark.hadoop.fs.s3a.secret.key              minioadmin
spark.hadoop.fs.s3a.path.style.access       true
spark.hadoop.fs.s3a.impl                    org.apache.hadoop.fs.s3a.S3AFileSystem
```

### Environment Variables

Key environment variables for testing:

```bash
# MinIO/S3 Configuration
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin
AWS_DEFAULT_REGION=us-east-1
AWS_ENDPOINT_URL=http://minio:9000

# Spark Configuration
SPARK_MASTER=spark://spark-master:7077
```

## Testing Scenarios

### 1. Basic Delta Lake Operations

- Create Delta Lake tables
- Write data in various formats (Parquet, JSON, CSV)
- Read data with different query patterns
- Test partitioning and optimization

### 2. Telemetry Data Testing

- Generate sample OpenTelemetry data
- Write telemetry data to Delta Lake
- Test time-based queries and aggregations
- Validate data consistency and ACID properties

### 3. Performance Testing

- Test large dataset ingestion
- Measure query performance
- Test concurrent read/write operations
- Validate Delta Lake optimizations

### 4. Integration Testing

- Test Orasi connector with Delta Lake
- Validate data flow from ingestion to storage
- Test error handling and recovery
- Validate schema evolution

## Monitoring and Debugging

### 1. Spark Web UI

- **Master UI**: http://localhost:8080
  - View cluster status
  - Monitor applications
  - Check resource usage

- **Worker UI**: http://localhost:8081
  - View worker status
  - Monitor executor metrics
  - Check task execution

- **History Server**: http://localhost:18080
  - View completed applications
  - Analyze job performance
  - Debug failed jobs

### 2. MinIO Console

- **URL**: http://localhost:9001
- **Credentials**: minioadmin / minioadmin
- **Features**:
  - Browse buckets and objects
  - Monitor storage usage
  - Manage access policies
  - View access logs

### 3. Logs

```bash
# View all logs
just delta-logs

# View specific service logs
just delta-logs-service spark-master
just delta-logs-service spark-worker
just delta-logs-service minio

# Follow logs in real-time
just delta-logs-follow

# Follow specific service logs
just delta-logs-follow-service spark-master
```

## Troubleshooting

### Common Issues

1. **Services not starting**
   ```bash
   # Check service health
   just delta-status
   
   # View detailed logs
   just delta-logs-service <service-name>
   
   # Restart services
   just delta-restart
   ```

2. **Spark connection issues**
   ```bash
   # Check Spark master status
   curl http://localhost:8080
   
   # Verify worker registration
   curl http://localhost:8081
   ```

3. **MinIO access issues**
   ```bash
   # Test MinIO connectivity
   curl http://localhost:9000/minio/health/live
   
   # Check bucket creation
   docker exec orasi-minio mc ls myminio
   ```

4. **Delta Lake configuration issues**
   ```bash
   # Verify Spark configuration
   docker exec orasi-spark-master cat /opt/bitnami/spark/conf/spark-defaults.conf
   
   # Check Delta Lake JARs
   docker exec orasi-spark-master ls /opt/bitnami/spark/jars/ | grep delta
   ```

### Performance Optimization

1. **Memory Configuration**
   ```properties
   spark.driver.memory=4g
   spark.executor.memory=4g
   spark.executor.cores=4
   ```

2. **Delta Lake Optimizations**
   ```properties
   spark.sql.adaptive.enabled=true
   spark.sql.adaptive.coalescePartitions.enabled=true
   spark.sql.adaptive.skewJoin.enabled=true
   ```

3. **S3/MinIO Optimizations**
   ```properties
   spark.hadoop.fs.s3a.connection.maximum=100
   spark.hadoop.fs.s3a.threads.max=20
   spark.hadoop.fs.s3a.block.size=134217728
   ```

## Cleanup

### Stop Services

```bash
# Stop all services
just delta-down

# Stop and remove volumes (WARNING: This will delete all data)
just delta-down-clean
```

### Clean Data

```bash
# Clean all data and volumes
just delta-clean-all

# Or clean specific components:
# Remove MinIO data
docker volume rm orasi_minio_data

# Remove Spark data
docker volume rm orasi_spark_data

# Remove local data directories
rm -rf data/*
rm -rf notebooks/.ipynb_checkpoints
```

## Development Workflow

1. **Start the testing platform**
   ```bash
   just delta-up
   ```

2. **Develop and test Delta Lake functionality**
   - Use Jupyter Notebook for interactive development
   - Write automated tests in Python
   - Test Orasi connector integration

3. **Monitor and debug**
   - Use Spark Web UI for performance monitoring
   - Check MinIO console for storage management
   - Review logs for debugging

4. **Iterate and improve**
   - Refine configurations based on test results
   - Optimize performance settings
   - Add new test scenarios

## Justfile Commands

The project includes comprehensive Justfile targets for easy management of the Delta Lake testing platform:

### Quick Commands
- `just delta-quickstart` - Complete setup and test in one command
- `just delta-setup` - Full setup and initialization
- `just delta-test` - Run Delta Lake tests
- `just delta-info` - Show platform information

### Service Management
- `just delta-up` - Start all services
- `just delta-down` - Stop all services
- `just delta-restart` - Restart all services
- `just delta-status` - Show service status

### Logging and Monitoring
- `just delta-logs` - View all logs
- `just delta-logs-follow` - Follow logs in real-time
- `just delta-logs-service <service>` - View specific service logs
- `just delta-test-minio` - Test MinIO connectivity
- `just delta-test-spark` - Test Spark connectivity

### Browser Access
- `just delta-open-all` - Open all service UIs in browser
- `just delta-minio` - Open MinIO console
- `just delta-spark` - Open Spark Master UI
- `just delta-jupyter` - Open Jupyter Notebook

### Testing and Development
- `just delta-run-test` - Run Delta Lake test container
- `just delta-run-python <script>` - Execute Python script
- `just delta-run-shell <cmd>` - Execute shell command
- `just delta-init-buckets` - Initialize MinIO buckets

### Cleanup
- `just delta-clean-all` - Full cleanup (data + volumes)
- `just delta-clean-data` - Clean local data
- `just delta-clean-volumes` - Clean Docker volumes

### Examples
```bash
# Quick start
just delta-quickstart

# Development workflow
just delta-up
just delta-logs-follow-service spark-master
just delta-run-python test_script.py
just delta-down

# Testing workflow
just delta-setup
just delta-test
just delta-info
```

## Integration with Orasi

The testing platform is designed to work seamlessly with the Orasi Delta Lake connector:

1. **Configuration**: Use the same S3/MinIO configuration as the testing platform
2. **Data Flow**: Test end-to-end data flow from OpenTelemetry ingestion to Delta Lake storage
3. **Validation**: Verify data consistency and query performance
4. **Development**: Use the platform for connector development and testing

For more information about the Orasi Delta Lake connector, see the [connector documentation](../crates/connectors/deltalake/README.md).
