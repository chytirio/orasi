//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTLP gRPC protocol implementation
//!
//! This module provides support for ingesting OpenTelemetry data over gRPC
//! using the standard OTLP protocol.

use ::prost::Message;
use async_trait::async_trait;
use bridge_core::{
    types::{
        HistogramBucket, LogData, LogLevel, MetricData, MetricType, MetricValue, ResourceInfo,
        ServiceInfo, SummaryQuantile, TelemetryData, TelemetryRecord, TelemetryType,
    },
    BridgeResult, TelemetryBatch,
};
use chrono::{DateTime, Utc};
use bridge_core::receivers::otlp::otlp::opentelemetry::proto::collector::{
    logs::v1::logs_service_server::LogsService,
    logs::v1::{ExportLogsServiceRequest, ExportLogsServiceResponse},
    metrics::v1::metrics_service_server::MetricsService,
    metrics::v1::{ExportMetricsServiceRequest, ExportMetricsServiceResponse},
};
use bridge_core::receivers::otlp::otlp::opentelemetry::proto::common::v1::any_value::Value as AnyValueValue;
use bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::metric::Data as OtlpMetricData;
use bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::number_data_point::Value as NumberDataPointValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{transport::Server, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

use super::{MessageHandler, ProtocolConfig, ProtocolHandler, ProtocolMessage, ProtocolStats};

/// OTLP gRPC protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpGrpcConfig {
    /// Protocol name
    pub name: String,

    /// Protocol version
    pub version: String,

    /// gRPC endpoint address
    pub endpoint: String,

    /// Port to listen on
    pub port: u16,

    /// Enable TLS
    pub enable_tls: bool,

    /// TLS certificate path
    pub tls_cert_path: Option<String>,

    /// TLS key path
    pub tls_key_path: Option<String>,

    /// Batch size for processing
    pub batch_size: usize,

    /// Buffer size for incoming data
    pub buffer_size: usize,

    /// Enable compression
    pub enable_compression: bool,

    /// Authentication token (optional)
    pub auth_token: Option<String>,

    /// Additional headers
    pub headers: HashMap<String, String>,

    /// Timeout in seconds
    pub timeout_secs: u64,

    /// Max concurrent requests
    pub max_concurrent_requests: usize,
}

impl OtlpGrpcConfig {
    /// Create new OTLP gRPC configuration
    pub fn new(endpoint: String, port: u16) -> Self {
        Self {
            name: "otlp-grpc".to_string(),
            version: "1.0.0".to_string(),
            endpoint,
            port,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            batch_size: 1000,
            buffer_size: 10000,
            enable_compression: true,
            auth_token: None,
            headers: HashMap::new(),
            timeout_secs: 30,
            max_concurrent_requests: 100,
        }
    }
}

