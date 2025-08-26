//! Component health monitoring and restart functionality

use crate::types::*;
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Get component status
pub async fn get_component_status(component_name: &str) -> ComponentStatusResponse {
    // Get component health status
    let health = check_component_health(component_name).await;
    
    // Get actual component metrics
    let metrics = get_component_metrics(component_name).await;
    
    // Get recent logs if component is unhealthy
    let logs = if health.status == HealthStatus::Unhealthy {
        get_component_logs(component_name, 10).await
    } else {
        None
    };

    ComponentStatusResponse {
        component_name: component_name.to_string(),
        status: if health.status == HealthStatus::Healthy {
            ComponentStatus::Running
        } else {
            ComponentStatus::Error
        },
        health,
        metrics,
        logs,
    }
}

/// Restart component
pub async fn restart_component(
    component_name: &str,
    request: &ComponentRestartRequest,
) -> Result<ComponentRestartResponse, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = Instant::now();
    
    // Get actual previous status
    let previous_status = get_component_status(component_name).await.status;
    
    // Perform graceful shutdown if requested
    if let Some(options) = &request.options {
        if options.graceful {
            if let Err(e) = graceful_shutdown_component(component_name).await {
                tracing::warn!("Graceful shutdown failed for {}: {}", component_name, e);
            }
        }
    }
    
    // Wait for component to stop
    let timeout = request.options
        .as_ref()
        .and_then(|opt| opt.timeout_seconds)
        .unwrap_or(30);
    
    let stopped = wait_for_component_stop(component_name, timeout).await;
    if !stopped {
        return Err(format!("Component {} failed to stop within {} seconds", component_name, timeout).into());
    }
    
    // Start the component
    if let Err(e) = start_component(component_name).await {
        return Err(format!("Failed to start component {}: {}", component_name, e).into());
    }
    
    // Wait for component to be healthy
    let healthy = wait_for_component_health(component_name, timeout).await;
    if !healthy {
        return Err(format!("Component {} failed to become healthy within {} seconds", component_name, timeout).into());
    }
    
    // Get new status
    let new_status = get_component_status(component_name).await.status;
    
    Ok(ComponentRestartResponse {
        component_name: component_name.to_string(),
        status: "success".to_string(),
        restart_time_ms: start_time.elapsed().as_millis() as u64,
        previous_status,
        new_status,
    })
}

/// Check component health
pub async fn check_component_health(component_name: &str) -> ComponentHealth {
    let now = chrono::Utc::now();

    match component_name {
        "api" => ComponentHealth {
            status: HealthStatus::Healthy,
            message: Some("API server is healthy".to_string()),
            last_check: now,
        },
        "bridge_core" => {
            // Check bridge core health
            match bridge_core::get_bridge_status().await {
                Ok(status) => ComponentHealth {
                    status: if status.status == "healthy" {
                        HealthStatus::Healthy
                    } else {
                        HealthStatus::Unhealthy
                    },
                    message: Some(format!("Bridge core status: {}", status.status)),
                    last_check: now,
                },
                Err(e) => ComponentHealth {
                    status: HealthStatus::Unhealthy,
                    message: Some(format!("Bridge core error: {}", e)),
                    last_check: now,
                },
            }
        }
        "database" => super::super::health::check_database_health().await,
        "cache" => super::super::health::check_cache_health().await,
        _ => {
            // Check if it's a system process
            if let Ok(running) = check_process_running(component_name).await {
                if running {
                    ComponentHealth {
                        status: HealthStatus::Healthy,
                        message: Some(format!("Process {} is running", component_name)),
                        last_check: now,
                    }
                } else {
                    ComponentHealth {
                        status: HealthStatus::Unhealthy,
                        message: Some(format!("Process {} is not running", component_name)),
                        last_check: now,
                    }
                }
            } else {
                ComponentHealth {
                    status: HealthStatus::Unhealthy,
                    message: Some(format!("Unknown component: {}", component_name)),
                    last_check: now,
                }
            }
        }
    }
}

