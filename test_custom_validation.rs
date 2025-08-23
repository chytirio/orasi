use bridge_core::types::{TelemetryRecord, TelemetryData, TelemetryType, MetricData, MetricType, MetricValue};
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use serde_json;

// Simple test to verify JSONPath custom validation works
fn main() {
    // Create a test record
    let record = TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(MetricData {
            name: "http_requests_total".to_string(),
            description: Some("Total HTTP requests".to_string()),
            unit: Some("count".to_string()),
            metric_type: MetricType::Counter,
            value: MetricValue::Counter(100.0),
            labels: HashMap::new(),
            timestamp: Utc::now(),
        }),
        attributes: {
            let mut attrs = HashMap::new();
            attrs.insert("service_name".to_string(), "api-gateway".to_string());
            attrs.insert("environment".to_string(), "production".to_string());
            attrs.insert("user_email".to_string(), "test@example.com".to_string());
            attrs.insert("response_time".to_string(), "150.5".to_string());
            attrs
        },
        tags: {
            let mut tags = HashMap::new();
            tags.insert("version".to_string(), "1.0.0".to_string());
            tags.insert("region".to_string(), "us-west-2".to_string());
            tags
        },
        resource: None,
        service: None,
    };

    // Convert to JSON
    let json_value = serde_json::to_value(&record).unwrap();
    println!("JSON: {}", serde_json::to_string_pretty(&json_value).unwrap());

    // Test JSONPath expressions
    let test_expressions = vec![
        "$.attributes.service_name",
        "$.tags.version",
        "$.attributes.nonexistent",
        "$.attributes.service_name == \"api-gateway\"",
        "$.attributes.response_time > 100",
        "$.attributes.user_email is_email()",
    ];

    for expr in test_expressions {
        match jsonpath_lib::select(&json_value, expr) {
            Ok(results) => {
                println!("Expression '{}': {:?}", expr, results);
            }
            Err(e) => {
                println!("Expression '{}': Error - {}", expr, e);
            }
        }
    }

    println!("Custom validation test completed!");
}