#[async_trait]
impl ProtocolConfig for OtlpGrpcConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.endpoint.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP gRPC endpoint cannot be empty",
            ));
        }

        if self.port == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP gRPC port cannot be 0",
            ));
        }

        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP gRPC batch size cannot be 0",
            ));
        }

        if self.enable_tls {
            if self.tls_cert_path.is_none() || self.tls_key_path.is_none() {
                return Err(bridge_core::BridgeError::configuration(
                    "TLS certificate and key paths are required when TLS is enabled",
                ));
            }
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// OTLP gRPC protocol handler
#[derive(Clone)]
pub struct OtlpGrpcProtocol {
    config: OtlpGrpcConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ProtocolStats>>,
    message_handler: Arc<dyn MessageHandler>,
    server: Option<OtlpGrpcServer>,
}

/// OTLP gRPC server wrapper
#[derive(Clone)]
struct OtlpGrpcServer {
    addr: std::net::SocketAddr,
    metrics_service: OtlpMetricsService,
    logs_service: OtlpLogsService,
}

impl OtlpGrpcProtocol {
    /// Create new OTLP gRPC protocol handler
    pub async fn new(config: &dyn ProtocolConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<OtlpGrpcConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid OTLP gRPC configuration")
            })?
            .clone();

        config.validate().await?;

        let stats = ProtocolStats {
            protocol: config.name.clone(),
            total_messages: 0,
            messages_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_message_time: None,
            is_connected: false,
        };

        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            message_handler: Arc::new(OtlpGrpcMessageHandler::new()),
            server: None,
        })
    }

    /// Initialize gRPC server
    async fn init_server(&mut self) -> BridgeResult<()> {
        info!(
            "Initializing OTLP gRPC server on {}:{}",
            self.config.endpoint, self.config.port
        );

        // Parse the endpoint address
        let addr = format!("{}:{}", self.config.endpoint, self.config.port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| {
                bridge_core::BridgeError::configuration(format!("Invalid OTLP gRPC address: {}", e))
            })?;

        // Create service instances
        let protocol_arc = Arc::new(self.clone());
        let metrics_service = OtlpMetricsService {
            protocol: Arc::clone(&protocol_arc),
        };
        let logs_service = OtlpLogsService {
            protocol: Arc::clone(&protocol_arc),
        };

        self.server = Some(OtlpGrpcServer {
            addr,
            metrics_service,
            logs_service,
        });

        info!("OTLP gRPC server initialized");
        Ok(())
    }

    /// Start gRPC server
    async fn start_server(&self) -> BridgeResult<()> {
        info!("Starting OTLP gRPC server");

        if let Some(server_wrapper) = &self.server {
            // Create the tonic server with OTLP services (metrics and logs)
            let server = Server::builder()
                .add_service(
                    bridge_core::receivers::otlp::otlp::opentelemetry::proto::collector::metrics::v1::metrics_service_server::MetricsServiceServer::new(
                        server_wrapper.metrics_service.clone()
                    )
                )
                .add_service(
                    bridge_core::receivers::otlp::otlp::opentelemetry::proto::collector::logs::v1::logs_service_server::LogsServiceServer::new(
                        server_wrapper.logs_service.clone()
                    )
                );

            // Start the server in a separate task
            let addr = server_wrapper.addr;
            tokio::spawn(async move {
                match server.serve(addr).await {
                    Ok(_) => info!("OTLP gRPC server stopped"),
                    Err(e) => error!("OTLP gRPC server error: {}", e),
                }
            });

            info!("OTLP gRPC server started on {}", addr);
        }

        Ok(())
    }

    /// Process OTLP gRPC metrics request
    async fn process_metrics_request(
        &self,
        request: ExportMetricsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        info!("Processing OTLP gRPC metrics request");

        let mut records = Vec::new();
        let mut total_size = 0;

        // Process each resource metrics in the request
        for resource_metrics in request.resource_metrics {
            // Extract resource information
            let resource_info = if let Some(resource) = resource_metrics.resource {
                let mut attributes = HashMap::new();
                for attr in resource.attributes {
                    if let Some(value) = attr.value {
                        attributes.insert(attr.key, self.extract_any_value(&value));
                    }
                }

                Some(ResourceInfo {
                    resource_type: attributes
                        .get("service.name")
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                    attributes: attributes.clone(),
                    resource_id: attributes.get("service.instance.id").cloned(),
                })
            } else {
                None
            };

            // Extract service information from resource
            let service_info = if let Some(ref resource) = resource_info {
                Some(ServiceInfo {
                    name: resource
                        .attributes
                        .get("service.name")
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                    version: resource.attributes.get("service.version").cloned(),
                    namespace: resource.attributes.get("service.namespace").cloned(),
                    instance_id: resource.attributes.get("service.instance.id").cloned(),
                })
            } else {
                None
            };

            // Process each scope metrics
            for scope_metrics in resource_metrics.scope_metrics {
                // Extract scope attributes
                let mut scope_attributes = HashMap::new();
                if let Some(scope) = scope_metrics.scope {
                    if !scope.name.is_empty() {
                        scope_attributes.insert("scope.name".to_string(), scope.name);
                    }
                    if !scope.version.is_empty() {
                        scope_attributes.insert("scope.version".to_string(), scope.version);
                    }
                    for attr in scope.attributes {
                        if let Some(value) = attr.value {
                            scope_attributes.insert(attr.key, self.extract_any_value(&value));
                        }
                    }
                }

                // Process each metric
                for metric in scope_metrics.metrics {
                    let metric_record = self.convert_otlp_metric_to_record(
                        metric,
                        resource_info.clone(),
                        service_info.clone(),
                        &scope_attributes,
                    )?;

                    total_size += std::mem::size_of_val(&metric_record);
                    records.push(metric_record);
                }
            }
        }

        let size = records.len();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_messages += 1;
            stats.total_bytes += total_size as u64;
            stats.last_message_time = Some(Utc::now());
        }

        Ok(TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            records,
            timestamp: Utc::now(),
            source: "otlp_grpc".to_string(),
            size,
            metadata: HashMap::new(),
        })
    }

    /// Process OTLP gRPC logs request
    async fn process_logs_request(
        &self,
        request: ExportLogsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        info!("Processing OTLP gRPC logs request");

        let mut records = Vec::new();
        let mut total_size = 0;

        // Process each resource logs in the request
        for resource_logs in request.resource_logs {
            // Extract resource information
            let resource_info = if let Some(resource) = resource_logs.resource {
                let mut attributes = HashMap::new();
                for attr in resource.attributes {
                    if let Some(value) = attr.value {
                        attributes.insert(attr.key, self.extract_any_value(&value));
                    }
                }

                Some(ResourceInfo {
                    resource_type: attributes
                        .get("service.name")
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                    attributes: attributes.clone(),
                    resource_id: attributes.get("service.instance.id").cloned(),
                })
            } else {
                None
            };

            // Extract service information from resource
            let service_info = if let Some(ref resource) = resource_info {
                Some(ServiceInfo {
                    name: resource
                        .attributes
                        .get("service.name")
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                    version: resource.attributes.get("service.version").cloned(),
                    namespace: resource.attributes.get("service.namespace").cloned(),
                    instance_id: resource.attributes.get("service.instance.id").cloned(),
                })
            } else {
                None
            };

            // Process each scope logs
            for scope_logs in resource_logs.scope_logs {
                // Extract scope attributes
                let mut scope_attributes = HashMap::new();
                if let Some(scope) = scope_logs.scope {
                    if !scope.name.is_empty() {
                        scope_attributes.insert("scope.name".to_string(), scope.name);
                    }
                    if !scope.version.is_empty() {
                        scope_attributes.insert("scope.version".to_string(), scope.version);
                    }
                    for attr in scope.attributes {
                        if let Some(value) = attr.value {
                            scope_attributes.insert(attr.key, self.extract_any_value(&value));
                        }
                    }
                }

                // Process each log record
                for log_record in scope_logs.log_records {
                    let log_record = self.convert_otlp_log_to_record(
                        log_record,
                        resource_info.clone(),
                        service_info.clone(),
                        &scope_attributes,
                    )?;

                    total_size += std::mem::size_of_val(&log_record);
                    records.push(log_record);
                }
            }
        }

        let size = records.len();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_messages += 1;
            stats.total_bytes += total_size as u64;
            stats.last_message_time = Some(Utc::now());
        }

        Ok(TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            records,
            timestamp: Utc::now(),
            source: "otlp_grpc".to_string(),
            size,
            metadata: HashMap::new(),
        })
    }

    /// Extract string value from OTLP AnyValue
    fn extract_any_value(
        &self,
        any_value: &bridge_core::receivers::otlp::otlp::opentelemetry::proto::common::v1::AnyValue,
    ) -> String {
        match &any_value.value {
            Some(AnyValueValue::StringValue(s)) => s.clone(),
            Some(AnyValueValue::BoolValue(b)) => b.to_string(),
            Some(AnyValueValue::IntValue(i)) => i.to_string(),
            Some(AnyValueValue::DoubleValue(d)) => d.to_string(),
            Some(AnyValueValue::BytesValue(b)) => format!("{:?}", b),
            Some(AnyValueValue::ArrayValue(arr)) => {
                let values: Vec<String> = arr
                    .values
                    .iter()
                    .map(|v| self.extract_any_value(v))
                    .collect();
                format!("[{}]", values.join(", "))
            }
            Some(AnyValueValue::KvlistValue(kv)) => {
                let pairs: Vec<String> = kv
                    .values
                    .iter()
                    .map(|kv| {
                        format!(
                            "{}: {}",
                            kv.key,
                            self.extract_any_value(&kv.value.as_ref().unwrap_or(
                                &bridge_core::receivers::otlp::otlp::opentelemetry::proto::common::v1::AnyValue::default()
                            ))
                        )
                    })
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
            None => "".to_string(),
        }
    }

    /// Convert OTLP metric to TelemetryRecord
    fn convert_otlp_metric_to_record(
        &self,
        metric: bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::Metric,
        resource_info: Option<ResourceInfo>,
        service_info: Option<ServiceInfo>,
        scope_attributes: &HashMap<String, String>,
    ) -> BridgeResult<TelemetryRecord> {
        let mut attributes = HashMap::new();

        // Add scope attributes
        for (key, value) in scope_attributes {
            attributes.insert(key.clone(), value.clone());
        }

        // Note: OTLP metrics don't have direct attributes, they use data point attributes
        // Attributes are extracted from data points in the specific metric type processing

        // Convert metric data based on type
        let metric_data = match metric.data {
            Some(OtlpMetricData::Gauge(gauge)) => {
                if let Some(data_point) = gauge.data_points.first() {
                    let value = match &data_point.value {
                        Some(NumberDataPointValue::AsDouble(d)) => *d,
                        Some(NumberDataPointValue::AsInt(i)) => *i as f64,
                        None => 0.0,
                    };

                    MetricData {
                        name: metric.name,
                        description: Some(metric.description),
                        unit: Some(metric.unit),
                        metric_type: MetricType::Gauge,
                        value: MetricValue::Gauge(value),
                        labels: attributes.clone(),
                        timestamp: DateTime::from_timestamp_millis(
                            (data_point.time_unix_nano / 1_000_000) as i64,
                        )
                        .unwrap_or_else(|| Utc::now()),
                    }
                } else {
                    return Err(bridge_core::BridgeError::processing(
                        "No data points in gauge metric",
                    ));
                }
            }
            Some(OtlpMetricData::Sum(sum)) => {
                if let Some(data_point) = sum.data_points.first() {
                    let value = match &data_point.value {
                        Some(NumberDataPointValue::AsDouble(d)) => *d,
                        Some(NumberDataPointValue::AsInt(i)) => *i as f64,
                        None => 0.0,
                    };

                    let metric_type = if sum.is_monotonic {
                        MetricType::Counter
                    } else {
                        MetricType::Gauge
                    };

                    let metric_value = if sum.is_monotonic {
                        MetricValue::Counter(value)
                    } else {
                        MetricValue::Gauge(value)
                    };

                    MetricData {
                        name: metric.name,
                        description: Some(metric.description),
                        unit: Some(metric.unit),
                        metric_type,
                        value: metric_value,
                        labels: attributes.clone(),
                        timestamp: DateTime::from_timestamp_millis(
                            (data_point.time_unix_nano / 1_000_000) as i64,
                        )
                        .unwrap_or_else(|| Utc::now()),
                    }
                } else {
                    return Err(bridge_core::BridgeError::processing(
                        "No data points in sum metric",
                    ));
                }
            }
            Some(OtlpMetricData::Histogram(histogram)) => {
                if let Some(data_point) = histogram.data_points.first() {
                    // Extract histogram buckets from OTLP data
                    let mut buckets = Vec::new();

                    // Process bucket counts and explicit bounds
                    if !data_point.bucket_counts.is_empty()
                        && !data_point.explicit_bounds.is_empty()
                    {
                        // Create buckets based on explicit bounds
                        for (i, &count) in data_point.bucket_counts.iter().enumerate() {
                            let upper_bound = if i < data_point.explicit_bounds.len() {
                                data_point.explicit_bounds[i]
                            } else {
                                // Last bucket goes to infinity
                                f64::INFINITY
                            };

                            buckets.push(HistogramBucket { upper_bound, count });
                        }
                    } else if !data_point.bucket_counts.is_empty() {
                        // If no explicit bounds, create default buckets
                        // This is a fallback for histograms without explicit bounds
                        let bucket_count = data_point.bucket_counts.len();
                        for (i, &count) in data_point.bucket_counts.iter().enumerate() {
                            let upper_bound = if i == bucket_count - 1 {
                                f64::INFINITY
                            } else {
                                // Create a simple linear scale as fallback
                                (i + 1) as f64
                            };

                            buckets.push(HistogramBucket { upper_bound, count });
                        }
                    } else {
                        // No bucket data available, create a single bucket with total count
                        buckets.push(HistogramBucket {
                            upper_bound: f64::INFINITY,
                            count: data_point.count,
                        });
                    }

                    // Extract attributes from data point
                    let mut data_point_attributes = attributes.clone();
                    for attr in &data_point.attributes {
                        if let Some(value) = &attr.value {
                            data_point_attributes
                                .insert(attr.key.clone(), self.extract_any_value(value));
                        }
                    }

                    MetricData {
                        name: metric.name,
                        description: Some(metric.description),
                        unit: Some(metric.unit),
                        metric_type: MetricType::Histogram,
                        value: MetricValue::Histogram {
                            buckets,
                            sum: data_point.sum.unwrap_or(0.0),
                            count: data_point.count,
                        },
                        labels: data_point_attributes,
                        timestamp: DateTime::from_timestamp_millis(
                            (data_point.time_unix_nano / 1_000_000) as i64,
                        )
                        .unwrap_or_else(|| Utc::now()),
                    }
                } else {
                    return Err(bridge_core::BridgeError::processing(
                        "No data points in histogram metric",
                    ));
                }
            }
            Some(OtlpMetricData::Summary(summary)) => {
                if let Some(data_point) = summary.data_points.first() {
                    let quantiles: Vec<SummaryQuantile> = data_point
                        .quantile_values
                        .iter()
                        .map(|q| SummaryQuantile {
                            quantile: q.quantile,
                            value: q.value,
                        })
                        .collect();

                    MetricData {
                        name: metric.name,
                        description: Some(metric.description),
                        unit: Some(metric.unit),
                        metric_type: MetricType::Summary,
                        value: MetricValue::Summary {
                            quantiles,
                            sum: data_point.sum,
                            count: data_point.count,
                        },
                        labels: attributes.clone(),
                        timestamp: DateTime::from_timestamp_millis(
                            (data_point.time_unix_nano / 1_000_000) as i64,
                        )
                        .unwrap_or_else(|| Utc::now()),
                    }
                } else {
                    return Err(bridge_core::BridgeError::processing(
                        "No data points in summary metric",
                    ));
                }
            }
            Some(OtlpMetricData::ExponentialHistogram(exponential_histogram)) => {
                if let Some(data_point) = exponential_histogram.data_points.first() {
                    // Convert exponential histogram to regular histogram buckets
                    let mut buckets = Vec::new();

                    // Calculate base from scale: base = 2^(2^-scale)
                    let scale = data_point.scale;
                    let base = 2.0_f64.powf(2.0_f64.powf(-scale as f64));

                    // Process positive buckets
                    if let Some(positive_buckets) = &data_point.positive {
                        for (i, &count) in positive_buckets.bucket_counts.iter().enumerate() {
                            if count > 0 {
                                let bucket_index = positive_buckets.offset + i as i32;
                                let lower_bound = if bucket_index == 0 {
                                    0.0
                                } else {
                                    base.powi(bucket_index)
                                };
                                let upper_bound = base.powi(bucket_index + 1);

                                buckets.push(HistogramBucket { upper_bound, count });
                            }
                        }
                    }

                    // Process negative buckets
                    if let Some(negative_buckets) = &data_point.negative {
                        for (i, &count) in negative_buckets.bucket_counts.iter().enumerate() {
                            if count > 0 {
                                let bucket_index = negative_buckets.offset + i as i32;
                                let lower_bound = -base.powi(bucket_index + 1);
                                let upper_bound = -base.powi(bucket_index);

                                buckets.push(HistogramBucket { upper_bound, count });
                            }
                        }
                    }

                    // Add zero bucket if it has counts
                    if data_point.zero_count > 0 {
                        buckets.push(HistogramBucket {
                            upper_bound: 0.0,
                            count: data_point.zero_count,
                        });
                    }

                    // Sort buckets by upper bound
                    buckets.sort_by(|a, b| {
                        a.upper_bound
                            .partial_cmp(&b.upper_bound)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                    // Extract attributes from data point
                    let mut data_point_attributes = attributes.clone();
                    for attr in &data_point.attributes {
                        if let Some(value) = &attr.value {
                            data_point_attributes
                                .insert(attr.key.clone(), self.extract_any_value(value));
                        }
                    }

                    // Add exponential histogram specific attributes
                    data_point_attributes
                        .insert("histogram_type".to_string(), "exponential".to_string());
                    data_point_attributes.insert("scale".to_string(), scale.to_string());
                    data_point_attributes.insert("base".to_string(), base.to_string());

                    MetricData {
                        name: metric.name,
                        description: Some(metric.description),
                        unit: Some(metric.unit),
                        metric_type: MetricType::Histogram,
                        value: MetricValue::Histogram {
                            buckets,
                            sum: data_point.sum.unwrap_or(0.0),
                            count: data_point.count,
                        },
                        labels: data_point_attributes,
                        timestamp: DateTime::from_timestamp_millis(
                            (data_point.time_unix_nano / 1_000_000) as i64,
                        )
                        .unwrap_or_else(|| Utc::now()),
                    }
                } else {
                    return Err(bridge_core::BridgeError::processing(
                        "No data points in exponential histogram metric",
                    ));
                }
            }
            None => {
                return Err(bridge_core::BridgeError::processing(
                    "No metric data available",
                ));
            }
        };

        Ok(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: metric_data.timestamp,
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(metric_data),
            attributes,
            tags: HashMap::new(),
            resource: resource_info,
            service: service_info,
        })
    }

    /// Convert OTLP log to TelemetryRecord
    fn convert_otlp_log_to_record(
        &self,
        log_record: bridge_core::receivers::otlp::otlp::opentelemetry::proto::logs::v1::LogRecord,
        resource_info: Option<ResourceInfo>,
        service_info: Option<ServiceInfo>,
        scope_attributes: &HashMap<String, String>,
    ) -> BridgeResult<TelemetryRecord> {
        let mut attributes = HashMap::new();

        // Add scope attributes
        for (key, value) in scope_attributes {
            attributes.insert(key.clone(), value.clone());
        }

        // Extract log attributes
        for attr in log_record.attributes {
            if let Some(value) = attr.value {
                attributes.insert(attr.key, self.extract_any_value(&value));
            }
        }

        // Convert severity number to log level
        let level = match log_record.severity_number {
            1..=4 => LogLevel::Trace,
            5..=8 => LogLevel::Debug,
            9..=12 => LogLevel::Info,
            13..=16 => LogLevel::Warn,
            17..=20 => LogLevel::Error,
            21..=24 => LogLevel::Fatal,
            _ => LogLevel::Info,
        };

        // Extract log body
        let body = if let Some(ref body) = log_record.body {
            Some(self.extract_any_value(body))
        } else {
            None
        };

        let log_data = LogData {
            timestamp: DateTime::from_timestamp_millis(
                (log_record.time_unix_nano / 1_000_000) as i64,
            )
            .unwrap_or_else(|| Utc::now()),
            level,
            message: log_record
                .body
                .as_ref()
                .map(|b| self.extract_any_value(b))
                .unwrap_or_else(|| "".to_string()),
            attributes,
            body,
            severity_number: Some(log_record.severity_number as u32),
            severity_text: Some(log_record.severity_text),
        };

        Ok(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: log_data.timestamp,
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(log_data),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: resource_info,
            service: service_info,
        })
    }
}

