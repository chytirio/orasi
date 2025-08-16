//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry data generators for OpenTelemetry-compatible data
//!
//! This module provides generators for creating realistic OpenTelemetry
//! telemetry data including metrics, traces, logs, and events.

use super::utils::{CorrelationGenerator, DistributionGenerator, IdGenerator, TimeGenerator};
use crate::config::GeneratorConfig;
use crate::models::*;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main telemetry generator that orchestrates all telemetry data generation
pub struct TelemetryGenerator {
    config: Arc<GeneratorConfig>,
    metrics_generator: MetricsGenerator,
    traces_generator: TracesGenerator,
    logs_generator: LogsGenerator,
    events_generator: EventsGenerator,
}

impl TelemetryGenerator {
    /// Create a new telemetry generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let metrics_generator = MetricsGenerator::new(Arc::clone(&config))?;
        let traces_generator = TracesGenerator::new(Arc::clone(&config))?;
        let logs_generator = LogsGenerator::new(Arc::clone(&config))?;
        let events_generator = EventsGenerator::new(Arc::clone(&config))?;

        Ok(Self {
            config,
            metrics_generator,
            traces_generator,
            logs_generator,
            events_generator,
        })
    }

    /// Generate correlated telemetry data based on workflows
    pub async fn generate_correlated(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<super::TelemetryData> {
        let mut telemetry = super::TelemetryData::new();

        // Generate traces for workflows
        telemetry.traces = self
            .traces_generator
            .generate_workflow_traces(workflows)
            .await?;

        // Generate metrics for workflows
        telemetry.metrics = self
            .metrics_generator
            .generate_workflow_metrics(workflows)
            .await?;

        // Generate logs for workflows
        telemetry.logs = self
            .logs_generator
            .generate_workflow_logs(workflows)
            .await?;

        // Generate events for workflows
        telemetry.events = self
            .events_generator
            .generate_workflow_events(workflows)
            .await?;

        Ok(telemetry)
    }

    /// Generate development scenario telemetry
    pub async fn generate_development_telemetry(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<super::TelemetryData> {
        let mut telemetry = super::TelemetryData::new();

        telemetry.traces = self
            .traces_generator
            .generate_development_traces(workflows)
            .await?;
        telemetry.metrics = self
            .metrics_generator
            .generate_development_metrics(workflows)
            .await?;
        telemetry.logs = self
            .logs_generator
            .generate_development_logs(workflows)
            .await?;
        telemetry.events = self
            .events_generator
            .generate_development_events(workflows)
            .await?;

        Ok(telemetry)
    }

    /// Generate failure scenario telemetry
    pub async fn generate_failure_telemetry(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<super::TelemetryData> {
        let mut telemetry = super::TelemetryData::new();

        telemetry.traces = self
            .traces_generator
            .generate_failure_traces(workflows)
            .await?;
        telemetry.metrics = self
            .metrics_generator
            .generate_failure_metrics(workflows)
            .await?;
        telemetry.logs = self.logs_generator.generate_failure_logs(workflows).await?;
        telemetry.events = self
            .events_generator
            .generate_failure_events(workflows)
            .await?;

        Ok(telemetry)
    }

    /// Generate scale scenario telemetry
    pub async fn generate_scale_telemetry(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<super::TelemetryData> {
        let mut telemetry = super::TelemetryData::new();

        telemetry.traces = self
            .traces_generator
            .generate_scale_traces(workflows)
            .await?;
        telemetry.metrics = self
            .metrics_generator
            .generate_scale_metrics(workflows)
            .await?;
        telemetry.logs = self.logs_generator.generate_scale_logs(workflows).await?;
        telemetry.events = self
            .events_generator
            .generate_scale_events(workflows)
            .await?;

        Ok(telemetry)
    }

    /// Generate evolution scenario telemetry
    pub async fn generate_evolution_telemetry(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<super::TelemetryData> {
        let mut telemetry = super::TelemetryData::new();

        telemetry.traces = self
            .traces_generator
            .generate_evolution_traces(workflows)
            .await?;
        telemetry.metrics = self
            .metrics_generator
            .generate_evolution_metrics(workflows)
            .await?;
        telemetry.logs = self
            .logs_generator
            .generate_evolution_logs(workflows)
            .await?;
        telemetry.events = self
            .events_generator
            .generate_evolution_events(workflows)
            .await?;

        Ok(telemetry)
    }
}

/// Generator for OpenTelemetry metrics
pub struct MetricsGenerator {
    config: Arc<GeneratorConfig>,
    id_generator: IdGenerator,
    time_generator: TimeGenerator,
    distribution_generator: DistributionGenerator,
}

impl MetricsGenerator {
    /// Create a new metrics generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let id_generator = IdGenerator::new(Arc::clone(&config))?;
        let time_generator = TimeGenerator::new(Arc::clone(&config))?;
        let distribution_generator = DistributionGenerator::new(Arc::clone(&config))?;

        Ok(Self {
            config,
            id_generator,
            time_generator,
            distribution_generator,
        })
    }

    /// Generate workflow-related metrics
    pub async fn generate_workflow_metrics(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        let mut metrics = Vec::new();

        for workflow in workflows {
            // Generate workflow duration metrics
            if let Some(duration) = workflow.duration {
                metrics.push(
                    self.generate_counter_metric(
                        "workflow.duration.total",
                        duration.num_seconds() as f64,
                        &workflow.metadata,
                    )
                    .await?,
                );
            }

            // Generate workflow status metrics
            metrics.push(
                self.generate_gauge_metric(
                    "workflow.status",
                    match workflow.status {
                        WorkflowStatus::Completed => 1.0,
                        WorkflowStatus::Failed => 0.0,
                        _ => 0.5,
                    },
                    &workflow.metadata,
                )
                .await?,
            );
        }

        Ok(metrics)
    }

    /// Generate development scenario metrics
    pub async fn generate_development_metrics(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        self.generate_workflow_metrics(workflows).await
    }

    /// Generate failure scenario metrics
    pub async fn generate_failure_metrics(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        let mut metrics = self.generate_workflow_metrics(workflows).await?;

        // Add failure-specific metrics
        for workflow in workflows {
            if workflow.status == WorkflowStatus::Failed {
                metrics.push(
                    self.generate_counter_metric(
                        "workflow.failures.total",
                        1.0,
                        &workflow.metadata,
                    )
                    .await?,
                );
            }
        }

        Ok(metrics)
    }

    /// Generate scale scenario metrics
    pub async fn generate_scale_metrics(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        let mut metrics = self.generate_workflow_metrics(workflows).await?;

        // Add scale-specific metrics
        metrics.push(
            self.generate_gauge_metric(
                "workflow.concurrent.total",
                workflows.len() as f64,
                &HashMap::new(),
            )
            .await?,
        );

        Ok(metrics)
    }

    /// Generate evolution scenario metrics
    pub async fn generate_evolution_metrics(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        self.generate_workflow_metrics(workflows).await
    }

    /// Generate a counter metric
    async fn generate_counter_metric(
        &self,
        name: &str,
        value: f64,
        attributes: &HashMap<String, String>,
    ) -> Result<Value> {
        let mut metric = HashMap::new();
        metric.insert("name".to_string(), Value::String(name.to_string()));
        metric.insert("type".to_string(), Value::String("counter".to_string()));
        metric.insert(
            "value".to_string(),
            Value::Number(
                serde_json::Number::from_f64(value).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        metric.insert(
            "timestamp".to_string(),
            Value::String(self.time_generator.generate_timestamp().await.to_rfc3339()),
        );
        metric.insert("attributes".to_string(), serde_json::to_value(attributes)?);

        Ok(Value::Object(serde_json::Map::from_iter(metric)))
    }

    /// Generate a gauge metric
    async fn generate_gauge_metric(
        &self,
        name: &str,
        value: f64,
        attributes: &HashMap<String, String>,
    ) -> Result<Value> {
        let mut metric = HashMap::new();
        metric.insert("name".to_string(), Value::String(name.to_string()));
        metric.insert("type".to_string(), Value::String("gauge".to_string()));
        metric.insert(
            "value".to_string(),
            Value::Number(
                serde_json::Number::from_f64(value).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        metric.insert(
            "timestamp".to_string(),
            Value::String(self.time_generator.generate_timestamp().await.to_rfc3339()),
        );
        metric.insert("attributes".to_string(), serde_json::to_value(attributes)?);

        Ok(Value::Object(serde_json::Map::from_iter(metric)))
    }
}

/// Generator for OpenTelemetry traces
pub struct TracesGenerator {
    config: Arc<GeneratorConfig>,
    id_generator: IdGenerator,
    time_generator: TimeGenerator,
    correlation_generator: CorrelationGenerator,
}

impl TracesGenerator {
    /// Create a new traces generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let id_generator = IdGenerator::new(Arc::clone(&config))?;
        let time_generator = TimeGenerator::new(Arc::clone(&config))?;
        let correlation_generator = CorrelationGenerator::new(Arc::clone(&config))?;

        Ok(Self {
            config,
            id_generator,
            time_generator,
            correlation_generator,
        })
    }

    /// Generate workflow-related traces
    pub async fn generate_workflow_traces(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        let mut traces = Vec::new();

        for workflow in workflows {
            let (trace_id, root_span_id) =
                self.correlation_generator.generate_trace_span_pair().await;

            // Generate root span for workflow
            traces.push(
                self.generate_span(
                    &trace_id,
                    &root_span_id,
                    None,
                    &format!(
                        "workflow.{}",
                        format!("{:?}", workflow.workflow_type).to_lowercase()
                    ),
                    &workflow.metadata,
                )
                .await?,
            );

            // Generate child spans for phases
            for phase in &workflow.phases {
                let span_id = self.id_generator.generate_span_id().await;
                traces.push(
                    self.generate_span(
                        &trace_id,
                        &span_id,
                        Some(&root_span_id),
                        &format!("phase.{}", format!("{:?}", phase.phase_type).to_lowercase()),
                        &phase
                            .activities
                            .iter()
                            .map(|a| (a.name.clone(), format!("{:?}", a.activity_type)))
                            .collect(),
                    )
                    .await?,
                );
            }
        }

        Ok(traces)
    }

    /// Generate development scenario traces
    pub async fn generate_development_traces(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        self.generate_workflow_traces(workflows).await
    }

    /// Generate failure scenario traces
    pub async fn generate_failure_traces(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        let mut traces = self.generate_workflow_traces(workflows).await?;

        // Add error spans for failed workflows
        for workflow in workflows {
            if workflow.status == WorkflowStatus::Failed {
                let (trace_id, span_id) =
                    self.correlation_generator.generate_trace_span_pair().await;
                traces.push(
                    self.generate_error_span(
                        &trace_id,
                        &span_id,
                        &format!(
                            "workflow.error.{}",
                            format!("{:?}", workflow.workflow_type).to_lowercase()
                        ),
                        "Workflow execution failed",
                    )
                    .await?,
                );
            }
        }

        Ok(traces)
    }

    /// Generate scale scenario traces
    pub async fn generate_scale_traces(&self, workflows: &[AgenticWorkflow]) -> Result<Vec<Value>> {
        self.generate_workflow_traces(workflows).await
    }

    /// Generate evolution scenario traces
    pub async fn generate_evolution_traces(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        self.generate_workflow_traces(workflows).await
    }

    /// Generate a span
    async fn generate_span(
        &self,
        trace_id: &str,
        span_id: &str,
        parent_span_id: Option<&str>,
        name: &str,
        attributes: &HashMap<String, String>,
    ) -> Result<Value> {
        let mut span = HashMap::new();
        span.insert("trace_id".to_string(), Value::String(trace_id.to_string()));
        span.insert("span_id".to_string(), Value::String(span_id.to_string()));
        if let Some(parent_id) = parent_span_id {
            span.insert(
                "parent_span_id".to_string(),
                Value::String(parent_id.to_string()),
            );
        }
        span.insert("name".to_string(), Value::String(name.to_string()));
        span.insert(
            "start_time".to_string(),
            Value::String(self.time_generator.generate_timestamp().await.to_rfc3339()),
        );
        span.insert(
            "end_time".to_string(),
            Value::String(self.time_generator.generate_timestamp().await.to_rfc3339()),
        );
        span.insert("attributes".to_string(), serde_json::to_value(attributes)?);

        Ok(Value::Object(serde_json::Map::from_iter(span)))
    }

    /// Generate an error span
    async fn generate_error_span(
        &self,
        trace_id: &str,
        span_id: &str,
        name: &str,
        error_message: &str,
    ) -> Result<Value> {
        let mut span = HashMap::new();
        span.insert("trace_id".to_string(), Value::String(trace_id.to_string()));
        span.insert("span_id".to_string(), Value::String(span_id.to_string()));
        span.insert("name".to_string(), Value::String(name.to_string()));
        span.insert(
            "start_time".to_string(),
            Value::String(self.time_generator.generate_timestamp().await.to_rfc3339()),
        );
        span.insert(
            "end_time".to_string(),
            Value::String(self.time_generator.generate_timestamp().await.to_rfc3339()),
        );
        span.insert("status".to_string(), Value::String("error".to_string()));
        span.insert(
            "error_message".to_string(),
            Value::String(error_message.to_string()),
        );

        Ok(Value::Object(serde_json::Map::from_iter(span)))
    }
}

/// Generator for OpenTelemetry logs
pub struct LogsGenerator {
    config: Arc<GeneratorConfig>,
    id_generator: IdGenerator,
    time_generator: TimeGenerator,
}

impl LogsGenerator {
    /// Create a new logs generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let id_generator = IdGenerator::new(Arc::clone(&config))?;
        let time_generator = TimeGenerator::new(Arc::clone(&config))?;

        Ok(Self {
            config,
            id_generator,
            time_generator,
        })
    }

    /// Generate workflow-related logs
    pub async fn generate_workflow_logs(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        let mut logs = Vec::new();

        for workflow in workflows {
            logs.push(
                self.generate_log(
                    "INFO",
                    &format!("Workflow {} started", workflow.id),
                    &workflow.metadata,
                )
                .await?,
            );

            if workflow.status == WorkflowStatus::Completed {
                logs.push(
                    self.generate_log(
                        "INFO",
                        &format!("Workflow {} completed successfully", workflow.id),
                        &workflow.metadata,
                    )
                    .await?,
                );
            } else if workflow.status == WorkflowStatus::Failed {
                logs.push(
                    self.generate_log(
                        "ERROR",
                        &format!("Workflow {} failed", workflow.id),
                        &workflow.metadata,
                    )
                    .await?,
                );
            }
        }

        Ok(logs)
    }

    /// Generate development scenario logs
    pub async fn generate_development_logs(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        self.generate_workflow_logs(workflows).await
    }

    /// Generate failure scenario logs
    pub async fn generate_failure_logs(&self, workflows: &[AgenticWorkflow]) -> Result<Vec<Value>> {
        let mut logs = self.generate_workflow_logs(workflows).await?;

        // Add failure-specific logs
        for workflow in workflows {
            if workflow.status == WorkflowStatus::Failed {
                logs.push(
                    self.generate_log(
                        "ERROR",
                        &format!(
                            "Critical error in workflow {}: {}",
                            workflow.id, "Simulated failure"
                        ),
                        &workflow.metadata,
                    )
                    .await?,
                );
            }
        }

        Ok(logs)
    }

    /// Generate scale scenario logs
    pub async fn generate_scale_logs(&self, workflows: &[AgenticWorkflow]) -> Result<Vec<Value>> {
        self.generate_workflow_logs(workflows).await
    }

    /// Generate evolution scenario logs
    pub async fn generate_evolution_logs(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        self.generate_workflow_logs(workflows).await
    }

    /// Generate a log entry
    async fn generate_log(
        &self,
        level: &str,
        message: &str,
        attributes: &HashMap<String, String>,
    ) -> Result<Value> {
        let mut log = HashMap::new();
        log.insert("level".to_string(), Value::String(level.to_string()));
        log.insert("message".to_string(), Value::String(message.to_string()));
        log.insert(
            "timestamp".to_string(),
            Value::String(self.time_generator.generate_timestamp().await.to_rfc3339()),
        );
        log.insert("attributes".to_string(), serde_json::to_value(attributes)?);

        Ok(Value::Object(serde_json::Map::from_iter(log)))
    }
}

/// Generator for OpenTelemetry events
pub struct EventsGenerator {
    config: Arc<GeneratorConfig>,
    id_generator: IdGenerator,
    time_generator: TimeGenerator,
}

impl EventsGenerator {
    /// Create a new events generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let id_generator = IdGenerator::new(Arc::clone(&config))?;
        let time_generator = TimeGenerator::new(Arc::clone(&config))?;

        Ok(Self {
            config,
            id_generator,
            time_generator,
        })
    }

    /// Generate workflow-related events
    pub async fn generate_workflow_events(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        let mut events = Vec::new();

        for workflow in workflows {
            events.push(
                self.generate_event(
                    "workflow.started",
                    &format!("Workflow {} started", workflow.id),
                    &workflow.metadata,
                )
                .await?,
            );

            match workflow.status {
                WorkflowStatus::Completed => {
                    events.push(
                        self.generate_event(
                            "workflow.completed",
                            &format!("Workflow {} completed", workflow.id),
                            &workflow.metadata,
                        )
                        .await?,
                    );
                }
                WorkflowStatus::Failed => {
                    events.push(
                        self.generate_event(
                            "workflow.failed",
                            &format!("Workflow {} failed", workflow.id),
                            &workflow.metadata,
                        )
                        .await?,
                    );
                }
                _ => {}
            }
        }

        Ok(events)
    }

    /// Generate development scenario events
    pub async fn generate_development_events(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        self.generate_workflow_events(workflows).await
    }

    /// Generate failure scenario events
    pub async fn generate_failure_events(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        let mut events = self.generate_workflow_events(workflows).await?;

        // Add failure-specific events
        for workflow in workflows {
            if workflow.status == WorkflowStatus::Failed {
                events.push(
                    self.generate_event(
                        "workflow.error",
                        &format!("Error in workflow {}: {}", workflow.id, "Simulated failure"),
                        &workflow.metadata,
                    )
                    .await?,
                );
            }
        }

        Ok(events)
    }

    /// Generate scale scenario events
    pub async fn generate_scale_events(&self, workflows: &[AgenticWorkflow]) -> Result<Vec<Value>> {
        self.generate_workflow_events(workflows).await
    }

    /// Generate evolution scenario events
    pub async fn generate_evolution_events(
        &self,
        workflows: &[AgenticWorkflow],
    ) -> Result<Vec<Value>> {
        self.generate_workflow_events(workflows).await
    }

    /// Generate an event
    async fn generate_event(
        &self,
        name: &str,
        message: &str,
        attributes: &HashMap<String, String>,
    ) -> Result<Value> {
        let mut event = HashMap::new();
        event.insert("name".to_string(), Value::String(name.to_string()));
        event.insert("message".to_string(), Value::String(message.to_string()));
        event.insert(
            "timestamp".to_string(),
            Value::String(self.time_generator.generate_timestamp().await.to_rfc3339()),
        );
        event.insert("attributes".to_string(), serde_json::to_value(attributes)?);

        Ok(Value::Object(serde_json::Map::from_iter(event)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GeneratorConfig;

    #[tokio::test]
    async fn test_telemetry_generator_creation() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = TelemetryGenerator::new(config);
        assert!(generator.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_generator() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = MetricsGenerator::new(config).unwrap();

        let workflows = vec![];
        let metrics = generator
            .generate_workflow_metrics(&workflows)
            .await
            .unwrap();
        assert_eq!(metrics.len(), 0);
    }

    #[tokio::test]
    async fn test_traces_generator() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = TracesGenerator::new(config).unwrap();

        let workflows = vec![];
        let traces = generator
            .generate_workflow_traces(&workflows)
            .await
            .unwrap();
        assert_eq!(traces.len(), 0);
    }
}
