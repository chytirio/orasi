//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Bridge API integration example
//! 
//! This example demonstrates how to integrate the streaming processor
//! with the bridge API for real-time query processing

use streaming_processor::{
    sources::{SourceManager, http_source::HttpSourceConfig},
    processors::{ProcessorPipeline, filter_processor::{FilterProcessorConfig, FilterRule, FilterMode, FilterOperator}},
    sinks::{SinkManager, http_sink::HttpSinkConfig},
    StreamingProcessorConfig,
};
use bridge_core::{BridgeResult, TelemetryQuery};
use bridge_core::traits::{TelemetryQueryResult, QueryResultStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};
use chrono::Utc;
use uuid::Uuid;

/// Bridge API integration service
pub struct BridgeIntegrationService {
    source_manager: Arc<RwLock<SourceManager>>,
    sink_manager: Arc<RwLock<SinkManager>>,
    pipeline: Arc<RwLock<ProcessorPipeline>>,
    config: BridgeIntegrationConfig,
}

/// Configuration for bridge integration
#[derive(Debug, Clone)]
pub struct BridgeIntegrationConfig {
    pub bridge_api_url: String,
    pub api_key: String,
    pub query_endpoint: String,
    pub results_endpoint: String,
    pub polling_interval_ms: u64,
    pub batch_size: usize,
    pub max_retries: u32,
}

impl Default for BridgeIntegrationConfig {
    fn default() -> Self {
        Self {
            bridge_api_url: "http://localhost:8080".to_string(),
            api_key: "demo-api-key".to_string(),
            query_endpoint: "/api/v1/telemetry/stream".to_string(),
            results_endpoint: "/api/v1/query/results".to_string(),
            polling_interval_ms: 1000,
            batch_size: 100,
            max_retries: 3,
        }
    }
}

impl BridgeIntegrationService {
    /// Create new bridge integration service
    pub fn new(config: BridgeIntegrationConfig) -> Self {
        Self {
            source_manager: Arc::new(RwLock::new(SourceManager::new())),
            sink_manager: Arc::new(RwLock::new(SinkManager::new())),
            pipeline: Arc::new(RwLock::new(ProcessorPipeline::new())),
            config,
        }
    }
    