#[async_trait]
impl ProtocolHandler for OtlpGrpcProtocol {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing OTLP gRPC protocol handler");

        // Validate configuration
        self.config.validate().await?;

        // Initialize gRPC server
        self.init_server().await?;

        info!("OTLP gRPC protocol handler initialized");
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting OTLP gRPC protocol handler");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
        }

        // Start gRPC server
        self.start_server().await?;

        info!("OTLP gRPC protocol handler started");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping OTLP gRPC protocol handler");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }

        info!("OTLP gRPC protocol handler stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        // This is a simplified check - in practice we'd need to handle the async nature
        false
    }

    async fn get_stats(&self) -> BridgeResult<ProtocolStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn receive_data(&self) -> BridgeResult<Option<TelemetryBatch>> {
        // For OTLP gRPC, data is received through the gRPC service handlers
        // This method is not used in the gRPC flow since data comes through
        // the service export methods directly

        // Update stats to indicate we're ready to receive data
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
            stats.last_message_time = Some(Utc::now());
        }

        // Return None since gRPC data is handled asynchronously through service calls
        Ok(None)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// OTLP gRPC message handler
pub struct OtlpGrpcMessageHandler;

impl OtlpGrpcMessageHandler {
    pub fn new() -> Self {
        Self
    }

