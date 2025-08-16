//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.com>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Lake House Connectors Test Runner
//!
//! This binary runs tests for all lake house connectors with proper configuration.
//! It replaces the shell script with a native Rust implementation.

use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "lakehouse-test-runner")]
#[command(about = "Lake House Connectors Test Runner")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Test mode
    #[arg(short, long, value_enum, default_value_t = TestMode::Unit)]
    mode: TestMode,

    /// Run tests in parallel
    #[arg(short, long)]
    parallel: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Skip tests requiring external dependencies
    #[arg(short, long)]
    skip_external: bool,

    /// Generate test coverage report
    #[arg(short, long)]
    coverage: bool,

    /// Connectors to test (default: all)
    #[arg(value_name = "CONNECTOR")]
    connectors: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available connectors
    List,

    /// Show test configuration
    Config,
}

#[derive(Clone, Copy, ValueEnum, Debug)]
enum TestMode {
    Unit,
    Integration,
    All,
}

impl std::fmt::Display for TestMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestMode::Unit => write!(f, "unit"),
            TestMode::Integration => write!(f, "integration"),
            TestMode::All => write!(f, "all"),
        }
    }
}

#[derive(Debug)]
struct TestConfig {
    mode: TestMode,
    parallel: bool,
    verbose: bool,
    skip_external: bool,
    coverage: bool,
    connectors: Vec<String>,
}

#[derive(Debug)]
struct TestResult {
    connector: String,
    mode: TestMode,
    success: bool,
    duration: std::time::Duration,
    output: String,
}

struct TestRunner {
    config: TestConfig,
    project_root: PathBuf,
    available_connectors: Vec<String>,
}

impl TestRunner {
    fn new(config: TestConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let project_root = env::current_dir()?;
        let available_connectors = Self::discover_connectors(&project_root)?;

        Ok(TestRunner {
            config,
            project_root,
            available_connectors,
        })
    }

