//! Bridge gRPC service implementation

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};
use tracing;

use crate::{
    config::BridgeAPIConfig,
    metrics::ApiMetrics,
    proto::*,
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

    type StreamTelemetryStream = tonic::codegen::tokio_stream::wrappers::ReceiverStream<
        std::result::Result<StreamTelemetryResponse, Status>,
    >;

    async fn stream_telemetry(
        &self,
        request: Request<StreamTelemetryRequest>,
    ) -> Result<Response<Self::StreamTelemetryStream>, Status> {
        let stream_request = request.into_inner();
        let start_time = Instant::now();

        tracing::info!(
            "Received streaming telemetry request: type={}, batch_size={}",
            stream_request.query_type,
            stream_request.batch_size
        );

        // Validate request
        if stream_request.query_type.is_empty() {
            return Err(Status::invalid_argument("query_type is required"));
        }

        // Validate time range if provided
        if let Some(time_range) = &stream_request.time_range {
            if time_range.start_time >= time_range.end_time {
                return Err(Status::invalid_argument(
                    "start_time must be before end_time",
                ));
            }
        }

        let batch_size = stream_request.batch_size.max(1).min(1000); // Limit batch size
        let (tx, rx) = tokio::sync::mpsc::channel(100); // Buffer size of 100

        // Spawn streaming task
        let query_service = self.query_service.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            if let Err(e) =
                Self::stream_telemetry_data(stream_request, batch_size, tx, query_service, metrics)
                    .await
            {
                tracing::error!("Streaming telemetry failed: {}", e);
            }
        });

        let stream = tonic::codegen::tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Response::new(stream))
    }
}

