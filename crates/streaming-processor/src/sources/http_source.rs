//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! HTTP source implementation
//! 
//! This module provides an HTTP source for streaming data from HTTP endpoints.

use async_trait::async_trait;
use bridge_core::{
    BridgeResult, TelemetryBatch, 
    types::{TelemetryRecord, TelemetryData, TelemetryType, MetricData, MetricValue, MetricType}
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use reqwest::{Client, Method};
use tokio::time::{Duration, sleep};
use std::time::Instant;

use super::{StreamSource, SourceConfig, SourceStats};

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
                "HTTP endpoint URL cannot be empty".to_string()
            ));
        }
        
        if self.polling_interval_ms == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Polling interval must be greater than 0".to_string()
            ));
        }
        
        if self.request_timeout_secs == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Request timeout must be greater than 0".to_string()
            ));
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
}

impl HttpSource {
    /// Create new HTTP source
    pub async fn new(config: &dyn SourceConfig) -> BridgeResult<Self> {
        let config = config.as_any()
            .downcast_ref::<HttpSourceConfig>()
            .ok_or_else(|| bridge_core::BridgeError::configuration(
                "Invalid HTTP source configuration".to_string()
            ))?
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
            if let Err(e) = self.poll_endpoint().await {
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
    
    /// Poll the HTTP endpoint
    async fn poll_endpoint(&mut self) -> BridgeResult<()> {
        let client = self.http_client.as_ref()
            .ok_or_else(|| bridge_core::BridgeError::internal("HTTP client not initialized"))?;
        
        // Build request
        let mut request = client.request(
            Method::from_bytes(self.config.method.as_bytes())
                .map_err(|e| bridge_core::BridgeError::internal(
                    format!("Invalid HTTP method: {}", e)
                ))?,
            &self.config.endpoint_url
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
        let response = request.send().await
            .map_err(|e| bridge_core::BridgeError::internal(
                format!("HTTP request failed: {}", e)
            ))?;
        
        let response_time = start_time.elapsed();
        
        // Check response status
        if !response.status().is_success() {
            return Err(bridge_core::BridgeError::internal(
                format!("HTTP request failed with status: {}", response.status())
            ));
        }
        
        // Read response body
        let response_data = response.bytes().await
            .map_err(|e| bridge_core::BridgeError::internal(
                format!("Failed to read response body: {}", e)
            ))?;
        
        // Update last request time
        {
            let mut last_time = self.last_request_time.write().await;
            *last_time = Some(Instant::now());
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_bytes += response_data.len() as u64;
            stats.last_record_time = Some(Utc::now());
        }
        
        info!(
            "HTTP request successful: {} bytes in {:?}",
            response_data.len(),
            response_time
        );
        
        Ok(())
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
            .map_err(|e| bridge_core::BridgeError::internal(
                format!("Failed to create HTTP client: {}", e)
            ))?;
        
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
        
        tokio::spawn(async move {
            let mut source = HttpSource {
                config: config_clone,
                is_running: is_running_clone,
                stats: stats_clone,
                http_client,
                last_request_time,
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
        // This is a simplified check - in practice we'd need to handle the async nature
        false
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
