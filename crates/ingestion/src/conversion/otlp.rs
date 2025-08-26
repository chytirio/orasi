//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTLP converter implementation

use ::prost::Message;
use bridge_core::types::{
    LogData, LogLevel, MetricData, MetricType, MetricValue, SpanKind, StatusCode, TelemetryData,
    TelemetryRecord, TelemetryType,
};
use bridge_core::{BridgeResult, TelemetryBatch};
use bridge_core::receivers::otlp::otlp::opentelemetry::proto::{
    collector::{
        logs::v1::ExportLogsServiceRequest, metrics::v1::ExportMetricsServiceRequest,
        trace::v1::ExportTraceServiceRequest,
    },
    common::v1::{any_value::Value as AnyValueValue, AnyValue, KeyValue},
    logs::v1::{LogRecord, ResourceLogs, ScopeLogs},
    metrics::v1::{
        metric::Data as MetricDataProto, number_data_point::Value as NumberDataPointValue, Metric,
        NumberDataPoint, ResourceMetrics, ScopeMetrics,
    },
    resource::v1::Resource,
    trace::v1::{ResourceSpans, ScopeSpans, Span},
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use super::converter::TelemetryConverter;

/// OTLP converter implementation
pub struct OtlpConverter;

#[async_trait::async_trait]
impl TelemetryConverter for OtlpConverter {
    async fn convert(
        &self,
        data: &[u8],
        source_format: &str,
        target_format: &str,
    ) -> BridgeResult<Vec<u8>> {
        match (source_format, target_format) {
            ("otlp", "internal") => {
                let batch = self.convert_from_otlp(data).await?;
                // Serialize to internal format using serde_json
                let serialized = serde_json::to_vec(&batch).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to serialize TelemetryBatch to JSON: {}",
                        e
                    ))
                })?;
                Ok(serialized)
            }
            ("internal", "otlp") => {
                // Deserialize from internal format using serde_json
                let batch: TelemetryBatch = serde_json::from_slice(data).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to deserialize TelemetryBatch from JSON: {}",
                        e
                    ))
                })?;
                self.convert_to_otlp(&batch).await
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported conversion: {} -> {}",
                source_format, target_format
            ))),
        }
    }

    fn supported_source_formats(&self) -> Vec<String> {
        vec!["otlp".to_string(), "internal".to_string()]
    }

    fn supported_target_formats(&self) -> Vec<String> {
        vec!["otlp".to_string(), "internal".to_string()]
    }
}

impl OtlpConverter {
    /// Create new OTLP converter
    pub fn new() -> Self {
        Self
    }

    /// Convert OTLP data to internal format
    pub async fn convert_from_otlp(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        info!(
            "Converting OTLP data of {} bytes to internal format",
            data.len()
        );

        // Try to decode as different OTLP message types
        if let Ok(metrics_request) = ExportMetricsServiceRequest::decode(data) {
            return self.convert_otlp_metrics(metrics_request).await;
        }

        if let Ok(logs_request) = ExportLogsServiceRequest::decode(data) {
            return self.convert_otlp_logs(logs_request).await;
        }

        if let Ok(traces_request) = ExportTraceServiceRequest::decode(data) {
            return self.convert_otlp_traces(traces_request).await;
        }

        Err(bridge_core::BridgeError::serialization(
            "Failed to decode OTLP data as any known message type".to_string(),
        ))
    }

