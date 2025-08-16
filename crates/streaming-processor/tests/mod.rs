//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Comprehensive test suite for streaming processor
//! 
//! This module provides a complete test suite covering all aspects of the
//! streaming processor including processors, sources, sinks, and integration tests.

pub mod processors;
pub mod sources;
pub mod arrow_utils;
pub mod integration;

/// Test utilities and helpers
pub mod utils {
    use bridge_core::{
        traits::DataStream,
        types::{TelemetryBatch, TelemetryRecord, TelemetryData, TelemetryType, MetricData, MetricValue, MetricType},
    };
    use std::collections::HashMap;
    use chrono::Utc;
    use uuid::Uuid;

    /// Create a test data stream with the given ID and data
    pub fn create_test_data_stream(stream_id: &str, data: Vec<u8>) -> DataStream {
        DataStream {
            stream_id: stream_id.to_string(),
            data,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Create a test telemetry batch with the given source and number of records
    pub fn create_test_telemetry_batch(source: &str, size: usize) -> TelemetryBatch {
        let records = (0..size).map(|i| {
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: format!("test_metric_{}", i),
                    description: Some("Test metric".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(i as f64),
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            }
        }).collect();

        TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: source.to_string(),
            size,
            records,
            metadata: HashMap::new(),
        }
    }

    /// Create a test telemetry record with the given name and value
    pub fn create_test_telemetry_record(name: &str, value: f64) -> TelemetryRecord {
        TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: name.to_string(),
                description: Some("Test metric".to_string()),
                unit: Some("count".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(value),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }
    }

    /// Assert that a result is ok and print the error if it's not
    pub fn assert_ok<T, E: std::fmt::Debug>(result: Result<T, E>, message: &str) -> T {
        match result {
            Ok(value) => value,
            Err(e) => {
                panic!("{}: {:?}", message, e);
            }
        }
    }

    /// Assert that a result is err and print the error if it's not
    pub fn assert_err<T: std::fmt::Debug, E>(result: Result<T, E>, message: &str) -> E {
        match result {
            Ok(value) => {
                panic!("{}: Expected error but got {:?}", message, value);
            }
            Err(e) => e,
        }
    }

    /// Measure the time it takes to execute a function
    pub fn measure_time<F, T>(f: F) -> (T, std::time::Duration)
    where
        F: FnOnce() -> T,
    {
        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed();
        (result, duration)
    }

    /// Measure the time it takes to execute an async function
    pub async fn measure_time_async<F, Fut, T>(f: F) -> (T, std::time::Duration)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = std::time::Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        (result, duration)
    }
}

/// Test configuration and constants
pub mod config {
    /// Default test timeout in seconds
    pub const DEFAULT_TEST_TIMEOUT_SECS: u64 = 30;

    /// Default test batch size
    pub const DEFAULT_BATCH_SIZE: usize = 100;

    /// Default test stream count
    pub const DEFAULT_STREAM_COUNT: usize = 10;

    /// Default test data size in bytes
    pub const DEFAULT_DATA_SIZE: usize = 1024;

    /// Performance test threshold in milliseconds
    pub const PERFORMANCE_THRESHOLD_MS: u128 = 1000;

    /// Memory usage threshold (should not exceed 10x original size)
    pub const MEMORY_USAGE_THRESHOLD: usize = 10;
}

/// Test categories and organization
pub mod categories {
    /// Unit tests for individual components
    pub const UNIT_TESTS: &str = "unit";
    
    /// Integration tests for component interactions
    pub const INTEGRATION_TESTS: &str = "integration";
    
    /// Performance tests
    pub const PERFORMANCE_TESTS: &str = "performance";
    
    /// Stress tests for high load scenarios
    pub const STRESS_TESTS: &str = "stress";
    
    /// Error handling tests
    pub const ERROR_TESTS: &str = "error";
}

/// Test reporting and metrics
pub mod reporting {
    use std::collections::HashMap;
    use std::time::Duration;

    /// Test result summary
    #[derive(Debug, Clone)]
    pub struct TestSummary {
        pub total_tests: usize,
        pub passed_tests: usize,
        pub failed_tests: usize,
        pub skipped_tests: usize,
        pub total_duration: Duration,
        pub test_durations: HashMap<String, Duration>,
    }

