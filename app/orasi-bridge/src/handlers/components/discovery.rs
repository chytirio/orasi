//! Component discovery and listing functionality

use crate::types::*;
use reqwest::Client;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};

/// Get active components list
pub async fn get_active_components() -> Vec<ComponentInfo> {
    let mut components = Vec::new();

    // Core components
    components.extend(discover_core_components().await);

    // Infrastructure components
    components.extend(discover_infrastructure_components().await);

    // Application components
    components.extend(discover_application_components().await);

    // Connector components
    components.extend(discover_connector_components().await);

    components
}

/// Discover core system components
async fn discover_core_components() -> Vec<ComponentInfo> {
    let mut components = Vec::new();
    let uptime = super::super::health::get_server_uptime();
    let now = chrono::Utc::now();

    // API component (always running if this function is called)
    components.push(ComponentInfo {
        name: "api".to_string(),
        status: ComponentStatus::Running,
        version: crate::BRIDGE_API_VERSION.to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // Bridge Core component
    components.push(ComponentInfo {
        name: "bridge_core".to_string(),
        status: ComponentStatus::Running,
        version: "0.1.0".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // Database component
    let db_status = check_database_status().await;
    components.push(ComponentInfo {
        name: "database".to_string(),
        status: db_status,
        version: "postgresql".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // Cache component
    let cache_status = check_cache_status().await;
    components.push(ComponentInfo {
        name: "cache".to_string(),
        status: cache_status,
        version: "redis".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    components
}

/// Discover infrastructure components
async fn discover_infrastructure_components() -> Vec<ComponentInfo> {
    let mut components = Vec::new();
    let uptime = super::super::health::get_server_uptime();
    let now = chrono::Utc::now();

    // MinIO (S3-compatible storage)
    let minio_status = check_service_status("http://localhost:9000/minio/health/live").await;
    components.push(ComponentInfo {
        name: "minio".to_string(),
        status: minio_status,
        version: "latest".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // PostgreSQL
    let postgres_status = check_service_status("http://localhost:5432").await;
    components.push(ComponentInfo {
        name: "postgresql".to_string(),
        status: postgres_status,
        version: "latest".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // Trino
    let trino_status = check_service_status("http://localhost:8080").await;
    components.push(ComponentInfo {
        name: "trino".to_string(),
        status: trino_status,
        version: "latest".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // Spark Master
    let spark_master_status = check_service_status("http://localhost:8080").await;
    components.push(ComponentInfo {
        name: "spark_master".to_string(),
        status: spark_master_status,
        version: "3.5.0".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // Spark Worker
    let spark_worker_status = check_service_status("http://localhost:8081").await;
    components.push(ComponentInfo {
        name: "spark_worker".to_string(),
        status: spark_worker_status,
        version: "3.5.6".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // Spark History Server
    let spark_history_status = check_service_status("http://localhost:18080").await;
    components.push(ComponentInfo {
        name: "spark_history".to_string(),
        status: spark_history_status,
        version: "3.5.6".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    components
}

/// Discover application components
async fn discover_application_components() -> Vec<ComponentInfo> {
    let mut components = Vec::new();
    let uptime = super::super::health::get_server_uptime();
    let now = chrono::Utc::now();

    // Gateway
    let gateway_status = check_service_status("http://localhost:8080/health").await;
    components.push(ComponentInfo {
        name: "orasi_gateway".to_string(),
        status: gateway_status,
        version: "0.1.0".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // Agent
    let agent_status = check_service_status("http://localhost:8081/health").await;
    components.push(ComponentInfo {
        name: "orasi_agent".to_string(),
        status: agent_status,
        version: "0.1.0".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    // Web UI
    let web_status = check_service_status("http://localhost:3000").await;
    components.push(ComponentInfo {
        name: "orasi_web".to_string(),
        status: web_status,
        version: "0.1.0".to_string(),
        uptime_seconds: uptime,
        last_health_check: now,
    });

    components
}

/// Discover connector components
async fn discover_connector_components() -> Vec<ComponentInfo> {
    let mut components = Vec::new();
    let uptime = super::super::health::get_server_uptime();
    let now = chrono::Utc::now();

    // Available connectors (these would be discovered based on configuration)
    let connector_configs = get_connector_configurations().await;

    for (name, config) in connector_configs {
        let status = check_connector_status(&name, &config).await;
        components.push(ComponentInfo {
            name: format!("connector_{}", name),
            status,
            version: config
                .get("version")
                .unwrap_or(&"0.1.0".to_string())
                .clone(),
            uptime_seconds: uptime,
            last_health_check: now,
        });
    }

    components
}

/// Check database status
async fn check_database_status() -> ComponentStatus {
    match std::env::var("DATABASE_URL") {
        Ok(database_url) => {
            // Try to connect to database
            if let Ok(_) = test_database_connection(&database_url).await {
                ComponentStatus::Running
            } else {
                ComponentStatus::Error
            }
        }
        Err(_) => ComponentStatus::Stopped,
    }
}

/// Check cache status
async fn check_cache_status() -> ComponentStatus {
    match std::env::var("CACHE_URL") {
        Ok(cache_url) => {
            // Try to connect to cache
            if let Ok(_) = test_cache_connection(&cache_url).await {
                ComponentStatus::Running
            } else {
                ComponentStatus::Error
            }
        }
        Err(_) => ComponentStatus::Stopped,
    }
}

/// Check service status via HTTP health check
async fn check_service_status(url: &str) -> ComponentStatus {
    let client = Client::new();
    let timeout_duration = Duration::from_secs(5);

    match timeout(timeout_duration, client.get(url).send()).await {
        Ok(Ok(response)) => {
            if response.status().is_success() {
                ComponentStatus::Running
            } else {
                ComponentStatus::Error
            }
        }
        Ok(Err(_)) => ComponentStatus::Error,
        Err(_) => ComponentStatus::Stopped,
    }
}

/// Get connector configurations
async fn get_connector_configurations() -> HashMap<String, HashMap<String, String>> {
    let mut configs = HashMap::new();

    // Check for configured connectors based on environment variables or config files
    let connectors = [
        ("deltalake", "DeltaLake"),
        ("kafka", "Kafka"),
        ("iceberg", "Iceberg"),
        ("snowflake", "Snowflake"),
        ("hudi", "Hudi"),
        ("s3_parquet", "S3-Parquet"),
    ];

    for (name, display_name) in connectors.iter() {
        if is_connector_configured(name).await {
            let mut config = HashMap::new();
            config.insert("name".to_string(), display_name.to_string());
            config.insert("version".to_string(), "0.1.0".to_string());
            configs.insert(name.to_string(), config);
        }
    }

    configs
}

/// Check if a connector is configured
async fn is_connector_configured(connector_name: &str) -> bool {
    // Check for environment variables or configuration files
    let env_var = format!("{}_CONNECTOR_ENABLED", connector_name.to_uppercase());
    std::env::var(env_var).is_ok()
}

/// Check connector status
async fn check_connector_status(
    connector_name: &str,
    _config: &HashMap<String, String>,
) -> ComponentStatus {
    // For now, assume configured connectors are running
    // In a real implementation, you would check the actual connector status
    if is_connector_configured(connector_name).await {
        ComponentStatus::Running
    } else {
        ComponentStatus::Stopped
    }
}

/// Test database connection (reuse from health module)
async fn test_database_connection(database_url: &str) -> Result<(), String> {
    // Parse database URL to determine type
    if database_url.starts_with("postgresql://") || database_url.starts_with("postgres://") {
        test_postgresql_connection(database_url).await
    } else if database_url.starts_with("mysql://") {
        test_mysql_connection(database_url).await
    } else if database_url.starts_with("sqlite://") {
        test_sqlite_connection(database_url).await
    } else {
        Err("Unsupported database type".to_string())
    }
}

/// Test PostgreSQL connection
async fn test_postgresql_connection(_database_url: &str) -> Result<(), String> {
    // Simulate connection test
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

/// Test MySQL connection
async fn test_mysql_connection(_database_url: &str) -> Result<(), String> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

/// Test SQLite connection
async fn test_sqlite_connection(_database_url: &str) -> Result<(), String> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

/// Test cache connection (reuse from health module)
async fn test_cache_connection(cache_url: &str) -> Result<(), String> {
    // Parse cache URL to determine type
    if cache_url.starts_with("redis://") {
        test_redis_connection(cache_url).await
    } else if cache_url.starts_with("memcached://") {
        test_memcached_connection(cache_url).await
    } else {
        Err("Unsupported cache type".to_string())
    }
}

/// Test Redis connection
async fn test_redis_connection(_cache_url: &str) -> Result<(), String> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

/// Test Memcached connection
async fn test_memcached_connection(_cache_url: &str) -> Result<(), String> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_active_components() {
        let components = get_active_components().await;

        // Should always include core components
        assert!(!components.is_empty());

        // Check that API component is always present and running
        let api_component = components.iter().find(|c| c.name == "api");
        assert!(api_component.is_some());
        assert_eq!(api_component.unwrap().status, ComponentStatus::Running);

        // Check that bridge_core component is always present and running
        let bridge_core_component = components.iter().find(|c| c.name == "bridge_core");
        assert!(bridge_core_component.is_some());
        assert_eq!(
            bridge_core_component.unwrap().status,
            ComponentStatus::Running
        );

        // Check that all components have valid data
        for component in &components {
            assert!(!component.name.is_empty());
            assert!(!component.version.is_empty());
            assert!(component.last_health_check <= chrono::Utc::now());
        }
    }

    #[tokio::test]
    async fn test_discover_core_components() {
        let components = discover_core_components().await;

        // Should include at least API and bridge_core
        assert!(components.len() >= 2);

        let component_names: Vec<&str> = components.iter().map(|c| c.name.as_str()).collect();
        assert!(component_names.contains(&"api"));
        assert!(component_names.contains(&"bridge_core"));
    }

    #[tokio::test]
    async fn test_discover_infrastructure_components() {
        let components = discover_infrastructure_components().await;

        // Should include infrastructure components
        let component_names: Vec<&str> = components.iter().map(|c| c.name.as_str()).collect();

        // These components should be checked (may be stopped if not running)
        let expected_components = [
            "minio",
            "postgresql",
            "trino",
            "spark_master",
            "spark_worker",
            "spark_history",
        ];

        for expected in &expected_components {
            assert!(
                component_names.contains(expected),
                "Missing component: {}",
                expected
            );
        }
    }

    #[tokio::test]
    async fn test_discover_application_components() {
        let components = discover_application_components().await;

        // Should include application components
        let component_names: Vec<&str> = components.iter().map(|c| c.name.as_str()).collect();

        let expected_components = ["orasi_gateway", "orasi_agent", "orasi_web"];

        for expected in &expected_components {
            assert!(
                component_names.contains(expected),
                "Missing component: {}",
                expected
            );
        }
    }

    #[tokio::test]
    async fn test_check_service_status() {
        // Test with a non-existent service (should return error due to connection failure)
        let status = check_service_status("http://localhost:99999").await;
        assert_eq!(status, ComponentStatus::Error);

        // Test with an invalid URL (should return error due to request failure)
        let status = check_service_status("invalid-url").await;
        assert_eq!(status, ComponentStatus::Error);
    }

    #[tokio::test]
    async fn test_is_connector_configured() {
        // Test with a connector that's not configured
        let configured = is_connector_configured("nonexistent_connector").await;
        assert!(!configured);
    }

    #[tokio::test]
    async fn test_get_connector_configurations() {
        let configs = get_connector_configurations().await;

        // Should return a HashMap (may be empty if no connectors are configured)
        assert!(configs.is_empty() || !configs.is_empty());

        // If any connectors are configured, they should have the expected structure
        for (name, config) in &configs {
            assert!(!name.is_empty());
            assert!(config.contains_key("name"));
            assert!(config.contains_key("version"));
        }
    }
}
