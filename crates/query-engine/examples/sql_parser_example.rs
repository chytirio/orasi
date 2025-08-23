//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! SQL Parser Example
//!
//! This example demonstrates how to use the SQL parser to parse and validate SQL queries.

use query_engine::parsers::{
    sql_parser::SqlParserConfig, ParserConfig, QueryParserTrait, SqlParser,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
    .unwrap();

    info!("Starting SQL Parser Example");

    // Create SQL parser configuration
    let config = SqlParserConfig {
        name: "example_sql_parser".to_string(),
        version: "1.0.0".to_string(),
        dialect: query_engine::parsers::sql_parser::SqlDialect::Standard,
        strict_mode: true,
        case_sensitive: false,
        additional_config: std::collections::HashMap::new(),
    };

    // Create SQL parser
    let mut parser = SqlParser::new(&config).await?;
    parser.init().await?;

    // Test queries
    let test_queries = vec![
        "SELECT * FROM users WHERE age > 18",
        "INSERT INTO users (name, age) VALUES ('John', 25)",
        "UPDATE users SET age = 26 WHERE name = 'John'",
        "DELETE FROM users WHERE age < 18",
        "CREATE TABLE users (id INT, name VARCHAR(255), age INT)",
        "DROP TABLE users",
        "SELECT COUNT(*) as count, department FROM employees GROUP BY department HAVING count > 5",
    ];

    for (i, query) in test_queries.iter().enumerate() {
        info!("Testing query {}: {}", i + 1, query);

        // Parse the query
        match parser.parse(query).await {
            Ok(parsed_query) => {
                info!("✅ Query parsed successfully");
                info!("  - Query ID: {}", parsed_query.id);
                info!("  - AST Node Count: {}", parsed_query.ast.node_count);
                info!("  - AST Depth: {}", parsed_query.ast.depth);
                info!(
                    "  - Query Type: {:?}",
                    parsed_query.metadata.get("query_type")
                );

                // Validate the parsed query
                match parser.validate(&parsed_query).await {
                    Ok(validation_result) => {
                        if validation_result.is_valid {
                            info!("✅ Query validation passed");
                        } else {
                            info!("❌ Query validation failed");
                            for error in &validation_result.errors {
                                info!("  - Error: {} ({})", error.message, error.code);
                            }
                        }

                        for warning in &validation_result.warnings {
                            info!("  - Warning: {} ({})", warning.message, warning.code);
                        }
                    }
                    Err(e) => {
                        info!("❌ Validation error: {}", e);
                    }
                }
            }
            Err(e) => {
                info!("❌ Parsing error: {}", e);
            }
        }

        info!("---");
    }

    // Get parser statistics
    match parser.get_stats().await {
        Ok(stats) => {
            info!("Parser Statistics:");
            info!("  - Total Queries: {}", stats.total_queries);
            info!("  - Successful Parses: {}", stats.successful_parses);
            info!("  - Failed Parses: {}", stats.failed_parses);
            info!("  - Average Parse Time: {:.2}ms", stats.avg_parse_time_ms);
        }
        Err(e) => {
            info!("❌ Failed to get statistics: {}", e);
        }
    }

    info!("SQL Parser Example completed");
    Ok(())
}
