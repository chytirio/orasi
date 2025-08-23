//! Streaming query functionality and configuration

use bridge_core::{
    traits::{QueryResultStatus, TelemetryQueryResult},
    types::{
        MetricData, MetricType, MetricValue, TelemetryBatch, TelemetryData, TelemetryQuery,
        TelemetryRecord, TelemetryType,
    },
    BridgeResult,
};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

use streaming_processor::{
    processors::filter_processor::{FilterMode, FilterOperator, FilterProcessorConfig, FilterRule},
    StreamingProcessorConfig,
};

/// Create streaming processor configuration
pub async fn create_streaming_config(
    query: &TelemetryQuery,
    batch_size: i32,
) -> BridgeResult<StreamingProcessorConfig> {
    let mut config = StreamingProcessorConfig::default();

    // Set configuration
    config.name = format!("query_processor_{}", query.id);
    config.version = "1.0.0".to_string();

    // Add source configuration
    let source_config = streaming_processor::config::SourceConfig {
        source_type: streaming_processor::config::SourceType::Http,
        name: "http_source".to_string(),
        version: "1.0.0".to_string(),
        config: HashMap::new(),
        auth: None,
        connection: streaming_processor::config::ConnectionConfig::default(),
    };
    config
        .sources
        .insert("http_source".to_string(), source_config);

    // Add processor configuration
    let processor_config = streaming_processor::config::ProcessorConfig {
        processor_type: streaming_processor::config::ProcessorType::Filter,
        name: "filter_processor".to_string(),
        version: "1.0.0".to_string(),
        config: HashMap::new(),
        order: 1,
    };
    config.processors.push(processor_config);

    // Add sink configuration
    let sink_config = streaming_processor::config::SinkConfig {
        sink_type: streaming_processor::config::SinkType::Http,
        name: "http_sink".to_string(),
        version: "1.0.0".to_string(),
        config: HashMap::new(),
        auth: None,
        connection: streaming_processor::config::ConnectionConfig::default(),
    };
    config.sinks.insert("http_sink".to_string(), sink_config);

    // Set processing configuration
    config.processing.batch_size = batch_size as usize;
    config.processing.buffer_size = 1000;

    Ok(config)
}

/// Create data stream from source
pub async fn create_data_stream_from_source(
    source: &dyn streaming_processor::sources::StreamSource,
) -> BridgeResult<bridge_core::traits::DataStream> {
    // Create a mock data stream for now
    // In a real implementation, this would consume data from the actual source
    let mock_batch = TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "mock_source".to_string(),
        size: 1,
        records: vec![TelemetryRecord::new(
            TelemetryType::Metric,
            TelemetryData::Metric(MetricData {
                name: "streaming_metric".to_string(),
                description: Some("Streaming metric data".to_string()),
                unit: Some("count".to_string()),
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1.0),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
        )],
        metadata: HashMap::new(),
    };

    Ok(bridge_core::traits::DataStream {
        stream_id: "mock_stream".to_string(),
        data: serde_json::to_vec(&mock_batch)?,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    })
}

/// Convert stream to query result
pub async fn convert_stream_to_query_result(
    stream: bridge_core::traits::DataStream,
    query: &TelemetryQuery,
) -> BridgeResult<TelemetryQueryResult> {
    // Deserialize the batch from the stream data
    let batch: TelemetryBatch = serde_json::from_slice(&stream.data)?;
    let records = batch.records.clone();
    let data = serde_json::to_value(records)?;

    Ok(TelemetryQueryResult {
        query_id: query.id,
        timestamp: Utc::now(),
        status: QueryResultStatus::Success,
        data,
        metadata: HashMap::from([
            ("source".to_string(), "streaming_processor".to_string()),
            ("batch_size".to_string(), batch.records.len().to_string()),
        ]),
        errors: Vec::new(),
    })
}

/// Create filter configuration based on query filters
pub async fn create_filter_config(query: &TelemetryQuery) -> BridgeResult<FilterProcessorConfig> {
    let mut filter_rules = Vec::new();

    // Convert query filters to filter rules
    for filter in &query.filters {
        let operator = match filter.operator {
            bridge_core::types::FilterOperator::Equals => FilterOperator::Equals,
            bridge_core::types::FilterOperator::NotEquals => FilterOperator::NotEquals,
            bridge_core::types::FilterOperator::Contains => FilterOperator::Contains,
            bridge_core::types::FilterOperator::NotContains => FilterOperator::NotContains,
            bridge_core::types::FilterOperator::GreaterThan => FilterOperator::GreaterThan,
            bridge_core::types::FilterOperator::LessThan => FilterOperator::LessThan,
            bridge_core::types::FilterOperator::GreaterThanOrEqual => {
                FilterOperator::GreaterThanOrEqual
            }
            bridge_core::types::FilterOperator::LessThanOrEqual => FilterOperator::LessThanOrEqual,
            bridge_core::types::FilterOperator::In => FilterOperator::In,
            bridge_core::types::FilterOperator::NotIn => FilterOperator::NotIn,
            bridge_core::types::FilterOperator::StartsWith => FilterOperator::Contains, // Map to Contains for now
            bridge_core::types::FilterOperator::EndsWith => FilterOperator::Contains, // Map to Contains for now
            bridge_core::types::FilterOperator::Regex => FilterOperator::Contains, // Map to Contains for now
            bridge_core::types::FilterOperator::Exists => FilterOperator::Equals, // Map to Equals for now
            bridge_core::types::FilterOperator::NotExists => FilterOperator::NotEquals, // Map to NotEquals for now
        };

        let value = match &filter.value {
            bridge_core::types::FilterValue::String(s) => s.clone(),
            bridge_core::types::FilterValue::Number(n) => n.to_string(),
            bridge_core::types::FilterValue::Boolean(b) => b.to_string(),
            bridge_core::types::FilterValue::Array(arr) => {
                // Convert array to comma-separated string
                arr.iter()
                    .map(|v| match v {
                        bridge_core::types::FilterValue::String(s) => s.clone(),
                        bridge_core::types::FilterValue::Number(n) => n.to_string(),
                        bridge_core::types::FilterValue::Boolean(b) => b.to_string(),
                        bridge_core::types::FilterValue::Array(_) => "[]".to_string(),
                        bridge_core::types::FilterValue::Null => "null".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            }
            bridge_core::types::FilterValue::Null => "null".to_string(),
        };

        filter_rules.push(FilterRule {
            name: format!("filter_{}", filter.field),
            field: filter.field.clone(),
            operator,
            value,
            enabled: true,
        });
    }

    Ok(FilterProcessorConfig {
        name: "query_filter".to_string(),
        version: "1.0.0".to_string(),
        filter_mode: FilterMode::Include,
        filter_rules,
        additional_config: HashMap::new(),
    })
}
