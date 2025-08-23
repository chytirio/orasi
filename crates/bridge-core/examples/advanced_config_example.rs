//! Advanced Configuration Management Example
//!
//! This example demonstrates the advanced configuration management capabilities including:
//! - Dynamic configuration updates
//! - Hot reloading
//! - Configuration validation
//! - Backup and restore
//! - Versioning
//! - Environment variable substitution

use bridge_core::{
    config::{
        advanced::{
            AdvancedConfigManager, ConfigBackupSettings, ConfigChangeEvent, ConfigChangeType,
            ConfigValidationRule, ConfigVersioningSettings,
        },
        bridge::BridgeConfig,
    },
    BridgeResult,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> BridgeResult<()> {
    println!("🚀 Advanced Configuration Management Example");
    println!("============================================\n");

    // Create temporary directory for config files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("bridge_config.json");

    // Create advanced configuration manager
    let mut config_manager = AdvancedConfigManager::new(config_path.clone())?;

    // Initialize the configuration manager
    config_manager.init().await?;
    println!("✅ Configuration manager initialized\n");

    // Run demonstrations
    println!("📝 Running Initial Configuration Demo...");
    run_initial_config_demo(&config_manager).await?;

    println!("\n🔍 Running Configuration Validation Demo...");
    run_config_validation_demo(&config_manager).await?;

    println!("\n⚡ Running Dynamic Updates Demo...");
    run_dynamic_updates_demo(&config_manager).await?;

    println!("\n💾 Running Backup and Restore Demo...");
    run_backup_restore_demo(&config_manager).await?;

    println!("\n📚 Running Versioning Demo...");
    run_versioning_demo(&config_manager).await?;

    println!("\n🔄 Running Hot Reloading Demo...");
    run_hot_reloading_demo(&config_manager, &config_path).await?;

    println!("\n🌍 Running Environment Substitution Demo...");
    run_env_substitution_demo(&config_manager).await?;

    println!("\n✅ Advanced Configuration Management Example completed successfully!");
    println!("Configuration file location: {}", config_path.display());

    Ok(())
}

async fn run_initial_config_demo(config_manager: &AdvancedConfigManager) -> BridgeResult<()> {
    println!("  Creating initial configuration...");

    // Create initial configuration
    let initial_config = serde_json::json!({
        "name": "orasi",
        "version": "0.1.0",
        "environment": "development"
    });

    // Save initial configuration
    config_manager.save_configuration(initial_config).await?;
    println!("  ✅ Initial configuration saved");

    // Load and display configuration
    let loaded_config = config_manager.get_configuration().await;
    println!("  📋 Configuration loaded successfully");
    println!("    Name: {}", loaded_config["name"]);
    println!("    Environment: {}", loaded_config["environment"]);

    Ok(())
}

async fn run_config_validation_demo(config_manager: &AdvancedConfigManager) -> BridgeResult<()> {
    println!("  Setting up custom validation rules...");

    // Add custom validation rule
    let validation_rule = ConfigValidationRule {
        name: "environment_validation".to_string(),
        description: "Environment must be one of: development, staging, production".to_string(),
        validator: Box::new(|value| {
            if let Some(env) = value.get("environment") {
                if let Some(env_str) = env.as_str() {
                    if !["development", "staging", "production"].contains(&env_str) {
                        return Err(bridge_core::BridgeError::configuration(
                            "Environment must be one of: development, staging, production",
                        ));
                    }
                }
            }
            Ok(())
        }),
    };

    config_manager.add_validation_rule(validation_rule).await;
    println!("  ✅ Custom validation rule added");

    // Test validation
    let validation_result = config_manager
        .validate_configuration(&config_manager.get_configuration().await)
        .await;
    println!("  🔍 Configuration validation result:");
    println!("    Valid: {}", validation_result.is_valid);
    if !validation_result.errors.is_empty() {
        println!("    Errors: {}", validation_result.errors.len());
        for error in &validation_result.errors {
            println!("      - {}", error.message);
        }
    }

    Ok(())
}

async fn run_dynamic_updates_demo(config_manager: &AdvancedConfigManager) -> BridgeResult<()> {
    println!("  Performing dynamic configuration updates...");

    // Update environment configuration
    config_manager
        .update_configuration_section(
            "environment",
            serde_json::json!("staging"),
            Some("admin".to_string()),
        )
        .await?;
    println!("  ✅ Environment configuration updated");

    // Update name configuration
    config_manager
        .update_configuration_section(
            "name",
            serde_json::json!("orasi-staging"),
            Some("admin".to_string()),
        )
        .await?;
    println!("  ✅ Name configuration updated");

    // Update version configuration
    config_manager
        .update_configuration_section(
            "version",
            serde_json::json!("0.2.0"),
            Some("admin".to_string()),
        )
        .await?;
    println!("  ✅ Version configuration updated");

    // Display updated configuration
    let updated_config = config_manager.get_configuration().await;
    println!("  📋 Updated configuration:");
    println!("    Name: {}", updated_config["name"]);
    println!("    Environment: {}", updated_config["environment"]);
    println!("    Version: {}", updated_config["version"]);

    Ok(())
}

async fn run_backup_restore_demo(config_manager: &AdvancedConfigManager) -> BridgeResult<()> {
    println!("  Testing backup and restore functionality...");

    // Create backup
    config_manager.create_backup().await?;
    println!("  💾 Backup created");

    // List backups
    let backups = config_manager.list_backups().await?;
    println!("  📋 Available backups: {}", backups.len());
    for backup in &backups {
        println!("    - {}", backup);
    }

    // Restore from backup
    if let Some(latest_backup) = backups.first() {
        config_manager.restore_from_backup(latest_backup).await?;
        println!("  🔄 Configuration restored from backup: {}", latest_backup);
    }

    Ok(())
}

async fn run_versioning_demo(config_manager: &AdvancedConfigManager) -> BridgeResult<()> {
    println!("  Testing configuration versioning...");

    // Create version
    config_manager.create_version().await?;
    println!("  📚 Version created");

    // List versions
    let versions = config_manager.list_versions().await?;
    println!("  📋 Available versions: {}", versions.len());
    for version in &versions {
        println!("    - {}", version);
    }

    Ok(())
}

async fn run_hot_reloading_demo(
    config_manager: &AdvancedConfigManager,
    config_path: &PathBuf,
) -> BridgeResult<()> {
    println!("  Testing hot reloading functionality...");

    // Simulate external file modification
    println!("  📝 Simulating external configuration file modification...");
    let modified_config = serde_json::json!({
        "name": "orasi-hot-reload",
        "version": "0.3.0",
        "environment": "production"
    });

    std::fs::write(config_path, serde_json::to_string_pretty(&modified_config)?)?;
    println!("  ✅ Configuration file modified externally");

    // Wait for hot reload
    sleep(Duration::from_millis(100)).await;
    println!("  🔄 Hot reload completed");

    // Display reloaded configuration
    let reloaded_config = config_manager.get_configuration().await;
    println!("  📋 Reloaded configuration:");
    println!("    Name: {}", reloaded_config["name"]);
    println!("    Environment: {}", reloaded_config["environment"]);
    println!("    Version: {}", reloaded_config["version"]);

    Ok(())
}

async fn run_env_substitution_demo(config_manager: &AdvancedConfigManager) -> BridgeResult<()> {
    println!("  Testing environment variable substitution...");

    // Set environment variables
    std::env::set_var("BRIDGE_NAME", "orasi-env");
    std::env::set_var("BRIDGE_ENVIRONMENT", "production");
    std::env::set_var("BRIDGE_VERSION", "0.4.0");

    // Create configuration with environment variable placeholders
    let env_config = serde_json::json!({
        "name": "${BRIDGE_NAME}",
        "environment": "${BRIDGE_ENVIRONMENT}",
        "version": "${BRIDGE_VERSION}"
    });

    // Save configuration with environment variables
    config_manager.save_configuration(env_config).await?;
    println!("  ✅ Configuration with environment variables saved");

    // Load configuration (should substitute environment variables)
    config_manager.load_configuration().await?;
    println!("  ✅ Configuration loaded with environment variable substitution");

    // Display final configuration
    let final_config = config_manager.get_configuration().await;
    println!("  📋 Final configuration with environment substitutions:");
    println!("    Name: {}", final_config["name"]);
    println!("    Environment: {}", final_config["environment"]);
    println!("    Version: {}", final_config["version"]);

    Ok(())
}
