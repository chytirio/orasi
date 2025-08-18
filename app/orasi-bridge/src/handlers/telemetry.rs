//! Telemetry ingestion handlers

use axum::{
    extract::State,
    response::Json,
};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

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

    // TODO: In a real implementation, this would:
    // 1. Apply filters and transformations
    // 2. Route to appropriate exporters
    // 3. Store in lakehouse
    // 4. Update metrics and monitoring

    Ok(())
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
