//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! HTTP source implementation
//!
//! This module provides an HTTP source for streaming data from HTTP endpoints.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::Utc;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{SourceConfig, SourceStats, StreamSource};

/// HTTP source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSourceConfig {
    /// Source name
    pub name: String,

    /// Source version
    pub version: String,

    /// HTTP endpoint URL
    pub endpoint_url: String,

    /// HTTP method
    pub method: String,

    /// Request headers
    pub headers: HashMap<String, String>,

    /// Request body
    pub body: Option<String>,

    /// Polling interval in milliseconds
    pub polling_interval_ms: u64,

    /// Request timeout in seconds
    pub request_timeout_secs: u64,

    /// Batch size for processing
    pub batch_size: usize,

    /// Buffer size for incoming data
    pub buffer_size: usize,

    /// Authentication token (optional)
    pub auth_token: Option<String>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,

    /// Retry configuration
    pub max_retries: u32,
    pub retry_delay_ms: u64,

    /// Rate limiting
    pub rate_limit_requests_per_second: Option<u32>,

    /// Response format
    pub response_format: HttpResponseFormat,

    /// Expected response content type
    pub expected_content_type: Option<String>,
}

/// HTTP response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpResponseFormat {
    Json,
    Xml,
    Csv,
    Text,
    Binary,
}

impl HttpSourceConfig {
    /// Create new HTTP source configuration
    pub fn new(endpoint_url: String) -> Self {
        Self {
            name: "http".to_string(),
            version: "1.0.0".to_string(),
            endpoint_url,
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
            polling_interval_ms: 1000,
            request_timeout_secs: 30,
            batch_size: 1000,
            buffer_size: 10000,
            auth_token: None,
            additional_config: HashMap::new(),
            max_retries: 3,
            retry_delay_ms: 1000,
            rate_limit_requests_per_second: None,
            response_format: HttpResponseFormat::Json,
            expected_content_type: None,
        }
    }
}

#[async_trait]
impl SourceConfig for HttpSourceConfig {
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

