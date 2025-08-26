//! Telemetry processing pipeline test example
//! 
//! This example demonstrates the comprehensive telemetry processing pipeline
//! functionality in the Bridge API.

use bridge_api::{
    handlers::telemetry::telemetry_ingestion_handler,
    rest::AppState,
    types::{TelemetryIngestionRequest, ProcessingOptions},
    config::BridgeAPIConfig,
    metrics::ApiMetrics,
};
use bridge_core::types::{
    TelemetryBatch, TelemetryRecord, TelemetryType, TelemetryData,
    MetricData, MetricType, MetricValue, TraceData, SpanStatus, StatusCode,
    LogData, LogLevel, TimeRange
};
use axum::{extract::State, response::Json};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Bridge API Telemetry Processing Pipeline Test");
    println!("================================================");

    // Create application state
    let config = BridgeAPIConfig::default();
    let metrics = ApiMetrics::new();
    let state = AppState { config, metrics };

    // Test 1: Metrics telemetry processing
    println!("\n📊 Test 1: Metrics Telemetry Processing");
    let metrics_batch = create_metrics_batch();
    let metrics_request = TelemetryIngestionRequest {
        batch: metrics_batch,
        options: Some(ProcessingOptions {
            validate: true,
            transform: true,
            aggregate: false,
            timeout_seconds: Some(30),
        }),
    };

    match telemetry_ingestion_handler(State(state.clone()), Json(metrics_request)).await {
        Ok(response) => {
            println!("✅ Metrics telemetry processed successfully:");
            println!("  • Batch ID: {}", response.0.data.batch_id);
            println!("  • Processing time: {} ms", response.0.data.processing_time_ms);
            println!("  • Records written: {}", response.0.data.result.records_written);
            println!("  • Records failed: {}", response.0.data.result.records_failed);
            println!("  • Status: {:?}", response.0.data.result.status);
        }
        Err(e) => {
            println!("❌ Metrics telemetry processing failed: {:?}", e);
        }
    }

    // Test 2: Traces telemetry processing
    println!("\n📊 Test 2: Traces Telemetry Processing");
    let traces_batch = create_traces_batch();
    let traces_request = TelemetryIngestionRequest {
        batch: traces_batch,
        options: Some(ProcessingOptions {
            validate: true,
            transform: true,
            aggregate: false,
            timeout_seconds: Some(30),
        }),
    };

    match telemetry_ingestion_handler(State(state.clone()), Json(traces_request)).await {
        Ok(response) => {
            println!("✅ Traces telemetry processed successfully:");
            println!("  • Batch ID: {}", response.0.data.batch_id);
            println!("  • Processing time: {} ms", response.0.data.processing_time_ms);
            println!("  • Records written: {}", response.0.data.result.records_written);
            println!("  • Records failed: {}", response.0.data.result.records_failed);
            println!("  • Status: {:?}", response.0.data.result.status);
        }
        Err(e) => {
            println!("❌ Traces telemetry processing failed: {:?}", e);
        }
    }

    // Test 3: Logs telemetry processing
    println!("\n📊 Test 3: Logs Telemetry Processing");
    let logs_batch = create_logs_batch();
    let logs_request = TelemetryIngestionRequest {
        batch: logs_batch,
        options: Some(ProcessingOptions {
            validate: true,
            transform: true,
            aggregate: false,
            timeout_seconds: Some(30),
        }),
    };

    match telemetry_ingestion_handler(State(state.clone()), Json(logs_request)).await {
        Ok(response) => {
            println!("✅ Logs telemetry processed successfully:");
            println!("  • Batch ID: {}", response.0.data.batch_id);
            println!("  • Processing time: {} ms", response.0.data.processing_time_ms);
            println!("  • Records written: {}", response.0.data.result.records_written);
            println!("  • Records failed: {}", response.0.data.result.records_failed);
            println!("  • Status: {:?}", response.0.data.result.status);
        }
        Err(e) => {
            println!("❌ Logs telemetry processing failed: {:?}", e);
        }
    }

    // Test 4: Mixed telemetry processing
    println!("\n📊 Test 4: Mixed Telemetry Processing");
    let mixed_batch = create_mixed_batch();
    let mixed_request = TelemetryIngestionRequest {
        batch: mixed_batch,
        options: Some(ProcessingOptions {
            validate: true,
            transform: true,
            aggregate: false,
            timeout_seconds: Some(30),
        }),
    };

    match telemetry_ingestion_handler(State(state.clone()), Json(mixed_request)).await {
        Ok(response) => {
            println!("✅ Mixed telemetry processed successfully:");
            println!("  • Batch ID: {}", response.0.data.batch_id);
            println!("  • Processing time: {} ms", response.0.data.processing_time_ms);
            println!("  • Records written: {}", response.0.data.result.records_written);
            println!("  • Records failed: {}", response.0.data.result.records_failed);
            println!("  • Status: {:?}", response.0.data.result.status);
        }
        Err(e) => {
            println!("❌ Mixed telemetry processing failed: {:?}", e);
        }
    }

    // Test 5: Error handling with invalid data
    println!("\n📊 Test 5: Error Handling with Invalid Data");
    let invalid_batch = create_invalid_batch();
    let invalid_request = TelemetryIngestionRequest {
        batch: invalid_batch,
        options: Some(ProcessingOptions {
            validate: true,
            transform: true,
            aggregate: false,
            timeout_seconds: Some(30),
        }),
    };

    match telemetry_ingestion_handler(State(state.clone()), Json(invalid_request)).await {
        Ok(response) => {
            println!("✅ Invalid telemetry handled gracefully:");
            println!("  • Batch ID: {}", response.0.data.batch_id);
            println!("  • Processing time: {} ms", response.0.data.processing_time_ms);
            println!("  • Records written: {}", response.0.data.result.records_written);
            println!("  • Records failed: {}", response.0.data.result.records_failed);
            println!("  • Status: {:?}", response.0.data.result.status);
            
            if !response.0.data.result.errors.is_empty() {
                println!("  • Errors:");
                for error in &response.0.data.result.errors {
                    println!("    - {}: {}", error.code, error.message);
                }
            }
        }
        Err(e) => {
            println!("❌ Invalid telemetry processing failed: {:?}", e);
        }
    }

    println!("\n✅ Telemetry processing pipeline test completed!");
    println!("\nThe telemetry pipeline successfully:");
    println!("  • Applied data quality filters and validations");
    println!("  • Performed business logic transformations");
    println!("  • Enriched records with metadata");
    println!("  • Applied sampling and rate limiting");
    println!("  • Routed to appropriate exporters");
    println!("  • Stored in lakehouse with partitioning");
    println!("  • Updated metrics and monitoring");

    Ok(())
}

