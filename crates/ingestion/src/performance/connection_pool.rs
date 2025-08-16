//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Connection pooling for high-performance telemetry ingestion
//!
//! This module provides connection pooling for HTTP and gRPC connections
//! to optimize performance and reduce connection overhead.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use std::net::SocketAddr;
// HttpConnector removed - not needed with reqwest
use reqwest::Client;
use hyper_tls::HttpsConnector;
use tonic::transport::{Channel, Endpoint};
use std::future::Future;
use std::pin::Pin;

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: usize,
    /// Maximum number of idle connections
    pub max_idle_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Idle connection timeout
    pub idle_timeout: Duration,
    /// Keep-alive timeout
    pub keep_alive_timeout: Duration,
    /// Connection retry attempts
    pub max_retries: usize,
    /// Retry backoff duration
    pub retry_backoff: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            max_idle_connections: 20,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            keep_alive_timeout: Duration::from_secs(60),
            max_retries: 3,
            retry_backoff: Duration::from_millis(100),
        }
    }
}

/// HTTP connection pool
pub struct HttpConnectionPool {
    config: ConnectionPoolConfig,
    clients: Arc<Mutex<HashMap<String, PooledHttpClient>>>,
    semaphore: Arc<Semaphore>,
    stats: Arc<Mutex<PoolStats>>,
}

/// Pooled HTTP client
#[derive(Clone)]
struct PooledHttpClient {
    client: Client,
    last_used: Instant,
    is_healthy: bool,
    error_count: u32,
}

/// gRPC connection pool
pub struct GrpcConnectionPool {
    config: ConnectionPoolConfig,
    channels: Arc<Mutex<HashMap<String, PooledGrpcChannel>>>,
    semaphore: Arc<Semaphore>,
    stats: Arc<Mutex<PoolStats>>,
}

/// Pooled gRPC channel
struct PooledGrpcChannel {
    channel: Channel,
    last_used: Instant,
    is_healthy: bool,
    error_count: u32,
}

/// Connection pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total connections created
    pub total_connections: u64,
    /// Active connections
    pub active_connections: u64,
    /// Idle connections
    pub idle_connections: u64,
    /// Connection errors
    pub connection_errors: u64,
    /// Connection timeouts
    pub connection_timeouts: u64,
    /// Average connection time
    pub avg_connection_time_ms: f64,
    /// Total requests processed
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
}

impl HttpConnectionPool {
    /// Create new HTTP connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        let stats = Arc::new(Mutex::new(PoolStats::default()));

