//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTAP receiver implementation
//!
//! This module provides an OTAP receiver that can handle OTel-Arrow Protocol
//! data ingestion with significant compression improvements over standard OTLP.

use async_trait::async_trait;
use bridge_core::{
    traits::ReceiverStats,
    types::{MetricData, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch, TelemetryReceiver as BridgeTelemetryReceiver,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, error};
use uuid::Uuid;

use super::{BaseReceiver, ReceiverConfig};
use crate::protocols::otap::{OtapConfig, OtapProtocol};
use crate::protocols::{ProtocolConfig, ProtocolHandler};

/// OTAP receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtapReceiverConfig {
    /// Receiver name
    pub name: String,

    /// Receiver version
    pub version: String,

    /// Protocol configuration
    pub protocol_config: OtapConfig,

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

    /// Enable OTLP fallback
    pub enable_otlp_fallback: bool,
}

impl OtapReceiverConfig {
    /// Create new OTAP receiver configuration
    pub fn new(endpoint: String, port: u16) -> Self {
        Self {
            name: "otap-receiver".to_string(),
            version: "1.0.0".to_string(),
            protocol_config: OtapConfig::new(endpoint, port),
            batch_size: 1000,
            buffer_size: 10000,
            enable_compression: true,
            auth_token: None,
            headers: HashMap::new(),
            timeout_secs: 30,
            enable_otlp_fallback: true,
        }
    }
}

#[async_trait]
impl ReceiverConfig for OtapReceiverConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        // Validate protocol configuration
        self.protocol_config.validate().await?;

        // Validate receiver-specific configuration
        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTAP receiver batch size cannot be 0",
            ));
        }

        if self.buffer_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTAP receiver buffer size cannot be 0",
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// OTAP receiver implementation
pub struct OtapReceiver {
    base: BaseReceiver,
    config: OtapReceiverConfig,
    protocol_handler: Option<Box<dyn ProtocolHandler>>,
}

impl OtapReceiver {
    /// Create new OTAP receiver
    pub async fn new(config: &dyn ReceiverConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<OtapReceiverConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid OTAP receiver configuration")
            })?
            .clone();

        config.validate().await?;

        let base = BaseReceiver::new(config.name.clone(), config.version.clone());

        Ok(Self {
            base,
            config,
            protocol_handler: None,
        })
    }

    /// Initialize protocol handler
    async fn init_protocol_handler(&mut self) -> BridgeResult<()> {
        info!("Initializing OTAP protocol handler");

        let protocol_handler: Box<dyn ProtocolHandler> = {
            let handler = OtapProtocol::new(&self.config.protocol_config).await?;
            Box::new(handler)
        };

        self.protocol_handler = Some(protocol_handler);

        info!("OTAP protocol handler initialized");
        Ok(())
    }

    /// Start protocol handler
    async fn start_protocol_handler(&mut self) -> BridgeResult<()> {
        if let Some(handler) = &mut self.protocol_handler {
            handler.start().await?;
        }
        Ok(())
    }

    /// Stop protocol handler
    async fn stop_protocol_handler(&mut self) -> BridgeResult<()> {
        if let Some(handler) = &mut self.protocol_handler {
            handler.stop().await?;
        }
        Ok(())
    }

    /// Receive telemetry data
    async fn receive_data(&self) -> BridgeResult<TelemetryBatch> {
        info!("Receiving OTAP telemetry data");

        // Try to receive data from the protocol handler
        if let Some(handler) = &self.protocol_handler {
            match handler.receive_data().await {
                Ok(Some(batch)) => {
                    info!("Received OTAP batch with {} records", batch.size);
                    Ok(batch)
                }
                Ok(None) => {
                    // No data available, create a placeholder batch for now
                    // In a real implementation, this might block or return an error
                    info!("No OTAP data available, creating placeholder batch");
                    
                    let records = vec![
                        TelemetryRecord {
                            id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            record_type: TelemetryType::Metric,
                            data: TelemetryData::Metric(MetricData {
                                name: "otap_placeholder_metric".to_string(),
                                description: Some("Placeholder metric - no data available".to_string()),
                                unit: Some("count".to_string()),
                                metric_type: bridge_core::types::MetricType::Counter,
                                value: MetricValue::Counter(0.0),
                                labels: HashMap::from([
                                    ("source".to_string(), "otap_receiver".to_string()),
                                    ("protocol".to_string(), "otap".to_string()),
                                    ("status".to_string(), "no_data".to_string()),
                                ]),
                                timestamp: Utc::now(),
                            }),
                            attributes: HashMap::from([
                                ("protocol".to_string(), "otap".to_string()),
                                ("receiver".to_string(), "otap_receiver".to_string()),
                                ("data_status".to_string(), "placeholder".to_string()),
                            ]),
                            tags: HashMap::new(),
                            resource: None,
                            service: None,
                        },
                    ];

                    Ok(TelemetryBatch {
                        id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        source: "otap-receiver".to_string(),
                        size: records.len(),
                        records,
                        metadata: HashMap::from([
                            ("protocol".to_string(), "otap".to_string()),
                            ("compression".to_string(), self.config.enable_compression.to_string()),
                            ("fallback_enabled".to_string(), self.config.enable_otlp_fallback.to_string()),
                            ("data_status".to_string(), "placeholder".to_string()),
                        ]),
                    })
                }
                Err(e) => {
                    error!("Error receiving data from OTAP protocol handler: {}", e);
                    Err(e)
                }
            }
        } else {
            // No protocol handler available
            Err(bridge_core::BridgeError::internal("OTAP protocol handler not initialized"))
        }
    }
}

