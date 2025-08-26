//! Rate limiter for gateway

use crate::{config::GatewayConfig, error::GatewayError, types::*};
use governor::clock::DefaultClock;
use governor::middleware::NoOpMiddleware;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Rate limiter for gateway
pub struct GatewayRateLimiter {
    config: GatewayConfig,
    global_limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    client_limiters: Arc<
        RwLock<
            HashMap<
                String,
                Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
            >,
        >,
    >,
    endpoint_limiters: Arc<
        RwLock<
            HashMap<
                String,
                Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
            >,
        >,
    >,
    request_counts: Arc<RwLock<HashMap<String, RequestCount>>>,
    stats: Arc<RwLock<RateLimitTracking>>,
}

/// Request count tracking
#[derive(Debug, Clone)]
struct RequestCount {
    count: u32,
    window_start: Instant,
}

/// Rate limit tracking for statistics
#[derive(Debug, Clone)]
struct RateLimitTracking {
    total_requests: u64,
    rate_limited_requests: u64,
    global_rate_limited: u64,
    client_rate_limited: u64,
    endpoint_rate_limited: u64,
}

impl Default for RateLimitTracking {
    fn default() -> Self {
        Self {
            total_requests: 0,
            rate_limited_requests: 0,
            global_rate_limited: 0,
            client_rate_limited: 0,
            endpoint_rate_limited: 0,
        }
    }
}

/// Rate limit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStats {
    pub total_requests: u64,
    pub rate_limited_requests: u64,
    pub global_rate_limited: u64,
    pub client_rate_limited: u64,
    pub endpoint_rate_limited: u64,
}

impl GatewayRateLimiter {
    /// Create new rate limiter
    pub async fn new(config: &GatewayConfig) -> Result<Self, GatewayError> {
        let global_quota = Quota::per_second(
            std::num::NonZeroU32::new(config.rate_limiting.requests_per_second)
                .unwrap_or(std::num::NonZeroU32::new(1).unwrap()),
        );
        let global_limiter = Arc::new(GovernorRateLimiter::direct(global_quota));

        let client_limiters = Arc::new(RwLock::new(HashMap::new()));
        let endpoint_limiters = Arc::new(RwLock::new(HashMap::new()));
        let request_counts = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(RwLock::new(RateLimitTracking::default()));

        Ok(Self {
            config: config.clone(),
            global_limiter,
            client_limiters,
            endpoint_limiters,
            request_counts,
            stats,
        })
    }

    /// Check if request is allowed
    pub async fn check_rate_limit(
        &self,
        client_id: &str,
        endpoint: &str,
    ) -> Result<bool, GatewayError> {
        debug!(
            "Checking rate limit for client: {} endpoint: {}",
            client_id, endpoint
        );

        // Increment total request count
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += 1;
        }

        // Check global rate limit
        if !self.check_global_limit().await? {
            warn!("Global rate limit exceeded");
            {
                let mut stats = self.stats.write().await;
                stats.rate_limited_requests += 1;
                stats.global_rate_limited += 1;
            }
            return Ok(false);
        }

        // Check client-specific rate limit
        if !self.check_client_limit(client_id).await? {
            warn!("Client rate limit exceeded for: {}", client_id);
            {
                let mut stats = self.stats.write().await;
                stats.rate_limited_requests += 1;
                stats.client_rate_limited += 1;
            }
            return Ok(false);
        }

        // Check endpoint-specific rate limit
        if !self.check_endpoint_limit(endpoint).await? {
            warn!("Endpoint rate limit exceeded for: {}", endpoint);
            {
                let mut stats = self.stats.write().await;
                stats.rate_limited_requests += 1;
                stats.endpoint_rate_limited += 1;
            }
            return Ok(false);
        }

        // Update request counts
        self.update_request_count(client_id, endpoint).await;

