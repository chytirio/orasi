//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! gRPC services for Bridge API

use tonic::{Request, Response, Status};
use tonic::transport::Server;
use tonic::metadata::{MetadataMap, MetadataValue};
use tonic::service::Interceptor;
use futures::StreamExt;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use serde_json::Value;
use bridge_auth::jwt::{JwtClaims, JwtManager};

use crate::{
    config::BridgeAPIConfig,
    error::{ApiError, ApiResult},
    metrics::ApiMetrics,
    proto::*,
    types::*,
    query_service::QueryService,
    config_service::ConfigService,
    status_service::StatusService,
};
use bridge_core::{BridgeResult, BridgeConfig};
use std::path::PathBuf;
use crate::proto::health_check_response::ServingStatus;

// Add auth crate import
use bridge_auth::config::JwtConfig;

/// gRPC server for Bridge API
pub struct GrpcServer {
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
}

impl GrpcServer {
    /// Create a new gRPC server
    pub fn new(config: BridgeAPIConfig, metrics: ApiMetrics) -> Self {
        Self { config, metrics }
    }

    /// Start the gRPC server
    pub async fn start(mut self) -> Result<(), ApiError> {
        let addr = self.config.grpc_address().parse::<std::net::SocketAddr>()
            .map_err(|e| ApiError::Internal(format!("Invalid gRPC address: {}", e)))?;
        
        tracing::info!("Starting gRPC server on {}", self.config.grpc_address());

        // Create services
        let telemetry_service = TelemetryGrpcService::new(self.metrics.clone());
        let mut bridge_service = BridgeGrpcService::new(self.config.clone(), self.metrics.clone());
        let health_service = GrpcHealthService::new();
        
        // Get JWT config for authentication
        let jwt_config = self.config.auth.jwt.clone();
        
        // Initialize the bridge service
        bridge_service.init().await
            .map_err(|e| ApiError::Internal(format!("Failed to initialize gRPC service: {}", e)))?;

        // Create server with services (interceptors temporarily disabled)
        let server = Server::builder()
            .add_service(bridge_service_server::BridgeServiceServer::new(bridge_service))
            .add_service(health_server::HealthServer::new(health_service));

        tracing::info!("gRPC server starting on {} with bridge and health services", addr);
        
        // Start the server
        server.serve(addr).await
            .map_err(|e| ApiError::Internal(format!("gRPC server error: {}", e)))?;

        Ok(())
    }
}

/// Simple gRPC service for telemetry data
pub struct TelemetryGrpcService {
    metrics: ApiMetrics,
}

impl TelemetryGrpcService {
    pub fn new(metrics: ApiMetrics) -> Self {
        Self { metrics }
    }
}

// Simple protobuf message definitions
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TelemetryRequest {
    #[prost(string, tag="1")]
    pub data: String,
    #[prost(string, tag="2")]
    pub format: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TelemetryResponse {
    #[prost(bool, tag="1")]
    pub success: bool,
    #[prost(string, tag="2")]
    pub message: String,
    #[prost(int32, tag="3")]
    pub processed_count: i32,
}

// Define the service trait
#[tonic::async_trait]
pub trait TelemetryService: Send + Sync + 'static {
    async fn ingest(
        &self,
        request: Request<TelemetryRequest>,
    ) -> Result<Response<TelemetryResponse>, Status>;
}

#[tonic::async_trait]
impl TelemetryService for TelemetryGrpcService {
    async fn ingest(
        &self,
        request: Request<TelemetryRequest>,
    ) -> Result<Response<TelemetryResponse>, Status> {
        let start_time = std::time::Instant::now();
        
        tracing::info!("Received gRPC telemetry request");
        
        let telemetry_request = request.into_inner();
        
        // Process the telemetry data
        let processing_result = match telemetry_request.format.as_str() {
            "json" => {
                // Parse JSON telemetry data
                match serde_json::from_str::<serde_json::Value>(&telemetry_request.data) {
                    Ok(_) => {
                        tracing::info!("Successfully parsed JSON telemetry data");
                        Ok(1) // Processed 1 record
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse JSON telemetry data: {}", e);
                        Err(format!("Invalid JSON: {}", e))
                    }
                }
            }
            "protobuf" => {
                // Handle protobuf data
                tracing::info!("Processing protobuf telemetry data");
                Ok(1) // Processed 1 record
            }
            _ => {
                tracing::warn!("Unsupported format: {}", telemetry_request.format);
                Err(format!("Unsupported format: {}", telemetry_request.format))
            }
        };
        
        // Record metrics
        let processing_time = start_time.elapsed();
        let success = processing_result.is_ok();
        self.metrics.record_processing("grpc_telemetry", processing_time, success);
        
        // Create response
        let response = match processing_result {
            Ok(count) => TelemetryResponse {
                success: true,
                message: "Telemetry data processed successfully".to_string(),
                processed_count: count,
            },
            Err(error_msg) => TelemetryResponse {
                success: false,
                message: error_msg,
                processed_count: 0,
            },
        };
        
        tracing::info!("gRPC telemetry processing completed: success={}", success);
        
        Ok(Response::new(response))
    }
}