    /// Convert OTLP metrics to internal format
    async fn convert_otlp_metrics(
        &self,
        request: ExportMetricsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        let mut all_records = Vec::new();

        for resource_metrics in request.resource_metrics {
            let resource_attrs = self.convert_attributes(&resource_metrics.resource);

            for scope_metrics in resource_metrics.scope_metrics {
                for metric in scope_metrics.metrics {
                    let records = self.convert_otlp_metric(metric, &resource_attrs).await?;
                    all_records.extend(records);
                }
            }
        }

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp_metrics".to_string(),
            size: all_records.len(),
            records: all_records,
            metadata: HashMap::new(),
        })
    }

    /// Convert a single OTLP metric to internal records
    async fn convert_otlp_metric(
        &self,
        metric: Metric,
        resource_attrs: &HashMap<String, String>,
    ) -> BridgeResult<Vec<TelemetryRecord>> {
        let mut records = Vec::new();

        match metric.data {
            Some(MetricDataProto::Gauge(gauge)) => {
                for point in gauge.data_points {
                    let value = match point.value {
                        Some(NumberDataPointValue::AsDouble(v)) => MetricValue::Gauge(v),
                        Some(NumberDataPointValue::AsInt(v)) => MetricValue::Gauge(v as f64),
                        None => continue,
                    };

                    let timestamp = if point.time_unix_nano > 0 {
                        DateTime::from_timestamp_nanos(point.time_unix_nano as i64)
                    } else {
                        Utc::now()
                    };

                    let record = TelemetryRecord {
                        id: Uuid::new_v4(),
                        timestamp,
                        record_type: TelemetryType::Metric,
                        data: TelemetryData::Metric(MetricData {
                            name: metric.name.clone(),
                            description: Some(metric.description.clone()),
                            unit: Some(metric.unit.clone()),
                            metric_type: MetricType::Gauge,
                            value,
                            labels: self.convert_key_values(&point.attributes),
                            timestamp,
                        }),
                        attributes: resource_attrs.clone(),
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    };

                    records.push(record);
                }
            }
            Some(MetricDataProto::Sum(sum)) => {
                for point in sum.data_points {
                    let value = match point.value {
                        Some(NumberDataPointValue::AsDouble(v)) => MetricValue::Gauge(v),
                        Some(NumberDataPointValue::AsInt(v)) => MetricValue::Gauge(v as f64),
                        None => continue,
                    };

                    let timestamp = if point.time_unix_nano > 0 {
                        DateTime::from_timestamp_nanos(point.time_unix_nano as i64)
                    } else {
                        Utc::now()
                    };

                    let metric_type = if sum.is_monotonic {
                        MetricType::Counter
                    } else {
                        MetricType::Gauge
                    };

                    let record = TelemetryRecord {
                        id: Uuid::new_v4(),
                        timestamp,
                        record_type: TelemetryType::Metric,
                        data: TelemetryData::Metric(MetricData {
                            name: metric.name.clone(),
                            description: Some(metric.description.clone()),
                            unit: Some(metric.unit.clone()),
                            metric_type,
                            value,
                            labels: self.convert_key_values(&point.attributes),
                            timestamp,
                        }),
                        attributes: resource_attrs.clone(),
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    };

                    records.push(record);
                }
            }
            Some(MetricDataProto::Histogram(histogram)) => {
                // For histogram, we'll create a summary metric
                for point in histogram.data_points {
                    let timestamp = if point.time_unix_nano > 0 {
                        DateTime::from_timestamp_nanos(point.time_unix_nano as i64)
                    } else {
                        Utc::now()
                    };

                    // Use count as the primary value for histogram
                    let value = MetricValue::Gauge(point.count as f64);

                    let record = TelemetryRecord {
                        id: Uuid::new_v4(),
                        timestamp,
                        record_type: TelemetryType::Metric,
                        data: TelemetryData::Metric(MetricData {
                            name: metric.name.clone(),
                            description: Some(metric.description.clone()),
                            unit: Some(metric.unit.clone()),
                            metric_type: MetricType::Histogram,
                            value,
                            labels: self.convert_key_values(&point.attributes),
                            timestamp,
                        }),
                        attributes: resource_attrs.clone(),
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    };

                    records.push(record);
                }
            }
            _ => {
                info!("Unsupported metric type for metric: {}", metric.name);
            }
        }

        Ok(records)
    }

    /// Convert OTLP logs to internal format
    async fn convert_otlp_logs(
        &self,
        request: ExportLogsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        let mut all_records = Vec::new();

        for resource_logs in request.resource_logs {
            let resource_attrs = self.convert_attributes(&resource_logs.resource);

            for scope_logs in resource_logs.scope_logs {
                for log_record in scope_logs.log_records {
                    let record = self.convert_otlp_log(log_record, &resource_attrs).await?;
                    all_records.push(record);
                }
            }
        }

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp_logs".to_string(),
            size: all_records.len(),
            records: all_records,
            metadata: HashMap::new(),
        })
    }

    /// Convert a single OTLP log to internal record
    async fn convert_otlp_log(
        &self,
        log: LogRecord,
        resource_attrs: &HashMap<String, String>,
    ) -> BridgeResult<TelemetryRecord> {
        let timestamp = if log.time_unix_nano > 0 {
            DateTime::from_timestamp_nanos(log.time_unix_nano as i64)
        } else {
            Utc::now()
        };

        let level = match log.severity_number {
            1..=4 => LogLevel::Trace,
            5..=8 => LogLevel::Debug,
            9..=12 => LogLevel::Info,
            13..=16 => LogLevel::Warn,
            17..=20 => LogLevel::Error,
            21..=24 => LogLevel::Fatal,
            _ => LogLevel::Info,
        };

        let message = log
            .body
            .and_then(|body| body.value)
            .map(|value| match value {
                AnyValueValue::StringValue(s) => s,
                AnyValueValue::IntValue(i) => i.to_string(),
                AnyValueValue::DoubleValue(d) => d.to_string(),
                AnyValueValue::BoolValue(b) => b.to_string(),
                _ => "".to_string(),
            })
            .unwrap_or_else(|| "".to_string());

        Ok(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp,
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp,
                level,
                message: message.clone(),
                attributes: self.convert_key_values(&log.attributes),
                body: Some(message),
                severity_number: Some(log.severity_number.try_into().unwrap_or(9)),
                severity_text: Some(log.severity_text),
            }),
            attributes: resource_attrs.clone(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        })
    }

    /// Convert OTLP traces to internal format
    async fn convert_otlp_traces(
        &self,
        request: ExportTraceServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        let mut all_records = Vec::new();

        for resource_spans in request.resource_spans {
            let resource_attrs = self.convert_attributes(&resource_spans.resource);

            for scope_spans in resource_spans.scope_spans {
                for span in scope_spans.spans {
                    let record = self.convert_otlp_span(span, &resource_attrs).await?;
                    all_records.push(record);
                }
            }
        }

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp_traces".to_string(),
            size: all_records.len(),
            records: all_records,
            metadata: HashMap::new(),
        })
    }

    /// Convert a single OTLP span to internal record
    async fn convert_otlp_span(
        &self,
        span: Span,
        resource_attrs: &HashMap<String, String>,
    ) -> BridgeResult<TelemetryRecord> {
        let timestamp = if span.start_time_unix_nano > 0 {
            DateTime::from_timestamp_nanos(span.start_time_unix_nano as i64)
        } else {
            Utc::now()
        };

        let kind = match span.kind() {
            bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Internal => SpanKind::Internal,
            bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Server => SpanKind::Server,
            bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Client => SpanKind::Client,
            bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Producer => SpanKind::Producer,
            bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Consumer => SpanKind::Consumer,
            _ => SpanKind::Internal,
        };

        let status = match span.status.as_ref() {
            Some(status) => match status.code() {
                bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::status::StatusCode::Ok => StatusCode::Ok,
                bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::status::StatusCode::Error => {
                    StatusCode::Error
                }
                _ => StatusCode::Unset,
            },
            None => StatusCode::Unset,
        };

        Ok(TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp,
            record_type: TelemetryType::Trace,
            data: TelemetryData::Trace(bridge_core::types::TraceData {
                trace_id: hex::encode(span.trace_id),
                span_id: hex::encode(span.span_id),
                parent_span_id: Some(hex::encode(span.parent_span_id)),
                name: span.name,
                kind,
                status: bridge_core::types::SpanStatus {
                    code: status,
                    message: None,
                },
                start_time: timestamp,
                end_time: if span.end_time_unix_nano > 0 {
                    Some(DateTime::from_timestamp_nanos(
                        span.end_time_unix_nano as i64,
                    ))
                } else {
                    None
                },
                duration_ns: if span.start_time_unix_nano > 0 && span.end_time_unix_nano > 0 {
                    Some((span.end_time_unix_nano - span.start_time_unix_nano) as u64)
                } else {
                    None
                },
                attributes: self.convert_key_values(&span.attributes),
                events: self.convert_otlp_span_events(&span.events),
                links: self.convert_otlp_span_links(&span.links),
            }),
            attributes: resource_attrs.clone(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        })
    }

    /// Convert KeyValue list to HashMap
    fn convert_key_values(&self, key_values: &[KeyValue]) -> HashMap<String, String> {
        key_values.iter().fold(HashMap::new(), |mut acc, kv| {
            let value = match &kv.value {
                Some(any_value) => match &any_value.value {
                    Some(AnyValueValue::StringValue(s)) => s.clone(),
                    Some(AnyValueValue::IntValue(i)) => i.to_string(),
                    Some(AnyValueValue::DoubleValue(d)) => d.to_string(),
                    Some(AnyValueValue::BoolValue(b)) => b.to_string(),
                    _ => "".to_string(),
                },
                None => "".to_string(),
            };
            acc.insert(kv.key.clone(), value);
            acc
        })
    }

    /// Convert OTLP span events to internal format
    fn convert_otlp_span_events(
        &self,
        events: &[bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::Event],
    ) -> Vec<bridge_core::types::SpanEvent> {
        events
            .iter()
            .map(|event| bridge_core::types::SpanEvent {
                name: event.name.clone(),
                timestamp: if event.time_unix_nano > 0 {
                    DateTime::from_timestamp_nanos(event.time_unix_nano as i64)
                } else {
                    Utc::now()
                },
                attributes: self.convert_key_values(&event.attributes),
            })
            .collect()
    }

    /// Convert OTLP span links to internal format
    fn convert_otlp_span_links(
        &self,
        links: &[bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::Link],
    ) -> Vec<bridge_core::types::SpanLink> {
        links
            .iter()
            .map(|link| bridge_core::types::SpanLink {
                trace_id: hex::encode(&link.trace_id),
                span_id: hex::encode(&link.span_id),
                attributes: self.convert_key_values(&link.attributes),
            })
            .collect()
    }

    /// Convert Resource attributes
    fn convert_attributes(&self, resource: &Option<Resource>) -> HashMap<String, String> {
        resource
            .as_ref()
            .map(|r| self.convert_key_values(&r.attributes))
            .unwrap_or_default()
    }

    /// Convert internal format to OTLP
    pub async fn convert_to_otlp(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        info!(
            "Converting internal format to OTLP for batch with {} records",
            batch.records.len()
        );

        // Group records by type
        let mut metrics = Vec::new();
        let mut logs = Vec::new();
        let mut traces = Vec::new();

        for record in &batch.records {
            match &record.record_type {
                TelemetryType::Metric => {
                    if let TelemetryData::Metric(metric_data) = &record.data {
                        let otlp_metric = self.convert_to_otlp_metric(metric_data, record).await?;
                        metrics.push(otlp_metric);
                    }
                }
                TelemetryType::Log => {
                    if let TelemetryData::Log(log_data) = &record.data {
                        let otlp_log = self.convert_to_otlp_log(log_data, record).await?;
                        logs.push(otlp_log);
                    }
                }
                TelemetryType::Trace => {
                    if let TelemetryData::Trace(trace_data) = &record.data {
                        let otlp_span = self.convert_to_otlp_span(trace_data, record).await?;
                        traces.push(otlp_span);
                    }
                }
                TelemetryType::Event => {
                    if let TelemetryData::Event(event_data) = &record.data {
                        // Convert event to log since OTLP doesn't have native events
                        let otlp_log = self.convert_event_to_otlp_log(event_data, record).await?;
                        logs.push(otlp_log);
                    }
                }
            }
        }

        // Create appropriate OTLP request based on content
        if !metrics.is_empty() {
            let mut request = ExportMetricsServiceRequest::default();
            request.resource_metrics = vec![ResourceMetrics {
                schema_url: "".to_string(),
                resource: Some(Resource {
                    attributes: self.convert_to_key_values(&batch.metadata),
                    dropped_attributes_count: 0,
                    entity_refs: vec![],
                }),
                scope_metrics: vec![ScopeMetrics {
                    scope: None,
                    metrics,
                    schema_url: "".to_string(),
                }],
            }];

            let mut buf = Vec::new();
            request.encode(&mut buf).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to encode OTLP metrics: {}",
                    e
                ))
            })?;
            return Ok(buf);
        }

        if !logs.is_empty() {
            let mut request = ExportLogsServiceRequest::default();
            request.resource_logs = vec![ResourceLogs {
                schema_url: "".to_string(),
                resource: Some(Resource {
                    attributes: self.convert_to_key_values(&batch.metadata),
                    dropped_attributes_count: 0,
                    entity_refs: vec![],
                }),
                scope_logs: vec![ScopeLogs {
                    schema_url: "".to_string(),
                    scope: None,
                    log_records: logs,
                }],
            }];

            let mut buf = Vec::new();
            request.encode(&mut buf).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to encode OTLP logs: {}",
                    e
                ))
            })?;
            return Ok(buf);
        }

        if !traces.is_empty() {
            let request = ExportTraceServiceRequest {
                resource_spans: vec![ResourceSpans {
                    schema_url: "".to_string(),
                    resource: Some(Resource {
                        dropped_attributes_count: 0,
                        attributes: self.convert_to_key_values(&batch.metadata),
                        entity_refs: vec![],
                    }),
                    scope_spans: vec![ScopeSpans {
                        schema_url: "".to_string(),
                        scope: None,
                        spans: traces,
                    }],
                }],
            };

            let mut buf = Vec::new();
            request.encode(&mut buf).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to encode OTLP traces: {}",
                    e
                ))
            })?;
            return Ok(buf);
        }

        // If no records, return empty metrics request
        let request = ExportMetricsServiceRequest {
            resource_metrics: vec![],
        };

        let mut buf = Vec::new();
        request.encode(&mut buf).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to encode empty OTLP request: {}",
                e
            ))
        })?;
        Ok(buf)
    }

    /// Convert internal metric to OTLP metric
    async fn convert_to_otlp_metric(
        &self,
        metric_data: &MetricData,
        record: &TelemetryRecord,
    ) -> BridgeResult<Metric> {
        let data_point = NumberDataPoint {
            attributes: self.convert_to_key_values(&metric_data.labels),
            time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
            value: Some(match metric_data.value {
                MetricValue::Gauge(v) => NumberDataPointValue::AsDouble(v),
                MetricValue::Counter(v) => NumberDataPointValue::AsDouble(v),
                MetricValue::Histogram {
                    buckets: _,
                    sum: _,
                    count: _,
                } => NumberDataPointValue::AsDouble(0.0),
                MetricValue::Summary { .. } => NumberDataPointValue::AsDouble(0.0),
            }),
            ..Default::default()
        };

        let metric_data_proto = match metric_data.metric_type {
            MetricType::Gauge => {
                MetricDataProto::Gauge(bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::Gauge {
                    data_points: vec![data_point],
                })
            }
            MetricType::Counter => {
                MetricDataProto::Sum(bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::Sum {
                    data_points: vec![data_point],
                    is_monotonic: true,
                    aggregation_temporality: 1, // Cumulative
                })
            }
            MetricType::Histogram => {
                MetricDataProto::Histogram(bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::Histogram {
                    data_points: vec![
                        bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::HistogramDataPoint {
                            attributes: self.convert_to_key_values(&metric_data.labels),
                            time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0)
                                as u64,
                            count: match &metric_data.value {
                                MetricValue::Gauge(v) => *v as u64,
                                MetricValue::Counter(v) => *v as u64,
                                MetricValue::Histogram {
                                    buckets: _,
                                    sum: _,
                                    count: _,
                                } => 0,
                                MetricValue::Summary { .. } => 0,
                            },
                            ..Default::default()
                        },
                    ],
                    aggregation_temporality: 1, // Cumulative
                })
            }
            MetricType::Summary => {
                MetricDataProto::Summary(bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::Summary {
                    data_points: vec![bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::SummaryDataPoint {
                        attributes: self.convert_to_key_values(&metric_data.labels),
                        time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
                        count: match &metric_data.value {
                            MetricValue::Gauge(v) => *v as u64,
                            MetricValue::Counter(v) => *v as u64,
                            MetricValue::Histogram {
                                buckets: _,
                                sum: _,
                                count: _,
                            } => 0,
                            MetricValue::Summary { .. } => 0,
                        },
                        ..Default::default()
                    }],
                })
            }
        };

        Ok(Metric {
            name: metric_data.name.clone(),
            description: metric_data.description.clone().unwrap_or_default(),
            unit: metric_data.unit.clone().unwrap_or_default(),
            data: Some(metric_data_proto),
            metadata: vec![],
        })
    }

    /// Convert internal log to OTLP log
    async fn convert_to_otlp_log(
        &self,
        log_data: &LogData,
        record: &TelemetryRecord,
    ) -> BridgeResult<LogRecord> {
        let severity_number = match log_data.level {
            LogLevel::Trace => 1,
            LogLevel::Debug => 5,
            LogLevel::Info => 9,
            LogLevel::Warn => 13,
            LogLevel::Error => 17,
            LogLevel::Fatal => 21,
        };

        Ok(LogRecord {
            time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
            severity_number,
            severity_text: log_data.severity_text.clone().unwrap_or_default(),
            body: Some(AnyValue {
                value: Some(AnyValueValue::StringValue(log_data.message.clone())),
            }),
            attributes: self.convert_to_key_values(&log_data.attributes),
            ..Default::default()
        })
    }

    /// Convert internal trace to OTLP span
    async fn convert_to_otlp_span(
        &self,
        trace_data: &bridge_core::types::TraceData,
        record: &TelemetryRecord,
    ) -> BridgeResult<Span> {
        let kind = match trace_data.kind {
            SpanKind::Internal => bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Internal,
            SpanKind::Server => bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Server,
            SpanKind::Client => bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Client,
            SpanKind::Producer => bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Producer,
            SpanKind::Consumer => bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::span::SpanKind::Consumer,
        };

        let status = match trace_data.status.code {
            StatusCode::Ok => bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::status::StatusCode::Ok,
            StatusCode::Error => bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::status::StatusCode::Error,
            StatusCode::Unset => bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::status::StatusCode::Unset,
        };

        Ok(Span {
            trace_id: trace_data.trace_id.clone().into(),
            span_id: trace_data.span_id.clone().into(),
            parent_span_id: trace_data
                .parent_span_id
                .clone()
                .map(|id| id.into())
                .unwrap_or_default(),
            name: trace_data.name.clone(),
            kind: kind as i32,
            start_time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
            end_time_unix_nano: trace_data
                .end_time
                .map(|t| t.timestamp_nanos_opt().unwrap_or(0) as u64)
                .unwrap_or(0),
            attributes: self.convert_to_key_values(&trace_data.attributes),
            status: Some(bridge_core::receivers::otlp::otlp::opentelemetry::proto::trace::v1::Status {
                code: status as i32,
                message: "".to_string(),
            }),
            ..Default::default()
        })
    }

    /// Convert internal event to OTLP log
    async fn convert_event_to_otlp_log(
        &self,
        event_data: &bridge_core::types::EventData,
        record: &TelemetryRecord,
    ) -> BridgeResult<LogRecord> {
        // Convert event to log since OTLP doesn't have native events
        let severity_number = 9; // INFO level for events

        let message = if let Some(data) = &event_data.data {
            format!(
                "Event: {} - {}",
                event_data.name,
                serde_json::to_string(data).unwrap_or_default()
            )
        } else {
            format!("Event: {}", event_data.name)
        };

        Ok(LogRecord {
            time_unix_nano: record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
            severity_number,
            severity_text: "INFO".to_string(),
            body: Some(AnyValue {
                value: Some(AnyValueValue::StringValue(message)),
            }),
            attributes: self.convert_to_key_values(&event_data.attributes),
            ..Default::default()
        })
    }

    /// Convert HashMap to KeyValue list
    fn convert_to_key_values(&self, map: &HashMap<String, String>) -> Vec<KeyValue> {
        map.iter()
            .map(|(k, v)| KeyValue {
                key: k.clone(),
                value: Some(AnyValue {
                    value: Some(AnyValueValue::StringValue(v.clone())),
                }),
            })
            .collect()
    }

    /// Convert to internal format (JSON)
    pub async fn convert_to_internal(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        serde_json::to_vec(batch).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to serialize TelemetryBatch to JSON: {}",
                e
            ))
        })
    }

    /// Convert from internal format (JSON)
    pub async fn convert_from_internal(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        serde_json::from_slice(data).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to deserialize TelemetryBatch from JSON: {}",
                e
            ))
        })
    }
}
