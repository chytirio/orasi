# Code Style Guide

This document outlines the coding standards and conventions for the Orasi project. Following these guidelines ensures consistent, readable, and maintainable code.

## Table of Contents

- [General Principles](#general-principles)
- [Rust Code Style](#rust-code-style)
- [Naming Conventions](#naming-conventions)
- [Error Handling](#error-handling)
- [Documentation](#documentation)
- [Testing](#testing)
- [Performance](#performance)
- [Security](#security)
- [File Organization](#file-organization)

## General Principles

### Code Quality
- **Readability**: Code should be self-documenting and easy to understand
- **Maintainability**: Code should be easy to modify and extend
- **Performance**: Code should be efficient without sacrificing readability
- **Safety**: Code should be safe and handle errors gracefully
- **Consistency**: Follow established patterns and conventions

### Rust Best Practices
- Use Rust's type system to prevent errors
- Prefer immutability by default
- Use ownership and borrowing to prevent data races
- Leverage the borrow checker for memory safety
- Use async/await for I/O operations

## Rust Code Style

### Formatting
- Use `cargo fmt` to format code automatically
- Follow the default Rust formatting rules
- Use 4 spaces for indentation
- Keep line length under 100 characters when possible

### Imports
```rust
// Standard library imports first
use std::collections::HashMap;
use std::sync::Arc;

// External crate imports
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

// Internal crate imports
use crate::config::Config;
use crate::error::Error;

// Re-exports
pub use self::types::*;
```

### Structs and Enums
```rust
/// Configuration for the OTLP receiver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpReceiverConfig {
    /// The host to bind to
    pub host: String,
    /// The port to bind to
    pub port: u16,
    /// Maximum batch size for processing
    pub batch_size: usize,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
}

/// Types of telemetry data
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryType {
    /// Metrics data
    Metrics,
    /// Logs data
    Logs,
    /// Traces data
    Traces,
}
```

### Functions
```rust
/// Creates a new OTLP receiver configuration
///
/// # Arguments
///
/// * `host` - The host to bind to
/// * `port` - The port to bind to
///
/// # Returns
///
/// A new OTLP receiver configuration with default values
///
/// # Examples
///
/// ```rust
/// let config = OtlpReceiverConfig::new("localhost".to_string(), 4317);
/// assert_eq!(config.host, "localhost");
/// assert_eq!(config.port, 4317);
/// ```
pub fn new(host: String, port: u16) -> Self {
    Self {
        host,
        port,
        batch_size: 1000,
        timeout_secs: 30,
    }
}

/// Validates the configuration
///
/// # Returns
///
/// Returns `Ok(())` if the configuration is valid, or an error description
pub fn validate(&self) -> Result<(), String> {
    if self.port == 0 {
        return Err("Port cannot be zero".to_string());
    }
    if self.batch_size == 0 {
        return Err("Batch size cannot be zero".to_string());
    }
    if self.timeout_secs == 0 {
        return Err("Timeout cannot be zero".to_string());
    }
    Ok(())
}
```

### Async Functions
```rust
/// Starts the OTLP receiver
///
/// This function binds to the configured host and port and begins
/// accepting incoming connections.
///
/// # Returns
///
/// Returns a receiver handle that can be used to stop the receiver
///
/// # Errors
///
/// Returns an error if the receiver cannot be started
pub async fn start(&self) -> Result<ReceiverHandle, ReceiverError> {
    let listener = TcpListener::bind(format!("{}:{}", self.host, self.port)).await?;
    
    let handle = ReceiverHandle::new(listener);
    
    // Start the receiver loop
    tokio::spawn({
        let handle = handle.clone();
        async move {
            handle.run().await;
        }
    });
    
    Ok(handle)
}
```

## Naming Conventions

### Variables and Functions
- Use `snake_case` for variables and functions
- Use descriptive names that explain the purpose
- Avoid abbreviations unless they are widely understood

```rust
// Good
let max_retry_attempts = 3;
let connection_timeout = Duration::from_secs(30);

fn process_telemetry_batch(batch: Vec<TelemetryRecord>) -> Result<usize, Error> {
    // Implementation
}

// Bad
let mra = 3;
let ct = Duration::from_secs(30);

fn proc_batch(b: Vec<TelemetryRecord>) -> Result<usize, Error> {
    // Implementation
}
```

### Constants
- Use `SCREAMING_SNAKE_CASE` for constants
- Use `UPPER_SNAKE_CASE` for module-level constants

```rust
const DEFAULT_BATCH_SIZE: usize = 1000;
const MAX_CONNECTIONS: usize = 100;
const DEFAULT_TIMEOUT_SECS: u64 = 30;
```

### Types and Traits
- Use `PascalCase` for types, traits, and enums
- Use descriptive names that indicate the purpose

```rust
pub struct OtlpReceiver;
pub trait TelemetryProcessor;
pub enum ProcessingError;
```

### Modules
- Use `snake_case` for module names
- Use descriptive names that indicate the module's purpose

```rust
pub mod telemetry_processor;
pub mod configuration_manager;
pub mod error_handling;
```

## Error Handling

### Error Types
- Use `thiserror` for library-level errors
- Use `anyhow` for application-level errors
- Define specific error types for different error conditions

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReceiverError {
    #[error("Failed to bind to {host}:{port}: {source}")]
    BindError {
        host: String,
        port: u16,
        #[source]
        source: std::io::Error,
    },
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    
    #[error("Processing error: {0}")]
    ProcessingError(#[from] ProcessingError),
}
```

### Error Propagation
- Use `?` operator for error propagation
- Provide context when wrapping errors
- Handle errors at the appropriate level

```rust
pub async fn start_receiver(config: &Config) -> Result<ReceiverHandle, ReceiverError> {
    let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))
        .await
        .map_err(|e| ReceiverError::BindError {
            host: config.host.clone(),
            port: config.port,
            source: e,
        })?;
    
    let handle = ReceiverHandle::new(listener);
    Ok(handle)
}
```

### Result Types
- Use `Result<T, E>` for functions that can fail
- Use `Option<T>` for functions that may return nothing
- Use `()` for functions that only succeed or fail

```rust
// Function that can fail
pub fn validate_config(config: &Config) -> Result<(), ConfigError> {
    // Implementation
}

// Function that may return nothing
pub fn find_connection(id: &str) -> Option<Connection> {
    // Implementation
}

// Function that only succeeds or fails
pub fn initialize_logging() -> Result<(), LoggingError> {
    // Implementation
}
```

## Documentation

### Code Comments
- Use `///` for documentation comments
- Use `//` for implementation comments
- Write comments that explain "why" not "what"
- Keep comments up to date with code changes

```rust
/// Processes telemetry data according to the configured pipeline
///
/// This function applies filters, transformations, and aggregations
/// to the incoming telemetry data before exporting it to the
/// configured destinations.
///
/// # Arguments
///
/// * `data` - The telemetry data to process
/// * `pipeline` - The processing pipeline configuration
///
/// # Returns
///
/// Returns the number of records processed successfully
///
/// # Errors
///
/// Returns an error if processing fails
pub async fn process_telemetry(
    data: TelemetryData,
    pipeline: &ProcessingPipeline,
) -> Result<usize, ProcessingError> {
    // Apply filters first to reduce processing load
    let filtered_data = pipeline.apply_filters(data).await?;
    
    // Apply transformations
    let transformed_data = pipeline.apply_transformations(filtered_data).await?;
    
    // Export to destinations
    let exported_count = pipeline.export(transformed_data).await?;
    
    Ok(exported_count)
}
```

### README Files
- Include a clear description of the component
- Provide usage examples
- Document configuration options
- Include troubleshooting section

### API Documentation
- Generate with `cargo doc`
- Include comprehensive examples
- Document all public types and functions
- Keep examples up to date

## Testing

### Unit Tests
- Write tests for all public functions
- Test both success and error cases
- Use descriptive test names
- Group related tests in modules

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otlp_receiver_config_creation() {
        let config = OtlpReceiverConfig::new("localhost".to_string(), 4317);
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 4317);
        assert_eq!(config.batch_size, 1000);
    }

    #[test]
    fn test_otlp_receiver_config_validation() {
        let mut config = OtlpReceiverConfig::new("localhost".to_string(), 4317);
        
        // Valid configuration
        assert!(config.validate().is_ok());
        
        // Invalid port
        config.port = 0;
        assert!(config.validate().is_err());
        
        // Invalid batch size
        config.port = 4317;
        config.batch_size = 0;
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_otlp_receiver_start() {
        let config = OtlpReceiverConfig::new("127.0.0.1".to_string(), 0);
        let receiver = OtlpReceiver::new(config);
        
        let handle = receiver.start().await.unwrap();
        assert!(handle.is_running());
        
        handle.stop().await.unwrap();
    }
}
```

### Integration Tests
- Test component interactions
- Test end-to-end workflows
- Use test fixtures and helpers
- Clean up test resources

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio::test;

    async fn setup_test_environment() -> TestEnvironment {
        // Setup test environment
        TestEnvironment::new().await
    }

    async fn cleanup_test_environment(env: TestEnvironment) {
        // Cleanup test environment
        env.cleanup().await;
    }

    #[tokio::test]
    async fn test_telemetry_processing_pipeline() {
        let env = setup_test_environment().await;
        
        // Test the complete pipeline
        let result = env.run_telemetry_pipeline().await;
        assert!(result.is_ok());
        
        cleanup_test_environment(env).await;
    }
}
```

### Performance Tests
- Use `criterion` for benchmarks
- Test critical performance paths
- Include memory usage tests
- Document performance characteristics

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_telemetry_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("telemetry_processing");
    
    group.bench_function("batch_processing", |b| {
        b.iter(|| {
            // Benchmark code here
        });
    });
    
    group.bench_function("stream_processing", |b| {
        b.iter(|| {
            // Benchmark code here
        });
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_telemetry_processing);
criterion_main!(benches);
```

## Performance

### Memory Management
- Use appropriate data structures
- Avoid unnecessary allocations
- Use `Arc` for shared ownership
- Use `Box` for large types on the stack

```rust
// Good: Use Arc for shared data
let shared_config = Arc::new(config);

// Good: Use Box for large types
let large_data = Box::new(LargeDataStructure::new());

// Bad: Unnecessary cloning
let config_clone = config.clone();
```

### Async Programming
- Use `tokio` for async runtime
- Avoid blocking operations in async functions
- Use appropriate async primitives
- Handle backpressure properly

```rust
// Good: Non-blocking async function
pub async fn process_data(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    // Process data asynchronously
    tokio::task::spawn_blocking(move || {
        // CPU-intensive work
        process_data_sync(data)
    }).await?
}

// Bad: Blocking in async function
pub async fn process_data(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    // This blocks the async runtime
    std::thread::sleep(Duration::from_secs(1));
    Ok(data)
}
```

### Resource Management
- Use RAII for resource management
- Implement `Drop` for custom resources
- Use connection pooling
- Handle cleanup properly

```rust
pub struct DatabaseConnection {
    connection: Option<Connection>,
}

impl DatabaseConnection {
    pub fn new() -> Result<Self, Error> {
        let connection = Connection::new()?;
        Ok(Self {
            connection: Some(connection),
        })
    }
}

impl Drop for DatabaseConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            // Cleanup connection
            let _ = conn.close();
        }
    }
}
```

## Security

### Input Validation
- Validate all inputs
- Use strong types to prevent invalid states
- Sanitize user inputs
- Use secure defaults

```rust
pub struct Port(u16);

impl Port {
    pub fn new(port: u16) -> Result<Self, Error> {
        if port == 0 {
            return Err(Error::InvalidPort);
        }
        Ok(Self(port))
    }
}

impl From<Port> for u16 {
    fn from(port: Port) -> Self {
        port.0
    }
}
```

### Error Handling
- Don't expose sensitive information in errors
- Use generic error messages for users
- Log detailed errors for debugging
- Handle errors gracefully

```rust
#[derive(Error, Debug)]
pub enum UserFacingError {
    #[error("Configuration error")]
    ConfigError,
    
    #[error("Connection failed")]
    ConnectionError,
    
    #[error("Processing failed")]
    ProcessingError,
}

impl From<InternalError> for UserFacingError {
    fn from(error: InternalError) -> Self {
        // Log the internal error for debugging
        log::error!("Internal error: {:?}", error);
        
        // Return generic user-facing error
        match error {
            InternalError::ConfigError(_) => UserFacingError::ConfigError,
            InternalError::ConnectionError(_) => UserFacingError::ConnectionError,
            InternalError::ProcessingError(_) => UserFacingError::ProcessingError,
        }
    }
}
```

### Secure Configuration
- Use environment variables for secrets
- Validate configuration at startup
- Use secure defaults
- Document security considerations

```rust
pub struct SecureConfig {
    pub api_key: String,
    pub database_url: String,
}

impl SecureConfig {
    pub fn from_env() -> Result<Self, Error> {
        let api_key = std::env::var("ORASI_API_KEY")
            .map_err(|_| Error::MissingApiKey)?;
        
        let database_url = std::env::var("ORASI_DATABASE_URL")
            .map_err(|_| Error::MissingDatabaseUrl)?;
        
        Ok(Self {
            api_key,
            database_url,
        })
    }
}
```

## File Organization

### Module Structure
- Use `mod.rs` files for module organization
- Group related functionality together
- Keep modules focused and cohesive
- Use public APIs to hide implementation details

```rust
// mod.rs
pub mod config;
pub mod error;
pub mod processor;
pub mod receiver;

pub use config::Config;
pub use error::Error;
pub use processor::Processor;
pub use receiver::Receiver;

// config.rs
pub struct Config {
    // Configuration fields
}

// error.rs
#[derive(Error, Debug)]
pub enum Error {
    // Error variants
}

// processor.rs
pub struct Processor {
    // Processor implementation
}

// receiver.rs
pub struct Receiver {
    // Receiver implementation
}
```

### File Naming
- Use `snake_case` for file names
- Use descriptive names that indicate the file's purpose
- Group related files together

```
src/
├── lib.rs
├── config.rs
├── error.rs
├── processor/
│   ├── mod.rs
│   ├── batch.rs
│   ├── stream.rs
│   └── filter.rs
└── receiver/
    ├── mod.rs
    ├── otlp.rs
    ├── kafka.rs
    └── http.rs
```

### Public API Design
- Design APIs for ease of use
- Provide sensible defaults
- Use builder patterns for complex configuration
- Keep APIs stable and backward compatible

```rust
pub struct ProcessorBuilder {
    config: ProcessorConfig,
}

impl ProcessorBuilder {
    pub fn new() -> Self {
        Self {
            config: ProcessorConfig::default(),
        }
    }
    
    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }
    
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }
    
    pub fn build(self) -> Result<Processor, Error> {
        Processor::new(self.config)
    }
}

// Usage
let processor = ProcessorBuilder::new()
    .batch_size(1000)
    .timeout(Duration::from_secs(30))
    .build()?;
```

## Tools and Automation

### Code Quality Tools
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Use `cargo audit` for security
- Use `cargo test` for testing

### Pre-commit Hooks
- Format code automatically
- Run linter checks
- Run basic tests
- Check for security issues

### CI/CD Pipeline
- Run all tests
- Generate documentation
- Check code coverage
- Build and test on multiple platforms

## Conclusion

Following these guidelines ensures that the Orasi codebase remains:
- **Readable**: Easy to understand and navigate
- **Maintainable**: Easy to modify and extend
- **Reliable**: Robust error handling and testing
- **Performant**: Efficient and scalable
- **Secure**: Safe and secure by default

Remember that these guidelines are living documents. They should evolve with the project and community feedback. When in doubt, prioritize readability and maintainability over cleverness.

---

**Happy coding! 🦀**
