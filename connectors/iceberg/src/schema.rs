//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Iceberg schema implementation
//!
//! This module provides the Apache Iceberg schema that manages
//! table schemas, schema evolution, and data type conversions.

use arrow::datatypes::{DataType, Field, Schema};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::config::IcebergSchemaConfig;
use crate::error::{IcebergError, IcebergResult};

/// Apache Iceberg schema implementation
pub struct IcebergSchema {
    /// Schema configuration
    config: IcebergSchemaConfig,
    /// Current Arrow schema
    arrow_schema: Schema,
    /// Schema version
    version: u32,
    /// Schema metadata
    metadata: HashMap<String, String>,
    /// Partition columns (from table config)
    partition_columns: Vec<String>,
}

impl IcebergSchema {
    /// Create a new Apache Iceberg schema
    pub async fn new(config: IcebergSchemaConfig) -> IcebergResult<Self> {
        info!("Creating Apache Iceberg schema");

        // Create the default telemetry schema
        let arrow_schema = Self::create_telemetry_schema().await?;

        Ok(Self {
            config,
            arrow_schema,
            version: 1,
            metadata: HashMap::new(),
            partition_columns: vec![
                "year".to_string(),
                "month".to_string(),
                "day".to_string(),
                "hour".to_string(),
            ],
        })
    }

    /// Create the default telemetry schema
    async fn create_telemetry_schema() -> IcebergResult<Schema> {
        debug!("Creating default telemetry schema");

        let fields = vec![
            // Common fields
            Field::new(
                "timestamp",
                DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None),
                false,
            ),
            Field::new("service_name", DataType::Utf8, false),
            Field::new("instance_id", DataType::Utf8, false),
            Field::new("telemetry_type", DataType::Utf8, false),
            // Metrics specific fields
            Field::new("metric_name", DataType::Utf8, true),
            Field::new("metric_value", DataType::Float64, true),
            Field::new("metric_unit", DataType::Utf8, true),
            Field::new("metric_labels", DataType::Utf8, true), // JSON string
            // Traces specific fields
            Field::new("trace_id", DataType::Utf8, true),
            Field::new("span_id", DataType::Utf8, true),
            Field::new("parent_span_id", DataType::Utf8, true),
            Field::new("operation_name", DataType::Utf8, true),
            Field::new("span_kind", DataType::Utf8, true),
            Field::new("span_status", DataType::Utf8, true),
            Field::new("span_duration_ms", DataType::Float64, true),
            Field::new("span_attributes", DataType::Utf8, true), // JSON string
            // Logs specific fields
            Field::new("log_level", DataType::Utf8, true),
            Field::new("log_message", DataType::Utf8, true),
            Field::new("log_attributes", DataType::Utf8, true), // JSON string
            // Partition fields
            Field::new("year", DataType::Int32, false),
            Field::new("month", DataType::Int32, false),
            Field::new("day", DataType::Int32, false),
            Field::new("hour", DataType::Int32, false),
        ];