        if self.polling_interval_ms == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Polling interval must be greater than 0".to_string(),
            ));
        }

        if self.request_timeout_secs == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Request timeout must be greater than 0".to_string(),
            ));
        }

        // Validate URL format
        if let Err(e) = url::Url::parse(&self.endpoint_url) {
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

/// HTTP source implementation
pub struct HttpSource {
    config: HttpSourceConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<SourceStats>>,
    http_client: Option<Client>,
    last_request_time: Arc<RwLock<Option<Instant>>>,
    rate_limiter: Arc<RwLock<Option<Instant>>>,
}

impl HttpSource {
    /// Create new HTTP source
    pub async fn new(config: &dyn SourceConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<HttpSourceConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration(
                    "Invalid HTTP source configuration".to_string(),
                )
            })?
            .clone();

        config.validate().await?;

        let stats = SourceStats {
            source: config.name.clone(),
            total_records: 0,
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_record_time: None,
            is_connected: false,
            lag: None,
        };

        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            http_client: None,
            last_request_time: Arc::new(RwLock::new(None)),
            rate_limiter: Arc::new(RwLock::new(None)),
        })
    }

    /// Get HTTP configuration
    pub fn get_config(&self) -> &HttpSourceConfig {
        &self.config
    }

    /// Start polling the HTTP endpoint
    async fn start_polling(&mut self) -> BridgeResult<()> {
        let polling_interval = Duration::from_millis(self.config.polling_interval_ms);

        while *self.is_running.read().await {
            if let Err(e) = self.poll_endpoint_with_retry().await {
                error!("HTTP polling error: {}", e);

                // Update error stats
                {
                    let mut stats = self.stats.write().await;
                    stats.error_count += 1;
                }
            }

            // Wait for next polling interval
            sleep(polling_interval).await;
        }

        Ok(())
    }

    /// Poll the HTTP endpoint with retry logic
    async fn poll_endpoint_with_retry(&mut self) -> BridgeResult<()> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.poll_endpoint().await {
                Ok(()) => {
                    // Success, reset error count
                    if attempt > 0 {
                        info!("HTTP request succeeded after {} retries", attempt);
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.config.max_retries {
                        warn!(
                            "HTTP request failed (attempt {}/{}), retrying in {}ms: {}",
                            attempt + 1,
                            self.config.max_retries + 1,
                            self.config.retry_delay_ms,
                            last_error.as_ref().unwrap()
                        );

                        // Exponential backoff
                        let delay = Duration::from_millis(
                            self.config.retry_delay_ms * (2_u64.pow(attempt)),
                        );
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            bridge_core::BridgeError::internal("HTTP request failed after all retries".to_string())
        }))
    }

    /// Poll the HTTP endpoint
    async fn poll_endpoint(&mut self) -> BridgeResult<()> {
        let client = self
            .http_client
            .as_ref()
            .ok_or_else(|| bridge_core::BridgeError::internal("HTTP client not initialized"))?;

        // Apply rate limiting if configured
        if let Some(rate_limit) = self.config.rate_limit_requests_per_second {
            self.apply_rate_limit(rate_limit).await?;
        }

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

        // Add authentication if provided
        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Add body if provided
        if let Some(body) = &self.config.body {
            request = request.body(body.clone());
        }

        // Execute request
        let start_time = Instant::now();
        let response = request.send().await.map_err(|e| {
            bridge_core::BridgeError::network(format!("HTTP request failed: {}", e))
        })?;

        let response_time = start_time.elapsed();

        // Check response status
        if !response.status().is_success() {
            return Err(bridge_core::BridgeError::network(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        // Validate content type if expected
        if let Some(expected_content_type) = &self.config.expected_content_type {
            if let Some(content_type) = response.headers().get("content-type") {
                if let Ok(content_type_str) = content_type.to_str() {
                    if !content_type_str.contains(expected_content_type) {
                        warn!(
                            "Unexpected content type: expected {}, got {}",
                            expected_content_type, content_type_str
                        );
                    }
                }
            }
        }

        // Read response body
        let response_data = response.bytes().await.map_err(|e| {
            bridge_core::BridgeError::network(format!("Failed to read response body: {}", e))
        })?;

        // Process response data
        let telemetry_batch = self.process_response_data(&response_data).await?;

        // Update last request time
        {
            let mut last_time = self.last_request_time.write().await;
            *last_time = Some(Instant::now());
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_records += telemetry_batch.size as u64;
            stats.total_bytes += response_data.len() as u64;
            stats.last_record_time = Some(Utc::now());
        }

        info!(
            "HTTP request successful: {} records, {} bytes in {:?}",
            telemetry_batch.size,
            response_data.len(),
            response_time
        );

        Ok(())
    }

    /// Apply rate limiting
    async fn apply_rate_limit(&self, requests_per_second: u32) -> BridgeResult<()> {
        let interval = Duration::from_secs(1) / requests_per_second;

        {
            let rate_limiter = self.rate_limiter.read().await;
            if let Some(last_request) = *rate_limiter {
                let elapsed = last_request.elapsed();
                if elapsed < interval {
                    let sleep_duration = interval - elapsed;
                    drop(rate_limiter); // Release lock before sleeping
                    sleep(sleep_duration).await;
                }
            }
        }

        {
            let mut rate_limiter = self.rate_limiter.write().await;
            *rate_limiter = Some(Instant::now());
        }

        Ok(())
    }

    /// Process response data and convert to telemetry batch
    async fn process_response_data(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        match self.config.response_format {
            HttpResponseFormat::Json => self.process_json_response(data).await,
            HttpResponseFormat::Xml => self.process_xml_response(data).await,
            HttpResponseFormat::Csv => self.process_csv_response(data).await,
            HttpResponseFormat::Text => self.process_text_response(data).await,
            HttpResponseFormat::Binary => self.process_binary_response(data).await,
        }
    }

    /// Process JSON response
    async fn process_json_response(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        // Try to parse as TelemetryBatch first
        if let Ok(batch) = serde_json::from_slice::<TelemetryBatch>(data) {
            return Ok(batch);
        }

        // Try to parse as array of records
        if let Ok(records) = serde_json::from_slice::<Vec<serde_json::Value>>(data) {
            let telemetry_records = records
                .into_iter()
                .filter_map(|record| {
                    serde_json::from_value::<bridge_core::types::TelemetryRecord>(record).ok()
                })
                .collect::<Vec<_>>();

            return Ok(TelemetryBatch {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source: format!("http_{}", self.config.name),
                size: telemetry_records.len(),
                records: telemetry_records,
                metadata: HashMap::new(),
            });
        }

        // Try to parse as single record
        if let Ok(record) = serde_json::from_slice::<bridge_core::types::TelemetryRecord>(data) {
            return Ok(TelemetryBatch {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source: format!("http_{}", self.config.name),
                size: 1,
                records: vec![record],
                metadata: HashMap::new(),
            });
        }

        // Fallback: create a basic metric from the response
        let record = bridge_core::types::TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: bridge_core::types::TelemetryType::Metric,
            data: bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
                name: "http_response_size".to_string(),
                description: Some("HTTP response size in bytes".to_string()),
                unit: Some("bytes".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: bridge_core::types::MetricValue::Gauge(data.len() as f64),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: format!("http_{}", self.config.name),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        })
    }

    /// Process XML response
    async fn process_xml_response(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        // For now, treat XML as text and create a basic metric
        // In production, you would use a proper XML parser
        warn!("XML processing not fully implemented, treating as text");
        self.process_text_response(data).await
    }

    /// Process CSV response
    async fn process_csv_response(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        // For now, treat CSV as text and create a basic metric
        // In production, you would use a proper CSV parser
        warn!("CSV processing not fully implemented, treating as text");
        self.process_text_response(data).await
    }

    /// Process text response
    async fn process_text_response(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        let text = String::from_utf8_lossy(data);

        let record = bridge_core::types::TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: bridge_core::types::TelemetryType::Log,
            data: bridge_core::types::TelemetryData::Log(bridge_core::types::LogData {
                message: text.to_string(),
                level: bridge_core::types::LogLevel::Info,
                timestamp: Utc::now(),
                attributes: HashMap::new(),
                body: None,
                severity_number: None,
                severity_text: None,
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: format!("http_{}", self.config.name),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        })
    }

    /// Process binary response
    async fn process_binary_response(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        let record = bridge_core::types::TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: bridge_core::types::TelemetryType::Metric,
            data: bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
                name: "http_binary_response_size".to_string(),
                description: Some("HTTP binary response size in bytes".to_string()),
                unit: Some("bytes".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: bridge_core::types::MetricValue::Gauge(data.len() as f64),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: format!("http_{}", self.config.name),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        })
    }
}

#[async_trait]
impl StreamSource for HttpSource {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing HTTP source: {}", self.config.endpoint_url);

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

        info!("HTTP source initialized");
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting HTTP source");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
        }

        // Start polling HTTP endpoint in background task
        let is_running_clone = self.is_running.clone();
        let stats_clone = self.stats.clone();
        let config_clone = self.config.clone();
        let http_client = self.http_client.clone();
        let last_request_time = self.last_request_time.clone();
        let rate_limiter = self.rate_limiter.clone();

        tokio::spawn(async move {
            let mut source = HttpSource {
                config: config_clone,
                is_running: is_running_clone,
                stats: stats_clone,
                http_client,
                last_request_time,
                rate_limiter,
            };

            if let Err(e) = source.start_polling().await {
                error!("HTTP polling failed: {}", e);
            }
        });

        info!("HTTP source started");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping HTTP source");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }

        info!("HTTP source stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        // Check if the source is running
        if let Some(_client) = &self.http_client {
            // In a real implementation, you would check the client's health
            true
        } else {
            false
        }
    }

    async fn get_stats(&self) -> BridgeResult<SourceStats> {
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
