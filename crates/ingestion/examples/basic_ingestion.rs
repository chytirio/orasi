//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Basic ingestion example
//!
//! This example demonstrates how to use the OpenTelemetry ingestion crate
//! with OTLP Arrow, Kafka, and OTLP gRPC protocols.

use ingestion::{
    exporters::batch_exporter::BatchExporterConfig,
    exporters::streaming_exporter::StreamingExporterConfig,

    // Exporters
    exporters::BatchExporter,
    exporters::ExporterFactory,
    exporters::ExporterPipeline,
    exporters::StreamingExporter,
    processors::batch_processor::BatchProcessorConfig,
    processors::filter_processor::{FilterMode, FilterOperator, FilterProcessorConfig, FilterRule},
    processors::transform_processor::{TransformProcessorConfig, TransformRule, TransformRuleType},

    // Processors
    processors::BatchProcessor,
    processors::FilterProcessor,
    processors::ProcessorFactory,
    processors::ProcessorPipeline,
    processors::TransformProcessor,
    protocols::kafka::KafkaConfig,
    // Protocols
    protocols::otlp_arrow::OtlpArrowConfig,
    protocols::otlp_grpc::OtlpGrpcConfig,

    receivers::http_receiver::HttpReceiverConfig,

    receivers::kafka_receiver::KafkaReceiverConfig,
    receivers::otlp_receiver::OtlpReceiverConfig,
    // Receivers
    receivers::HttpReceiver,
    receivers::KafkaReceiver,
    receivers::OtlpReceiver,
    receivers::ReceiverFactory,
    receivers::ReceiverManager,
    // Validation
    validation::DataValidator,
    validation::SchemaValidator,
    validation::TelemetryValidator,
};

use bridge_core::{BridgeResult, TelemetryBatch};
use tracing::{error, info};

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting OpenTelemetry ingestion example");

    // Create receiver manager
    let mut receiver_manager = ReceiverManager::new();

    // Configure OTLP Arrow receiver
    let otlp_arrow_config = OtlpReceiverConfig::new_arrow("0.0.0.0".to_string(), 4318);
    let otlp_arrow_receiver = ReceiverFactory::create_receiver(&otlp_arrow_config).await?;
    receiver_manager.add_receiver("otlp-arrow".to_string(), otlp_arrow_receiver);

    // Configure OTLP gRPC receiver
    let otlp_grpc_config = OtlpReceiverConfig::new_grpc("0.0.0.0".to_string(), 4317);
    let otlp_grpc_receiver = ReceiverFactory::create_receiver(&otlp_grpc_config).await?;
    receiver_manager.add_receiver("otlp-grpc".to_string(), otlp_grpc_receiver);

    // Configure Kafka receiver
    let kafka_config = KafkaReceiverConfig::new(
        vec!["localhost:9092".to_string()],
        "telemetry-data".to_string(),
        "ingestion-group".to_string(),
    );
    let kafka_receiver = ReceiverFactory::create_receiver(&kafka_config).await?;
    receiver_manager.add_receiver("kafka".to_string(), kafka_receiver);

    // Start all receivers
    receiver_manager.start_all().await?;

    // Create processor pipeline
    let mut processor_pipeline = ProcessorPipeline::new();

    // Configure batch processor
    let batch_config = BatchProcessorConfig::new(1000, 5000);
    let batch_processor = ProcessorFactory::create_processor(&batch_config).await?;
    processor_pipeline.add_processor(batch_processor);

    // Configure filter processor
    let filter_rules = vec![FilterRule {
        name: "exclude_debug_logs".to_string(),
        field: "severity".to_string(),
        operator: FilterOperator::NotEquals,
        value: "debug".to_string(),
        enabled: true,
    }];
    let filter_config = FilterProcessorConfig::new(filter_rules, FilterMode::Include);
    let filter_processor = ProcessorFactory::create_processor(&filter_config).await?;
    processor_pipeline.add_processor(filter_processor);

    // Configure transform processor
    let transform_rules = vec![TransformRule {
        name: "add_timestamp".to_string(),
        rule_type: TransformRuleType::Add,
        source_field: "".to_string(),
        target_field: "ingestion_timestamp".to_string(),
        transform_value: Some("${timestamp}".to_string()),
        enabled: true,
    }];
    let transform_config = TransformProcessorConfig::new(transform_rules);
    let transform_processor = ProcessorFactory::create_processor(&transform_config).await?;
    processor_pipeline.add_processor(transform_processor);

    // Start processor pipeline
    processor_pipeline.start().await?;

    // Create exporter pipeline
    let mut exporter_pipeline = ExporterPipeline::new();

    // Configure batch exporter
    let batch_exporter_config =
        BatchExporterConfig::new("http://localhost:8080/export".to_string(), 1000);
    let batch_exporter = ExporterFactory::create_exporter(&batch_exporter_config).await?;
    exporter_pipeline.add_exporter(batch_exporter);

    // Configure streaming exporter
    let streaming_exporter_config =
        StreamingExporterConfig::new("http://localhost:8081/stream".to_string(), 100);
    let streaming_exporter = ExporterFactory::create_exporter(&streaming_exporter_config).await?;
    exporter_pipeline.add_exporter(streaming_exporter);

    // Start exporter pipeline
    exporter_pipeline.start().await?;

    // Create validators
    let schema_validator = SchemaValidator::new();
    let data_validator = DataValidator::new();

    info!("OpenTelemetry ingestion setup completed");

    // Main processing loop
    loop {
        // Receive data from all receivers
        for receiver_name in receiver_manager.get_receiver_names() {
            if let Some(receiver) = receiver_manager.get_receiver(&receiver_name) {
                match receiver.receive().await {
                    Ok(batch) => {
                        info!(
                            "Received batch from {}: {} records",
                            receiver_name, batch.size
                        );

                        // Validate data
                        let schema_result = schema_validator.validate(&batch).await?;
                        if !schema_result.is_valid {
                            error!(
                                "Schema validation failed for batch from {}: {:?}",
                                receiver_name, schema_result.errors
                            );
                            continue;
                        }

                        let data_result = data_validator.validate(&batch).await?;
                        if !data_result.is_valid {
                            error!(
                                "Data validation failed for batch from {}: {:?}",
                                receiver_name, data_result.errors
                            );
                            continue;
                        }

                        // Process data through pipeline (currently returns empty ProcessedBatch)
                        let processed_batch = processor_pipeline.process(batch.clone()).await?;

                        // For now, export the original batch since processor pipeline is not fully implemented
                        let export_results = exporter_pipeline.export(batch).await?;

                        info!(
                            "Exported batch from {}: {} results",
                            receiver_name,
                            export_results.len()
                        );
                    }
                    Err(e) => {
                        error!("Failed to receive data from {}: {}", receiver_name, e);
                    }
                }
            }
        }

        // Sleep for a short interval
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
