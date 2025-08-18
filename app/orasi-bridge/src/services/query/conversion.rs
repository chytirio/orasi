//! Query conversion utilities for converting between different formats

use bridge_core::{
    traits::{QueryResultStatus, TelemetryQueryResult},
    types::{Filter, TelemetryQuery, TelemetryRecord, TimeRange},
    BridgeResult,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use crate::proto::{
    QueryStatus, QueryTelemetryRequest, QueryTelemetryResponse, StreamTelemetryResponse,
    TelemetryRecord as GrpcTelemetryRecord,
};

/// Convert gRPC query request to bridge-core TelemetryQuery
pub fn convert_grpc_to_telemetry_query(
    query_request: &QueryTelemetryRequest,
) -> BridgeResult<TelemetryQuery> {
    // Convert time range
    let time_range = if let Some(grpc_time_range) = &query_request.time_range {
        TimeRange::new(
            DateTime::from_timestamp(grpc_time_range.start_time, 0)
                .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(1)),
            DateTime::from_timestamp(grpc_time_range.end_time, 0).unwrap_or_else(|| Utc::now()),
        )
    } else {
        TimeRange::last_hours(1)
    };

    // Convert filters
    let filters: Vec<Filter> = query_request
        .filters
        .iter()
        .map(|f| {
            // Convert string operator to FilterOperator enum
            let operator = match f.operator.as_str() {
                "eq" => bridge_core::types::FilterOperator::Equals,
                "ne" => bridge_core::types::FilterOperator::NotEquals,
                "gt" => bridge_core::types::FilterOperator::GreaterThan,
                "lt" => bridge_core::types::FilterOperator::LessThan,
                "gte" => bridge_core::types::FilterOperator::GreaterThanOrEqual,
                "lte" => bridge_core::types::FilterOperator::LessThanOrEqual,
                "contains" => bridge_core::types::FilterOperator::Contains,
                "regex" => bridge_core::types::FilterOperator::Contains, // Map regex to contains for now
                _ => bridge_core::types::FilterOperator::Equals,
            };

            // Convert string value to FilterValue enum
            let value = bridge_core::types::FilterValue::String(f.value.clone());

            Filter {
                field: f.field.clone(),
                operator,
                value,
            }
        })
        .collect();

    // Convert aggregations (empty for now, can be extended)
    let aggregations = Vec::new();

    // Create telemetry query
    let telemetry_query = TelemetryQuery {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        time_range,
        filters,
        aggregations,
        limit: query_request.limit.try_into().ok(),
        metadata: HashMap::new(),
        offset: query_request.offset.try_into().ok(),
        query_type: match query_request.query_type.as_str() {
            "metrics" => bridge_core::types::TelemetryQueryType::Metrics,
            "traces" => bridge_core::types::TelemetryQueryType::Traces,
            "logs" => bridge_core::types::TelemetryQueryType::Logs,
            "all" => bridge_core::types::TelemetryQueryType::All,
            _ => bridge_core::types::TelemetryQueryType::All,
        },
    };

    Ok(telemetry_query)
}

/// Convert TelemetryQueryResult to gRPC QueryTelemetryResponse
pub fn convert_telemetry_result_to_grpc(
    result: TelemetryQueryResult,
    execution_time: Duration,
) -> BridgeResult<QueryTelemetryResponse> {
    // Convert telemetry records to gRPC format
    let records = if let Value::Array(data_array) = &result.data {
        data_array
            .iter()
            .filter_map(|item| {
                if let Ok(record) = serde_json::from_value::<TelemetryRecord>(item.clone()) {
                    Some(GrpcTelemetryRecord {
                        id: record.id.to_string(),
                        r#type: format!("{:?}", record.record_type).to_lowercase(),
                        timestamp: record.timestamp.timestamp(),
                        data: serde_json::to_string(&record.data).unwrap_or_default(),
                        metadata: record.attributes,
                    })
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    // Convert status
    let status = match result.status {
        QueryResultStatus::Success => QueryStatus::Success,
        QueryResultStatus::Partial => QueryStatus::Partial,
        QueryResultStatus::Failed => QueryStatus::Error,
        QueryResultStatus::Timeout => QueryStatus::Timeout,
    };

    // Convert errors
    let error_message = if !result.errors.is_empty() {
        result
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.code, e.message))
            .collect::<Vec<_>>()
            .join("; ")
    } else {
        String::new()
    };

    let total_count = records.len() as i64;
    Ok(QueryTelemetryResponse {
        records,
        total_count,
        execution_time_ms: execution_time.as_millis() as i64,
        status: status.into(),
        error_message,
    })
}

/// Convert streaming results to gRPC responses
pub fn convert_stream_results_to_grpc(
    results: Vec<TelemetryQueryResult>,
) -> BridgeResult<Vec<StreamTelemetryResponse>> {
    let mut responses = Vec::new();
    let total_results = results.len();

    for (index, result) in results.into_iter().enumerate() {
        let is_last_batch = index == total_results - 1;

        // Convert the result to records
        let records = if let Value::Array(data_array) = &result.data {
            data_array
                .iter()
                .filter_map(|item| {
                    if let Ok(record) = serde_json::from_value::<TelemetryRecord>(item.clone()) {
                        Some(GrpcTelemetryRecord {
                            id: record.id.to_string(),
                            r#type: format!("{:?}", record.record_type).to_lowercase(),
                            timestamp: record.timestamp.timestamp(),
                            data: serde_json::to_string(&record.data).unwrap_or_default(),
                            metadata: record.attributes,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        responses.push(StreamTelemetryResponse {
            records,
            is_last_batch,
            total_processed: (index + 1) as i64,
        });
    }

    Ok(responses)
}