        Self {
            config,
            clients: Arc::new(Mutex::new(HashMap::new())),
            semaphore,
            stats,
        }
    }

    /// Get HTTP client for endpoint
    pub async fn get_client(&self, endpoint: &str) -> Result<PooledHttpClient, PoolError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| PoolError::PoolExhausted)?;
        
        let mut clients = self.clients.lock().await;
        
        // Check if we have an existing healthy client
        if let Some(client) = clients.get_mut(endpoint) {
            if client.is_healthy && client.last_used.elapsed() < self.config.idle_timeout {
                client.last_used = Instant::now();
                return Ok(client.clone());
            }
        }

        // Create new client
        let client = self.create_http_client(endpoint).await?;
        let pooled_client = PooledHttpClient {
            client,
            last_used: Instant::now(),
            is_healthy: true,
            error_count: 0,
        };

        clients.insert(endpoint.to_string(), pooled_client.clone());
        
        // Update stats
        let mut stats = self.stats.lock().await;
        stats.total_connections += 1;
        stats.active_connections += 1;

        Ok(pooled_client)
    }

    /// Create new HTTP client
    async fn create_http_client(&self, endpoint: &str) -> Result<Client, PoolError> {
        let start_time = Instant::now();
        
        let client = Client::builder()
            .pool_idle_timeout(self.config.keep_alive_timeout)
            .pool_max_idle_per_host(self.config.max_idle_connections)
            .build()
            .map_err(|e| PoolError::ConnectionFailed(e.to_string()))?;

        // Test connection
        let test_uri = format!("{}/health", endpoint).parse::<reqwest::Url>()
            .map_err(|_| PoolError::InvalidEndpoint(endpoint.to_string()))?;
        
        let response = timeout(
            self.config.connection_timeout, 
            client.get(test_uri).send()
        ).await
            .map_err(|_| PoolError::ConnectionTimeout)?
            .map_err(|e| PoolError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(PoolError::ConnectionFailed(format!("HTTP {}", response.status())));
        }

        // Update stats
        let connection_time = start_time.elapsed();
        let mut stats = self.stats.lock().await;
        stats.avg_connection_time_ms = (stats.avg_connection_time_ms * stats.total_connections as f64 + connection_time.as_millis() as f64) 
            / (stats.total_connections + 1) as f64;

        Ok(client)
    }

    /// Clean up idle connections
    pub async fn cleanup_idle_connections(&self) {
        let mut clients = self.clients.lock().await;
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (endpoint, client) in clients.iter() {
            if client.last_used.elapsed() > self.config.idle_timeout {
                to_remove.push(endpoint.clone());
            }
        }

        for endpoint in to_remove {
            if let Some(client) = clients.remove(&endpoint) {
                info!("Removed idle HTTP connection for endpoint: {}", endpoint);
                let mut stats = self.stats.lock().await;
                stats.idle_connections = stats.idle_connections.saturating_sub(1);
            }
        }
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }

    /// Mark connection as unhealthy
    pub async fn mark_unhealthy(&self, endpoint: &str) {
        let mut clients = self.clients.lock().await;
        if let Some(client) = clients.get_mut(endpoint) {
            client.is_healthy = false;
            client.error_count += 1;
            
            let mut stats = self.stats.lock().await;
            stats.connection_errors += 1;
        }
    }
}

impl GrpcConnectionPool {
    /// Create new gRPC connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        let stats = Arc::new(Mutex::new(PoolStats::default()));

        Self {
            config,
            channels: Arc::new(Mutex::new(HashMap::new())),
            semaphore,
            stats,
        }
    }

    /// Get gRPC channel for endpoint
    pub async fn get_channel(&self, endpoint: &str) -> Result<PooledGrpcChannel, PoolError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| PoolError::PoolExhausted)?;
        
        let mut channels = self.channels.lock().await;
        
        // Check if we have an existing healthy channel
        if let Some(channel) = channels.get_mut(endpoint) {
            if channel.is_healthy && channel.last_used.elapsed() < self.config.idle_timeout {
                channel.last_used = Instant::now();
                return Ok(channel.clone());
            }
        }

        // Create new channel
        let channel = self.create_grpc_channel(endpoint).await?;
        let pooled_channel = PooledGrpcChannel {
            channel,
            last_used: Instant::now(),
            is_healthy: true,
            error_count: 0,
        };

        channels.insert(endpoint.to_string(), pooled_channel.clone());
        
        // Update stats
        let mut stats = self.stats.lock().await;
        stats.total_connections += 1;
        stats.active_connections += 1;

        Ok(pooled_channel)
    }

    /// Create new gRPC channel
    async fn create_grpc_channel(&self, endpoint: &str) -> Result<Channel, PoolError> {
        let start_time = Instant::now();
        
        let channel = Endpoint::from_shared(endpoint.to_string())
            .map_err(|e| PoolError::InvalidEndpoint(e.to_string()))?
            .timeout(self.config.connection_timeout)
            .connect_timeout(self.config.connection_timeout)
            .keep_alive_while_idle(true)
            .http2_keep_alive_interval(self.config.keep_alive_timeout)
            .connect()
            .await
            .map_err(|e| PoolError::ConnectionFailed(e.to_string()))?;

        // Update stats
        let connection_time = start_time.elapsed();
        let mut stats = self.stats.lock().await;
        stats.avg_connection_time_ms = (stats.avg_connection_time_ms * stats.total_connections as f64 + connection_time.as_millis() as f64) 
            / (stats.total_connections + 1) as f64;

        Ok(channel)
    }

    /// Clean up idle connections
    pub async fn cleanup_idle_connections(&self) {
        let mut channels = self.channels.lock().await;
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (endpoint, channel) in channels.iter() {
            if channel.last_used.elapsed() > self.config.idle_timeout {
                to_remove.push(endpoint.clone());
            }
        }

        for endpoint in to_remove {
            if let Some(channel) = channels.remove(&endpoint) {
                info!("Removed idle gRPC connection for endpoint: {}", endpoint);
                let mut stats = self.stats.lock().await;
                stats.idle_connections = stats.idle_connections.saturating_sub(1);
            }
        }
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }

    /// Mark connection as unhealthy
    pub async fn mark_unhealthy(&self, endpoint: &str) {
        let mut channels = self.channels.lock().await;
        if let Some(channel) = channels.get_mut(endpoint) {
            channel.is_healthy = false;
            channel.error_count += 1;
            
            let mut stats = self.stats.lock().await;
            stats.connection_errors += 1;
        }
    }
}