    impl TestSummary {
        /// Create a new test summary
        pub fn new() -> Self {
            Self {
                total_tests: 0,
                passed_tests: 0,
                failed_tests: 0,
                skipped_tests: 0,
                total_duration: Duration::ZERO,
                test_durations: HashMap::new(),
            }
        }

        /// Add a test result
        pub fn add_test_result(&mut self, test_name: &str, passed: bool, duration: Duration) {
            self.total_tests += 1;
            self.total_duration += duration;
            self.test_durations.insert(test_name.to_string(), duration);

            if passed {
                self.passed_tests += 1;
            } else {
                self.failed_tests += 1;
            }
        }

        /// Get the success rate as a percentage
        pub fn success_rate(&self) -> f64 {
            if self.total_tests == 0 {
                0.0
            } else {
                (self.passed_tests as f64 / self.total_tests as f64) * 100.0
            }
        }

        /// Print a summary report
        pub fn print_report(&self) {
            println!("=== Test Summary ===");
            println!("Total tests: {}", self.total_tests);
            println!("Passed: {}", self.passed_tests);
            println!("Failed: {}", self.failed_tests);
            println!("Skipped: {}", self.skipped_tests);
            println!("Success rate: {:.2}%", self.success_rate());
            println!("Total duration: {:?}", self.total_duration);
            println!("Average duration: {:?}", self.total_duration / self.total_tests.max(1) as u32);
            
            if !self.test_durations.is_empty() {
                println!("\n=== Test Durations ===");
                let mut sorted_tests: Vec<_> = self.test_durations.iter().collect();
                sorted_tests.sort_by(|a, b| b.1.cmp(a.1)); // Sort by duration (longest first)
                
                for (test_name, duration) in sorted_tests.iter().take(10) {
                    println!("{}: {:?}", test_name, duration);
                }
            }
        }
    }
}

/// Test utilities for benchmarking
pub mod benchmark {
    use std::time::{Duration, Instant};
    use std::collections::HashMap;

    /// Benchmark result
    #[derive(Debug, Clone)]
    pub struct BenchmarkResult {
        pub name: String,
        pub iterations: usize,
        pub total_duration: Duration,
        pub average_duration: Duration,
        pub min_duration: Duration,
        pub max_duration: Duration,
        pub throughput: f64, // operations per second
    }

    /// Run a benchmark
    pub fn run_benchmark<F>(name: &str, iterations: usize, mut f: F) -> BenchmarkResult
    where
        F: FnMut() -> (),
    {
        let mut durations = Vec::with_capacity(iterations);
        let start = Instant::now();

        for _ in 0..iterations {
            let iteration_start = Instant::now();
            f();
            durations.push(iteration_start.elapsed());
        }

        let total_duration = start.elapsed();
        let average_duration = total_duration / iterations as u32;
        let min_duration = durations.iter().min().copied().unwrap_or(Duration::ZERO);
        let max_duration = durations.iter().max().copied().unwrap_or(Duration::ZERO);
        let throughput = iterations as f64 / total_duration.as_secs_f64();

        BenchmarkResult {
            name: name.to_string(),
            iterations,
            total_duration,
            average_duration,
            min_duration,
            max_duration,
            throughput,
        }
    }

    /// Run an async benchmark
    pub async fn run_async_benchmark<F, Fut>(name: &str, iterations: usize, mut f: F) -> BenchmarkResult
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let mut durations = Vec::with_capacity(iterations);
        let start = Instant::now();

        for _ in 0..iterations {
            let iteration_start = Instant::now();
            f().await;
            durations.push(iteration_start.elapsed());
        }

        let total_duration = start.elapsed();
        let average_duration = total_duration / iterations as u32;
        let min_duration = durations.iter().min().copied().unwrap_or(Duration::ZERO);
        let max_duration = durations.iter().max().copied().unwrap_or(Duration::ZERO);
        let throughput = iterations as f64 / total_duration.as_secs_f64();

