//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTLP HTTP receiver implementation
//!
//! This module provides an OTLP HTTP receiver that can handle OTLP data
//! over HTTP/REST endpoints.

use async_trait::async_trait;
use bridge_core::{
    traits::ReceiverStats,
    types::{MetricData, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch, TelemetryReceiver as BridgeTelemetryReceiver,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode as HttpStatusCode},
    response::Json,
    routing::{post, get},
    Router,
};
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;

use super::{BaseReceiver, ReceiverConfig};

/// OTLP HTTP receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpHttpReceiverConfig {
    /// Receiver name
    pub name: String,

    /// Receiver version
    pub version: String,

    /// HTTP server host
    pub host: String,

    /// HTTP server port
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

    /// Enable CORS
    pub enable_cors: bool,

    /// CORS origins
    pub cors_origins: Vec<String>,

    /// Request size limit in bytes
    pub request_size_limit: usize,
}

impl OtlpHttpReceiverConfig {
    /// Create new OTLP HTTP receiver configuration
    pub fn new(host: String, port: u16) -> Self {
        Self {
            name: "otlp-http".to_string(),
            version: "1.0.0".to_string(),
            host,
            port,
            batch_size: 1000,
            buffer_size: 10000,
            enable_compression: true,
            auth_token: None,
            headers: HashMap::new(),
            timeout_secs: 30,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            request_size_limit: 10 * 1024 * 1024, // 10MB
        }
    }

    /// Get server address
    pub fn server_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid server address")
    }
}

#[async_trait]
impl ReceiverConfig for OtlpHttpReceiverConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP HTTP receiver batch size cannot be 0",
            ));
        }

        if self.buffer_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP HTTP receiver buffer size cannot be 0",
            ));
        }

        if self.request_size_limit == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP HTTP receiver request size limit cannot be 0",
            ));
        }

        // Validate server address
        self.server_addr();

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// OTLP HTTP receiver implementation
pub struct OtlpHttpReceiver {
    base: BaseReceiver,
    config: OtlpHttpReceiverConfig,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    data_buffer: Arc<Mutex<Vec<TelemetryBatch>>>,
}

