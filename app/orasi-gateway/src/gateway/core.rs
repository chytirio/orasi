//! Main gateway core implementation

use crate::{
    config::GatewayConfig,
    discovery::ServiceDiscovery,
    error::GatewayError,
    gateway::health::HealthChecker,
    gateway::rate_limiter::GatewayRateLimiter,
    gateway::state::GatewayState,
    load_balancer::LoadBalancer,
    metrics::MetricsCollector,
    routing::proxy::Proxy,
    routing::Router,
    types::{GatewayInfo, GatewayMetrics, GatewayStatus, HealthStatus},
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

/// Main Orasi Gateway implementation
pub struct OrasiGateway {
    /// Gateway configuration
    config: GatewayConfig,

    /// Gateway state
    state: Arc<RwLock<GatewayState>>,

    /// Service discovery
    service_discovery: Arc<ServiceDiscovery>,

    /// Router
    router: Arc<Router>,

    /// Load balancer
    load_balancer: Arc<LoadBalancer>,

    /// Proxy
    proxy: Arc<Proxy>,

    /// Rate limiter
    rate_limiter: Arc<GatewayRateLimiter>,

    /// Health checker
    health_checker: Arc<HealthChecker>,

    /// Metrics collector
    metrics_collector: Arc<MetricsCollector>,

    /// Shutdown signal receiver
    shutdown_rx: mpsc::Receiver<()>,

    /// Shutdown signal sender
    shutdown_tx: mpsc::Sender<()>,
}

impl OrasiGateway {
    /// Create a new Orasi Gateway
    pub async fn new(config: GatewayConfig) -> Result<Self, GatewayError> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        // Initialize gateway state
        let state = Arc::new(RwLock::new(GatewayState::new(&config)));

        // Initialize service discovery
        let service_discovery = Arc::new(ServiceDiscovery::new(&config).await?);

        // Initialize router
        let router = Arc::new(Router::new(&config, state.clone()).await?);

        // Initialize load balancer
        let load_balancer = Arc::new(LoadBalancer::new(&config, state.clone()).await?);

        // Initialize proxy
        let proxy = Arc::new(Proxy::new(&config, state.clone()).await?);

        // Initialize rate limiter
        let rate_limiter = Arc::new(GatewayRateLimiter::new(&config).await?);

        // Initialize health checker
        let health_checker = Arc::new(HealthChecker::new(&config).await?);

        // Initialize metrics collector
        let metrics_collector = Arc::new(MetricsCollector::new(&config).await?);

        Ok(Self {
            config,
            state,
            service_discovery,
            router,
            load_balancer,
            proxy,
            rate_limiter,
            health_checker,
            metrics_collector,
            shutdown_rx,
            shutdown_tx,
        })
    }

    /// Start the gateway
    pub async fn start(&mut self) -> Result<(), GatewayError> {
        info!("Starting Orasi gateway {}", self.config.gateway_id);

        // Update gateway status
        {
            let mut state = self.state.write().await;
            state.set_status(GatewayStatus::Starting);
        }

        // Start service discovery
        self.service_discovery.start().await?;

        // Start router
        self.router.start().await?;

        // Start load balancer
        self.load_balancer.start().await?;

        // Start proxy
        self.proxy.start().await?;

        // Start rate limiter
        self.rate_limiter.start().await?;

        // Start health checker
        self.health_checker.start().await?;

        // Start metrics collector
        self.metrics_collector.start().await?;

        // Update gateway status
        {
            let mut state = self.state.write().await;
            state.set_status(GatewayStatus::Running);
        }

        info!("Orasi gateway started successfully");
        Ok(())
    }

    /// Run the gateway main loop
    pub async fn run(&mut self) -> Result<(), GatewayError> {
        info!("Running Orasi gateway main loop");

        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = self.shutdown_rx.recv() => {
                    info!("Received shutdown signal");
                    break;
                }

                // Handle service discovery updates
                result = self.service_discovery.refresh_services() => {
                    match result {
                        Ok(services) => {
                            if let Err(e) = self.update_services(services).await {
                                error!("Error updating services: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error refreshing services: {}", e);
                        }
                    }
                }

                // Handle health check requests
                result = self.health_checker.check_health() => {
                    match result {
                        Ok(status) => {
                            if let Err(e) = self.update_health_status(status).await {
                                error!("Error updating health status: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error during health check: {}", e);
                        }
                    }
                }

                // Handle metrics collection
                result = self.metrics_collector.collect_metrics() => {
                    match result {
                        Ok(metrics) => {
                            if let Err(e) = self.update_metrics(metrics).await {
                                error!("Error updating metrics: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error collecting metrics: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Shutdown the gateway
    pub async fn shutdown(&self) -> Result<(), GatewayError> {
        info!("Shutting down Orasi gateway");

        // Update gateway status
        {
            let mut state = self.state.write().await;
            state.set_status(GatewayStatus::Stopping);
        }

        // Stop metrics collector
        self.metrics_collector.stop().await?;

        // Stop health checker
        self.health_checker.stop().await?;

        // Stop rate limiter
        self.rate_limiter.stop().await?;

        // Stop proxy
        self.proxy.stop().await?;

        // Stop load balancer
        self.load_balancer.stop().await?;

        // Stop router
        self.router.stop().await?;

        // Stop service discovery
        self.service_discovery.stop().await?;

        // Update gateway status
        {
            let mut state = self.state.write().await;
            state.set_status(GatewayStatus::Stopped);
        }

        info!("Orasi gateway shutdown completed");
        Ok(())
    }

    /// Send shutdown signal
    pub async fn send_shutdown_signal(&self) -> Result<(), GatewayError> {
        self.shutdown_tx
            .send(())
            .await
            .map_err(|e| GatewayError::Shutdown(e.to_string()))?;
        Ok(())
    }

    /// Update services
    async fn update_services(
        &self,
        services: std::collections::HashMap<String, crate::types::ServiceInfo>,
    ) -> Result<(), GatewayError> {
        let mut state = self.state.write().await;
        state.update_services(services);
        Ok(())
    }

    /// Update health status
    async fn update_health_status(&self, status: HealthStatus) -> Result<(), GatewayError> {
        // TODO: Update gateway health status
        Ok(())
    }

    /// Update metrics
    async fn update_metrics(&self, metrics: GatewayMetrics) -> Result<(), GatewayError> {
        let mut state = self.state.write().await;
        state.update_metrics(metrics);
        Ok(())
    }

    /// Get gateway information
    pub async fn get_gateway_info(&self) -> GatewayInfo {
        let state = self.state.read().await;
        state.get_gateway_info()
    }

    /// Get gateway status
    pub async fn get_status(&self) -> GatewayStatus {
        let state = self.state.read().await;
        state.get_status()
    }

    /// Get gateway metrics
    pub async fn get_metrics(&self) -> GatewayMetrics {
        let state = self.state.read().await;
        state.get_metrics()
    }

    /// Get gateway state
    pub async fn get_state(&self) -> Arc<RwLock<GatewayState>> {
        self.state.clone()
    }
}