/// Get component metrics
async fn get_component_metrics(component_name: &str) -> Option<HashMap<String, f64>> {
    let mut metrics = HashMap::new();
    
    match component_name {
        "api" => {
            // API-specific metrics
            metrics.insert("requests_per_second".to_string(), get_api_rps().await);
            metrics.insert("response_time_ms".to_string(), get_api_response_time().await);
            metrics.insert("error_rate".to_string(), get_api_error_rate().await);
            metrics.insert("active_connections".to_string(), get_api_active_connections().await);
        }
        "bridge_core" => {
            // Bridge core metrics
            if let Ok(status) = bridge_core::get_bridge_status().await {
                metrics.insert("uptime_seconds".to_string(), status.uptime_seconds as f64);
                // Note: memory_usage_mb and cpu_usage_percent are not available in BridgeStatus
                // These would need to be collected separately if needed
            }
        }
        "database" => {
            // Database metrics
            metrics.insert("connection_pool_size".to_string(), get_db_connection_pool_size().await);
            metrics.insert("active_connections".to_string(), get_db_active_connections().await);
            metrics.insert("query_time_ms".to_string(), get_db_query_time().await);
        }
        "cache" => {
            // Cache metrics
            metrics.insert("cache_hit_rate".to_string(), get_cache_hit_rate().await);
            metrics.insert("cache_size_mb".to_string(), get_cache_size().await);
            metrics.insert("eviction_rate".to_string(), get_cache_eviction_rate().await);
        }
        _ => {
            // System process metrics
            if let Ok(pid) = get_process_pid(component_name).await {
                if let Ok(process_metrics) = get_process_metrics(pid).await {
                    metrics.insert("cpu_usage_percent".to_string(), process_metrics.cpu_usage);
                    metrics.insert("memory_usage_mb".to_string(), process_metrics.memory_usage);
                    metrics.insert("uptime_seconds".to_string(), process_metrics.uptime);
                }
            }
        }
    }
    
    if metrics.is_empty() {
        None
    } else {
        Some(metrics)
    }
}

/// Get component logs
async fn get_component_logs(component_name: &str, max_lines: usize) -> Option<Vec<String>> {
    let log_path = get_component_log_path(component_name);
    
    if let Ok(content) = tokio::fs::read_to_string(&log_path).await {
        let lines: Vec<String> = content
            .lines()
            .rev() // Get most recent lines first
            .take(max_lines)
            .map(|line| line.to_string())
            .collect();
        
        if lines.is_empty() {
            None
        } else {
            Some(lines.into_iter().rev().collect()) // Restore chronological order
        }
    } else {
        None
    }
}

/// Check if a process is running
async fn check_process_running(process_name: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg(process_name)
        .output()?;
    
    Ok(output.status.success())
}

/// Get process PID
async fn get_process_pid(process_name: &str) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg(process_name)
        .output()?;
    
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let pid_str = output_str.trim();
        Ok(pid_str.parse()?)
    } else {
        Err("Process not found".into())
    }
}

/// Process metrics structure
struct ProcessMetrics {
    cpu_usage: f64,
    memory_usage: f64,
    uptime: f64,
}

/// Get process metrics
async fn get_process_metrics(pid: u32) -> Result<ProcessMetrics, Box<dyn std::error::Error + Send + Sync>> {
    // Use ps command to get process metrics
    let output = Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "pcpu,pmem,etime"])
        .output()?;
    
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = output_str.lines().collect();
        
        if lines.len() >= 2 {
            let parts: Vec<&str> = lines[1].split_whitespace().collect();
            if parts.len() >= 3 {
                let cpu_usage = parts[0].parse::<f64>().unwrap_or(0.0);
                let memory_usage = parts[1].parse::<f64>().unwrap_or(0.0);
                let uptime = parse_uptime(parts[2]);
                
                Ok(ProcessMetrics {
                    cpu_usage,
                    memory_usage,
                    uptime,
                })
            } else {
                Err("Invalid ps output format".into())
            }
        } else {
            Err("No process found".into())
        }
    } else {
        Err("Failed to get process metrics".into())
    }
}