    fn discover_connectors(project_root: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let connectors_dir = project_root.join("crates").join("connectors");
        let mut connectors = Vec::new();

        if connectors_dir.exists() {
            for entry in std::fs::read_dir(connectors_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    let cargo_toml = path.join("Cargo.toml");
                    if cargo_toml.exists() {
                        if let Some(name) = path.file_name() {
                            connectors.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(connectors)
    }

    fn validate_connectors(
        &self,
        connectors: &[String],
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let available_set: HashSet<_> = self.available_connectors.iter().collect();
        let mut valid_connectors = Vec::new();
        let mut invalid_connectors = Vec::new();

        for connector in connectors {
            if available_set.contains(connector) {
                valid_connectors.push(connector.clone());
            } else {
                invalid_connectors.push(connector.clone());
            }
        }

        if !invalid_connectors.is_empty() {
            eprintln!(
                "{} Invalid connectors: {}",
                "✗".red(),
                invalid_connectors.join(", ")
            );
            eprintln!(
                "{} Available connectors: {}",
                "ℹ".yellow(),
                self.available_connectors.join(", ")
            );
            return Err("Invalid connectors specified".into());
        }

        Ok(valid_connectors)
    }

    fn get_connectors_to_test(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if self.config.connectors.is_empty() {
            Ok(self.available_connectors.clone())
        } else {
            self.validate_connectors(&self.config.connectors)
        }
    }

    fn print_status(&self, message: &str) {
        println!("{} {}", "ℹ".blue(), message);
    }

    fn print_success(&self, message: &str) {
        println!("{} {}", "✓".green(), message);
    }

    fn print_error(&self, message: &str) {
        println!("{} {}", "✗".red(), message);
    }

    fn print_warning(&self, message: &str) {
        println!("{} {}", "⚠".yellow(), message);
    }

    fn run_connector_test(
        &self,
        connector: &str,
    ) -> Result<TestResult, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let connector_dir = self
            .project_root
            .join("crates")
            .join("connectors")
            .join(connector);

        if !connector_dir.exists() {
            return Err(
                format!("Connector directory not found: {}", connector_dir.display()).into(),
            );
        }

        // Build test arguments
        let mut args = vec!["test"];

        match self.config.mode {
            TestMode::Unit => args.push("--lib"),
            TestMode::Integration => {
                args.push("--test");
                args.push("integration");
            }
            TestMode::All => {}
        }

        if self.config.verbose {
            args.extend_from_slice(&["--", "--nocapture"]);
        }

        if self.config.parallel {
            args.extend_from_slice(&["--", "--test-threads=4"]);
        }

        // Set environment variables for external dependencies
        if self.config.skip_external {
            match connector {
                "kafka" => env::set_var("SKIP_KAFKA_TESTS", "1"),
                "hudi" => env::set_var("SKIP_HUDI_TESTS", "1"),
                "iceberg" => env::set_var("SKIP_ICEBERG_TESTS", "1"),
                "snowflake" => env::set_var("SKIP_SNOWFLAKE_TESTS", "1"),
                "s3-parquet" => env::set_var("SKIP_S3_TESTS", "1"),
                _ => {}
            }
        }

        // Run the test command
        let output = Command::new("cargo")
            .args(&args)
            .current_dir(&connector_dir)
            .output()?;

        let duration = start_time.elapsed();
        let success = output.status.success();
        let output_str = String::from_utf8_lossy(&output.stdout).to_string();

        Ok(TestResult {
            connector: connector.to_string(),
            mode: self.config.mode,
            success,
            duration,
            output: output_str,
        })
    }

    fn run_coverage_tests(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.print_status("Running coverage tests...");

        // Check if cargo-tarpaulin is installed
        let tarpaulin_check = Command::new("cargo")
            .args(&["tarpaulin", "--version"])
            .output();

        if tarpaulin_check.is_err() {
            self.print_warning("cargo-tarpaulin not found. Installing...");
            Command::new("cargo")
                .args(&["install", "cargo-tarpaulin"])
                .status()?;
        }

        let connectors = self.get_connectors_to_test()?;
        let coverage_dir = self.project_root.join("target").join("coverage");

        // Create coverage directory
        std::fs::create_dir_all(&coverage_dir)?;

        for connector in connectors {
            self.print_status(&format!("Running coverage for {}...", connector));

            let connector_dir = self
                .project_root
                .join("crates")
                .join("connectors")
                .join(&connector);

            let output_dir = coverage_dir.join(&connector);

            let result = Command::new("cargo")
                .args(&[
                    "tarpaulin",
                    "--out",
                    "Html",
                    "--output-dir",
                    output_dir.to_str().unwrap(),
                ])
                .current_dir(&connector_dir)
                .output()?;

            if result.status.success() {
                self.print_success(&format!("{} coverage generated", connector));
            } else {
                self.print_error(&format!("{} coverage failed", connector));
            }
        }

        self.print_success(&format!(
            "Coverage reports generated in {}",
            coverage_dir.display()
        ));
        Ok(())
    }

    fn run_tests(&self) -> Result<(), Box<dyn std::error::Error>> {
        let connectors = self.get_connectors_to_test()?;

        self.print_status("Test Configuration:");
        println!("  Mode: {}", self.config.mode);
        println!("  Connectors: {}", connectors.join(", "));
        println!("  Parallel: {}", self.config.parallel);
        println!("  Verbose: {}", self.config.verbose);
        println!("  Skip External: {}", self.config.skip_external);
        println!("  Coverage: {}", self.config.coverage);
        println!();

        if self.config.coverage {
            return self.run_coverage_tests();
        }

        let mut results = Vec::new();
        let progress_bar = if !self.config.verbose {
            let pb = ProgressBar::new(connectors.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            Some(pb)
        } else {
            None
        };

        for connector in &connectors {
            if let Some(pb) = &progress_bar {
                pb.set_message(format!("Testing {}", connector));
            }

            match self.run_connector_test(connector) {
                Ok(result) => {
                    results.push(result);
                    if let Some(pb) = &progress_bar {
                        pb.inc(1);
                    }
                }
                Err(e) => {
                    self.print_error(&format!("Failed to run tests for {}: {}", connector, e));
                    return Err(e);
                }
            }
        }

        if let Some(pb) = progress_bar {
            pb.finish_with_message("Tests completed");
        }

        // Print results
        let mut passed = Vec::new();
        let mut failed = Vec::new();

        for result in results {
            if result.success {
                passed.push(result.connector.clone());
                self.print_success(&format!(
                    "{} {} tests passed ({:.2}s)",
                    result.connector,
                    result.mode,
                    result.duration.as_secs_f64()
                ));
            } else {
                failed.push(result.connector.clone());
                self.print_error(&format!(
                    "{} {} tests failed ({:.2}s)",
                    result.connector,
                    result.mode,
                    result.duration.as_secs_f64()
                ));
            }
        }

        // Print summary
        println!();
        self.print_status("Test Summary:");
        println!("  Mode: {}", self.config.mode);
        println!("  Total: {}", connectors.len());
        println!("  Passed: {}", passed.len());
        println!("  Failed: {}", failed.len());

        if !passed.is_empty() {
            self.print_success(&format!("Passed: {}", passed.join(", ")));
        }

        if !failed.is_empty() {
            self.print_error(&format!("Failed: {}", failed.join(", ")));
            return Err("Some tests failed".into());
        }

        self.print_success("All tests passed! 🎉");
        Ok(())
    }

    fn list_connectors(&self) {
        println!("Available connectors:");
        for connector in &self.available_connectors {
            println!("  - {}", connector);
        }
    }

    fn show_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let connectors = self.get_connectors_to_test()?;

        println!("Test Configuration:");
        println!("  Mode: {}", self.config.mode);
        println!("  Connectors: {}", connectors.join(", "));
        println!("  Parallel: {}", self.config.parallel);
        println!("  Verbose: {}", self.config.verbose);
        println!("  Skip External: {}", self.config.skip_external);
        println!("  Coverage: {}", self.config.coverage);
        println!("  Project Root: {}", self.project_root.display());
        println!(
            "  Available Connectors: {}",
            self.available_connectors.join(", ")
        );

        Ok(())
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let config = TestConfig {
        mode: cli.mode,
        parallel: cli.parallel,
        verbose: cli.verbose,
        skip_external: cli.skip_external,
        coverage: cli.coverage,
        connectors: cli.connectors,
    };

    let runner = match TestRunner::new(config) {
        Ok(runner) => runner,
        Err(e) => {
            eprintln!("{} Failed to initialize test runner: {}", "✗".red(), e);
            return ExitCode::FAILURE;
        }
    };

    match cli.command {
        Some(Commands::List) => {
            runner.list_connectors();
            ExitCode::SUCCESS
        }
        Some(Commands::Config) => match runner.show_config() {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{} Failed to show config: {}", "✗".red(), e);
                ExitCode::FAILURE
            }
        },
        None => match runner.run_tests() {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{} Test run failed: {}", "✗".red(), e);
                ExitCode::FAILURE
            }
        },
    }
}
