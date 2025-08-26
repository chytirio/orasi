//! State synchronization implementation for cluster coordination

use serde::{Deserialize, Serialize};

/// State synchronization message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSyncMessage {
    /// State data
    pub state_data: serde_json::Value,

    /// Sync timestamp
    pub timestamp: u64,

    /// Sync type
    pub sync_type: StateSyncType,
}

/// State sync type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateSyncType {
    Full,
    Incremental,
    Request,
}

/// State sync result
#[derive(Debug, Clone)]
pub struct StateSyncResult {
    /// Whether the sync was successful
    pub success: bool,
    /// Sync timestamp
    pub timestamp: u64,
    /// Error message if sync failed
    pub error: Option<String>,
    /// Sync data
    pub data: Option<serde_json::Value>,
}

/// State sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSyncRequest {
    /// Request ID
    pub request_id: String,
    /// Sync type
    pub sync_type: StateSyncType,
    /// Request timestamp
    pub timestamp: u64,
    /// Requesting member ID
    pub requesting_member: String,
}

impl StateSyncMessage {
    /// Create a new state sync message
    pub fn new(state_data: serde_json::Value, sync_type: StateSyncType) -> Self {
        Self {
            state_data,
            timestamp: super::types::current_timestamp(),
            sync_type,
        }
    }

    /// Create a full sync message
    pub fn full_sync(state_data: serde_json::Value) -> Self {
        Self::new(state_data, StateSyncType::Full)
    }

    /// Create an incremental sync message
    pub fn incremental_sync(state_data: serde_json::Value) -> Self {
        Self::new(state_data, StateSyncType::Incremental)
    }

    /// Create a sync request message
    pub fn request_sync(request_id: String, requesting_member: String) -> Self {
        Self {
            state_data: serde_json::json!({}),
            timestamp: super::types::current_timestamp(),
            sync_type: StateSyncType::Request,
        }
    }
}

impl StateSyncResult {
    /// Create a successful sync result
    pub fn success(data: Option<serde_json::Value>) -> Self {
        Self {
            success: true,
            timestamp: super::types::current_timestamp(),
            error: None,
            data,
        }
    }

    /// Create a failed sync result
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            timestamp: super::types::current_timestamp(),
            error: Some(error),
            data: None,
        }
    }
}

impl StateSyncRequest {
    /// Create a new state sync request
    pub fn new(request_id: String, sync_type: StateSyncType, requesting_member: String) -> Self {
        Self {
            request_id,
            sync_type,
            timestamp: super::types::current_timestamp(),
            requesting_member,
        }
    }
}