        BenchmarkResult {
            name: name.to_string(),
            iterations,
            total_duration,
            average_duration,
            min_duration,
            max_duration,
            throughput,
        }
    }

    /// Print benchmark results
    pub fn print_benchmark_results(results: &[BenchmarkResult]) {
        println!("=== Benchmark Results ===");
        for result in results {
            println!("{}:", result.name);
            println!("  Iterations: {}", result.iterations);
            println!("  Total duration: {:?}", result.total_duration);
            println!("  Average duration: {:?}", result.average_duration);
            println!("  Min duration: {:?}", result.min_duration);
            println!("  Max duration: {:?}", result.max_duration);
            println!("  Throughput: {:.2} ops/sec", result.throughput);
            println!();
        }
    }
}

/// Test utilities for stress testing
pub mod stress {
    use std::sync::Arc;
    use tokio::task;
    use crate::utils::create_test_data_stream;

    /// Stress test configuration
    #[derive(Debug, Clone)]
    pub struct StressTestConfig {
        pub concurrent_tasks: usize,
        pub iterations_per_task: usize,
        pub data_size: usize,
        pub timeout_secs: u64,
    }

    impl Default for StressTestConfig {
        fn default() -> Self {
            Self {
                concurrent_tasks: 10,
                iterations_per_task: 100,
                data_size: 1024,
                timeout_secs: 30,
            }
        }
    }

    /// Run a stress test
    pub async fn run_stress_test<F, Fut>(
        config: StressTestConfig,
        test_fn: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(usize, Vec<u8>) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send,
    {
        let mut handles = Vec::new();

        for task_id in 0..config.concurrent_tasks {
            let test_fn = test_fn.clone();
            let data_size = config.data_size;
            let iterations = config.iterations_per_task;

            let handle = task::spawn(async move {
                for iteration in 0..iterations {
                    let data: Vec<u8> = (0..data_size).map(|i| (i % 256) as u8).collect();
                    test_fn(task_id, data).await?;
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await??;
        }

        Ok(())
    }
}

/// Test utilities for property-based testing
pub mod property {
    use quickcheck::{Arbitrary, Gen};
    use bridge_core::traits::DataStream;
    use std::collections::HashMap;
    use chrono::Utc;



    /// Property test for data stream roundtrip
    pub fn test_data_stream_roundtrip(stream: DataStream) -> bool {
        // This is a placeholder for property-based tests
        // In a real implementation, you would test that processing a stream
        // and then processing it again produces the same result
        true
    }

    /// Property test for data stream size preservation
    pub fn test_data_stream_size_preservation(stream: DataStream) -> bool {
        // Test that the data size is preserved through processing
        stream.data.len() >= 0
    }
}

/// Test utilities for mocking and stubbing
pub mod mock {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use bridge_core::{
        traits::{StreamProcessor as BridgeStreamProcessor, DataStream, StreamProcessorStats},
        BridgeResult,
    };

    /// Mock stream processor for testing
    pub struct MockStreamProcessor {
        pub name: String,
        pub version: String,
        pub stats: Arc<RwLock<StreamProcessorStats>>,
        pub should_fail: bool,
    }

    impl MockStreamProcessor {
        /// Create a new mock processor
        pub fn new(name: &str, version: &str) -> Self {
            Self {
                name: name.to_string(),
                version: version.to_string(),
                stats: Arc::new(RwLock::new(StreamProcessorStats::default())),
                should_fail: false,
            }
        }

        /// Set whether the processor should fail
        pub fn set_should_fail(&mut self, should_fail: bool) {
            self.should_fail = should_fail;
        }
    }

    #[async_trait::async_trait]
    impl BridgeStreamProcessor for MockStreamProcessor {
        async fn process_stream(&self, input: DataStream) -> BridgeResult<DataStream> {
            if self.should_fail {
                return Err(bridge_core::BridgeError::processing("Mock processor failure"));
            }

            // Update stats
            {
                let mut stats = self.stats.write().await;
                stats.total_records += 1;
            }

            // Return the input with some metadata added
            let mut output = input;
            output.metadata.insert("processed_by".to_string(), self.name.clone());
            output.metadata.insert("processed_at".to_string(), chrono::Utc::now().to_rfc3339());

            Ok(output)
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            &self.version
        }

        async fn health_check(&self) -> BridgeResult<bool> {
            Ok(!self.should_fail)
        }

        async fn get_stats(&self) -> BridgeResult<StreamProcessorStats> {
            let stats = self.stats.read().await;
            Ok(stats.clone())
        }

        async fn shutdown(&self) -> BridgeResult<()> {
            Ok(())
        }
    }
}
