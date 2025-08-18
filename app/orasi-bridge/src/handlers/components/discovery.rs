//! Component discovery and listing functionality

use crate::types::*;

/// Get active components list
pub async fn get_active_components() -> Vec<ComponentInfo> {
    // TODO: Implement actual component discovery
    // This would typically involve:
    // 1. Scanning for running components
    // 2. Getting component status and health
    // 3. Collecting component metadata

    vec![
        ComponentInfo {
            name: "api".to_string(),
            status: ComponentStatus::Running,
            version: crate::BRIDGE_API_VERSION.to_string(),
            uptime_seconds: super::super::health::get_server_uptime(),
            last_health_check: chrono::Utc::now(),
        },
        ComponentInfo {
            name: "bridge_core".to_string(),
            status: ComponentStatus::Running,
            version: "0.1.0".to_string(),
            uptime_seconds: super::super::health::get_server_uptime(),
            last_health_check: chrono::Utc::now(),
        },
    ]
}
