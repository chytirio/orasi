//! Delta Lake data conversion utilities

use crate::error::BridgeResult;
use crate::types::ProcessedRecord;
use arrow::{
    array::{BooleanArray, Float64Array, Int64Array, StringArray, TimestampNanosecondArray},
    datatypes::{DataType, Field, Schema as ArrowSchema, TimeUnit},
    record_batch::RecordBatch,
};
use chrono::{DateTime, Utc};
use serde_json;
use std::sync::Arc;
use tracing::info;

/// Convert ProcessedRecord to Arrow RecordBatch
pub fn convert_records_to_arrow(
    records: &[ProcessedRecord],
) -> BridgeResult<RecordBatch> {
    let mut ids = Vec::new();
    let mut timestamps = Vec::new();
    let mut record_types = Vec::new();
    let mut service_names = Vec::new();
    let mut operation_names = Vec::new();
    let mut duration_ms = Vec::new();
    let mut status_codes = Vec::new();
    let mut attributes = Vec::new();
    let mut tags = Vec::new();

    for record in records {
        ids.push(record.original_id.to_string());
        timestamps.push(
            record
                .metadata
                .get("timestamp")
                .and_then(|ts| ts.parse::<i64>().ok())
                .unwrap_or_else(|| chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
        );
        record_types.push(
            record
                .metadata
                .get("record_type")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
        );
        service_names.push(
            record
                .metadata
                .get("service_name")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
        );
        operation_names.push(
            record
                .metadata
                .get("operation_name")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
        );
        duration_ms.push(
            record
                .metadata
                .get("duration_ms")
                .and_then(|d| d.parse::<f64>().ok())
                .unwrap_or(0.0),
        );
        status_codes.push(
            record
                .metadata
                .get("status_code")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
        );
        attributes
            .push(serde_json::to_string(&record.metadata).unwrap_or_else(|_| "{}".to_string()));
        tags.push(serde_json::to_string(&record.metadata).unwrap_or_else(|_| "{}".to_string()));
    }

    let schema = ArrowSchema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new(
            "timestamp",
            DataType::Timestamp(TimeUnit::Nanosecond, None),
            false,
        ),
        Field::new("record_type", DataType::Utf8, false),
        Field::new("service_name", DataType::Utf8, true),
        Field::new("operation_name", DataType::Utf8, true),
        Field::new("duration_ms", DataType::Float64, true),
        Field::new("status_code", DataType::Utf8, true),
        Field::new("attributes", DataType::Utf8, true),
        Field::new("tags", DataType::Utf8, true),
    ]);

    let batch = RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(TimestampNanosecondArray::from(timestamps)),
            Arc::new(StringArray::from(record_types)),
            Arc::new(StringArray::from(service_names)),
            Arc::new(StringArray::from(operation_names)),
            Arc::new(Float64Array::from(duration_ms)),
            Arc::new(StringArray::from(status_codes)),
            Arc::new(StringArray::from(attributes)),
            Arc::new(StringArray::from(tags)),
        ],
    )
    .map_err(|e| {
        crate::error::BridgeError::lakehouse(&format!(
            "Failed to create Arrow RecordBatch: {}",
            e
        ))
    })?;

    Ok(batch)
}

/// Create Delta Lake schema JSON
pub fn create_delta_schema_json() -> serde_json::Value {
    serde_json::json!({
        "type": "struct",
        "fields": [
            {
                "name": "id",
                "type": "string",
                "nullable": false,
                "metadata": {}
            },
            {
                "name": "timestamp",
                "type": "timestamp",
                "nullable": false,
                "metadata": {}
            },
            {
                "name": "telemetry_type",
                "type": "string",
                "nullable": false,
                "metadata": {}
            },
            {
                "name": "service_name",
                "type": "string",
                "nullable": true,
                "metadata": {}
            },
            {
                "name": "resource_attributes",
                "type": "string",
                "nullable": true,
                "metadata": {}
            },
            {
                "name": "data_json",
                "type": "string",
                "nullable": false,
                "metadata": {}
            }
        ]
    })
}

/// Create Delta Lake protocol entry
pub fn create_protocol_entry() -> String {
    serde_json::to_string(&serde_json::json!({
        "protocol": {
            "minReaderVersion": 1,
            "minWriterVersion": 2
        }
    }))
    .unwrap()
}

/// Create Delta Lake metadata entry
pub fn create_metadata_entry(
    table_id: &str,
    partition_columns: &[String],
) -> String {
    let schema_json = create_delta_schema_json();
    
    serde_json::to_string(&serde_json::json!({
        "metaData": {
            "id": table_id,
            "format": {
                "provider": "parquet",
                "options": {}
            },
            "schemaString": serde_json::to_string(&schema_json).unwrap(),
            "partitionColumns": partition_columns,
            "configuration": {},
            "createdTime": chrono::Utc::now().timestamp_millis()
        }
    }))
    .unwrap()
}

/// Create Delta Lake add entry
pub fn create_add_entry(parquet_file: &str, file_size: u64) -> String {
    serde_json::to_string(&serde_json::json!({
        "add": {
            "path": parquet_file,
            "partitionValues": {},
            "size": file_size,
            "modificationTime": chrono::Utc::now().timestamp_millis(),
            "dataChange": true
        }
    }))
    .unwrap()
}