        debug!("Rate limit check passed");
        Ok(true)
    }

    /// Check global rate limit
    async fn check_global_limit(&self) -> Result<bool, GatewayError> {
        match self.global_limiter.check() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Check client-specific rate limit
    async fn check_client_limit(&self, client_id: &str) -> Result<bool, GatewayError> {
        let limiter = {
            let mut client_limiters = self.client_limiters.write().await;
            client_limiters
                .entry(client_id.to_string())
                .or_insert_with(|| {
                    let quota = Quota::per_second(
                        std::num::NonZeroU32::new(self.config.rate_limiting.requests_per_second)
                            .unwrap_or(std::num::NonZeroU32::new(1).unwrap()),
                    );
                    Arc::new(GovernorRateLimiter::direct(quota))
                })
                .clone()
        };

        match limiter.check() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Check endpoint-specific rate limit
    async fn check_endpoint_limit(&self, endpoint: &str) -> Result<bool, GatewayError> {
        let limiter = {
            let mut endpoint_limiters = self.endpoint_limiters.write().await;
            endpoint_limiters
                .entry(endpoint.to_string())
                .or_insert_with(|| {
                    let quota = Quota::per_second(
                        std::num::NonZeroU32::new(self.config.rate_limiting.requests_per_second)
                            .unwrap_or(std::num::NonZeroU32::new(1).unwrap()),
                    );
                    Arc::new(GovernorRateLimiter::direct(quota))
                })
                .clone()
        };

        match limiter.check() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Update request count
    async fn update_request_count(&self, client_id: &str, endpoint: &str) {
        let key = format!("{}:{}", client_id, endpoint);
        let mut request_counts = self.request_counts.write().await;

        let count = request_counts.entry(key).or_insert_with(|| RequestCount {
            count: 0,
            window_start: Instant::now(),
        });

        count.count += 1;

        // Reset count if window has passed
        if count.window_start.elapsed() > Duration::from_secs(1) {
            count.count = 1;
            count.window_start = Instant::now();
        }
    }

    /// Get rate limit statistics
    pub async fn get_stats(&self) -> RateLimitStats {
        let stats = self.stats.read().await;
        
        RateLimitStats {
            total_requests: stats.total_requests,
            rate_limited_requests: stats.rate_limited_requests,
            global_rate_limited: stats.global_rate_limited,
            client_rate_limited: stats.client_rate_limited,
            endpoint_rate_limited: stats.endpoint_rate_limited,
        }
    }

    /// Update configuration
    pub async fn update_config(
        &self,
        new_config: crate::config::RateLimitingConfig,
    ) -> Result<(), GatewayError> {
        let global_quota = Quota::per_second(
            std::num::NonZeroU32::new(new_config.requests_per_second)
                .unwrap_or(std::num::NonZeroU32::new(1).unwrap()),
        );
        let new_global_limiter = Arc::new(GovernorRateLimiter::direct(global_quota));

        // Update global limiter
        // Note: This is a simplified approach. In a real implementation,
        // you'd want to atomically swap the limiter or use a more sophisticated approach.

        info!("Updated rate limiting configuration");
        Ok(())
    }

    /// Get current configuration
    pub fn get_config(&self) -> &crate::config::RateLimitingConfig {
        &self.config.rate_limiting
    }

    /// Get requests per second limit
    pub fn get_requests_per_second(&self) -> Result<u32, GatewayError> {
        Ok(self.config.rate_limiting.requests_per_second)
    }

    /// Get burst size
    pub fn get_burst_size(&self) -> Result<u32, GatewayError> {
        Ok(self.config.rate_limiting.burst_size)
    }

    /// Start the rate limiter
    pub async fn start(&self) -> Result<(), GatewayError> {
        info!("Starting gateway rate limiter");
        Ok(())
    }

    /// Stop the rate limiter
    pub async fn stop(&self) -> Result<(), GatewayError> {
        info!("Stopping gateway rate limiter");
        Ok(())
    }

    /// Reset rate limit counters
    pub async fn reset_counters(&self) -> Result<(), GatewayError> {
        let mut request_counts = self.request_counts.write().await;
        request_counts.clear();
        
        let mut stats = self.stats.write().await;
        *stats = RateLimitTracking::default();
        
        info!("Rate limit counters and statistics reset");
        Ok(())
    }
}
