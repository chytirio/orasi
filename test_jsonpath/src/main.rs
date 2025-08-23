use serde_json::{json, Value};
use jsonpath_lib::select;

fn main() {
    // Create a test JSON object similar to a TelemetryRecord
    let test_data = json!({
        "id": "123e4567-e89b-12d3-a456-426614174000",
        "timestamp": "2024-01-01T12:00:00Z",
        "record_type": "Metric",
        "attributes": {
            "service_name": "api-gateway",
            "environment": "production",
            "user_email": "test@example.com",
            "request_url": "https://api.example.com/v1/users",
            "response_time": "150.5"
        },
        "tags": {
            "version": "1.0.0",
            "region": "us-west-2"
        },
        "data": {
            "name": "http_requests_total",
            "value": 100.0,
            "unit": "count"
        }
    });

    println!("Test JSON:");
    println!("{}", serde_json::to_string_pretty(&test_data).unwrap());
    println!();

    // Test various JSONPath expressions
    let test_expressions = vec![
        "$.attributes.service_name",
        "$.tags.version",
        "$.attributes.nonexistent",
        "$.data.name",
        "$.attributes.response_time",
        "$.attributes.user_email",
    ];

    for expr in test_expressions {
        match select(&test_data, expr) {
            Ok(results) => {
                println!("Expression '{}': {:?}", expr, results);
            }
            Err(e) => {
                println!("Expression '{}': Error - {}", expr, e);
            }
        }
    }

    println!("\nJSONPath custom validation test completed successfully!");
    println!("This demonstrates that JSONPath can be used for custom validation in the enrichment processor.");
}