/// Parse uptime string (e.g., "1-02:30:45" or "02:30:45")
fn parse_uptime(uptime_str: &str) -> f64 {
    let parts: Vec<&str> = uptime_str.split(':').collect();
    match parts.len() {
        3 => {
            // Format: "HH:MM:SS"
            let hours: f64 = parts[0].parse().unwrap_or(0.0);
            let minutes: f64 = parts[1].parse().unwrap_or(0.0);
            let seconds: f64 = parts[2].parse().unwrap_or(0.0);
            hours * 3600.0 + minutes * 60.0 + seconds
        }
        4 => {
            // Format: "D-HH:MM:SS"
            let days: f64 = parts[0].parse().unwrap_or(0.0);
            let hours: f64 = parts[1].parse().unwrap_or(0.0);
            let minutes: f64 = parts[2].parse().unwrap_or(0.0);
            let seconds: f64 = parts[3].parse().unwrap_or(0.0);
            days * 86400.0 + hours * 3600.0 + minutes * 60.0 + seconds
        }
        _ => 0.0,
    }
}

/// Get component log path
fn get_component_log_path(component_name: &str) -> std::path::PathBuf {
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "/var/log/orasi".to_string());
    std::path::PathBuf::from(log_dir).join(format!("{}.log", component_name))
}

/// Graceful shutdown component
async fn graceful_shutdown_component(component_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(pid) = get_process_pid(component_name).await {
        // Send SIGTERM for graceful shutdown
        Command::new("kill")
            .arg("-TERM")
            .arg(&pid.to_string())
            .output()?;
    }
    Ok(())
}

/// Wait for component to stop
async fn wait_for_component_stop(component_name: &str, timeout_seconds: u64) -> bool {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_seconds);
    
    while start.elapsed() < timeout {
        if let Ok(running) = check_process_running(component_name).await {
            if !running {
                return true;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
    
    false
}

/// Start component
async fn start_component(component_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // This would typically involve starting the component process
    // For now, we'll simulate it
    tracing::info!("Starting component: {}", component_name);
    
    // In a real implementation, you would:
    // 1. Look up the component configuration
    // 2. Start the process with appropriate arguments
    // 3. Handle any startup errors
    
    Ok(())
}

/// Wait for component to be healthy
async fn wait_for_component_health(component_name: &str, timeout_seconds: u64) -> bool {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_seconds);
    
    while start.elapsed() < timeout {
        let health = check_component_health(component_name).await;
        if health.status == HealthStatus::Healthy {
            return true;
        }
        sleep(Duration::from_millis(500)).await;
    }
    
    false
}

// Mock metric collection functions - these would be implemented based on your actual metrics system

async fn get_api_rps() -> f64 {
    // Mock implementation - replace with actual metrics collection
    150.0
}

async fn get_api_response_time() -> f64 {
    // Mock implementation - replace with actual metrics collection
    25.5
}

async fn get_api_error_rate() -> f64 {
    // Mock implementation - replace with actual metrics collection
    0.02
}

async fn get_api_active_connections() -> f64 {
    // Mock implementation - replace with actual metrics collection
    45.0
}

async fn get_db_connection_pool_size() -> f64 {
    // Mock implementation - replace with actual metrics collection
    10.0
}

async fn get_db_active_connections() -> f64 {
    // Mock implementation - replace with actual metrics collection
    7.0
}

async fn get_db_query_time() -> f64 {
    // Mock implementation - replace with actual metrics collection
    12.3
}

async fn get_cache_hit_rate() -> f64 {
    // Mock implementation - replace with actual metrics collection
    0.95
}

async fn get_cache_size() -> f64 {
    // Mock implementation - replace with actual metrics collection
    256.0
}

async fn get_cache_eviction_rate() -> f64 {
    // Mock implementation - replace with actual metrics collection
    0.01
}
