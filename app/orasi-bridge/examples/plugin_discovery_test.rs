use std::path::PathBuf;
use std::collections::HashMap;
use crate::handlers::plugins::PluginManager;
use crate::types::{PluginInfo, PluginStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Testing Plugin Discovery System");
    println!("================================");

    // Create plugin manager with test plugins directory
    let plugins_dir = PathBuf::from("./plugins");
    let plugin_manager = PluginManager::new(plugins_dir);

    // Discover plugins
    println!("Discovering plugins...");
    match plugin_manager.discover_plugins().await {
        Ok(()) => println!("✓ Plugin discovery completed successfully"),
        Err(e) => {
            println!("✗ Plugin discovery failed: {}", e);
            return Err(e);
        }
    }

    // Get all plugins
    let plugins = plugin_manager.get_plugins().await;
    println!("\nDiscovered {} plugins:", plugins.len());
    
    for plugin in &plugins {
        println!("  - {} (v{}) - {:?}", plugin.name, plugin.version, plugin.status);
        println!("    Description: {}", plugin.description);
        println!("    Capabilities: {:?}", plugin.capabilities);
        println!();
    }

    // Get capabilities
    let capabilities = plugin_manager.get_capabilities().await;
    println!("Available capabilities:");
    for (capability, plugin_names) in &capabilities {
        println!("  - {}: {:?}", capability, plugin_names);
    }

    // Test specific plugin lookup
    println!("\nTesting plugin lookups:");
    
    let test_plugins = vec!["analytics", "query-engine", "streaming-processor", "example-plugin"];
    
    for plugin_name in test_plugins {
        match plugin_manager.get_plugin(plugin_name).await {
            Some(plugin) => {
                println!("  ✓ Found plugin '{}': {:?}", plugin_name, plugin.status);
            }
            None => {
                println!("  ✗ Plugin '{}' not found", plugin_name);
            }
        }
    }

    // Test plugin query execution
    println!("\nTesting plugin query execution:");
    
    let test_query = serde_json::json!({
        "query": "SELECT * FROM telemetry_data",
        "filters": {
            "timestamp": {
                "start": "2024-01-01T00:00:00Z",
                "end": "2024-01-02T00:00:00Z"
            }
        }
    });

    let test_request = crate::types::PluginQueryRequest {
        plugin_name: "analytics".to_string(),
        query_data: test_query,
        options: None,
    };

    match crate::handlers::plugins::execute_plugin_query(&test_request).await {
        Ok(result) => {
            println!("  ✓ Query execution successful");
            println!("  Result: {}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            println!("  ✗ Query execution failed: {}", e);
        }
    }

    println!("\nPlugin discovery test completed successfully!");
    Ok(())
}
