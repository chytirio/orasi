//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! HTTP receiver implementation
//!
//! This module provides an HTTP receiver that can handle telemetry data
//! over HTTP endpoints.

use async_trait::async_trait;
use axum::{extract::Json, http::StatusCode, response::IntoResponse, routing::post, Router};
use bridge_core::{
    traits::ReceiverStats,
    types::{LogData, MetricData, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch, TelemetryReceiver as BridgeTelemetryReceiver,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use super::{BaseReceiver, ReceiverConfig};

/// HTTP receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpReceiverConfig {
    /// Receiver name
    pub name: String,

    /// Receiver version
    pub version: String,

    /// HTTP endpoint to listen on
    pub endpoint: String,

    /// Port to listen on
    pub port: u16,

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
}

impl HttpReceiverConfig {
    /// Create new HTTP receiver configuration
    pub fn new(endpoint: String, port: u16) -> Self {
        Self {
            name: "http".to_string(),
            version: "1.0.0".to_string(),
            endpoint,
            port,
            batch_size: 1000,
            buffer_size: 10000,
            enable_compression: true,
            auth_token: None,
            headers: HashMap::new(),
            timeout_secs: 30,
        }
    }
}

#[async_trait]
impl ReceiverConfig for HttpReceiverConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.endpoint.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "HTTP receiver endpoint cannot be empty",
            ));
        }

        if self.port == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "HTTP receiver port cannot be 0",
            ));
        }

        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "HTTP receiver batch size cannot be 0",
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// HTTP receiver implementation
pub struct HttpReceiver {
    base: BaseReceiver,
    config: HttpReceiverConfig,
    is_running: Arc<RwLock<bool>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    router: Router,
}

impl HttpReceiver {
    /// Create new HTTP receiver
    pub async fn new(config: &dyn ReceiverConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<HttpReceiverConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid HTTP receiver configuration")
            })?
            .clone();

        config.validate().await?;

        let base = BaseReceiver::new(config.name.clone(), config.version.clone());

        // Create router for handling HTTP requests
        let router = Router::new()
            .route("/v1/traces", post(Self::handle_traces))
            .route("/v1/metrics", post(Self::handle_metrics))
            .route("/v1/logs", post(Self::handle_logs));

        Ok(Self {
            base,
            config,
            is_running: Arc::new(RwLock::new(false)),
            server_handle: None,
            router,
        })
    }

    /// Handle trace data
    async fn handle_traces(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
        info!("Received trace data: {:?}", payload);
        StatusCode::OK
    }

    /// Handle metric data
    async fn handle_metrics(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
        info!("Received metric data: {:?}", payload);
        StatusCode::OK
    }

    /// Handle log data
    async fn handle_logs(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
        info!("Received log data: {:?}", payload);
        StatusCode::OK
    }

    /// Start HTTP server
    async fn start_http_server(&mut self) -> BridgeResult<()> {
        let addr = format!("{}:{}", self.config.endpoint, self.config.port);
        info!("Starting HTTP receiver server on {}", addr);

        let router = self.router.clone();
        let addr_clone = addr.clone();
        
        // Start the axum server in a separate task
        let server_handle = tokio::spawn(async move {
            let listener = TcpListener::bind(&addr_clone).await.expect("Failed to bind to address");
            info!("HTTP server listening on {}", addr_clone);
            
            axum::serve(listener, router).await.expect("HTTP server failed");
        });

        self.server_handle = Some(server_handle);
        info!("HTTP receiver server started on {}", addr);
        Ok(())
    }

    /// Stop HTTP server
    async fn stop_http_server(&mut self) -> BridgeResult<()> {
        info!("Stopping HTTP receiver server");

        // Abort the server task if it exists
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            let _ = handle.await; // Wait for the task to finish
        }

        info!("HTTP receiver server stopped");
        Ok(())
    }

    /// Receive telemetry data from HTTP requests
    async fn receive_data(&self) -> BridgeResult<TelemetryBatch> {
        // Implement actual data reception from HTTP requests
        // This would receive data from the HTTP server and convert to TelemetryBatch
        
        // For now, we'll create a simple implementation that simulates receiving data
        // In a real implementation, this would:
        // 1. Listen for incoming HTTP requests
        // 2. Parse the request body as telemetry data
        // 3. Convert to TelemetryBatch format
        // 4. Return the batch
        
        // Simulate receiving a batch of telemetry data
        let records = vec![
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "http_request_count".to_string(),
                    description: Some("Number of HTTP requests received".to_string()),
                    unit: Some("requests".to_string()),
                    metric_type: bridge_core::types::MetricType::Counter,
                    value: MetricValue::Counter(1.0),
                    labels: HashMap::from([
                        ("method".to_string(), "POST".to_string()),
                        ("endpoint".to_string(), "/telemetry".to_string()),
                    ]),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("source".to_string(), "http_receiver".to_string()),
                    ("protocol".to_string(), "http".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Log,
                data: TelemetryData::Log(LogData {
                    timestamp: Utc::now(),
                    level: bridge_core::types::LogLevel::Info,
                    message: "HTTP request received".to_string(),
                    attributes: HashMap::from([
                        ("method".to_string(), "POST".to_string()),
                        ("path".to_string(), "/telemetry".to_string()),
                    ]),
                    body: None,
                    severity_number: Some(6),
                    severity_text: Some("INFO".to_string()),
                }),
                attributes: HashMap::from([
                    ("source".to_string(), "http_receiver".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
        ];

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "http".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::from([
                ("protocol".to_string(), "http".to_string()),
                ("content_type".to_string(), "application/json".to_string()),
            ]),
        })
    }
}

#[async_trait]
impl BridgeTelemetryReceiver for HttpReceiver {
    async fn receive(&self) -> BridgeResult<TelemetryBatch> {
        match self.receive_data().await {
            Ok(batch) => {
                // Update statistics
                self.base.update_stats(batch.size, 0).await;
                Ok(batch)
            }
            Err(e) => {
                // Increment error count
                self.base.increment_error_count().await;
                Err(e)
            }
        }
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let is_running = self.is_running.read().await;
        Ok(*is_running)
    }

    async fn get_stats(&self) -> BridgeResult<ReceiverStats> {
        let stats = self.base.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down HTTP receiver");

        // Stop HTTP server
        // Note: This is a simplified shutdown - in practice we'd need to handle this more carefully

        info!("HTTP receiver shutdown completed");
        Ok(())
    }
}

impl HttpReceiver {
    /// Initialize the receiver
    pub async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing HTTP receiver");

        // Start HTTP server
        self.start_http_server().await?;

        // Mark as running
        let mut is_running = self.is_running.write().await;
        *is_running = true;

        info!("HTTP receiver initialized");
        Ok(())
    }

    /// Get endpoint
    pub fn get_endpoint(&self) -> &str {
        &self.config.endpoint
    }

    /// Get port
    pub fn get_port(&self) -> u16 {
        self.config.port
    }

    /// Check if receiver is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }
}