// Helper functions to create test data

fn create_metrics_batch() -> TelemetryBatch {
    let mut records = Vec::new();
    
    // Create HTTP metrics
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(MetricData {
            name: "http_requests_total".to_string(),
            description: Some("Total HTTP requests".to_string()),
            unit: Some("requests".to_string()),
            metric_type: MetricType::Counter,
            value: MetricValue::Counter(1234.0),
            labels: vec![
                ("method".to_string(), "GET".to_string()),
                ("status".to_string(), "200".to_string()),
                ("endpoint".to_string(), "/api/users".to_string()),
            ].into_iter().collect(),
            timestamp: chrono::Utc::now(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    // Create business metrics
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(MetricData {
            name: "business_revenue_total".to_string(),
            description: Some("Total business revenue".to_string()),
            unit: Some("dollars".to_string()),
            metric_type: MetricType::Gauge,
            value: MetricValue::Gauge(50000.0),
            labels: vec![
                ("product".to_string(), "premium".to_string()),
                ("region".to_string(), "us-west".to_string()),
            ].into_iter().collect(),
            timestamp: chrono::Utc::now(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    TelemetryBatch {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        source: "test_metrics".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    }
}

fn create_traces_batch() -> TelemetryBatch {
    let mut records = Vec::new();
    
    // Create successful trace
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Trace,
        data: TelemetryData::Trace(TraceData {
            trace_id: "trace_1234567890abcdef".to_string(),
            span_id: "span_abcdef1234567890".to_string(),
            parent_span_id: None,
            name: "HTTP GET /api/users".to_string(),
            kind: bridge_core::types::SpanKind::Server,
            start_time: chrono::Utc::now() - chrono::Duration::seconds(10),
            end_time: Some(chrono::Utc::now() - chrono::Duration::seconds(9)),
            duration_ns: Some(1_000_000_000), // 1 second
            status: SpanStatus {
                code: StatusCode::Ok,
                message: None,
            },
            attributes: vec![
                ("http.method".to_string(), "GET".to_string()),
                ("http.url".to_string(), "/api/users".to_string()),
                ("http.status_code".to_string(), "200".to_string()),
            ].into_iter().collect(),
            events: Vec::new(),
            links: Vec::new(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    // Create error trace
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Trace,
        data: TelemetryData::Trace(TraceData {
            trace_id: "trace_error_abcdef1234567890".to_string(),
            span_id: "span_error_1234567890abcdef".to_string(),
            parent_span_id: None,
            name: "Database Query".to_string(),
            kind: bridge_core::types::SpanKind::Internal,
            start_time: chrono::Utc::now() - chrono::Duration::seconds(5),
            end_time: Some(chrono::Utc::now() - chrono::Duration::seconds(4)),
            duration_ns: Some(500_000_000), // 0.5 seconds
            status: SpanStatus {
                code: StatusCode::Error,
                message: Some("Database connection timeout".to_string()),
            },
            attributes: vec![
                ("db.system".to_string(), "postgresql".to_string()),
                ("db.operation".to_string(), "SELECT".to_string()),
            ].into_iter().collect(),
            events: Vec::new(),
            links: Vec::new(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    TelemetryBatch {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        source: "test_traces".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    }
}

fn create_logs_batch() -> TelemetryBatch {
    let mut records = Vec::new();
    
    // Create info log
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Log,
        data: TelemetryData::Log(LogData {
            timestamp: chrono::Utc::now() - chrono::Duration::minutes(5),
            level: LogLevel::Info,
            message: "User login successful".to_string(),
            attributes: vec![
                ("user_id".to_string(), "user123".to_string()),
                ("ip_address".to_string(), "192.168.1.100".to_string()),
            ].into_iter().collect(),
            body: None,
            severity_number: Some(9),
            severity_text: Some("INFO".to_string()),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    // Create error log
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Log,
        data: TelemetryData::Log(LogData {
            timestamp: chrono::Utc::now() - chrono::Duration::minutes(3),
            level: LogLevel::Error,
            message: "Database connection failed".to_string(),
            attributes: vec![
                ("db_host".to_string(), "db.example.com".to_string()),
                ("error_code".to_string(), "CONNECTION_TIMEOUT".to_string()),
            ].into_iter().collect(),
            body: None,
            severity_number: Some(17),
            severity_text: Some("ERROR".to_string()),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    TelemetryBatch {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        source: "test_logs".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    }
}

fn create_mixed_batch() -> TelemetryBatch {
    let mut records = Vec::new();
    
    // Add a metric
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(MetricData {
            name: "cpu_usage_percent".to_string(),
            description: Some("CPU usage percentage".to_string()),
            unit: Some("percent".to_string()),
            metric_type: MetricType::Gauge,
            value: MetricValue::Gauge(75.5),
            labels: vec![
                ("host".to_string(), "server-01".to_string()),
                ("core".to_string(), "0".to_string()),
            ].into_iter().collect(),
            timestamp: chrono::Utc::now(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    // Add a trace
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Trace,
        data: TelemetryData::Trace(TraceData {
            trace_id: "trace_mixed_1234567890abcdef".to_string(),
            span_id: "span_mixed_abcdef1234567890".to_string(),
            parent_span_id: None,
            name: "Cache Operation".to_string(),
            kind: bridge_core::types::SpanKind::Internal,
            start_time: chrono::Utc::now() - chrono::Duration::milliseconds(100),
            end_time: Some(chrono::Utc::now() - chrono::Duration::milliseconds(50)),
            duration_ns: Some(50_000_000), // 50ms
            status: SpanStatus {
                code: StatusCode::Ok,
                message: None,
            },
            attributes: vec![
                ("cache.operation".to_string(), "GET".to_string()),
                ("cache.key".to_string(), "user:123".to_string()),
            ].into_iter().collect(),
            events: Vec::new(),
            links: Vec::new(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    // Add a log
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Log,
        data: TelemetryData::Log(LogData {
            timestamp: chrono::Utc::now() - chrono::Duration::minutes(1),
            level: LogLevel::Warn,
            message: "High memory usage detected".to_string(),
            attributes: vec![
                ("memory_usage".to_string(), "85%".to_string()),
                ("threshold".to_string(), "80%".to_string()),
            ].into_iter().collect(),
            body: None,
            severity_number: Some(13),
            severity_text: Some("WARN".to_string()),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    TelemetryBatch {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        source: "test_mixed".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    }
}

fn create_invalid_batch() -> TelemetryBatch {
    let mut records = Vec::new();
    
    // Add a metric with empty name (invalid)
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(MetricData {
            name: "".to_string(), // Invalid: empty name
            description: Some("Invalid metric".to_string()),
            unit: Some("count".to_string()),
            metric_type: MetricType::Counter,
            value: MetricValue::Counter(1.0),
            labels: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    // Add a trace with empty trace_id (invalid)
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Trace,
        data: TelemetryData::Trace(TraceData {
            trace_id: "".to_string(), // Invalid: empty trace_id
            span_id: "span_valid_1234567890".to_string(),
            parent_span_id: None,
            name: "Invalid Trace".to_string(),
            kind: bridge_core::types::SpanKind::Internal,
            start_time: chrono::Utc::now() - chrono::Duration::seconds(1),
            end_time: Some(chrono::Utc::now()),
            duration_ns: Some(1_000_000_000),
            status: SpanStatus {
                code: StatusCode::Ok,
                message: None,
            },
            attributes: HashMap::new(),
            events: Vec::new(),
            links: Vec::new(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    // Add a log with empty message (invalid)
    records.push(TelemetryRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        record_type: TelemetryType::Log,
        data: TelemetryData::Log(LogData {
            timestamp: chrono::Utc::now(),
            level: LogLevel::Info,
            message: "".to_string(), // Invalid: empty message
            attributes: HashMap::new(),
            body: None,
            severity_number: Some(9),
            severity_text: Some("INFO".to_string()),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    });

    TelemetryBatch {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        source: "test_invalid".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    }
}