    /// Execute a streaming query
    pub async fn execute_streaming_query(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<Vec<TelemetryQueryResult>> {
        info!("Executing streaming query: {}", query.id);
        
        // Create streaming processor configuration
        let streaming_config = self.create_streaming_config(query).await?;
        
        // Create HTTP source for data ingestion
        let http_source_config = HttpSourceConfig {
            name: "bridge_query_source".to_string(),
            version: "1.0.0".to_string(),
            endpoint_url: format!("{}{}", self.config.bridge_api_url, self.config.query_endpoint),
            method: "POST".to_string(),
            headers: {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers.insert("Authorization".to_string(), format!("Bearer {}", self.config.api_key));
                headers
            },
            body: Some(serde_json::to_string(query)?),
            polling_interval_ms: self.config.polling_interval_ms,
            request_timeout_secs: 30,
            batch_size: self.config.batch_size,
            buffer_size: 1000,
            auth_token: Some(self.config.api_key.clone()),
            additional_config: HashMap::new(),
            max_retries: self.config.max_retries,
            retry_delay_ms: 1000,
            rate_limit_requests_per_second: Some(10),
        };
        
        // Create filter processor based on query filters
        let filter_config = self.create_filter_config(query).await?;
        
        // Create HTTP sink for results
        let http_sink_config = HttpSinkConfig {
            name: "bridge_results_sink".to_string(),
            version: "1.0.0".to_string(),
            endpoint_url: format!("{}{}", self.config.bridge_api_url, self.config.results_endpoint),
            method: "POST".to_string(),
            headers: {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers.insert("Authorization".to_string(), format!("Bearer {}", self.config.api_key));
                headers
            },
            request_timeout_secs: 30,
            retry_count: self.config.max_retries,
            retry_delay_ms: 1000,
            batch_size: self.config.batch_size,
            auth_token: Some(self.config.api_key.clone()),
            additional_config: HashMap::new(),
            content_type: "application/json".to_string(),
            rate_limit_requests_per_second: Some(10),
        };
        
        // Process streaming data
        let mut results = Vec::new();
        let mut batch_count = 0;
        let max_batches = 5; // Limit to prevent infinite streaming
        
        while batch_count < max_batches {
            // Simulate streaming data processing
            let batch_result = self.create_mock_query_result(query, batch_count).await?;
            results.push(batch_result);
            
            batch_count += 1;
            
            // Simulate processing delay
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            
            // Check if we have enough data
            if batch_count >= 3 {
                break;
            }
        }
        
        info!("Completed streaming query: {} batches processed", batch_count);
        Ok(results)
    }
    
    /// Create streaming processor configuration
    async fn create_streaming_config(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<StreamingProcessorConfig> {
        let mut config = StreamingProcessorConfig::default();
        
        // Set global configuration
        config.global.log_level = "info".to_string();
        config.global.metrics_interval_secs = 30;
        config.global.health_check_interval_secs = 60;
        config.global.default_buffer_size = 1000;
        config.global.default_batch_size = self.config.batch_size;
        
        // Add pipeline configuration
        config.pipelines.push(streaming_processor::config::PipelineConfig {
            name: format!("bridge_pipeline_{}", query.id),
            description: Some("Bridge API integration pipeline".to_string()),
            components: vec![
                streaming_processor::config::PipelineComponent {
                    name: "http_source".to_string(),
                    component_type: "source".to_string(),
                    config_ref: "http_source".to_string(),
                    settings: HashMap::new(),
                },
                streaming_processor::config::PipelineComponent {
                    name: "filter_processor".to_string(),
                    component_type: "processor".to_string(),
                    config_ref: "filter_processor".to_string(),
                    settings: HashMap::new(),
                },
                streaming_processor::config::PipelineComponent {
                    name: "http_sink".to_string(),
                    component_type: "sink".to_string(),
                    config_ref: "http_sink".to_string(),
                    settings: HashMap::new(),
                },
            ],
            settings: streaming_processor::config::PipelineSettings {
                enabled: true,
                auto_start: true,
                timeout_secs: Some(300),
                retry: streaming_processor::config::RetryConfig {
                    max_attempts: 3,
                    delay_ms: 1000,
                    backoff_multiplier: 2.0,
                },
            },
        });
        
        Ok(config)
    }
    
    /// Create filter configuration based on query filters
    async fn create_filter_config(&self, query: &TelemetryQuery) -> BridgeResult<FilterProcessorConfig> {
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
                bridge_core::types::FilterOperator::GreaterThanOrEqual => FilterOperator::GreaterThanOrEqual,
                bridge_core::types::FilterOperator::LessThanOrEqual => FilterOperator::LessThanOrEqual,
                bridge_core::types::FilterOperator::In => FilterOperator::In,
                bridge_core::types::FilterOperator::NotIn => FilterOperator::NotIn,
                bridge_core::types::FilterOperator::StartsWith => FilterOperator::Contains, // Map to Contains for now
                bridge_core::types::FilterOperator::EndsWith => FilterOperator::Contains, // Map to Contains for now
                bridge_core::types::FilterOperator::Regex => FilterOperator::Regex,
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
            name: "bridge_query_filter".to_string(),
            version: "1.0.0".to_string(),
            filter_mode: FilterMode::Include,
            filter_rules,
            additional_config: HashMap::new(),
        })
    }
    
    /// Create mock query result for demonstration
    async fn create_mock_query_result(
        &self,
        query: &TelemetryQuery,
        batch_num: usize,
    ) -> BridgeResult<TelemetryQueryResult> {
        // Create mock data based on query type
        let mock_data = match self.determine_query_type(query) {
            Ok(query_type) => match query_type.as_str() {
                "metric" => self.create_mock_metrics_data(),
                "trace" => self.create_mock_traces_data(),
                "log" => self.create_mock_logs_data(),
                _ => self.create_mock_generic_data(),
            },
            Err(_) => self.create_mock_generic_data(),
        };
        
        Ok(TelemetryQueryResult {
            query_id: query.id,
            timestamp: Utc::now(),
            status: QueryResultStatus::Success,
            data: serde_json::to_value(mock_data)?,
            metadata: HashMap::from([
                ("batch_number".to_string(), batch_num.to_string()),
                ("source".to_string(), "bridge_integration".to_string()),
                ("processed_at".to_string(), Utc::now().to_rfc3339()),
            ]),
            errors: Vec::new(),
        })
    }
    