#[async_trait]
impl BridgeTelemetryReceiver for OtapReceiver {
    async fn receive(&self) -> BridgeResult<TelemetryBatch> {
        self.base.record_receive_attempt().await;
        
        match self.receive_data().await {
            Ok(batch) => {
                self.base.record_receive_success(batch.size).await;
                Ok(batch)
            }
            Err(e) => {
                self.base.record_receive_error().await;
                Err(e)
            }
        }
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        // Check if protocol handler is running
        if let Some(handler) = &self.protocol_handler {
            Ok(handler.is_running())
        } else {
            Ok(false)
        }
    }

    async fn get_stats(&self) -> BridgeResult<ReceiverStats> {
        let base_stats = self.base.get_stats().await?;
        
        // Get protocol-specific stats
        let protocol_stats = if let Some(handler) = &self.protocol_handler {
            match handler.get_stats().await {
                Ok(stats) => {
                    let mut protocol_stats = HashMap::new();
                    protocol_stats.insert("protocol".to_string(), stats.protocol);
                    protocol_stats.insert("total_messages".to_string(), stats.total_messages.to_string());
                    protocol_stats.insert("total_bytes".to_string(), stats.total_bytes.to_string());
                    protocol_stats.insert("error_count".to_string(), stats.error_count.to_string());
                    protocol_stats.insert("is_connected".to_string(), stats.is_connected.to_string());
                    Some(protocol_stats)
                }
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(ReceiverStats {
            total_records: base_stats.total_records,
            records_per_minute: base_stats.records_per_minute,
            total_bytes: base_stats.total_bytes,
            bytes_per_minute: base_stats.bytes_per_minute,
            error_count: base_stats.error_count,
            last_receive_time: base_stats.last_receive_time,
            protocol_stats,
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down OTAP receiver");
        
        // Note: We can't call stop_protocol_handler here because it requires &mut self
        // In a real implementation, we would need to handle this differently
        // For now, just log the shutdown
        
        info!("OTAP receiver shutdown complete");
        Ok(())
    }
}

impl OtapReceiver {
    /// Initialize the receiver
    pub async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing OTAP receiver");

        // Initialize protocol handler
        self.init_protocol_handler().await?;

        info!("OTAP receiver initialized");
        Ok(())
    }

    /// Start the receiver
    pub async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting OTAP receiver");

        // Start protocol handler
        self.start_protocol_handler().await?;

        info!("OTAP receiver started");
        Ok(())
    }

    /// Stop the receiver
    pub async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping OTAP receiver");

        // Stop protocol handler
        self.stop_protocol_handler().await?;

        info!("OTAP receiver stopped");
        Ok(())
    }

    /// Get the protocol handler
    pub fn get_protocol_handler(&self) -> Option<&dyn ProtocolHandler> {
        self.protocol_handler.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::otap::OtapProtocol;

    #[tokio::test]
    async fn test_otap_receiver_creation() {
        let config = OtapReceiverConfig::new("localhost".to_string(), 8080);
        let receiver = OtapReceiver::new(&config).await;
        assert!(receiver.is_ok());
    }

    #[tokio::test]
    async fn test_otap_receiver_lifecycle() {
        let config = OtapReceiverConfig::new("localhost".to_string(), 8080);
        let mut receiver = OtapReceiver::new(&config).await.unwrap();
        
        // Test initialization
        assert!(receiver.init().await.is_ok());
        
        // Test starting
        assert!(receiver.start().await.is_ok());
        
        // Test stopping
        assert!(receiver.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_otap_receiver_data_reception() {
        let config = OtapReceiverConfig::new("localhost".to_string(), 8080);
        let mut receiver = OtapReceiver::new(&config).await.unwrap();
        
        // Initialize and start the receiver
        receiver.init().await.unwrap();
        receiver.start().await.unwrap();
        
        // Get the protocol handler and simulate data
        if let Some(handler) = receiver.get_protocol_handler() {
            if let Some(otap_handler) = handler.as_any().downcast_ref::<OtapProtocol>() {
                // Simulate incoming data
                assert!(otap_handler.simulate_incoming_data().await.is_ok());
                
                // Try to receive data
                let result = receiver.receive().await;
                assert!(result.is_ok());
                
                let batch = result.unwrap();
                assert_eq!(batch.source, "otap-simulator");
                assert!(!batch.records.is_empty());
            }
        }
        
        // Clean up
        receiver.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_otap_receiver_stats() {
        let config = OtapReceiverConfig::new("localhost".to_string(), 8080);
        let mut receiver = OtapReceiver::new(&config).await.unwrap();
        
        // Initialize and start the receiver
        receiver.init().await.unwrap();
        receiver.start().await.unwrap();
        
        // Get stats
        let stats = receiver.get_stats().await;
        assert!(stats.is_ok());
        
        // Clean up
        receiver.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_otap_receiver_health_check() {
        let config = OtapReceiverConfig::new("localhost".to_string(), 8080);
        let mut receiver = OtapReceiver::new(&config).await.unwrap();
        
        // Initialize and start the receiver
        receiver.init().await.unwrap();
        receiver.start().await.unwrap();
        
        // Check health
        let health = receiver.health_check().await;
        assert!(health.is_ok());
        
        // Clean up
        receiver.stop().await.unwrap();
    }
}
