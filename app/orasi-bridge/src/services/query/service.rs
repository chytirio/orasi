//! Query service for integrating gRPC telemetry queries with the query engine

use std::time::Instant;
use tracing::info;

use bridge_core::{
    traits::{QueryResultStatus, TelemetryQueryResult},
    types::TelemetryQuery,
    BridgeResult,
};

use crate::{
    config::BridgeAPIConfig,
    metrics::ApiMetrics,
    proto::{
        QueryTelemetryRequest, QueryTelemetryResponse, StreamTelemetryRequest,
        StreamTelemetryResponse,
    },
};

use super::{conversion, engine::QueryEngineIntegration, execution};

/// Query service for handling telemetry queries
#[derive(Clone)]
pub struct QueryService {
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
    query_engine: Option<QueryEngineIntegration>,
}

impl QueryService {
    /// Create a new query service
    pub fn new(config: BridgeAPIConfig, metrics: ApiMetrics) -> Self {
        Self {
            config,
            metrics,
            query_engine: None,
        }
    }

    /// Initialize the query engine integration
    pub async fn init_query_engine(&mut self) -> BridgeResult<()> {
        tracing::info!("Initializing query engine integration");

        let query_engine =
            QueryEngineIntegration::new(self.config.clone(), self.metrics.clone()).await?;
        self.query_engine = Some(query_engine);

        tracing::info!("Query engine integration initialized successfully");
        Ok(())
    }

    /// Query telemetry data
    pub async fn query_telemetry(
        &self,
        query_request: &QueryTelemetryRequest,
    ) -> BridgeResult<QueryTelemetryResponse> {
        let start_time = Instant::now();

        tracing::info!(
            "Processing telemetry query: type={}",
            query_request.query_type
        );

        // Convert gRPC request to bridge-core query
        let telemetry_query = conversion::convert_grpc_to_telemetry_query(query_request)?;

        // Execute the query
        let query_result = self.execute_telemetry_query(&telemetry_query).await?;

        // Convert result back to gRPC response
        let response =
            conversion::convert_telemetry_result_to_grpc(query_result, start_time.elapsed())?;

        // Record metrics
        self.metrics
            .record_processing("telemetry_query", start_time.elapsed(), true);

        tracing::info!("Telemetry query completed successfully");

        Ok(response)
    }

    /// Stream telemetry data
    pub async fn stream_telemetry(
        &self,
        stream_request: &StreamTelemetryRequest,
    ) -> BridgeResult<Vec<StreamTelemetryResponse>> {
        let start_time = Instant::now();

        tracing::info!(
            "Processing telemetry stream: type={}",
            stream_request.query_type
        );

        // Convert gRPC request to bridge-core query
        let telemetry_query =
            conversion::convert_grpc_to_telemetry_query(&QueryTelemetryRequest {
                query_type: stream_request.query_type.clone(),
                time_range: stream_request.time_range.clone(),
                filters: stream_request.filters.clone(),
                limit: stream_request.batch_size,
                offset: 0,
                sort_by: String::new(),
                sort_order: String::new(),
            })?;

        // Execute the query with streaming
        let query_results = execution::execute_telemetry_stream(
            &telemetry_query,
            stream_request.batch_size,
            &self.config,
        )
        .await?;

        // Convert results to gRPC streaming responses
        let responses = conversion::convert_stream_results_to_grpc(query_results)?;

        // Record metrics
        self.metrics
            .record_processing("telemetry_stream", start_time.elapsed(), true);

        tracing::info!("Telemetry stream completed successfully");

        Ok(responses)
    }

    /// Execute telemetry query using the query engine
    async fn execute_telemetry_query(
        &self,
        query: &TelemetryQuery,
    ) -> BridgeResult<TelemetryQueryResult> {
        // Use real query engine if available
        if let Some(query_engine) = &self.query_engine {
            tracing::info!("Using real query engine for query: {:?}", query.id);
            return query_engine.execute_query(query).await;
        }

        // Fallback to mock data if query engine not initialized
        tracing::warn!(
            "Query engine not initialized, using mock data for query: {:?}",
            query.id
        );

        let query_type = execution::determine_query_type(query)?;

        match query_type.as_str() {
            "traces" => execution::execute_traces_query(query).await,
            "metrics" => execution::execute_metrics_query(query).await,
            "logs" => execution::execute_logs_query(query).await,
            "events" => execution::execute_events_query(query).await,
            _ => execution::execute_generic_query(query).await,
        }
    }
}