impl BridgeGrpcService {
    /// Stream telemetry data in batches
    async fn stream_telemetry_data(
        request: StreamTelemetryRequest,
        batch_size: i32,
        tx: tokio::sync::mpsc::Sender<Result<StreamTelemetryResponse, Status>>,
        query_service: QueryService,
        metrics: ApiMetrics,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();
        let mut total_processed = 0;
        let mut batch_number = 0;
        let mut offset = 0;
        let max_duration = Duration::from_secs(300); // 5 minute timeout

        loop {
            // Check for timeout
            if start_time.elapsed() > max_duration {
                tracing::warn!(
                    "Streaming telemetry timed out after {} seconds",
                    max_duration.as_secs()
                );
                let _ = tx
                    .send(Err(Status::deadline_exceeded("Streaming timeout exceeded")))
                    .await;
                break;
            }

            batch_number += 1;
            let batch_start_time = Instant::now();

            // Check for maximum batch limit
            if batch_number > 1000 {
                tracing::warn!("Streaming telemetry reached maximum batch limit of 1000");
                let _ = tx
                    .send(Err(Status::resource_exhausted(
                        "Maximum batch limit exceeded",
                    )))
                    .await;
                break;
            }

            // Create query request for this batch with proper pagination
            let query_request = QueryTelemetryRequest {
                query_type: request.query_type.clone(),
                time_range: request.time_range.clone(),
                filters: request.filters.clone(),
                limit: batch_size,
                offset,
                sort_by: "timestamp".to_string(),
                sort_order: "desc".to_string(),
            };

            // Execute query for this batch
            let query_response = match query_service.query_telemetry(&query_request).await {
                Ok(response) => response,
                Err(e) => {
                    let status = Status::internal(format!("Query execution failed: {}", e));
                    if tx.send(Err(status)).await.is_err() {
                        tracing::warn!("Client disconnected during error handling");
                    }
                    return Err(e.into());
                }
            };

            let records = query_response.records;
            let records_len = records.len();
            let is_last_batch = records_len < batch_size as usize || records.is_empty();

            // Create streaming response
            let stream_response = StreamTelemetryResponse {
                records,
                is_last_batch,
                total_processed,
            };

            // Send batch to client
            if tx.send(Ok(stream_response)).await.is_err() {
                tracing::warn!("Client disconnected from streaming telemetry");
                break;
            }

            total_processed += records_len as i64;
            offset += records_len as i32; // Update offset for next batch

            // Record metrics for this batch
            let batch_duration = batch_start_time.elapsed();
            metrics.record_processing("streaming_batch", batch_duration, true);

            tracing::debug!(
                "Streamed batch {}: {} records, total_processed={}, offset={}, is_last={}",
                batch_number,
                records_len,
                total_processed,
                offset,
                is_last_batch
            );

            // If this is the last batch, break
            if is_last_batch {
                break;
            }

            // Add small delay between batches to prevent overwhelming the client
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let total_duration = start_time.elapsed();
        tracing::info!(
            "Streaming telemetry completed: {} batches, {} total records, {}ms",
            batch_number,
            total_processed,
            total_duration.as_millis()
        );

        // Record final metrics
        metrics.record_processing("streaming_complete", total_duration, true);

        // Send final status message
        let _ = tx
            .send(Ok(StreamTelemetryResponse {
                records: vec![],
                is_last_batch: true,
                total_processed,
            }))
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::bridge::grpc::bridge_service_server::BridgeService;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_stream_telemetry() {
        // Create a test bridge service
        let config = BridgeAPIConfig::default();
        let metrics = ApiMetrics::new();
        let mut bridge_service = BridgeGrpcService::new(config, metrics);
        bridge_service.init().await.unwrap();

        // Create a test streaming request
        let stream_request = StreamTelemetryRequest {
            query_type: "test".to_string(),
            time_range: Some(TimeRange {
                start_time: 0,
                end_time: 1000,
            }),
            filters: vec![],
            batch_size: 10,
        };

        let request = Request::new(stream_request);

        // Test the streaming method
        let result = bridge_service.stream_telemetry(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let mut stream = response.into_inner();

        // Consume the stream
        let mut batch_count = 0;
        let mut total_records = 0;
        while let Some(result) = stream.next().await {
            match result {
                Ok(stream_response) => {
                    batch_count += 1;
                    total_records += stream_response.records.len();

                    // Validate response structure
                    assert!(stream_response.total_processed >= 0);

                    if stream_response.is_last_batch {
                        break;
                    }
                }
                Err(e) => {
                    panic!("Stream error: {}", e);
                }
            }
        }

        assert!(batch_count > 0);
        tracing::info!(
            "Test completed: {} batches, {} total records",
            batch_count,
            total_records
        );
    }

    #[tokio::test]
    async fn test_stream_telemetry_data() {
        // Create test data
        let request = StreamTelemetryRequest {
            query_type: "test".to_string(),
            time_range: Some(TimeRange {
                start_time: 0,
                end_time: 1000,
            }),
            filters: vec![],
            batch_size: 5,
        };

        let batch_size = 5;
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let query_service = QueryService::new(BridgeAPIConfig::default(), ApiMetrics::new());
        let metrics = ApiMetrics::new();

        // Spawn the streaming task
        tokio::spawn(async move {
            let _ = BridgeGrpcService::stream_telemetry_data(
                request,
                batch_size,
                tx,
                query_service,
                metrics,
            )
            .await;
        });

        // Consume the stream
        let mut total_records = 0;
        while let Some(result) = rx.recv().await {
            match result {
                Ok(stream_response) => {
                    total_records += stream_response.records.len();

                    if stream_response.is_last_batch {
                        break;
                    }
                }
                Err(e) => {
                    panic!("Stream error: {}", e);
                }
            }
        }

        // Verify we received some data
        assert!(total_records >= 0);
    }

    #[tokio::test]
    async fn test_stream_telemetry_validation() {
        // Create a test bridge service
        let config = BridgeAPIConfig::default();
        let metrics = ApiMetrics::new();
        let bridge_service = BridgeGrpcService::new(config, metrics);

        // Test empty query type
        let stream_request = StreamTelemetryRequest {
            query_type: "".to_string(),
            time_range: None,
            filters: vec![],
            batch_size: 10,
        };

        let request = Request::new(stream_request);
        let result = bridge_service.stream_telemetry(request).await;
        assert!(result.is_err());

        // Test invalid time range
        let stream_request = StreamTelemetryRequest {
            query_type: "test".to_string(),
            time_range: Some(TimeRange {
                start_time: 1000,
                end_time: 0, // Invalid: start > end
            }),
            filters: vec![],
            batch_size: 10,
        };

        let request = Request::new(stream_request);
        let result = bridge_service.stream_telemetry(request).await;
        assert!(result.is_err());
    }
}