    /// Process metrics request
    async fn process_metrics_request(
        &self,
        request: ExportMetricsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        // For now, return empty batch - implement actual processing later
        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp-grpc".to_string(),
            size: 0,
            records: vec![],
            metadata: HashMap::new(),
        })
    }

    /// Process logs request
    async fn process_logs_request(
        &self,
        request: ExportLogsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        // For now, return empty batch - implement actual processing later
        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp-grpc".to_string(),
            size: 0,
            records: vec![],
            metadata: HashMap::new(),
        })
    }

    /// Decode OTLP gRPC message
    async fn decode_otlp_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // Try to decode as ExportMetricsServiceRequest first
        if let Ok(request) = ExportMetricsServiceRequest::decode(payload) {
            let batch = self.process_metrics_request(request).await?;
            return Ok(batch.records);
        }

        // Try to decode as ExportLogsServiceRequest
        if let Ok(request) = ExportLogsServiceRequest::decode(payload) {
            let batch = self.process_logs_request(request).await?;
            return Ok(batch.records);
        }

        // Try to decode as ExportTraceServiceRequest (if traces are enabled)
        // if let Ok(request) = ExportTraceServiceRequest::decode(payload) {
        //     let batch = self.process_traces_request(request).await?;
        //     return Ok(batch.records);
        // }

        // If none of the above work, try to decode as a generic OTLP message
        // This is a fallback for unknown message types
        info!("Unknown OTLP message type, creating generic record");

        Ok(vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: bridge_core::types::LogLevel::Warn,
                message: "Unknown OTLP message type".to_string(),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otlp-grpc".to_string()),
                    ("message_size".to_string(), payload.len().to_string()),
                ]),
                body: None,
                severity_number: Some(13),
                severity_text: Some("warn".to_string()),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }])
    }
}

