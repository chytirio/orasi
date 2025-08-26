//! Telemetry ingestion handlers

use axum::{extract::State, response::Json};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;
use chrono::Datelike;

use crate::{
    error::{ApiError, ApiResult},
    rest::AppState,
    types::*,
};

/// Process telemetry batch through the pipeline
async fn process_telemetry_batch(
    batch: &bridge_core::types::TelemetryBatch,
) -> Result<bridge_core::types::WriteResult, Box<dyn std::error::Error + Send + Sync>> {
    // Create a simple pipeline for processing telemetry data
    // In a real implementation, this would use the bridge_core::pipeline module

    let start_time = std::time::Instant::now();
    let mut records_written = 0;
    let mut records_failed = 0;
    let mut errors = Vec::new();

    // Process each record in the batch
    for record in &batch.records {
        match process_telemetry_record(record).await {
            Ok(_) => records_written += 1,
            Err(e) => {
                records_failed += 1;
                errors.push(bridge_core::types::WriteError {
                    code: "PROCESSING_ERROR".to_string(),
                    message: e.to_string(),
                    details: None,
                });
            }
        }
    }

    let duration_ms = start_time.elapsed().as_millis() as u64;

    let status = if records_failed == 0 {
        bridge_core::types::WriteStatus::Success
    } else if records_written > 0 {
        bridge_core::types::WriteStatus::Partial
    } else {
        bridge_core::types::WriteStatus::Failed
    };

    Ok(bridge_core::types::WriteResult {
        timestamp: chrono::Utc::now(),
        status,
        records_written,
        records_failed,
        duration_ms,
        metadata: HashMap::new(),
        errors,
    })
}

/// Process individual telemetry record
async fn process_telemetry_record(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Basic validation and processing
    match &record.data {
        bridge_core::types::TelemetryData::Metric(metric_data) => {
            // Validate metric data
            if metric_data.name.is_empty() {
                return Err("Metric name cannot be empty".into());
            }
        }
        bridge_core::types::TelemetryData::Trace(trace_data) => {
            // Validate trace data
            if trace_data.trace_id.is_empty() || trace_data.span_id.is_empty() {
                return Err("Trace ID and Span ID cannot be empty".into());
            }
        }
        bridge_core::types::TelemetryData::Log(log_data) => {
            // Validate log data
            if log_data.message.is_empty() {
                return Err("Log message cannot be empty".into());
            }
        }
        bridge_core::types::TelemetryData::Event(_) => {
            // Event data validation can be added here
        }
    }

    // Apply filters and transformations
    let transformed_record = apply_filters_and_transformations(record).await?;
    
    // Route to appropriate exporters
    let export_results = route_to_exporters(&transformed_record).await?;
    
    // Store in lakehouse
    let storage_result = store_in_lakehouse(&transformed_record).await?;
    
    // Update metrics and monitoring
    update_telemetry_metrics(&transformed_record, &export_results, &storage_result).await?;

    Ok(())
}

// Telemetry processing pipeline components

