//! Common utilities for data processing

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::state::AgentState;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Get current timestamp in milliseconds
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Base processor trait for common functionality
pub trait BaseProcessor {
    fn config(&self) -> &AgentConfig;
    fn state(&self) -> &Arc<RwLock<AgentState>>;
}

/// Common data I/O operations
pub struct DataIO;

impl DataIO {
    /// Simulate reading input data
    pub async fn simulate_read_input_data(input_location: &str) -> Result<Vec<Value>, AgentError> {
        // Simulate different data sources and formats
        let record_count = match input_location {
            loc if loc.contains("large") => 10000,
            loc if loc.contains("medium") => 5000,
            loc if loc.contains("small") => 1000,
            _ => 2500,
        };
        
        let mut records = Vec::new();
        for i in 0..record_count {
            records.push(serde_json::json!({
                "id": i,
                "name": format!("record_{}", i),
                "value": (i % 100) as f64,
                "category": match i % 5 {
                    0 => "A",
                    1 => "B", 
                    2 => "C",
                    3 => "D",
                    _ => "E"
                },
                "timestamp": current_timestamp() + i as u64,
                "metadata": {
                    "source": input_location,
                    "version": "1.0"
                }
            }));
        }
        
        Ok(records)
    }
    
    /// Simulate writing output data
    pub async fn simulate_write_output_data(output_destination: &str, records: &[Value]) -> Result<(), AgentError> {
        // Simulate writing to different output destinations
        match output_destination {
            dest if dest.contains("s3://") => {
                info!("Simulating S3 write to: {}", dest);
                // Simulate S3 upload delay
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            dest if dest.contains("file://") => {
                info!("Simulating file write to: {}", dest);
                // Simulate file I/O delay
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
            dest if dest.contains("database://") => {
                info!("Simulating database write to: {}", dest);
                // Simulate database insert delay
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
            _ => {
                info!("Simulating generic write to: {}", output_destination);
                tokio::time::sleep(tokio::time::Duration::from_millis(75)).await;
            }
        }
        
        Ok(())
    }
}

/// Configuration parsing utilities
pub struct ConfigParser;

impl ConfigParser {
    /// Parse JSON configuration from options
    pub fn parse_json_config(options: &HashMap<String, String>, key: &str) -> Result<Value, AgentError> {
        let default_json = "{}".to_string();
        let config_json = options.get(key).unwrap_or(&default_json);
        serde_json::from_str(config_json)
            .map_err(|e| AgentError::InvalidInput(format!("Invalid config for key '{}': {}", key, e)))
    }
    
    /// Parse array configuration from options
    pub fn parse_array_config(options: &HashMap<String, String>, key: &str) -> Result<Vec<Value>, AgentError> {
        let default_json = "[]".to_string();
        let config_json = options.get(key).unwrap_or(&default_json);
        serde_json::from_str(config_json)
            .map_err(|e| AgentError::InvalidInput(format!("Invalid array config for key '{}': {}", key, e)))
    }
}
