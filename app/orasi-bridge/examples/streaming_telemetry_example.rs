use bridge_api::proto::bridge::grpc::{
    bridge_service_client::BridgeServiceClient, Filter, StreamTelemetryRequest, TimeRange,
};
use tokio_stream::StreamExt;
use tonic::{Request, Response};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Connect to the bridge service
    let mut client = BridgeServiceClient::connect("http://[::1]:50051").await?;

    // Create a streaming telemetry request
    let request = StreamTelemetryRequest {
        query_type: "traces".to_string(),
        time_range: Some(TimeRange {
            start_time: chrono::Utc::now().timestamp() - 3600, // 1 hour ago
            end_time: chrono::Utc::now().timestamp(),
        }),
        filters: vec![Filter {
            field: "service.name".to_string(),
            operator: "eq".to_string(),
            value: "example-service".to_string(),
        }],
        batch_size: 100,
    };

    println!("Starting streaming telemetry request...");
    println!("Query type: {}", request.query_type);
    println!(
        "Time range: {} to {}",
        chrono::DateTime::from_timestamp(request.time_range.as_ref().unwrap().start_time, 0)
            .unwrap(),
        chrono::DateTime::from_timestamp(request.time_range.as_ref().unwrap().end_time, 0).unwrap()
    );
    println!("Batch size: {}", request.batch_size);

    // Make the streaming request
    let request = Request::new(request);
    let mut stream = client.stream_telemetry(request).await?.into_inner();

    // Process the streaming response
    let mut total_records = 0;
    let mut batch_count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                batch_count += 1;
                let records_in_batch = response.records.len();
                total_records += records_in_batch;

                println!(
                    "Received batch {}: {} records (total: {})",
                    batch_count, records_in_batch, total_records
                );

                // Process each record in the batch
                for record in &response.records {
                    println!(
                        "  Record: ID={}, Type={}, Timestamp={}",
                        record.id,
                        record.r#type,
                        chrono::DateTime::from_timestamp(record.timestamp, 0).unwrap()
                    );
                }

                if response.is_last_batch {
                    println!(
                        "Stream completed. Total batches: {}, Total records: {}",
                        batch_count, total_records
                    );
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error receiving stream: {}", e);
                break;
            }
        }
    }

    println!("Streaming telemetry example completed successfully!");
    Ok(())
}