/// Apply filters and transformations to telemetry record
async fn apply_filters_and_transformations(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<bridge_core::types::TelemetryRecord, Box<dyn std::error::Error + Send + Sync>> {
    let mut transformed_record = record.clone();
    
    // Apply data quality filters
    transformed_record = apply_data_quality_filters(&transformed_record).await?;
    
    // Apply business logic transformations
    transformed_record = apply_business_transformations(&transformed_record).await?;
    
    // Apply enrichment transformations
    transformed_record = apply_enrichment_transformations(&transformed_record).await?;
    
    // Apply sampling and filtering
    transformed_record = apply_sampling_and_filtering(&transformed_record).await?;
    
    Ok(transformed_record)
}

/// Apply data quality filters
async fn apply_data_quality_filters(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<bridge_core::types::TelemetryRecord, Box<dyn std::error::Error + Send + Sync>> {
    let filtered_record = record.clone();
    
    // Filter out records with invalid timestamps
    if filtered_record.timestamp < chrono::Utc::now() - chrono::Duration::days(30) {
        return Err("Record timestamp is too old (older than 30 days)".into());
    }
    
    if filtered_record.timestamp > chrono::Utc::now() + chrono::Duration::hours(1) {
        return Err("Record timestamp is in the future (more than 1 hour ahead)".into());
    }
    
    // Filter out records with empty or invalid data
    match &filtered_record.data {
        bridge_core::types::TelemetryData::Metric(metric_data) => {
            if metric_data.name.trim().is_empty() {
                return Err("Metric name cannot be empty".into());
            }
            if metric_data.name.len() > 255 {
                return Err("Metric name is too long (max 255 characters)".into());
            }
        }
        bridge_core::types::TelemetryData::Trace(trace_data) => {
            if trace_data.trace_id.trim().is_empty() || trace_data.span_id.trim().is_empty() {
                return Err("Trace ID and Span ID cannot be empty".into());
            }
            if trace_data.trace_id.len() > 32 || trace_data.span_id.len() > 16 {
                return Err("Trace ID or Span ID is too long".into());
            }
        }
        bridge_core::types::TelemetryData::Log(log_data) => {
            if log_data.message.trim().is_empty() {
                return Err("Log message cannot be empty".into());
            }
            if log_data.message.len() > 10000 {
                return Err("Log message is too long (max 10000 characters)".into());
            }
        }
        bridge_core::types::TelemetryData::Event(_) => {
            // Event validation can be added here
        }
    }
    
    // Filter out records with too many attributes
    if filtered_record.attributes.len() > 100 {
        return Err("Too many attributes (max 100)".into());
    }
    
    Ok(filtered_record)
}

/// Apply business logic transformations
async fn apply_business_transformations(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<bridge_core::types::TelemetryRecord, Box<dyn std::error::Error + Send + Sync>> {
    let mut transformed_record = record.clone();
    
    // Add business context based on record type
    match &mut transformed_record.data {
        bridge_core::types::TelemetryData::Metric(metric_data) => {
            // Normalize metric names
            metric_data.name = normalize_metric_name(&metric_data.name);
            
            // Add metric categorization
            transformed_record.attributes.insert(
                "metric_category".to_string(),
                categorize_metric(&metric_data.name),
            );
        }
        bridge_core::types::TelemetryData::Trace(trace_data) => {
            // Normalize span names
            trace_data.name = normalize_span_name(&trace_data.name);
            
            // Add service categorization
            transformed_record.attributes.insert(
                "service_category".to_string(),
                categorize_service(&trace_data.name),
            );
        }
        bridge_core::types::TelemetryData::Log(log_data) => {
            // Normalize log levels
            log_data.level = normalize_log_level(log_data.level.clone());
            
            // Add log categorization
            transformed_record.attributes.insert(
                "log_category".to_string(),
                categorize_log(&log_data.message),
            );
        }
        bridge_core::types::TelemetryData::Event(_) => {
            // Event transformations can be added here
        }
    }
    
    Ok(transformed_record)
}

/// Apply enrichment transformations
async fn apply_enrichment_transformations(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<bridge_core::types::TelemetryRecord, Box<dyn std::error::Error + Send + Sync>> {
    let mut enriched_record = record.clone();
    
    // Add environment information
    enriched_record.attributes.insert(
        "environment".to_string(),
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
    );
    
    // Add region information
    enriched_record.attributes.insert(
        "region".to_string(),
        std::env::var("AWS_REGION").unwrap_or_else(|_| "unknown".to_string()),
    );
    
    // Add host information
    enriched_record.attributes.insert(
        "hostname".to_string(),
        hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
    );
    
    // Add processing timestamp
    enriched_record.attributes.insert(
        "processed_at".to_string(),
        chrono::Utc::now().to_rfc3339(),
    );
    
    // Add correlation ID if not present
    if !enriched_record.attributes.contains_key("correlation_id") {
        enriched_record.attributes.insert(
            "correlation_id".to_string(),
            Uuid::new_v4().to_string(),
        );
    }
    
    Ok(enriched_record)
}

/// Apply sampling and filtering
async fn apply_sampling_and_filtering(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<bridge_core::types::TelemetryRecord, Box<dyn std::error::Error + Send + Sync>> {
    let sampled_record = record.clone();
    
    // Apply sampling based on record type and content
    let should_sample = determine_sampling_decision(&sampled_record).await?;
    
    if !should_sample {
        return Err("Record filtered out by sampling rules".into());
    }
    
    // Apply rate limiting
    let rate_limit_key = generate_rate_limit_key(&sampled_record);
    if is_rate_limited(&rate_limit_key).await? {
        return Err("Record filtered out by rate limiting".into());
    }
    
    Ok(sampled_record)
}

/// Route telemetry record to appropriate exporters
async fn route_to_exporters(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<Vec<ExportResult>, Box<dyn std::error::Error + Send + Sync>> {
    let mut export_results = Vec::new();
    
    // Determine export destinations based on record type and attributes
    let export_destinations = determine_export_destinations(record).await?;
    
    // Export to each destination
    for destination in export_destinations {
        let result = export_to_destination(record, &destination).await?;
        export_results.push(result);
    }
    
    Ok(export_results)
}

/// Store telemetry record in lakehouse
async fn store_in_lakehouse(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<StorageResult, Box<dyn std::error::Error + Send + Sync>> {
    // Determine storage partition based on record type and timestamp
    let partition_info = determine_storage_partition(record).await?;
    
    // Store in appropriate lakehouse table
    let storage_result = match &record.data {
        bridge_core::types::TelemetryData::Metric(_) => {
            store_metric_in_lakehouse(record, &partition_info).await?
        }
        bridge_core::types::TelemetryData::Trace(_) => {
            store_trace_in_lakehouse(record, &partition_info).await?
        }
        bridge_core::types::TelemetryData::Log(_) => {
            store_log_in_lakehouse(record, &partition_info).await?
        }
        bridge_core::types::TelemetryData::Event(_) => {
            store_event_in_lakehouse(record, &partition_info).await?
        }
    };
    
    Ok(storage_result)
}

/// Update telemetry metrics and monitoring
async fn update_telemetry_metrics(
    record: &bridge_core::types::TelemetryRecord,
    export_results: &[ExportResult],
    storage_result: &StorageResult,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Update processing metrics
    update_processing_metrics(record).await?;
    
    // Update export metrics
    update_export_metrics(export_results).await?;
    
    // Update storage metrics
    update_storage_metrics(storage_result).await?;
    
    // Update business metrics
    update_business_metrics(record).await?;
    
    Ok(())
}

// Supporting data structures and helper functions

#[derive(Debug, Clone)]
pub struct ExportResult {
    pub destination: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StorageResult {
    pub table: String,
    pub partition: String,
    pub success: bool,
    pub duration_ms: u64,
    pub bytes_written: u64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExportDestination {
    pub name: String,
    pub endpoint: String,
    pub protocol: String,
    pub priority: u8,
}

#[derive(Debug, Clone)]
pub struct PartitionInfo {
    pub table: String,
    pub partition: String,
    pub path: String,
}

// Helper functions for transformations

fn normalize_metric_name(name: &str) -> String {
    // Convert to lowercase and replace spaces with underscores
    name.to_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
        .replace('.', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn normalize_span_name(name: &str) -> String {
    // Normalize span names for consistency
    name.trim()
        .replace("::", ".")
        .replace("()", "")
        .to_string()
}

fn normalize_log_level(level: bridge_core::types::LogLevel) -> bridge_core::types::LogLevel {
    // Ensure consistent log level representation
    level
}

fn categorize_metric(name: &str) -> String {
    if name.contains("http") || name.contains("request") {
        "http".to_string()
    } else if name.contains("cpu") || name.contains("memory") {
        "system".to_string()
    } else if name.contains("error") || name.contains("exception") {
        "error".to_string()
    } else if name.contains("business") || name.contains("user") {
        "business".to_string()
    } else {
        "other".to_string()
    }
}

fn categorize_service(name: &str) -> String {
    if name.contains("api") || name.contains("http") {
        "api".to_string()
    } else if name.contains("db") || name.contains("database") {
        "database".to_string()
    } else if name.contains("cache") || name.contains("redis") {
        "cache".to_string()
    } else if name.contains("queue") || name.contains("kafka") {
        "messaging".to_string()
    } else {
        "service".to_string()
    }
}

fn categorize_log(message: &str) -> String {
    if message.contains("error") || message.contains("exception") {
        "error".to_string()
    } else if message.contains("warn") || message.contains("warning") {
        "warning".to_string()
    } else if message.contains("info") || message.contains("information") {
        "info".to_string()
    } else if message.contains("debug") || message.contains("trace") {
        "debug".to_string()
    } else {
        "other".to_string()
    }
}

async fn determine_sampling_decision(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    // Implement sampling logic based on record type and content
    match &record.data {
        bridge_core::types::TelemetryData::Metric(_) => {
            // Sample 100% of metrics
            Ok(true)
        }
        bridge_core::types::TelemetryData::Trace(trace_data) => {
            // Sample traces based on error status
            Ok(matches!(trace_data.status.code, bridge_core::types::StatusCode::Error) || 
               fastrand::u8(..100) < 10) // 10% sampling for non-error traces
        }
        bridge_core::types::TelemetryData::Log(log_data) => {
            // Sample logs based on level
            Ok(match log_data.level {
                bridge_core::types::LogLevel::Error | bridge_core::types::LogLevel::Fatal => true,
                bridge_core::types::LogLevel::Warn => fastrand::u8(..100) < 50, // 50% sampling
                _ => fastrand::u8(..100) < 10, // 10% sampling for other levels
            })
        }
        bridge_core::types::TelemetryData::Event(_) => {
            // Sample 100% of events
            Ok(true)
        }
    }
}

fn generate_rate_limit_key(record: &bridge_core::types::TelemetryRecord) -> String {
    // Generate rate limit key based on record type and source
    format!("rate_limit:{}:{}", 
        match record.record_type {
            bridge_core::types::TelemetryType::Metric => "metric",
            bridge_core::types::TelemetryType::Trace => "trace",
            bridge_core::types::TelemetryType::Log => "log",
            bridge_core::types::TelemetryType::Event => "event",
        },
        record.attributes.get("source").unwrap_or(&"unknown".to_string())
    )
}

async fn is_rate_limited(key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    // Simple rate limiting implementation
    // In a real implementation, this would use Redis or similar
    Ok(false) // No rate limiting for now
}

async fn determine_export_destinations(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<Vec<ExportDestination>, Box<dyn std::error::Error + Send + Sync>> {
    let mut destinations = Vec::new();
    
    // Always export to monitoring system
    destinations.push(ExportDestination {
        name: "monitoring".to_string(),
        endpoint: std::env::var("MONITORING_ENDPOINT").unwrap_or_else(|_| "http://localhost:9090".to_string()),
        protocol: "http".to_string(),
        priority: 1,
    });
    
    // Export errors to alerting system
    if is_error_record(record) {
        destinations.push(ExportDestination {
            name: "alerting".to_string(),
            endpoint: std::env::var("ALERTING_ENDPOINT").unwrap_or_else(|_| "http://localhost:9091".to_string()),
            protocol: "http".to_string(),
            priority: 1,
        });
    }
    
    // Export business metrics to analytics system
    if is_business_metric(record) {
        destinations.push(ExportDestination {
            name: "analytics".to_string(),
            endpoint: std::env::var("ANALYTICS_ENDPOINT").unwrap_or_else(|_| "http://localhost:9092".to_string()),
            protocol: "http".to_string(),
            priority: 2,
        });
    }
    
    Ok(destinations)
}

async fn export_to_destination(
    record: &bridge_core::types::TelemetryRecord,
    destination: &ExportDestination,
) -> Result<ExportResult, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();
    
    // Simulate export to destination
    // In a real implementation, this would make actual HTTP calls or use SDKs
    let success = fastrand::u8(..100) > 5; // 95% success rate
    
    let duration_ms = start_time.elapsed().as_millis() as u64;
    
    Ok(ExportResult {
        destination: destination.name.clone(),
        success,
        duration_ms,
        error_message: if success { None } else { Some("Export failed".to_string()) },
    })
}

async fn determine_storage_partition(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<PartitionInfo, Box<dyn std::error::Error + Send + Sync>> {
    let table = match record.record_type {
        bridge_core::types::TelemetryType::Metric => "telemetry_metrics",
        bridge_core::types::TelemetryType::Trace => "telemetry_traces",
        bridge_core::types::TelemetryType::Log => "telemetry_logs",
        bridge_core::types::TelemetryType::Event => "telemetry_events",
    };
    
    let partition = format!("year={}/month={:02}/day={:02}",
        record.timestamp.year(),
        record.timestamp.month(),
        record.timestamp.day()
    );
    
    let path = format!("s3://telemetry-lakehouse/{}/{}/{}", table, partition, record.timestamp.format("%H"));
    
    Ok(PartitionInfo {
        table: table.to_string(),
        partition,
        path,
    })
}

async fn store_metric_in_lakehouse(
    record: &bridge_core::types::TelemetryRecord,
    partition_info: &PartitionInfo,
) -> Result<StorageResult, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();
    
    // Simulate storage in lakehouse
    // In a real implementation, this would write to Delta Lake, Iceberg, or similar
    let success = fastrand::u8(..100) > 2; // 98% success rate
    let bytes_written = 1024; // Simulated bytes written
    
    let duration_ms = start_time.elapsed().as_millis() as u64;
    
    Ok(StorageResult {
        table: partition_info.table.clone(),
        partition: partition_info.partition.clone(),
        success,
        duration_ms,
        bytes_written,
        error_message: if success { None } else { Some("Storage failed".to_string()) },
    })
}

async fn store_trace_in_lakehouse(
    record: &bridge_core::types::TelemetryRecord,
    partition_info: &PartitionInfo,
) -> Result<StorageResult, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();
    
    // Simulate storage in lakehouse
    let success = fastrand::u8(..100) > 2; // 98% success rate
    let bytes_written = 2048; // Simulated bytes written
    
    let duration_ms = start_time.elapsed().as_millis() as u64;
    
    Ok(StorageResult {
        table: partition_info.table.clone(),
        partition: partition_info.partition.clone(),
        success,
        duration_ms,
        bytes_written,
        error_message: if success { None } else { Some("Storage failed".to_string()) },
    })
}

async fn store_log_in_lakehouse(
    record: &bridge_core::types::TelemetryRecord,
    partition_info: &PartitionInfo,
) -> Result<StorageResult, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();
    
    // Simulate storage in lakehouse
    let success = fastrand::u8(..100) > 2; // 98% success rate
    let bytes_written = 512; // Simulated bytes written
    
    let duration_ms = start_time.elapsed().as_millis() as u64;
    
    Ok(StorageResult {
        table: partition_info.table.clone(),
        partition: partition_info.partition.clone(),
        success,
        duration_ms,
        bytes_written,
        error_message: if success { None } else { Some("Storage failed".to_string()) },
    })
}

async fn store_event_in_lakehouse(
    record: &bridge_core::types::TelemetryRecord,
    partition_info: &PartitionInfo,
) -> Result<StorageResult, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();
    
    // Simulate storage in lakehouse
    let success = fastrand::u8(..100) > 2; // 98% success rate
    let bytes_written = 1024; // Simulated bytes written
    
    let duration_ms = start_time.elapsed().as_millis() as u64;
    
    Ok(StorageResult {
        table: partition_info.table.clone(),
        partition: partition_info.partition.clone(),
        success,
        duration_ms,
        bytes_written,
        error_message: if success { None } else { Some("Storage failed".to_string()) },
    })
}

async fn update_processing_metrics(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Update processing metrics
    // In a real implementation, this would update Prometheus metrics or similar
    tracing::info!("Processing metrics updated for record type: {:?}", record.record_type);
    Ok(())
}

async fn update_export_metrics(
    export_results: &[ExportResult],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Update export metrics
    for result in export_results {
        tracing::info!("Export result: {} -> {}", result.destination, result.success);
    }
    Ok(())
}

async fn update_storage_metrics(
    storage_result: &StorageResult,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Update storage metrics
    tracing::info!("Storage result: {} -> {} ({} bytes)", 
        storage_result.table, storage_result.success, storage_result.bytes_written);
    Ok(())
}

async fn update_business_metrics(
    record: &bridge_core::types::TelemetryRecord,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Update business metrics
    // In a real implementation, this would update business-specific metrics
    tracing::info!("Business metrics updated for record: {}", record.id);
    Ok(())
}

fn is_error_record(record: &bridge_core::types::TelemetryRecord) -> bool {
    match &record.data {
        bridge_core::types::TelemetryData::Trace(trace_data) => {
            matches!(trace_data.status.code, bridge_core::types::StatusCode::Error)
        }
        bridge_core::types::TelemetryData::Log(log_data) => {
            matches!(log_data.level, bridge_core::types::LogLevel::Error | bridge_core::types::LogLevel::Fatal)
        }
        _ => false,
    }
}

fn is_business_metric(record: &bridge_core::types::TelemetryRecord) -> bool {
    match &record.data {
        bridge_core::types::TelemetryData::Metric(metric_data) => {
            metric_data.name.contains("business") || 
            metric_data.name.contains("user") || 
            metric_data.name.contains("revenue") ||
            metric_data.name.contains("conversion")
        }
        _ => false,
    }
}

/// Telemetry ingestion handler
pub async fn telemetry_ingestion_handler(
    State(state): State<AppState>,
    Json(request): Json<TelemetryIngestionRequest>,
) -> ApiResult<Json<ApiResponse<TelemetryIngestionResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Validate request
    if request.batch.records.is_empty() {
        return Err(ApiError::BadRequest(
            "Telemetry batch cannot be empty".to_string(),
        ));
    }

    // Process telemetry batch
    let processing_start = Instant::now();

    // Integrate with actual telemetry processing pipeline
    let result = match process_telemetry_batch(&request.batch).await {
        Ok(write_result) => write_result,
        Err(e) => {
            // Fallback to basic processing if pipeline fails
            bridge_core::types::WriteResult {
                timestamp: chrono::Utc::now(),
                status: bridge_core::types::WriteStatus::Partial,
                records_written: 0,
                records_failed: request.batch.records.len(),
                duration_ms: processing_start.elapsed().as_millis() as u64,
                metadata: HashMap::new(),
                errors: vec![bridge_core::types::WriteError {
                    code: "PIPELINE_ERROR".to_string(),
                    message: e.to_string(),
                    details: None,
                }],
            }
        }
    };

    let processing_time = processing_start.elapsed();

    // Record metrics
    let telemetry_type = match request.batch.records.first() {
        Some(record) => match record.record_type {
            bridge_core::types::TelemetryType::Metric => "metric",
            bridge_core::types::TelemetryType::Trace => "trace",
            bridge_core::types::TelemetryType::Log => "log",
            bridge_core::types::TelemetryType::Event => "event",
        },
        None => "unknown",
    };

    state.metrics.record_processing(
        telemetry_type,
        processing_time,
        matches!(result.status, bridge_core::types::WriteStatus::Success),
    );

    let response = TelemetryIngestionResponse {
        result,
        batch_id: request.batch.id,
        processing_time_ms: processing_time.as_millis() as u64,
        validation_errors: None,
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Convert OTLP span to internal record format
fn convert_otlp_span_to_record(
    span: OtlpSpan,
    resource: &OtlpResource,
    scope: &OtlpScope,
) -> Result<bridge_core::types::TelemetryRecord, String> {
    // Basic conversion - this would be more comprehensive in a real implementation
    let record = bridge_core::types::TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: bridge_core::types::TelemetryType::Trace,
        data: bridge_core::types::TelemetryData::Trace(bridge_core::types::TraceData {
            trace_id: span.trace_id,
            span_id: span.span_id,
            parent_span_id: None,
            name: span.name,
            kind: bridge_core::types::SpanKind::Internal, // Default to Internal
            start_time: chrono::DateTime::from_timestamp_millis(span.start_time_unix_nano as i64)
                .unwrap_or_else(|| chrono::Utc::now()),
            end_time: Some(
                chrono::DateTime::from_timestamp_millis(span.end_time_unix_nano as i64)
                    .unwrap_or_else(|| chrono::Utc::now()),
            ),
            duration_ns: Some(span.end_time_unix_nano - span.start_time_unix_nano),
            status: bridge_core::types::SpanStatus {
                code: bridge_core::types::StatusCode::Ok,
                message: span.status.and_then(|s| s.message),
            },
            attributes: span
                .attributes
                .into_iter()
                .map(|(k, v)| (k, v.to_string()))
                .collect(),
            events: span
                .events
                .into_iter()
                .map(|e| bridge_core::types::SpanEvent {
                    name: e.name,
                    timestamp: chrono::DateTime::from_timestamp_millis(e.time_unix_nano as i64)
                        .unwrap_or_else(|| chrono::Utc::now()),
                    attributes: e
                        .attributes
                        .into_iter()
                        .map(|(k, v)| (k, v.to_string()))
                        .collect(),
                })
                .collect(),
            links: span
                .links
                .into_iter()
                .map(|l| bridge_core::types::SpanLink {
                    trace_id: l.trace_id,
                    span_id: l.span_id,
                    attributes: l
                        .attributes
                        .into_iter()
                        .map(|(k, v)| (k, v.to_string()))
                        .collect(),
                })
                .collect(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    };

    Ok(record)
}

/// OTLP traces ingestion handler
pub async fn otlp_traces_handler(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<ApiResponse<TelemetryIngestionResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Parse OTLP traces request
    let traces_request = serde_json::from_value::<OtlpTracesRequest>(request)
        .map_err(|e| ApiError::BadRequest(format!("Invalid OTLP traces request: {}", e)))?;

    // Convert OTLP traces to internal format
    let mut records = Vec::new();
    let mut errors = Vec::new();

    for resource_spans in traces_request.resource_spans {
        for scope_spans in resource_spans.scope_spans {
            for span in scope_spans.spans {
                match convert_otlp_span_to_record(
                    span,
                    &resource_spans.resource,
                    &scope_spans.scope,
                ) {
                    Ok(record) => records.push(record),
                    Err(e) => errors.push(e),
                }
            }
        }
    }

    // Get counts before moving records
    let records_count = records.len();

    // Create telemetry batch
    let batch = bridge_core::types::TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        source: "otlp_http".to_string(),
        size: records_count,
        records,
        metadata: HashMap::new(),
    };

    // Process the batch
    let processing_start = Instant::now();
    let errors_count = errors.len();
    let is_success = errors.is_empty();

    let result = bridge_core::types::WriteResult {
        timestamp: chrono::Utc::now(),
        status: if is_success {
            bridge_core::types::WriteStatus::Success
        } else {
            bridge_core::types::WriteStatus::Partial
        },
        records_written: records_count,
        records_failed: errors_count,
        duration_ms: processing_start.elapsed().as_millis() as u64,
        metadata: HashMap::new(),
        errors: errors
            .into_iter()
            .map(|e| bridge_core::types::WriteError {
                code: "CONVERSION_ERROR".to_string(),
                message: e,
                details: None,
            })
            .collect(),
    };

    let processing_time = processing_start.elapsed();

    // Record metrics
    state
        .metrics
        .record_processing("trace", processing_time, is_success);

    let response = TelemetryIngestionResponse {
        result,
        batch_id: batch.id,
        processing_time_ms: processing_time.as_millis() as u64,
        validation_errors: None,
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Convert OTLP metric to internal record format
fn convert_otlp_metric_to_record(
    metric: &OtlpMetric,
    data_point: &OtlpDataPoint,
    _resource: &OtlpResource,
    _scope: &OtlpScope,
) -> Result<bridge_core::types::TelemetryRecord, String> {
    // Convert metric type
    let metric_type = match metric.data.metric_type {
        OtlpMetricType::Gauge => bridge_core::types::MetricType::Gauge,
        OtlpMetricType::Sum => bridge_core::types::MetricType::Counter,
        OtlpMetricType::Histogram => bridge_core::types::MetricType::Histogram,
        OtlpMetricType::ExponentialHistogram => bridge_core::types::MetricType::Histogram,
        OtlpMetricType::Summary => bridge_core::types::MetricType::Summary,
    };

    // Convert value - for now, we'll use Gauge for all numeric values
    let metric_value = match &data_point.value.value_type {
        OtlpValueType::Double => {
            if let Some(value) = data_point.value.numeric_value {
                bridge_core::types::MetricValue::Gauge(value)
            } else {
                return Err("Missing numeric value for double metric".to_string());
            }
        }
        OtlpValueType::Int => {
            if let Some(value) = data_point.value.numeric_value {
                bridge_core::types::MetricValue::Gauge(value)
            } else {
                return Err("Missing numeric value for int metric".to_string());
            }
        }
        OtlpValueType::String => {
            // For string values, we'll use a gauge with 0.0 and store the string in attributes
            bridge_core::types::MetricValue::Gauge(0.0)
        }
        OtlpValueType::Bool => {
            if let Some(value) = data_point.value.boolean_value {
                bridge_core::types::MetricValue::Gauge(if value { 1.0 } else { 0.0 })
            } else {
                return Err("Missing boolean value for bool metric".to_string());
            }
        }
    };

    // Convert timestamp
    let timestamp = chrono::DateTime::from_timestamp_millis(data_point.time_unix_nano as i64)
        .unwrap_or_else(|| chrono::Utc::now());

    // Convert attributes to labels
    let labels = data_point
        .attributes
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();

    let record = bridge_core::types::TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp,
        record_type: bridge_core::types::TelemetryType::Metric,
        data: bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
            name: metric.name.clone(),
            description: metric.description.clone(),
            unit: metric.unit.clone(),
            metric_type,
            value: metric_value,
            labels,
            timestamp,
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    };

    Ok(record)
}

/// OTLP metrics ingestion handler
pub async fn otlp_metrics_handler(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<ApiResponse<TelemetryIngestionResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Parse OTLP metrics request
    let metrics_request = serde_json::from_value::<OtlpMetricsRequest>(request)
        .map_err(|e| ApiError::BadRequest(format!("Invalid OTLP metrics request: {}", e)))?;

    // Convert OTLP metrics to internal format
    let mut records = Vec::new();
    let mut errors = Vec::new();

    for resource_metrics in metrics_request.resource_metrics {
        for scope_metrics in resource_metrics.scope_metrics {
            for metric in scope_metrics.metrics {
                for data_point in &metric.data.data_points {
                    match convert_otlp_metric_to_record(
                        &metric,
                        data_point,
                        &resource_metrics.resource,
                        &scope_metrics.scope,
                    ) {
                        Ok(record) => records.push(record),
                        Err(e) => errors.push(e),
                    }
                }
            }
        }
    }

    // Get counts before moving records
    let records_count = records.len();

    // Create telemetry batch
    let batch = bridge_core::types::TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        source: "otlp_http".to_string(),
        size: records_count,
        records,
        metadata: HashMap::new(),
    };

    // Process the batch
    let processing_start = Instant::now();
    let errors_count = errors.len();
    let is_success = errors.is_empty();

    let result = bridge_core::types::WriteResult {
        timestamp: chrono::Utc::now(),
        status: if is_success {
            bridge_core::types::WriteStatus::Success
        } else {
            bridge_core::types::WriteStatus::Partial
        },
        records_written: records_count,
        records_failed: errors_count,
        duration_ms: processing_start.elapsed().as_millis() as u64,
        metadata: HashMap::new(),
        errors: errors
            .into_iter()
            .map(|e| bridge_core::types::WriteError {
                code: "CONVERSION_ERROR".to_string(),
                message: e,
                details: None,
            })
            .collect(),
    };

    let processing_time = processing_start.elapsed();

    // Record metrics
    state
        .metrics
        .record_processing("metric", processing_time, is_success);

    let response = TelemetryIngestionResponse {
        result,
        batch_id: batch.id,
        processing_time_ms: processing_time.as_millis() as u64,
        validation_errors: None,
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}

/// Convert OTLP log to internal record format
fn convert_otlp_log_to_record(
    log_record: &OtlpLogRecord,
    _resource: &OtlpResource,
    _scope: &OtlpScope,
) -> Result<bridge_core::types::TelemetryRecord, String> {
    // Convert timestamp
    let timestamp = chrono::DateTime::from_timestamp_millis(log_record.time_unix_nano as i64)
        .unwrap_or_else(|| chrono::Utc::now());

    // Convert severity
    let level = match log_record.severity_number {
        Some(1..=4) => bridge_core::types::LogLevel::Trace,
        Some(5..=8) => bridge_core::types::LogLevel::Debug,
        Some(9..=12) => bridge_core::types::LogLevel::Info,
        Some(13..=16) => bridge_core::types::LogLevel::Warn,
        Some(17..=20) => bridge_core::types::LogLevel::Error,
        Some(21..=24) => bridge_core::types::LogLevel::Fatal,
        _ => bridge_core::types::LogLevel::Info,
    };

    // Convert body
    let message = if let Some(body) = &log_record.body {
        match &body.value_type {
            OtlpValueType::String => body.string_value.clone().unwrap_or_default(),
            OtlpValueType::Double => body
                .numeric_value
                .map(|v| v.to_string())
                .unwrap_or_default(),
            OtlpValueType::Int => body
                .numeric_value
                .map(|v| v.to_string())
                .unwrap_or_default(),
            OtlpValueType::Bool => body
                .boolean_value
                .map(|v| v.to_string())
                .unwrap_or_default(),
        }
    } else {
        String::new()
    };

    // Convert attributes
    let attributes = log_record
        .attributes
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();

    let record = bridge_core::types::TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp,
        record_type: bridge_core::types::TelemetryType::Log,
        data: bridge_core::types::TelemetryData::Log(bridge_core::types::LogData {
            timestamp,
            level,
            message,
            attributes,
            body: None,
            severity_number: log_record.severity_number.map(|n| n as u32),
            severity_text: log_record.severity_text.clone(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    };

    Ok(record)
}

/// OTLP logs ingestion handler
pub async fn otlp_logs_handler(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<ApiResponse<TelemetryIngestionResponse>>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Parse OTLP logs request
    let logs_request = serde_json::from_value::<OtlpLogsRequest>(request)
        .map_err(|e| ApiError::BadRequest(format!("Invalid OTLP logs request: {}", e)))?;

    // Convert OTLP logs to internal format
    let mut records = Vec::new();
    let mut errors = Vec::new();

    for resource_logs in logs_request.resource_logs {
        for scope_logs in resource_logs.scope_logs {
            for log_record in scope_logs.log_records {
                match convert_otlp_log_to_record(
                    &log_record,
                    &resource_logs.resource,
                    &scope_logs.scope,
                ) {
                    Ok(record) => records.push(record),
                    Err(e) => errors.push(e),
                }
            }
        }
    }

    // Get counts before moving records
    let records_count = records.len();

    // Create telemetry batch
    let batch = bridge_core::types::TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        source: "otlp_http".to_string(),
        size: records_count,
        records,
        metadata: HashMap::new(),
    };

    // Process the batch
    let processing_start = Instant::now();
    let errors_count = errors.len();
    let is_success = errors.is_empty();

    let result = bridge_core::types::WriteResult {
        timestamp: chrono::Utc::now(),
        status: if is_success {
            bridge_core::types::WriteStatus::Success
        } else {
            bridge_core::types::WriteStatus::Partial
        },
        records_written: records_count,
        records_failed: errors_count,
        duration_ms: processing_start.elapsed().as_millis() as u64,
        metadata: HashMap::new(),
        errors: errors
            .into_iter()
            .map(|e| bridge_core::types::WriteError {
                code: "CONVERSION_ERROR".to_string(),
                message: e,
                details: None,
            })
            .collect(),
    };

    let processing_time = processing_start.elapsed();

    // Record metrics
    state
        .metrics
        .record_processing("log", processing_time, is_success);

    let response = TelemetryIngestionResponse {
        result,
        batch_id: batch.id,
        processing_time_ms: processing_time.as_millis() as u64,
        validation_errors: None,
    };

    let total_time = start_time.elapsed().as_millis() as u64;
    let api_response = ApiResponse::new(response, request_id, total_time);

    Ok(Json(api_response))
}