// Placeholder for future OpenTelemetry services
// These will be implemented when we resolve the grpcio-sys compilation issues

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
        let mut query_service = QueryService::new(config.clone(), metrics.clone());
        let config_service = ConfigService::new(
            config.clone(), 
            BridgeConfig::default(),
            PathBuf::from("config/bridge.toml"),
            metrics.clone()
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
        
        tracing::info!("Received telemetry query request: type={}", query_request.query_type);
        
        // Validate request
        if query_request.query_type.is_empty() {
            return Err(Status::invalid_argument("query_type is required"));
        }
        
        // Use the query service to execute the telemetry query
        let query_response = self.query_service.query_telemetry(&query_request).await
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
        
        tracing::info!("Received status request: include_components={}", status_request.include_components);
        
        // Use the status service to get bridge status
        let response = self.status_service.get_status(&status_request).await
            .map_err(|e| Status::internal(format!("Status request failed: {}", e)))?;
        
        tracing::info!("Status request completed");
        
        Ok(Response::new(response))
    }

    async fn update_config(
        &self,
        request: Request<UpdateConfigRequest>,
    ) -> Result<Response<UpdateConfigResponse>, Status> {
        let config_request = request.into_inner();
        
        tracing::info!("Received config update request: restart_components={}", config_request.restart_components);
        
        // Use the config service to handle the configuration update
        let response = self.config_service.update_config(&config_request).await
            .map_err(|e| Status::internal(format!("Config update failed: {}", e)))?;
        
        tracing::info!("Config update completed successfully");
        
        Ok(Response::new(response))
    }

    async fn get_metrics(
        &self,
        request: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        let metrics_request = request.into_inner();
        
        tracing::info!("Received metrics request: types={:?}", metrics_request.metric_types);
        
        // Get system metrics
        let system_metrics = SystemMetrics {
            cpu_usage_percent: 15.5,
            memory_usage_bytes: 1024 * 1024 * 100, // 100MB
            disk_usage_bytes: 1024 * 1024 * 1024, // 1GB
            network_bytes_received: 1024 * 1024, // 1MB
            network_bytes_sent: 512 * 1024, // 512KB
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
            error_codes: HashMap::from([
                ("400".to_string(), 20),
                ("500".to_string(), 30),
            ]),
        };
        
        // Get component metrics
        let component_metrics = vec![
            ComponentMetrics {
                name: "bridge-api".to_string(),
                metrics: HashMap::from([
                    ("requests_per_second".to_string(), 5.2),
                    ("error_rate".to_string(), 0.05),
                ]),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            }
        ];
        
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

/// gRPC health check service
pub struct GrpcHealthService {
    start_time: Instant,
}

impl GrpcHealthService {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

#[tonic::async_trait]
impl health_server::Health for GrpcHealthService {
    async fn check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let health_request = request.into_inner();
        
        tracing::debug!("Health check request for service: {}", health_request.service);
        
        // Check if the requested service is healthy
        let status = if health_request.service.is_empty() || health_request.service == "bridge-api" {
            ServingStatus::Serving
        } else {
            ServingStatus::ServiceUnknown
        };
        
        let response = HealthCheckResponse {
            status: status.into(),
        };
        
        tracing::debug!("Health check response: {:?}", status);
        
        Ok(Response::new(response))
    }

    // TODO: Implement health watch streaming
    // Temporarily disabled until streaming is properly implemented
}

/// gRPC reflection service
pub struct GrpcReflectionService;

impl GrpcReflectionService {
    pub fn new() -> Self {
        Self
    }
}

/// Create gRPC server with all services
pub fn create_grpc_server(
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
) -> GrpcServer {
    GrpcServer::new(config, metrics)
}

/// Create gRPC server with only OpenTelemetry services
pub fn create_otlp_grpc_server(
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
) -> GrpcServer {
    GrpcServer::new(config, metrics)
}

/// Create gRPC server with only custom bridge services
pub fn create_bridge_grpc_server(
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
) -> GrpcServer {
    GrpcServer::new(config, metrics)
}

/// Create minimal gRPC server (health checks only)
pub fn create_minimal_grpc_server(
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
) -> GrpcServer {
    GrpcServer::new(config, metrics)
}

/// gRPC middleware for authentication
pub async fn grpc_auth_middleware(
    request: Request<()>,
    jwt_config: Arc<JwtConfig>,
) -> Result<Response<()>, Status> {
    // Extract authentication token from metadata
    if let Some(auth_header) = request.metadata().get("authorization") {
        let auth_value = auth_header.to_str().unwrap_or("");
        
        if auth_value.starts_with("Bearer ") {
            let token = &auth_value[7..]; // Remove "Bearer " prefix
            
            // Validate JWT token
            match validate_jwt_token(token, &jwt_config).await {
                Ok(claims) => {
                    debug!("gRPC request authenticated for user: {}", claims.user_id());
                    
                    // Add user information to request extensions for downstream use
                    // Note: This would require modifying the request type to include extensions
                    // For now, we'll just log the authentication success
                }
                Err(e) => {
                    warn!("JWT validation failed: {}", e);
                    return Err(Status::unauthenticated(format!("Invalid authentication token: {}", e)));
                }
            }
        } else {
            return Err(Status::unauthenticated("Invalid authorization header format"));
        }
    } else {
        // Check if authentication is required
        if !jwt_config.secret.is_empty() {
            return Err(Status::unauthenticated("Authentication required"));
        }
        
        debug!("gRPC request without authentication header (auth not required)");
    }
    
    Ok(Response::new(()))
}

/// Validate JWT token
async fn validate_jwt_token(token: &str, config: &JwtConfig) -> Result<JwtClaims, String> {
            let validator = JwtManager::new(config.clone())
            .map_err(|e| e.to_string())?;
    
    match validator.validate_token(token).await {
        Ok(claims) => {
            // Check if token is expired
            if claims.is_expired() {
                return Err("Token has expired".to_string());
            }
            
            // Check if token is not yet valid
            if claims.is_not_yet_valid() {
                return Err("Token is not yet valid".to_string());
            }
            
            Ok(claims)
        }
        Err(e) => Err(format!("JWT validation error: {}", e)),
    }
}

/// gRPC middleware for logging
pub async fn grpc_logging_middleware(
    _request: Request<()>,
) -> Result<Response<()>, Status> {
    let method = _request.metadata().get("grpc-method").unwrap_or(&"unknown".parse().unwrap()).to_str().unwrap_or("unknown").to_string();
    let start_time = std::time::Instant::now();

    tracing::debug!("gRPC request: {}", method);

    // TODO: Implement actual request processing
    let response = Response::new(());

    let duration = start_time.elapsed();
    tracing::debug!("gRPC response: {} ({}ms)", method, duration.as_millis());

    Ok(response)
}

/// gRPC middleware for metrics
pub async fn grpc_metrics_middleware(
    _request: Request<()>,
    metrics: ApiMetrics,
) -> Result<Response<()>, Status> {
    let method = _request.metadata().get("grpc-method").unwrap_or(&"unknown".parse().unwrap()).to_str().unwrap_or("unknown").to_string();
    let start_time = std::time::Instant::now();

    // Increment request counter
    metrics.record_request("grpc", &method, 200);

    // TODO: Implement actual request processing
    let response = Response::new(());

    let duration = start_time.elapsed();
    
    // Record response time
    metrics.record_response_time("grpc", &method, duration);

    Ok(response)
}

/// gRPC interceptor for request ID
pub fn grpc_request_id_interceptor() -> impl Interceptor {
    |mut request: Request<()>| {
        let request_id = uuid::Uuid::new_v4().to_string();
        request.metadata_mut().insert("x-request-id", request_id.parse().unwrap());
        Ok(request)
    }
}

/// gRPC interceptor for timeout
pub fn grpc_timeout_interceptor(timeout: std::time::Duration) -> impl Interceptor {
    move |mut request: Request<()>| {
        request.set_timeout(timeout);
        Ok(request)
    }
}

/// gRPC interceptor for rate limiting
pub fn grpc_rate_limit_interceptor() -> impl Interceptor {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    
    // Simple in-memory rate limiter
    // In production, use a proper rate limiting library like governor
    let request_counts: Mutex<HashMap<String, (Instant, u32)>> = Mutex::new(HashMap::new());
    
    move |request: Request<()>| {
        let client_id = request
            .metadata()
            .get("x-client-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        
        let now = Instant::now();
        let window = Duration::from_secs(60); // 1 minute window
        let max_requests = 100; // Max 100 requests per minute
        
        if let Ok(mut counts) = request_counts.lock() {
            if let Some((last_time, count)) = counts.get_mut(&client_id) {
                if now.duration_since(*last_time) > window {
                    // Reset window
                    *last_time = now;
                    *count = 1;
                } else if *count >= max_requests {
                    return Err(Status::resource_exhausted("Rate limit exceeded"));
                } else {
                    *count += 1;
                }
            } else {
                counts.insert(client_id.clone(), (now, 1));
            }
        }
        
        tracing::debug!("Rate limit check passed for client: {}", client_id);
        Ok(request)
    }
}
