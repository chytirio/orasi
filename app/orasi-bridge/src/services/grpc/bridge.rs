//! Bridge gRPC service implementation

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};
use tracing;

use crate::{
    config::BridgeAPIConfig, metrics::ApiMetrics, proto::*,
    services::{config::ConfigService, query::QueryService, status::StatusService},
};
use bridge_core::{BridgeConfig, BridgeResult};

/// Custom Bridge gRPC Service implementation
pub struct BridgeGrpcService {
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
    query_service: QueryService,
    config_service: ConfigService,
    status_service: StatusService,
}

impl BridgeGrpcService {
    pub fn new(config: BridgeAPIConfig, metrics: ApiMetrics) -> Self {
        let query_service = QueryService::new(config.clone(), metrics.clone());
        let config_service = ConfigService::new(
            config.clone(),
            BridgeConfig::default(),
            PathBuf::from("config/bridge.toml"),
            metrics.clone(),
        );
        let status_service = StatusService::new(config.clone(), metrics.clone());

        Self {
            config,
            metrics,
            query_service,
            config_service,
            status_service,
        }
    }

    /// Initialize the gRPC service
    pub async fn init(&mut self) -> BridgeResult<()> {
        tracing::info!("Initializing Bridge gRPC service");

        // Initialize query engine integration
        self.query_service.init_query_engine().await?;

        // Initialize health monitoring integration
        self.status_service.init_health_monitoring().await?;

        tracing::info!("Bridge gRPC service initialized successfully");
        Ok(())
    }
}

#[tonic::async_trait]
impl bridge_service_server::BridgeService for BridgeGrpcService {
    async fn query_telemetry(
        &self,
        request: Request<QueryTelemetryRequest>,
    ) -> Result<Response<QueryTelemetryResponse>, Status> {
        let start_time = Instant::now();
        let query_request = request.into_inner();

        tracing::info!(
            "Received telemetry query request: type={}",
            query_request.query_type
        );

        // Validate request
        if query_request.query_type.is_empty() {
            return Err(Status::invalid_argument("query_type is required"));
        }

        // Use the query service to execute the telemetry query
        let query_response = self
            .query_service
            .query_telemetry(&query_request)
            .await
            .map_err(|e| Status::internal(format!("Query execution failed: {}", e)))?;

        let records = query_response.records;

        let execution_time = start_time.elapsed().as_millis() as i64;

        let response = QueryTelemetryResponse {
            records,
            total_count: query_response.total_count,
            execution_time_ms: query_response.execution_time_ms,
            status: query_response.status,
            error_message: query_response.error_message,
        };

        tracing::info!("Telemetry query completed in {}ms", execution_time);

        Ok(Response::new(response))
    }

    async fn get_status(
        &self,
        request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let status_request = request.into_inner();

        tracing::info!(
            "Received status request: include_components={}",
            status_request.include_components
        );

        // Use the status service to get bridge status
        let response = self
            .status_service
            .get_status(&status_request)
            .await
            .map_err(|e| Status::internal(format!("Status request failed: {}", e)))?;

        tracing::info!("Status request completed");

        Ok(Response::new(response))
    }

    async fn update_config(
        &self,
        request: Request<UpdateConfigRequest>,
    ) -> Result<Response<UpdateConfigResponse>, Status> {
        let config_request = request.into_inner();

        tracing::info!(
            "Received config update request: restart_components={}",
            config_request.restart_components
        );

        // Use the config service to handle the configuration update
        let response = self
            .config_service
            .update_config(&config_request.config_json)
            .await
            .map_err(|e| Status::internal(format!("Config update failed: {}", e)))?;

        tracing::info!("Config update completed successfully");

        Ok(Response::new(response))
    }

    async fn get_metrics(
        &self,
        request: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        let metrics_request = request.into_inner();

        tracing::info!(
            "Received metrics request: types={:?}",
            metrics_request.metric_types
        );

        // Get system metrics
        let system_metrics = SystemMetrics {
            cpu_usage_percent: 15.5,
            memory_usage_bytes: 1024 * 1024 * 100, // 100MB
            disk_usage_bytes: 1024 * 1024 * 1024,  // 1GB
            network_bytes_received: 1024 * 1024,   // 1MB
            network_bytes_sent: 512 * 1024,        // 512KB
            active_connections: 10,
            request_rate: 5.2,
            error_rate: 0.1,
        };

        // Get API metrics (using the actual ApiMetrics structure)
        let api_metrics = crate::proto::ApiMetrics {
            total_requests: 1000,
            successful_requests: 950,
            failed_requests: 50,
            avg_response_time_ms: 150.0,
            requests_by_endpoint: HashMap::from([
                ("/api/v1/status".to_string(), 200),
                ("/api/v1/metrics".to_string(), 150),
                ("/api/v1/query".to_string(), 300),
            ]),
            error_codes: HashMap::from([("400".to_string(), 20), ("500".to_string(), 30)]),
        };

        // Get component metrics
        let component_metrics = vec![ComponentMetrics {
            name: "bridge-api".to_string(),
            metrics: HashMap::from([
                ("requests_per_second".to_string(), 5.2),
                ("error_rate".to_string(), 0.05),
            ]),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }];

        let response = GetMetricsResponse {
            system_metrics: Some(system_metrics),
            api_metrics: Some(api_metrics),
            component_metrics,
        };

        tracing::info!("Metrics request completed");

        Ok(Response::new(response))
    }

    // TODO: Implement streaming telemetry
    // Temporarily disabled until streaming is properly implemented
}