#[async_trait]
impl MessageHandler for OtlpGrpcMessageHandler {
    async fn handle_message(&self, message: ProtocolMessage) -> BridgeResult<TelemetryBatch> {
        let records = match &message.payload {
            super::MessagePayload::Protobuf(data) => self.decode_otlp_message(data).await?,
            super::MessagePayload::Json(data) => self.decode_otlp_message(data).await?,
            super::MessagePayload::Arrow(data) => self.decode_otlp_message(data).await?,
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: message.timestamp,
            source: message.protocol,
            size: records.len(),
            records,
            metadata: message.metadata,
        })
    }

    async fn handle_batch(
        &self,
        messages: Vec<ProtocolMessage>,
    ) -> BridgeResult<Vec<TelemetryBatch>> {
        let mut batches = Vec::new();

        for message in messages {
            let batch = self.handle_message(message).await?;
            batches.push(batch);
        }

        Ok(batches)
    }
}

/// OTLP gRPC metrics service implementation
#[derive(Clone)]
pub struct OtlpMetricsService {
    protocol: Arc<OtlpGrpcProtocol>,
}

#[tonic::async_trait]
impl MetricsService for OtlpMetricsService {
    async fn export(
        &self,
        request: tonic::Request<ExportMetricsServiceRequest>,
    ) -> Result<tonic::Response<ExportMetricsServiceResponse>, Status> {
        let request_data = request.into_inner();

        match self.protocol.process_metrics_request(request_data).await {
            Ok(_batch) => {
                let response = ExportMetricsServiceResponse {
                    partial_success: None,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to process metrics request: {}", e);
                Err(Status::internal("Failed to process metrics"))
            }
        }
    }
}

/// OTLP gRPC logs service implementation
#[derive(Clone)]
pub struct OtlpLogsService {
    protocol: Arc<OtlpGrpcProtocol>,
}

#[tonic::async_trait]
impl LogsService for OtlpLogsService {
    async fn export(
        &self,
        request: tonic::Request<ExportLogsServiceRequest>,
    ) -> Result<tonic::Response<ExportLogsServiceResponse>, Status> {
        let request_data = request.into_inner();

        match self.protocol.process_logs_request(request_data).await {
            Ok(_batch) => {
                let response = ExportLogsServiceResponse {
                    partial_success: None,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to process logs request: {}", e);
                Err(Status::internal("Failed to process logs"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::receivers::otlp::otlp::opentelemetry::proto::collector::logs::v1::ExportLogsServiceRequest;
    use bridge_core::receivers::otlp::otlp::opentelemetry::proto::collector::metrics::v1::ExportMetricsServiceRequest;

    #[tokio::test]
    async fn test_otlp_grpc_config_creation() {
        let config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
        assert_eq!(config.name, "otlp-grpc");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.endpoint, "127.0.0.1");
        assert_eq!(config.port, 4317);
    }

    #[tokio::test]
    async fn test_otlp_grpc_protocol_creation() {
        let config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
        let protocol = OtlpGrpcProtocol::new(&config).await;
        assert!(protocol.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_processing() {
        let config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
        let protocol = OtlpGrpcProtocol::new(&config).await.unwrap();

        // Create an empty metrics request
        let request = ExportMetricsServiceRequest {
            resource_metrics: vec![],
        };

        let result = protocol.process_metrics_request(request).await;
        assert!(result.is_ok());

        let batch = result.unwrap();
        assert_eq!(batch.source, "otlp_grpc");
        assert_eq!(batch.size, 0);
    }

    #[tokio::test]
    async fn test_logs_processing() {
        let config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
        let protocol = OtlpGrpcProtocol::new(&config).await.unwrap();

        // Create an empty logs request
        let request = ExportLogsServiceRequest {
            resource_logs: vec![],
        };

        let result = protocol.process_logs_request(request).await;
        assert!(result.is_ok());

        let batch = result.unwrap();
        assert_eq!(batch.source, "otlp_grpc");
        assert_eq!(batch.size, 0);
    }
}