impl OtlpHttpReceiver {
    /// Create new OTLP HTTP receiver
    pub async fn new(config: &dyn ReceiverConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<OtlpHttpReceiverConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid OTLP HTTP receiver configuration")
            })?
            .clone();

        config.validate().await?;

        let base = BaseReceiver::new(config.name.clone(), config.version.clone());
        let data_buffer = Arc::new(Mutex::new(Vec::new()));

        Ok(Self {
            base,
            config,
            server_handle: None,
            data_buffer,
        })
    }

    /// Start HTTP server
    async fn start_server(&mut self) -> BridgeResult<()> {
        info!("Starting OTLP HTTP server on {}", self.config.server_addr());

        let data_buffer = self.data_buffer.clone();
        let config = self.config.clone();

        // Create router
        let mut router = Router::new()
            .route("/v1/traces", post(Self::handle_traces))
            .route("/v1/metrics", post(Self::handle_metrics))
            .route("/v1/logs", post(Self::handle_logs))
            .route("/health", get(Self::handle_health))
            .with_state(Arc::new(config.clone()));

        // Add CORS if enabled
        if self.config.enable_cors {
            let cors = CorsLayer::permissive();
            router = router.layer(cors);
        }

        // Start server
        let server_handle = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(config.server_addr())
                .await
                .expect("Failed to bind to address");

            axum::serve(listener, router)
                .await
                .expect("Server error");
        });

        self.server_handle = Some(server_handle);

        info!("OTLP HTTP server started successfully");
        Ok(())
    }

    /// Stop HTTP server
    async fn stop_server(&mut self) -> BridgeResult<()> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            info!("OTLP HTTP server stopped");
        }
        Ok(())
    }

    /// Handle OTLP traces endpoint
    async fn handle_traces(
        State(config): State<Arc<OtlpHttpReceiverConfig>>,
        headers: HeaderMap,
        body: axum::body::Bytes,
    ) -> Json<serde_json::Value> {
        info!("Received OTLP traces request");

        // Validate authentication
        if let Some(auth_token) = &config.auth_token {
            if let Some(authorization) = headers.get("authorization") {
                if authorization.as_bytes() != format!("Bearer {}", auth_token).as_bytes() {
                    return Json(serde_json::json!({
                        "error": "Unauthorized",
                        "code": 401
                    }));
                }
            } else {
                return Json(serde_json::json!({
                    "error": "Missing authorization header",
                    "code": 401
                }));
            }
        }

        // Validate content type
        if let Some(content_type) = headers.get("content-type") {
            if !content_type.to_str().unwrap_or("").contains("application/x-protobuf") {
                return Json(serde_json::json!({
                    "error": "Invalid content type, expected application/x-protobuf",
                    "code": 400
                }));
            }
        }

        // Process traces data
        match Self::process_traces_data(&body).await {
            Ok(batch) => {
                // Store in buffer (in a real implementation, this would be sent to a queue)
                info!("Processed traces batch with {} records", batch.records.len());
                Json(serde_json::json!({
                    "status": "success",
                    "records_processed": batch.records.len()
                }))
            }
            Err(e) => {
                error!("Failed to process traces: {}", e);
                Json(serde_json::json!({
                    "error": format!("Failed to process traces: {}", e),
                    "code": 500
                }))
            }
        }
    }

    /// Handle OTLP metrics endpoint
    async fn handle_metrics(
        State(config): State<Arc<OtlpHttpReceiverConfig>>,
        headers: HeaderMap,
        body: axum::body::Bytes,
    ) -> Json<serde_json::Value> {
        info!("Received OTLP metrics request");

        // Validate authentication
        if let Some(auth_token) = &config.auth_token {
            if let Some(authorization) = headers.get("authorization") {
                if authorization.as_bytes() != format!("Bearer {}", auth_token).as_bytes() {
                    return Json(serde_json::json!({
                        "error": "Unauthorized",
                        "code": 401
                    }));
                }
            } else {
                return Json(serde_json::json!({
                    "error": "Missing authorization header",
                    "code": 401
                }));
            }
        }

        // Validate content type
        if let Some(content_type) = headers.get("content-type") {
            if !content_type.to_str().unwrap_or("").contains("application/x-protobuf") {
                return Json(serde_json::json!({
                    "error": "Invalid content type, expected application/x-protobuf",
                    "code": 400
                }));
            }
        }

        // Process metrics data
        match Self::process_metrics_data(&body).await {
            Ok(batch) => {
                info!("Processed metrics batch with {} records", batch.records.len());
                Json(serde_json::json!({
                    "status": "success",
                    "records_processed": batch.records.len()
                }))
            }
            Err(e) => {
                error!("Failed to process metrics: {}", e);
                Json(serde_json::json!({
                    "error": format!("Failed to process metrics: {}", e),
                    "code": 500
                }))
            }
        }
    }

    /// Handle OTLP logs endpoint
    async fn handle_logs(
        State(config): State<Arc<OtlpHttpReceiverConfig>>,
        headers: HeaderMap,
        body: axum::body::Bytes,
    ) -> Json<serde_json::Value> {
        info!("Received OTLP logs request");

        // Validate authentication
        if let Some(auth_token) = &config.auth_token {
            if let Some(authorization) = headers.get("authorization") {
                if authorization.as_bytes() != format!("Bearer {}", auth_token).as_bytes() {
                    return Json(serde_json::json!({
                        "error": "Unauthorized",
                        "code": 401
                    }));
                }
            } else {
                return Json(serde_json::json!({
                    "error": "Missing authorization header",
                    "code": 401
                }));
            }
        }

        // Validate content type
        if let Some(content_type) = headers.get("content-type") {
            if !content_type.to_str().unwrap_or("").contains("application/x-protobuf") {
                return Json(serde_json::json!({
                    "error": "Invalid content type, expected application/x-protobuf",
                    "code": 400
                }));
            }
        }

        // Process logs data
        match Self::process_logs_data(&body).await {
            Ok(batch) => {
                info!("Processed logs batch with {} records", batch.records.len());
                Json(serde_json::json!({
                    "status": "success",
                    "records_processed": batch.records.len()
                }))
            }
            Err(e) => {
                error!("Failed to process logs: {}", e);
                Json(serde_json::json!({
                    "error": format!("Failed to process logs: {}", e),
                    "code": 500
                }))
            }
        }
    }

    /// Handle health check endpoint
    async fn handle_health() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "status": "healthy",
            "timestamp": Utc::now().to_rfc3339()
        }))
    }

    /// Process traces data
    async fn process_traces_data(data: &[u8]) -> BridgeResult<TelemetryBatch> {
        use crate::conversion::{OtlpConverter, TelemetryConverter};
        
        let converter = OtlpConverter::new();
        converter.convert_from_otlp(data).await
    }

    /// Process metrics data
    async fn process_metrics_data(data: &[u8]) -> BridgeResult<TelemetryBatch> {
        use crate::conversion::{OtlpConverter, TelemetryConverter};
        
        let converter = OtlpConverter::new();
        converter.convert_from_otlp(data).await
    }

    /// Process logs data
    async fn process_logs_data(data: &[u8]) -> BridgeResult<TelemetryBatch> {
        use crate::conversion::{OtlpConverter, TelemetryConverter};
        
        let converter = OtlpConverter::new();
        converter.convert_from_otlp(data).await
    }
}

#[async_trait]
impl BridgeTelemetryReceiver for OtlpHttpReceiver {
    async fn receive(&self) -> BridgeResult<TelemetryBatch> {
        // Get data from buffer
        let mut buffer = self.data_buffer.lock().await;
        
        if let Some(batch) = buffer.pop() {
            self.base.update_stats(batch.size, 0).await;
            Ok(batch)
        } else {
            // Return empty batch if no data available
            Ok(TelemetryBatch {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source: "otlp-http".to_string(),
                size: 0,
                records: vec![],
                metadata: HashMap::new(),
            })
        }
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        // Check if server is running
        Ok(self.server_handle.is_some() && !self.server_handle.as_ref().unwrap().is_finished())
    }

    async fn get_stats(&self) -> BridgeResult<ReceiverStats> {
        let stats = self.base.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down OTLP HTTP receiver");
        
        // Stop server
        if let Some(handle) = &self.server_handle {
            handle.abort();
        }
        
        info!("OTLP HTTP receiver shutdown completed");
        Ok(())
    }
}

impl OtlpHttpReceiver {
    /// Initialize the receiver
    pub async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing OTLP HTTP receiver");

        // Start HTTP server
        self.start_server().await?;

        info!("OTLP HTTP receiver initialized");
        Ok(())
    }

    /// Check if receiver is running
    pub async fn is_running(&self) -> bool {
        self.server_handle.is_some() && !self.server_handle.as_ref().unwrap().is_finished()
    }

    /// Get server address
    pub fn get_server_addr(&self) -> SocketAddr {
        self.config.server_addr()
    }
}
