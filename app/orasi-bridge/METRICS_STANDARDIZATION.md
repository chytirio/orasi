# Metrics Standardization

## Overview

This document describes the standardization of metric names in the Bridge API metrics system. All metric names and labels have been centralized into constants to ensure consistency and maintainability.

## Changes Made

### 1. Metric Name Constants

All metric names are now defined in the `metric_names` module:

```rust
pub mod metric_names {
    // API Request/Response Metrics
    pub const API_REQUESTS_TOTAL: &str = "api_requests_total";
    pub const API_RESPONSE_TIME_SECONDS: &str = "api_response_time_seconds";
    pub const API_ACTIVE_CONNECTIONS: &str = "api_active_connections";
    pub const API_ERRORS_TOTAL: &str = "api_errors_total";
    pub const API_PROCESSING_TIME_SECONDS: &str = "api_processing_time_seconds";
    pub const API_PROCESSING_TOTAL: &str = "api_processing_total";

    // Telemetry Metrics
    pub const TELEMETRY_INGESTION_TOTAL: &str = "telemetry_ingestion_total";
    pub const TELEMETRY_BATCH_SIZE: &str = "telemetry_batch_size";
    pub const TELEMETRY_PROCESSING_TIME_SECONDS: &str = "telemetry_processing_time_seconds";
    pub const TELEMETRY_VALIDATION_ERRORS_TOTAL: &str = "telemetry_validation_errors_total";

    // Query Metrics
    pub const QUERY_TOTAL: &str = "query_total";
    pub const QUERY_EXECUTION_TIME_SECONDS: &str = "query_execution_time_seconds";
    pub const QUERY_CACHE_HIT_RATE: &str = "query_cache_hit_rate";
    pub const QUERY_RESULT_COUNT: &str = "query_result_count";

    // System Metrics
    pub const SYSTEM_UPTIME_SECONDS: &str = "system_uptime_seconds";
}
```

### 2. Metric Label Constants

All metric labels are now defined in the `metric_labels` module:

```rust
pub mod metric_labels {
    // Common labels
    pub const METHOD: &str = "method";
    pub const PATH: &str = "path";
    pub const STATUS_CODE: &str = "status_code";
    pub const ERROR_TYPE: &str = "error_type";
    pub const OPERATION: &str = "operation";
    pub const SUCCESS: &str = "success";
    pub const TYPE: &str = "type";
    pub const CACHE_HIT: &str = "cache_hit";
}
```

### 3. Updated Metric Recording

All metric recording calls now use the standardized constants:

```rust
// Before
counter!("api_requests_total", 1, "method" => method.to_string(), "path" => path.to_string());

// After
counter!(
    metric_names::API_REQUESTS_TOTAL,
    1,
    metric_labels::METHOD => method.to_string(),
    metric_labels::PATH => path.to_string()
);
```

### 4. Updated Metrics Output

The `get_metrics()` function now uses the standardized metric names:

```rust
// Before
metrics_text.push_str("# HELP bridge_api_requests_total Total number of requests\n");

// After
metrics_text.push_str(&format!("# HELP {} Total number of requests\n", metric_names::API_REQUESTS_TOTAL));
```

## Benefits

1. **Consistency**: All metric names and labels are centralized and consistent
2. **Maintainability**: Changes to metric names only need to be made in one place
3. **Type Safety**: Compile-time checking of metric name usage
4. **Documentation**: Clear overview of all available metrics
5. **Refactoring**: Easy to rename metrics without searching through code

## Testing

Unit tests have been added to verify:
- All metric name constants are properly defined
- All metric label constants are properly defined
- The `get_metrics()` function uses the standardized constants

## Usage

To use the standardized metrics:

```rust
use crate::metrics::{metric_names, metric_labels, ApiMetrics};

let metrics = ApiMetrics::new();
metrics.record_request("GET", "/api/v1/health", 200);
```

## Future Considerations

- Consider adding metric name validation at compile time
- Add metric documentation generation from constants
- Consider using a macro to generate metric constants from a configuration file