/// Connection pool error types
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Connection pool exhausted")]
    PoolExhausted,
    #[error("Connection timeout")]
    ConnectionTimeout,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("Request timeout")]
    RequestTimeout,
    #[error("Request failed: {0}")]
    RequestFailed(String),
}

/// Connection pool manager
pub struct ConnectionPoolManager {
    http_pool: HttpConnectionPool,
    grpc_pool: GrpcConnectionPool,
    cleanup_interval: Duration,
}

impl ConnectionPoolManager {
    /// Create new connection pool manager
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let http_pool = HttpConnectionPool::new(config.clone());
        let grpc_pool = GrpcConnectionPool::new(config);
        let cleanup_interval = Duration::from_secs(60);

        Self {
            http_pool,
            grpc_pool,
            cleanup_interval,
        }
    }

    /// Start cleanup task
    pub async fn start_cleanup_task(&self) {
        let http_pool = self.http_pool.clone();
        let grpc_pool = self.grpc_pool.clone();
        let interval = self.cleanup_interval;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                
                // Cleanup HTTP connections
                http_pool.cleanup_idle_connections().await;
                
                // Cleanup gRPC connections
                grpc_pool.cleanup_idle_connections().await;
            }
        });
    }

    /// Get HTTP connection pool
    pub fn http_pool(&self) -> &HttpConnectionPool {
        &self.http_pool
    }

    /// Get gRPC connection pool
    pub fn grpc_pool(&self) -> &GrpcConnectionPool {
        &self.grpc_pool
    }

    /// Get combined statistics
    pub async fn get_stats(&self) -> CombinedPoolStats {
        let http_stats = self.http_pool.get_stats().await;
        let grpc_stats = self.grpc_pool.get_stats().await;

        CombinedPoolStats {
            http: http_stats,
            grpc: grpc_stats,
        }
    }
}

/// Combined pool statistics
#[derive(Debug, Clone)]
pub struct CombinedPoolStats {
    pub http: PoolStats,
    pub grpc: PoolStats,
}

// Manual Clone implementation removed - using derive instead

impl Clone for PooledGrpcChannel {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            last_used: self.last_used,
            is_healthy: self.is_healthy,
            error_count: self.error_count,
        }
    }
}

// Implement Clone for connection pools
impl Clone for HttpConnectionPool {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            clients: self.clients.clone(),
            semaphore: self.semaphore.clone(),
            stats: self.stats.clone(),
        }
    }
}

impl Clone for GrpcConnectionPool {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            channels: self.channels.clone(),
            semaphore: self.semaphore.clone(),
            stats: self.stats.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_http_connection_pool_creation() {
        let config = ConnectionPoolConfig::default();
        let pool = HttpConnectionPool::new(config);
        
        let stats = pool.get_stats().await;
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_grpc_connection_pool_creation() {
        let config = ConnectionPoolConfig::default();
        let pool = GrpcConnectionPool::new(config);
        
        let stats = pool.get_stats().await;
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_connection_pool_manager() {
        let config = ConnectionPoolConfig::default();
        let manager = ConnectionPoolManager::new(config);
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.http.total_connections, 0);
        assert_eq!(stats.grpc.total_connections, 0);
    }
}
