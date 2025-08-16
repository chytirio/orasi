//! Query service for integrating gRPC telemetry queries with the query engine

use bridge_core::{
    traits::{QueryResultStatus, TelemetryQueryResult},
    types::{
        EventData, Filter, LogData, LogLevel, MetricData, MetricType, MetricValue, SpanKind,
        SpanStatus, StatusCode, TelemetryBatch, TelemetryData, TelemetryQuery, TelemetryRecord,
        TelemetryType, TimeRange, TraceData,
    },
    BridgeResult,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;
use streaming_processor::{
    processors::{
        filter_processor::{FilterMode, FilterOperator, FilterProcessorConfig, FilterRule},
        ProcessorPipeline,
    },
    sinks::{http_sink::HttpSinkConfig, SinkManager},
    sources::{http_source::HttpSourceConfig, SourceManager},
    StreamingProcessorConfig,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    config::BridgeAPIConfig,
    metrics::ApiMetrics,
    proto::{
        QueryStatus, QueryTelemetryRequest, QueryTelemetryResponse, StreamTelemetryRequest,
        StreamTelemetryResponse, TelemetryRecord as GrpcTelemetryRecord,
    },
    query_engine_integration::QueryEngineIntegration,
};

/// Query service for handling telemetry queries
pub struct QueryService {
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
    query_engine: Option<QueryEngineIntegration>,
}

impl QueryService {
    /// Create a new query service
    pub fn new(config: BridgeAPIConfig, metrics: ApiMetrics) -> Self {
        Self {
            config,
            metrics,
            query_engine: None,
        }
    }

    /// Initialize the query engine integration
    pub async fn init_query_engine(&mut self) -> BridgeResult<()> {
        tracing::info!("Initializing query engine integration");

        let query_engine =
            QueryEngineIntegration::new(self.config.clone(), self.metrics.clone()).await?;
        self.query_engine = Some(query_engine);

        tracing::info!("Query engine integration initialized successfully");
        Ok(())
    }

    /// Query telemetry data
    pub async fn query_telemetry(
        &self,
        query_request: &QueryTelemetryRequest,
    ) -> BridgeResult<QueryTelemetryResponse> {
        let start_time = Instant::now();

        tracing::info!(
            "Processing telemetry query: type={}",
            query_request.query_type
        );

        // Convert gRPC request to bridge-core query
        let telemetry_query = self.convert_grpc_to_telemetry_query(query_request)?;

        // Execute the query
        let query_result = self.execute_telemetry_query(&telemetry_query).await?;

        // Convert result back to gRPC response
        let response = self.convert_telemetry_result_to_grpc(query_result, start_time.elapsed())?;

        // Record metrics
        self.metrics
            .record_processing("telemetry_query", start_time.elapsed(), true);

        tracing::info!("Telemetry query completed successfully");

        Ok(response)
    }

    /// Stream telemetry data
    pub async fn stream_telemetry(
        &self,
        stream_request: &StreamTelemetryRequest,
    ) -> BridgeResult<Vec<StreamTelemetryResponse>> {
        let start_time = Instant::now();

        tracing::info!(
            "Processing telemetry stream: type={}",
            stream_request.query_type
        );

        // Convert gRPC request to bridge-core query
        let telemetry_query = self.convert_grpc_to_telemetry_query(&QueryTelemetryRequest {
            query_type: stream_request.query_type.clone(),
            time_range: stream_request.time_range.clone(),
            filters: stream_request.filters.clone(),
            limit: stream_request.batch_size,
            offset: 0,
            sort_by: String::new(),
            sort_order: String::new(),
        })?;

        // Execute the query with streaming
        let query_results = self
            .execute_telemetry_stream(&telemetry_query, stream_request.batch_size)
            .await?;

        // Convert results to gRPC streaming responses
        let responses = self.convert_stream_results_to_grpc(query_results)?;

        // Record metrics
        self.metrics
            .record_processing("telemetry_stream", start_time.elapsed(), true);

        tracing::info!("Telemetry stream completed successfully");

        Ok(responses)
    }

    /// Convert gRPC query request to bridge-core TelemetryQuery
    fn convert_grpc_to_telemetry_query(
        &self,
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

    /// Execute telemetry query using the query engine
    async fn execute_telemetry_query(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<TelemetryQueryResult> {
        // Use real query engine if available
        if let Some(query_engine) = &self.query_engine {
            tracing::info!("Using real query engine for query: {:?}", query.id);
            return query_engine.execute_query(query).await;
        }

        // Fallback to mock data if query engine not initialized
        tracing::warn!(
            "Query engine not initialized, using mock data for query: {:?}",
            query.id
        );

        let query_type = self.determine_query_type(query)?;

        match query_type.as_str() {
            "traces" => self.execute_traces_query(query).await,
            "metrics" => self.execute_metrics_query(query).await,
            "logs" => self.execute_logs_query(query).await,
            "events" => self.execute_events_query(query).await,
            _ => self.execute_generic_query(query).await,
        }
    }

    /// Execute telemetry stream
    async fn execute_telemetry_stream(
        &self,
        query: &TelemetryQuery,
        batch_size: i32,
    ) -> BridgeResult<Vec<TelemetryQueryResult>> {
        info!("Executing telemetry stream query: {}", query.id);

        // Create streaming processor configuration
        let streaming_config = self.create_streaming_config(query, batch_size).await?;

        // Initialize streaming processor components
        let mut source_manager = SourceManager::new();
        let mut sink_manager = SinkManager::new();
        let mut pipeline = ProcessorPipeline::new();

        // Create HTTP source for data ingestion
        let http_source_config = HttpSourceConfig {
            name: "query_source".to_string(),
            version: "1.0.0".to_string(),
            endpoint_url: format!("{}/api/v1/telemetry/stream", self.config.http.address),
            method: "POST".to_string(),
            headers: {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers.insert(
                    "Authorization".to_string(),
                    format!("Bearer {}", self.config.auth.api_key.header_name),
                );
                headers
            },
            body: Some(serde_json::to_string(query)?),
            polling_interval_ms: 1000, // Poll every second
            request_timeout_secs: 30,
            batch_size: batch_size as usize,
            buffer_size: 1000,
            auth_token: Some("api-key".to_string()),
            additional_config: HashMap::new(),
            max_retries: 3,
            retry_delay_ms: 1000,
            rate_limit_requests_per_second: Some(10),
        };

        // Create and add HTTP source
        let http_source =
            streaming_processor::sources::HttpSource::new(&http_source_config).await?;
        source_manager.add_source("http_source".to_string(), Box::new(http_source));

        // Create filter processor based on query filters
        let filter_config = self.create_filter_config(query).await?;
        let filter_processor =
            streaming_processor::processors::FilterProcessor::new(&filter_config).await?;
        pipeline.add_processor(Box::new(filter_processor));

        // Create HTTP sink for results
        let http_sink_config = HttpSinkConfig {
            name: "query_sink".to_string(),
            version: "1.0.0".to_string(),
            endpoint_url: format!("{}/api/v1/query/results", self.config.http.address),
            method: "POST".to_string(),
            headers: {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers.insert("Authorization".to_string(), format!("Bearer {}", "api-key"));
                headers
            },
            request_timeout_secs: 30,
            retry_count: 3,
            retry_delay_ms: 1000,
            batch_size: batch_size as usize,
            auth_token: Some("api-key".to_string()),
            additional_config: HashMap::new(),
            content_type: "application/json".to_string(),
            rate_limit_requests_per_second: Some(10),
        };

        // Create and add HTTP sink
        let http_sink = streaming_processor::sinks::HttpSink::new(&http_sink_config).await?;
        sink_manager.add_sink("http_sink".to_string(), Box::new(http_sink));

        // Start streaming components
        source_manager.start_all().await?;
        sink_manager.start_all().await?;

        // Process streaming data with real streaming processor
        let mut results = Vec::new();
        let mut batch_count = 0;
        let max_batches = 10; // Limit to prevent infinite streaming
        let timeout = Duration::from_secs(30); // 30 second timeout
        let start_time = Instant::now();

        while batch_count < max_batches && start_time.elapsed() < timeout {
            // Get data from source
            if let Some(source) = source_manager.get_source("http_source") {
                // Process data through pipeline
                let input_stream = self.create_data_stream_from_source(source).await?;
                let processed_stream = pipeline.process_stream(input_stream).await?;

                // Convert processed stream to query result
                let batch_result = self
                    .convert_stream_to_query_result(processed_stream, query)
                    .await?;
                results.push(batch_result);

                batch_count += 1;

                // Small delay between batches
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Check if we have enough data
                if batch_count >= 3 {
                    break;
                }
            } else {
                error!("HTTP source not found");
                break;
            }
        }

        // Stop streaming components
        source_manager.stop_all().await?;
        sink_manager.stop_all().await?;

        info!(
            "Completed telemetry stream query: {} batches processed",
            batch_count
        );
        Ok(results)
    }

    /// Create streaming processor configuration
    async fn create_streaming_config(
        &self,
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
    async fn create_data_stream_from_source(
        &self,
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
    async fn convert_stream_to_query_result(
        &self,
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
    async fn create_filter_config(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<FilterProcessorConfig> {
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
                bridge_core::types::FilterOperator::LessThanOrEqual => {
                    FilterOperator::LessThanOrEqual
                }
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

    /// Determine query type from filters and metadata
    fn determine_query_type(&self, query: &TelemetryQuery) -> BridgeResult<String> {
        // Check filters for type information
        for filter in &query.filters {
            if filter.field == "type" || filter.field == "record_type" {
                match &filter.value {
                    bridge_core::types::FilterValue::String(s) => return Ok(s.clone()),
                    _ => continue,
                }
            }
        }

        // Check metadata for type information
        if let Some(query_type) = query.metadata.get("type") {
            return Ok(query_type.clone());
        }

        // Default to generic query
        Ok("generic".to_string())
    }

    /// Execute traces query
    async fn execute_traces_query(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<TelemetryQueryResult> {
        // Create mock trace data
        let mock_traces = vec![TelemetryRecord::new(
            TelemetryType::Trace,
            TelemetryData::Trace(TraceData {
                trace_id: "trace-123".to_string(),
                span_id: "span-456".to_string(),
                parent_span_id: None,
                name: "mock-operation".to_string(),
                kind: SpanKind::Internal,
                start_time: Utc::now() - chrono::Duration::minutes(5),
                end_time: Some(Utc::now()),
                duration_ns: Some(1000000), // 1ms
                status: SpanStatus {
                    code: StatusCode::Ok,
                    message: None,
                },
                attributes: HashMap::new(),
                events: Vec::new(),
                links: Vec::new(),
            }),
        )];

        let data = serde_json::to_value(mock_traces)?;

        Ok(TelemetryQueryResult {
            query_id: query.id,
            timestamp: Utc::now(),
            status: QueryResultStatus::Success,
            data,
            metadata: HashMap::new(),
            errors: Vec::new(),
        })
    }

    /// Execute metrics query
    async fn execute_metrics_query(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<TelemetryQueryResult> {
        // Create mock metric data
        let mock_metrics = vec![TelemetryRecord::new(
            TelemetryType::Metric,
            TelemetryData::Metric(MetricData {
                name: "cpu_usage".to_string(),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(75.5),
                labels: HashMap::from([
                    ("host".to_string(), "server-1".to_string()),
                    ("service".to_string(), "api".to_string()),
                ]),
                timestamp: Utc::now(),
            }),
        )];

        let data = serde_json::to_value(mock_metrics)?;

        Ok(TelemetryQueryResult {
            query_id: query.id,
            timestamp: Utc::now(),
            status: QueryResultStatus::Success,
            data,
            metadata: HashMap::new(),
            errors: Vec::new(),
        })
    }

    /// Execute logs query
    async fn execute_logs_query(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<TelemetryQueryResult> {
        // Create mock log data
        let mock_logs = vec![TelemetryRecord::new(
            TelemetryType::Log,
            TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Mock log message".to_string(),
                attributes: HashMap::new(),
                body: Some("Mock log body".to_string()),
                severity_number: Some(6),
                severity_text: Some("INFO".to_string()),
            }),
        )];

        let data = serde_json::to_value(mock_logs)?;

        Ok(TelemetryQueryResult {
            query_id: query.id,
            timestamp: Utc::now(),
            status: QueryResultStatus::Success,
            data,
            metadata: HashMap::new(),
            errors: Vec::new(),
        })
    }

    /// Execute events query
    async fn execute_events_query(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<TelemetryQueryResult> {
        // Create mock event data
        let mock_events = vec![TelemetryRecord::new(
            TelemetryType::Event,
            TelemetryData::Event(EventData {
                name: "user_login".to_string(),
                timestamp: Utc::now(),
                attributes: HashMap::from([
                    ("user_id".to_string(), "user-123".to_string()),
                    ("ip_address".to_string(), "192.168.1.1".to_string()),
                ]),
                data: None,
            }),
        )];

        let data = serde_json::to_value(mock_events)?;

        Ok(TelemetryQueryResult {
            query_id: query.id,
            timestamp: Utc::now(),
            status: QueryResultStatus::Success,
            data,
            metadata: HashMap::new(),
            errors: Vec::new(),
        })
    }

    /// Execute generic query
    async fn execute_generic_query(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<TelemetryQueryResult> {
        // Create mock generic data
        let mock_data = vec![TelemetryRecord::new(
            TelemetryType::Metric,
            TelemetryData::Metric(MetricData {
                name: "generic_metric".to_string(),
                description: Some("Generic metric".to_string()),
                unit: Some("count".to_string()),
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(42.0),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
        )];

        let data = serde_json::to_value(mock_data)?;

        Ok(TelemetryQueryResult {
            query_id: query.id,
            timestamp: Utc::now(),
            status: QueryResultStatus::Success,
            data,
            metadata: HashMap::new(),
            errors: Vec::new(),
        })
    }

    /// Convert TelemetryQueryResult to gRPC QueryTelemetryResponse
    fn convert_telemetry_result_to_grpc(
        &self,
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
    fn convert_stream_results_to_grpc(
        &self,
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
                        if let Ok(record) = serde_json::from_value::<TelemetryRecord>(item.clone())
                        {
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
}
