# Maintenance Processing

This directory contains maintenance processing functionality for the Orasi Agent, including cleanup operations, health checks, and database file reference checking.

## Cleanup Processor

The `CleanupProcessor` handles various cleanup operations to maintain system health and free up disk space.

### Supported Cleanup Targets

1. **temp_files** - Removes temporary files older than 24 hours
2. **old_logs** - Removes log files older than 7 days
3. **cache** - Removes cache files older than 1 hour
4. **orphaned_data** - Removes data files older than 30 days that are not referenced in the database

### Database File Reference Checking

The cleanup processor includes sophisticated database file reference checking to ensure that files are not removed if they are still referenced in the system.

#### How It Works

1. **Database Connection**: Connects to the local SQLite database (`agent.db`)
2. **Table Scanning**: Checks multiple tables for file references:
   - `data_files` - Registered data files
   - `processed_files` - Files that have been processed
   - `index_files` - Index files
   - `backup_files` - Backup files
   - `export_files` - Export files
   - `cache_files` - Cache files
   - `temp_files` - Temporary files
   - `file_metadata` - File metadata
   - `file_references` - File reference tracking
   - `data_sources` - Data source configurations
   - `processing_tasks` - Processing task records
   - `indexing_tasks` - Indexing task records
   - `export_tasks` - Export task records

3. **Column Checking**: For each table, checks common column names:
   - `file_path`, `path`, `location`
   - `source_path`, `target_path`
   - `input_path`, `output_path`
   - `data_path`, `file_location`
   - `file_name`, `filename`, `name`

4. **Partial Path Matching**: Also checks for partial path matches to handle relative paths

5. **Safe Default**: If database connection fails, assumes files are referenced (safe default)

#### Implementation Details

```rust
async fn is_file_referenced(&self, path: &Path) -> Result<bool, AgentError> {
    let file_path = path.to_string_lossy();
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");

    // Connect to database
    let db_path = std::path::Path::new(&self.config.storage.local_path).join("agent.db");
    let database_url = format!("sqlite:{}", db_path.display());

    match SqliteConnection::connect(&database_url).await {
        Ok(mut conn) => {
            // Check multiple tables for file references
            let is_referenced = self.check_file_references_in_database(&mut conn, &file_path, file_name).await?;
            Ok(is_referenced)
        }
        Err(e) => {
            // Safe default: assume file is referenced
            warn!("Failed to connect to database for file reference check: {}", e);
            Ok(true)
        }
    }
}
```

### Usage

```rust
use crate::processing::maintenance::cleanup::CleanupProcessor;

let processor = CleanupProcessor::new(&config);

// Perform cleanup on specific targets
let task = MaintenanceTask {
    maintenance_type: "cleanup".to_string(),
    targets: vec!["orphaned_data".to_string(), "temp_files".to_string()],
    options: HashMap::new(),
};

let result = processor.perform_cleanup(&task).await?;
```

### Database Statistics

The processor also provides database statistics for monitoring:

```rust
let stats = processor.get_database_stats().await?;
// Returns counts for various tables
```

### Safety Features

1. **Safe Defaults**: If database connection fails, files are assumed to be referenced
2. **Comprehensive Checking**: Checks multiple tables and column names
3. **Partial Matching**: Handles relative paths and partial matches
4. **Error Handling**: Graceful handling of missing tables or columns
5. **Logging**: Detailed logging of file reference checks and cleanup operations

### Testing

The implementation includes comprehensive tests:

- `test_file_reference_check()` - Tests database file reference checking
- `test_cleanup_orphaned_data()` - Tests orphaned data cleanup
- `test_cleanup_temp_files()` - Tests temporary file cleanup

### Configuration

The cleanup processor uses the following configuration:

- `config.storage.local_path` - Base path for local storage
- Database location: `{local_path}/agent.db`
- Data directory: `{local_path}/data`
- Log directory: `{local_path}/logs`
- Cache directory: `{local_path}/cache`

## Health Check Processor

The `HealthProcessor` provides comprehensive system health monitoring and diagnostics.

### Supported Health Check Targets

1. **system** - System resource monitoring (CPU, memory, disk)
2. **connectivity** - Network and service endpoint connectivity
3. **storage** - Local and backup storage health
4. **database** - Database connectivity and performance

### Network Connectivity Checking

The health processor includes sophisticated network connectivity testing:

#### Features
- **Multiple DNS Server Testing**: Tests connectivity to Google DNS (8.8.8.8), Cloudflare DNS (1.1.1.1), and OpenDNS (208.67.222.222)
- **HTTP Connectivity Testing**: Tests HTTP connectivity to httpbin.org
- **Endpoint Health Checking**: Tests individual service endpoints
- **Timeout Handling**: Configurable timeouts for all connectivity tests
- **Graceful Degradation**: Returns partial connectivity status when some tests fail

#### Implementation Details

```rust
async fn check_network_connectivity(&self) -> Result<String, AgentError> {
    let test_endpoints = vec![
        "8.8.8.8:53",      // Google DNS
        "1.1.1.1:53",      // Cloudflare DNS
        "208.67.222.222:53" // OpenDNS
    ];

    let mut successful_connections = 0;
    let timeout_duration = Duration::from_secs(5);

    for endpoint in test_endpoints {
        match timeout(timeout_duration, self.test_tcp_connection(endpoint)).await {
            Ok(Ok(_)) => successful_connections += 1,
            _ => warn!("Network connectivity test failed for {}", endpoint),
        }
    }

    // Also test HTTP connectivity
    match timeout(timeout_duration, self.test_http_connectivity()).await {
        Ok(Ok(_)) => successful_connections += 1,
        _ => warn!("HTTP connectivity test failed"),
    }

    if successful_connections >= 2 {
        Ok("connected".to_string())
    } else if successful_connections >= 1 {
        Ok("partial".to_string())
    } else {
        Ok("disconnected".to_string())
    }
}
```

### Database Health Checking

The health processor includes comprehensive database health monitoring:

#### Features
- **Connection Testing**: Tests database connectivity with timeout handling
- **Query Performance Testing**: Measures response time for simple queries
- **Database Statistics**: Reports table count, database size, and performance metrics
- **Error Handling**: Graceful handling of connection failures and timeouts

#### Implementation Details

```rust
async fn check_database_connectivity(&self) -> Result<String, AgentError> {
    let db_path = std::path::Path::new(&self.config.storage.local_path).join("agent.db");
    let database_url = format!("sqlite:{}", db_path.display());

    match timeout(Duration::from_secs(5), SqliteConnection::connect(&database_url)).await {
        Ok(Ok(mut conn)) => {
            // Test a simple query to ensure responsiveness
            match timeout(Duration::from_secs(2), sqlx::query("SELECT 1").fetch_one(&mut conn)).await {
                Ok(Ok(_)) => Ok("connected".to_string()),
                _ => Ok("error".to_string()),
            }
        }
        _ => Ok("disconnected".to_string()),
    }
}
```

### Storage Health Monitoring

The health processor monitors both local and backup storage:

#### Local Storage Monitoring
- **Availability Check**: Verifies storage path exists and is accessible
- **Space Monitoring**: Reports total space, available space, and usage percentage
- **Warning Thresholds**: Alerts when storage usage exceeds 90%

#### Backup Storage Monitoring
- **Backup Count**: Counts available backup files
- **Storage Size**: Calculates total backup storage size
- **Recency Check**: Verifies if recent backups exist (within 24 hours)

### System Resource Monitoring

The health processor monitors system resources using `sysinfo`:

- **Memory Usage**: Tracks memory usage percentage
- **CPU Usage**: Monitors CPU utilization
- **Disk Usage**: Checks disk space utilization
- **Warning Thresholds**: Alerts when resource usage exceeds 90%

### Usage

```rust
use crate::processing::maintenance::health::HealthProcessor;

let processor = HealthProcessor::new(&config);

// Perform health check on specific targets
let task = MaintenanceTask {
    maintenance_type: "health_check".to_string(),
    targets: vec!["system".to_string(), "connectivity".to_string(), "database".to_string()],
    options: HashMap::new(),
};

let result = processor.perform_health_check(&task).await?;
```

### Health Status Levels

- **healthy** - All checks passed successfully
- **warning** - Some issues detected but system is operational
- **error** - Critical issues detected
- **disconnected** - Network connectivity issues
- **partial** - Partial connectivity (some tests failed)
- **timeout** - Health checks timed out

### Testing

The implementation includes comprehensive tests:

- `test_network_connectivity_check()` - Tests network connectivity
- `test_database_connectivity_check()` - Tests database connectivity
- `test_storage_health_check()` - Tests storage health monitoring
- `test_system_health_check()` - Tests system resource monitoring
- `test_full_health_check()` - Tests complete health check workflow

### Dependencies

- `sqlx` - Database connectivity and querying
- `reqwest` - HTTP connectivity testing
- `sysinfo` - System resource monitoring
- `serde_json` - JSON result formatting
- `tracing` - Logging and observability
- `tokio` - Async runtime support
- `filetime` - File modification time handling