        Ok(Schema::new(fields))
    }

    /// Get the current Arrow schema
    pub async fn get_schema(&self) -> IcebergResult<Schema> {
        Ok(self.arrow_schema.clone())
    }

    /// Get the schema version
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Get schema metadata
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Add metadata to the schema
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Update the schema (schema evolution)
    pub async fn update_schema(&mut self, new_schema: Schema) -> IcebergResult<()> {
        info!(
            "Updating schema from version {} to {}",
            self.version,
            self.version + 1
        );

        // Validate schema compatibility
        Self::validate_schema_compatibility(&self.arrow_schema, &new_schema).await?;

        // Update schema
        self.arrow_schema = new_schema;
        self.version += 1;

        info!("Schema updated successfully to version {}", self.version);
        Ok(())
    }

    /// Validate schema compatibility for evolution
    async fn validate_schema_compatibility(
        old_schema: &Schema,
        new_schema: &Schema,
    ) -> IcebergResult<()> {
        debug!("Validating schema compatibility");

        // Check if all required fields from old schema exist in new schema
        for field in old_schema.fields() {
            if !field.is_nullable() {
                // Required field must exist in new schema
                match new_schema.field_with_name(field.name()) {
                    Ok(new_field) => {
                        if new_field.is_nullable() {
                            return Err(IcebergError::schema(format!(
                                "Required field '{}' cannot become nullable in schema evolution",
                                field.name()
                            )));
                        }
                    }
                    Err(_) => {
                        return Err(IcebergError::schema(format!(
                            "Required field '{}' missing in new schema",
                            field.name()
                        )));
                    }
                }
            }
        }

        // Check data type compatibility for existing fields
        for field in old_schema.fields() {
            if let Ok(new_field) = new_schema.field_with_name(field.name()) {
                if !Self::is_type_compatible(field.data_type(), new_field.data_type()) {
                    return Err(IcebergError::schema(format!(
                        "Incompatible type change for field '{}': {:?} -> {:?}",
                        field.name(),
                        field.data_type(),
                        new_field.data_type()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Check if two data types are compatible for schema evolution
    fn is_type_compatible(old_type: &DataType, new_type: &DataType) -> bool {
        match (old_type, new_type) {
            // Same type is always compatible
            (old, new) if old == new => true,

            // Widening conversions are allowed
            (DataType::Int8, DataType::Int16) => true,
            (DataType::Int8, DataType::Int32) => true,
            (DataType::Int8, DataType::Int64) => true,
            (DataType::Int16, DataType::Int32) => true,
            (DataType::Int16, DataType::Int64) => true,
            (DataType::Int32, DataType::Int64) => true,
            (DataType::Float32, DataType::Float64) => true,

            // String types are compatible
            (DataType::Utf8, DataType::LargeUtf8) => true,
            (DataType::LargeUtf8, DataType::Utf8) => true,

            // Timestamp precision changes are allowed
            (DataType::Timestamp(old_unit, _), DataType::Timestamp(new_unit, _)) => {
                match (old_unit, new_unit) {
                    (
                        arrow::datatypes::TimeUnit::Second,
                        arrow::datatypes::TimeUnit::Millisecond,
                    ) => true,
                    (
                        arrow::datatypes::TimeUnit::Second,
                        arrow::datatypes::TimeUnit::Microsecond,
                    ) => true,
                    (
                        arrow::datatypes::TimeUnit::Second,
                        arrow::datatypes::TimeUnit::Nanosecond,
                    ) => true,
                    (
                        arrow::datatypes::TimeUnit::Millisecond,
                        arrow::datatypes::TimeUnit::Microsecond,
                    ) => true,
                    (
                        arrow::datatypes::TimeUnit::Millisecond,
                        arrow::datatypes::TimeUnit::Nanosecond,
                    ) => true,
                    (
                        arrow::datatypes::TimeUnit::Microsecond,
                        arrow::datatypes::TimeUnit::Nanosecond,
                    ) => true,
                    _ => false,
                }
            }

            // Default: not compatible
            _ => false,
        }
    }

    /// Get partition columns from schema
    pub fn get_partition_columns(&self) -> Vec<String> {
        self.partition_columns.clone()
    }

    /// Check if a field is a partition column
    pub fn is_partition_column(&self, field_name: &str) -> bool {
        self.partition_columns.contains(&field_name.to_string())
    }

    /// Get the schema as JSON
    pub async fn to_json(&self) -> IcebergResult<serde_json::Value> {
        let schema_json = serde_json::json!({
            "version": self.version,
            "fields": self.arrow_schema.fields().iter().map(|field| {
                serde_json::json!({
                    "name": field.name(),
                    "type": format!("{:?}", field.data_type()),
                    "nullable": field.is_nullable()
                })
            }).collect::<Vec<_>>(),
            "metadata": self.metadata,
            "partition_columns": self.partition_columns
        });

        Ok(schema_json)
    }

    /// Create schema from JSON
    pub async fn from_json(json: serde_json::Value) -> IcebergResult<Self> {
        let version = json["version"].as_u64().unwrap_or(1) as u32;

        let fields_json = json["fields"].as_array().ok_or_else(|| {
            IcebergError::schema("Invalid schema JSON: missing fields".to_string())
        })?;

        let mut fields = Vec::new();
        for field_json in fields_json {
            let name = field_json["name"]
                .as_str()
                .ok_or_else(|| {
                    IcebergError::schema("Invalid field JSON: missing name".to_string())
                })?
                .to_string();

            let type_str = field_json["type"].as_str().ok_or_else(|| {
                IcebergError::schema("Invalid field JSON: missing type".to_string())
            })?;

            let data_type = Self::parse_data_type(type_str)?;

            let nullable = field_json["nullable"].as_bool().unwrap_or(true);

            fields.push(Field::new(name, data_type, nullable));
        }

        let arrow_schema = Schema::new(fields);

        let partition_columns = json["partition_columns"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let config = IcebergSchemaConfig {
            enable_schema_evolution: true,
            validation_mode: "strict".to_string(),
            enable_column_mapping: false,
            column_mapping_mode: "name".to_string(),
        };

        let metadata = json["metadata"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            config,
            arrow_schema,
            version,
            metadata,
            partition_columns,
        })
    }

    /// Parse data type from string
    fn parse_data_type(type_str: &str) -> IcebergResult<DataType> {
        match type_str {
            "Int8" => Ok(DataType::Int8),
            "Int16" => Ok(DataType::Int16),
            "Int32" => Ok(DataType::Int32),
            "Int64" => Ok(DataType::Int64),
            "UInt8" => Ok(DataType::UInt8),
            "UInt16" => Ok(DataType::UInt16),
            "UInt32" => Ok(DataType::UInt32),
            "UInt64" => Ok(DataType::UInt64),
            "Float32" => Ok(DataType::Float32),
            "Float64" => Ok(DataType::Float64),
            "Utf8" => Ok(DataType::Utf8),
            "LargeUtf8" => Ok(DataType::LargeUtf8),
            "Boolean" => Ok(DataType::Boolean),
            "Timestamp(Microsecond, None)" => Ok(DataType::Timestamp(
                arrow::datatypes::TimeUnit::Microsecond,
                None,
            )),
            "Timestamp(Millisecond, None)" => Ok(DataType::Timestamp(
                arrow::datatypes::TimeUnit::Millisecond,
                None,
            )),
            "Timestamp(Nanosecond, None)" => Ok(DataType::Timestamp(
                arrow::datatypes::TimeUnit::Nanosecond,
                None,
            )),
            _ => Err(IcebergError::schema(format!(
                "Unsupported data type: {}",
                type_str
            ))),
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &IcebergSchemaConfig {
        &self.config
    }
}