    /// Determine query type from filters and metadata
    fn determine_query_type(&self, query: &TelemetryQuery) -> BridgeResult<String> {
        // Check filters for type information
        for filter in &query.filters {
            if filter.field == "type" || filter.field == "record_type" {
                return Ok(match &filter.value {
                    bridge_core::types::FilterValue::String(s) => s.clone(),
                    bridge_core::types::FilterValue::Number(n) => n.to_string(),
                    bridge_core::types::FilterValue::Boolean(b) => b.to_string(),
                    bridge_core::types::FilterValue::Array(arr) => {
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
                });
            }
        }
        
        // Check metadata for type information
        if let Some(query_type) = query.metadata.get("type") {
            return Ok(query_type.clone());
        }
        
        // Default to generic query
        Ok("generic".to_string())
    }
    
    /// Create mock metrics data
    fn create_mock_metrics_data(&self) -> Vec<bridge_core::types::TelemetryRecord> {
        vec![
            bridge_core::types::TelemetryRecord::new(
                bridge_core::types::TelemetryType::Metric,
                bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
                    name: "cpu_usage".to_string(),
                    description: Some("CPU usage percentage".to_string()),
                    unit: Some("percent".to_string()),
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: bridge_core::types::MetricValue::Gauge(75.5),
                    labels: HashMap::from([
                        ("host".to_string(), "server-1".to_string()),
                        ("service".to_string(), "bridge-api".to_string()),
                    ]),
                    timestamp: Utc::now(),
                })
            )
        ]
    }
    
    /// Create mock traces data
    fn create_mock_traces_data(&self) -> Vec<bridge_core::types::TelemetryRecord> {
        vec![
            bridge_core::types::TelemetryRecord::new(
                bridge_core::types::TelemetryType::Trace,
                bridge_core::types::TelemetryData::Trace(bridge_core::types::TraceData {
                    trace_id: "trace-123".to_string(),
                    span_id: "span-456".to_string(),
                    parent_span_id: None,
                    name: "bridge-operation".to_string(),
                    kind: bridge_core::types::SpanKind::Internal,
                    start_time: Utc::now() - chrono::Duration::minutes(5),
                    end_time: Some(Utc::now()),
                    duration_ns: Some(1000000), // 1ms
                    status: bridge_core::types::SpanStatus {
                        code: bridge_core::types::StatusCode::Ok,
                        message: Some("Operation completed successfully".to_string()),
                    },
                    attributes: HashMap::new(),
                    events: Vec::new(),
                    links: Vec::new(),
                })
            )
        ]
    }
    
    /// Create mock logs data
    fn create_mock_logs_data(&self) -> Vec<bridge_core::types::TelemetryRecord> {
        vec![
            bridge_core::types::TelemetryRecord::new(
                bridge_core::types::TelemetryType::Log,
                bridge_core::types::TelemetryData::Log(bridge_core::types::LogData {
                    timestamp: Utc::now(),
                    level: bridge_core::types::LogLevel::Info,
                    message: "Bridge integration log message".to_string(),
                    attributes: HashMap::new(),
                    body: Some("Bridge integration log body".to_string()),
                    severity_number: Some(6),
                    severity_text: Some("INFO".to_string()),
                })
            )
        ]
    }
    
    /// Create mock generic data
    fn create_mock_generic_data(&self) -> Vec<bridge_core::types::TelemetryRecord> {
        vec![
            bridge_core::types::TelemetryRecord::new(
                bridge_core::types::TelemetryType::Metric,
                bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
                    name: "generic_metric".to_string(),
                    description: Some("Generic metric for demonstration".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Counter,
                    value: bridge_core::types::MetricValue::Counter(42.0),
                    labels: HashMap::from([
                        ("source".to_string(), "bridge_integration".to_string()),
                    ]),
                    timestamp: Utc::now(),
                })
            )
        ]
    }
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("Starting Bridge API integration example");
    
    // Create configuration
    let config = BridgeIntegrationConfig::default();
    
    // Create bridge integration service
    let service = BridgeIntegrationService::new(config);
    
    // Create a sample query
    let query = TelemetryQuery {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        time_range: bridge_core::types::TimeRange::last_hours(1),
        filters: vec![
            bridge_core::types::Filter::equals(
                "record_type".to_string(),
                bridge_core::types::FilterValue::String("metric".to_string()),
            ),
        ],
        limit: Some(100),
        aggregations: Vec::new(),
        metadata: HashMap::from([
            ("type".to_string(), "metric".to_string()),
            ("source".to_string(), "bridge_integration".to_string()),
        ]),
    };
    
    // Execute streaming query
    match service.execute_streaming_query(&query).await {
        Ok(results) => {
            info!("Successfully executed streaming query");
            info!("Number of result batches: {}", results.len());
            
            for (i, result) in results.iter().enumerate() {
                info!("Batch {}: {} records", i + 1, 
                    if let serde_json::Value::Array(arr) = &result.data {
                        arr.len()
                    } else {
                        0
                    }
                );
            }
        }
        Err(e) => {
            error!("Failed to execute streaming query: {}", e);
        }
    }
    
    info!("Bridge API integration example completed");
    Ok(())
}
