//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! HTTP sink implementation
//!
//! This module provides an HTTP sink for streaming data to HTTP endpoints.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::Utc;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::info;

use super::{SinkConfig, SinkStats, StreamSink};

/// HTTP sink configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSinkConfig {
    /// Sink name
    pub name: String,

    /// Sink version
    pub version: String,

    /// HTTP endpoint URL
    pub endpoint_url: String,

    /// HTTP method
    pub method: String,

    /// Request headers
    pub headers: HashMap<String, String>,

    /// Request timeout in seconds
    pub request_timeout_secs: u64,

    /// Retry count
    pub retry_count: u32,

    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,

    /// Batch size for sending
    pub batch_size: usize,

    /// Authentication token (optional)
    pub auth_token: Option<String>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,

    /// Content type for requests
    pub content_type: String,

    /// Rate limiting
    pub rate_limit_requests_per_second: Option<u32>,
}

impl HttpSinkConfig {
    /// Create new HTTP sink configuration
    pub fn new(endpoint_url: String) -> Self {
        Self {
            name: "http".to_string(),
            version: "1.0.0".to_string(),
            endpoint_url,
            method: "POST".to_string(),
            headers: HashMap::new(),
            request_timeout_secs: 30,
            retry_count: 3,
            retry_delay_ms: 1000,
            batch_size: 1000,
            auth_token: None,
            additional_config: HashMap::new(),
            content_type: "application/json".to_string(),
            rate_limit_requests_per_second: None,
        }
    }
}

#[async_trait]
impl SinkConfig for HttpSinkConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.endpoint_url.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "HTTP endpoint URL cannot be empty".to_string(),
            ));
        }

        // Validate URL format
        if let Err(e) = reqwest::Url::parse(&self.endpoint_url) {
            return Err(bridge_core::BridgeError::configuration(format!(
                "Invalid URL format: {}",
                e
            )));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// HTTP sink implementation
pub struct HttpSink {
    config: HttpSinkConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<SinkStats>>,
    http_client: Option<reqwest::Client>,
}

#[async_trait]
impl StreamSink for HttpSink {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing HTTP sink: {}", self.config.endpoint_url);

        // Create HTTP client with configuration
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.request_timeout_secs))
            .build()
            .map_err(|e| {
                bridge_core::BridgeError::internal(format!("Failed to create HTTP client: {}", e))
            })?;

        self.http_client = Some(client);

        // Validate configuration
        self.config.validate().await?;

        info!("HTTP sink initialized");
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting HTTP sink");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
        }

        info!("HTTP sink started");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping HTTP sink");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }

        info!("HTTP sink stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        // This is a simplified check - in practice we'd need to handle the async nature
        false
    }

    async fn send(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        if !*self.is_running.read().await {
            return Err(bridge_core::BridgeError::internal(
                "HTTP sink is not running".to_string(),
            ));
        }

        self.send_batch(batch).await
    }

    async fn get_stats(&self) -> BridgeResult<SinkStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn version(&self) -> &str {
        &self.config.version
    }
}

impl HttpSink {
    /// Create new HTTP sink
    pub async fn new(config: &dyn SinkConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<HttpSinkConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration(
                    "Invalid HTTP sink configuration".to_string(),
                )
            })?
            .clone();

        config.validate().await?;

        let stats = SinkStats {
            sink: config.name.clone(),
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_send_time: None,
            is_connected: false,
            latency_ms: 0,
        };

        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            http_client: None,
        })
    }

    /// Get HTTP configuration
    pub fn get_config(&self) -> &HttpSinkConfig {
        &self.config
    }

    /// Send batch to HTTP endpoint
    async fn send_batch(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        let client = self
            .http_client
            .as_ref()
            .ok_or_else(|| bridge_core::BridgeError::internal("HTTP client not initialized"))?;

        // Serialize batch to JSON
        let json_data = serde_json::to_vec(&batch).map_err(|e| {
            bridge_core::BridgeError::internal(format!("Failed to serialize batch: {}", e))
        })?;

        // Build request
        let mut request = client.request(
            Method::from_bytes(self.config.method.as_bytes()).map_err(|e| {
                bridge_core::BridgeError::internal(format!("Invalid HTTP method: {}", e))
            })?,
            &self.config.endpoint_url,
        );

        // Add headers
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        // Add content type header
        request = request.header("Content-Type", &self.config.content_type);

        // Add authentication if provided
        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Add body
        request = request.body(json_data.clone());

        // Execute request with retry logic
        let mut last_error = None;
        for attempt in 0..=self.config.retry_count {
            match request.try_clone() {
                Some(req) => {
                    let start_time = Instant::now();
                    match req.send().await {
                        Ok(response) => {
                            let response_time = start_time.elapsed();

                            if response.status().is_success() {
                                // Update stats
                                {
                                    let mut stats = self.stats.write().await;
                                    stats.total_batches += 1;
                                    stats.total_records += batch.size as u64;
                                    stats.total_bytes += json_data.len() as u64;
                                    stats.last_send_time = Some(Utc::now());
                                    stats.latency_ms = response_time.as_millis() as u64;
                                }

                                info!(
                                    "HTTP request successful: {} records in {:?}",
                                    batch.size, response_time
                                );

                                return Ok(());
                            } else {
                                last_error = Some(bridge_core::BridgeError::internal(format!(
                                    "HTTP request failed with status: {}",
                                    response.status()
                                )));
                            }
                        }
                        Err(e) => {
                            last_error = Some(bridge_core::BridgeError::internal(format!(
                                "HTTP request failed: {}",
                                e
                            )));
                        }
                    }
                }
                None => {
                    last_error = Some(bridge_core::BridgeError::internal(
                        "Failed to clone HTTP request for retry".to_string(),
                    ));
                    break;
                }
            }

            // Wait before retry (except on last attempt)
            if attempt < self.config.retry_count {
                tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
            }
        }

        // Update error stats
        {
            let mut stats = self.stats.write().await;
            stats.error_count += 1;
        }

        Err(last_error.unwrap_or_else(|| {
            bridge_core::BridgeError::internal("HTTP request failed after all retries".to_string())
        }))
    }

    /// Check if sink is connected
    pub async fn is_connected(&self) -> bool {
        if let Some(_client) = &self.http_client {
            // Check if we have sent a recent request
            if let Some(last_send) = self.stats.read().await.last_send_time {
                last_send > Utc::now() - chrono::Duration::minutes(5) // Consider connected if sent in last 5 minutes
            } else {
                false
            }
        } else {
            false
        }
    }
}
