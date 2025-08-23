# Contributing to Orasi

Thank you for your interest in contributing to Orasi! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Standards](#code-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation Guidelines](#documentation-guidelines)
- [Pull Request Process](#pull-request-process)
- [Release Process](#release-process)
- [Community Guidelines](#community-guidelines)

## Code of Conduct

This project and everyone participating in it is governed by our Code of Conduct. By participating, you are expected to uphold this code.

### Our Standards

- **Be respectful and inclusive**: Use welcoming and inclusive language
- **Be collaborative**: Work together to achieve common goals
- **Be constructive**: Provide constructive feedback and suggestions
- **Be professional**: Maintain professional behavior in all interactions

### Reporting Issues

If you experience or witness unacceptable behavior, please report it by:
- Emailing the project maintainers
- Creating a private GitHub issue
- Contacting the project leads directly

## Getting Started

### Prerequisites

- Rust 1.70+
- Git
- Basic understanding of Rust and async programming
- Familiarity with OpenTelemetry concepts (helpful but not required)

### Development Setup

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Set up the development environment**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/orasi.git
   cd orasi
   just install-dev-deps
   cargo build
   ```

4. **Read the [Development Setup Guide](docs/DEVELOPMENT_SETUP.md)** for detailed instructions

### Finding Issues to Work On

- **Good First Issues**: Look for issues labeled `good first issue`
- **Help Wanted**: Issues labeled `help wanted` are open for contribution
- **Bug Reports**: Issues labeled `bug` need investigation and fixes
- **Enhancement**: Issues labeled `enhancement` are feature requests

## Development Workflow

### 1. Create a Feature Branch

```bash
# Ensure you're on main and up to date
git checkout main
git pull origin main

# Create a new branch for your work
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
# or
git checkout -b docs/your-documentation-update
```

### 2. Make Your Changes

- Write your code following the [Code Standards](#code-standards)
- Add tests for new functionality
- Update documentation as needed
- Keep commits focused and atomic

### 3. Test Your Changes

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p your-crate-name

# Run integration tests
cargo test --test integration

# Run clippy
cargo clippy

# Check formatting
cargo fmt --check
```

### 4. Commit Your Changes

Use conventional commit messages:

```bash
# Format: type(scope): description
git commit -m "feat(ingestion): add new OTLP receiver configuration"
git commit -m "fix(bridge-api): resolve memory leak in connection pool"
git commit -m "docs(readme): update installation instructions"
git commit -m "test(query-engine): add performance benchmarks"
```

**Commit Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### 5. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a pull request on GitHub with:
- Clear title and description
- Reference to related issues
- Summary of changes
- Testing instructions

## Code Standards

### Rust Code Style

- Follow the [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/style/naming/README.html)
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for common issues
- Follow Rust naming conventions

### Code Organization

- Keep functions small and focused
- Use meaningful variable and function names
- Add comments for complex logic
- Use proper error handling with `Result<T, E>`
- Prefer `anyhow` for application-level errors
- Use `thiserror` for library-level errors

### Example Code Structure

```rust
/// Configuration for the OTLP receiver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpReceiverConfig {
    /// The host to bind to
    pub host: String,
    /// The port to bind to
    pub port: u16,
    /// Maximum batch size
    pub batch_size: usize,
}

impl OtlpReceiverConfig {
    /// Create a new OTLP receiver configuration
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            batch_size: 1000,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.port == 0 {
            return Err(ConfigError::InvalidPort);
        }
        if self.batch_size == 0 {
            return Err(ConfigError::InvalidBatchSize);
        }
        Ok(())
    }
}
```

### Error Handling

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
}
```

## Testing Guidelines

### Unit Tests

- Write unit tests for all public functions
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

    #[test]
    async fn test_otlp_receiver_integration() {
        let config = OtlpReceiverConfig::new("127.0.0.1".to_string(), 0);
        let receiver = OtlpReceiver::new(config).await.unwrap();
        
        // Test receiver functionality
        let result = receiver.start().await;
        assert!(result.is_ok());
        
        // Cleanup
        receiver.stop().await.unwrap();
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

fn benchmark_otlp_processing(c: &mut Criterion) {
    c.bench_function("otlp_batch_processing", |b| {
        b.iter(|| {
            // Benchmark code here
        });
    });
}

criterion_group!(benches, benchmark_otlp_processing);
criterion_main!(benches);
```

## Documentation Guidelines

### Code Documentation

- Document all public APIs with `///` comments
- Use examples in documentation
- Document error conditions
- Keep documentation up to date

```rust
/// Processes a batch of OpenTelemetry data
///
/// This function takes a batch of telemetry data and processes it according
/// to the configured pipeline. The data is filtered, transformed, and
/// exported to the configured destinations.
///
/// # Arguments
///
/// * `batch` - The batch of telemetry data to process
/// * `config` - Processing configuration
///
/// # Returns
///
/// Returns the number of records processed successfully.
///
/// # Errors
///
/// Returns an error if:
/// - The batch is empty
/// - Processing configuration is invalid
/// - Export destinations are unavailable
///
/// # Examples
///
/// ```rust
/// use orasi_ingestion::{process_batch, BatchConfig};
///
/// let config = BatchConfig::default();
/// let batch = vec![/* telemetry data */];
/// let processed = process_batch(batch, config).await?;
/// println!("Processed {} records", processed);
/// ```
pub async fn process_batch(
    batch: Vec<TelemetryRecord>,
    config: BatchConfig,
) -> Result<usize, ProcessingError> {
    // Implementation
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

## Pull Request Process

### Before Submitting

1. **Ensure code quality**:
   ```bash
   just check  # Runs fmt, clippy, test, audit
   ```

2. **Update documentation**:
   ```bash
   cargo doc --no-deps
   ```

3. **Test thoroughly**:
   ```bash
   cargo test
   cargo test --test integration
   ```

4. **Check for security issues**:
   ```bash
   cargo audit
   ```

### Pull Request Template

```markdown
## Description

Brief description of the changes made.

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing

- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed
- [ ] Performance impact assessed

## Checklist

- [ ] Code follows the style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added/updated
- [ ] No breaking changes (or breaking changes documented)

## Related Issues

Closes #123
Related to #456
```

### Review Process

1. **Automated Checks**: CI/CD pipeline runs tests and checks
2. **Code Review**: At least one maintainer reviews the PR
3. **Discussion**: Address any feedback or questions
4. **Approval**: PR is approved and merged

## Release Process

### Versioning

We follow [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist

- [ ] All tests pass
- [ ] Documentation is up to date
- [ ] CHANGELOG.md is updated
- [ ] Version numbers are updated
- [ ] Release notes are prepared
- [ ] Docker images are built and tested
- [ ] GitHub release is created

## Community Guidelines

### Communication

- **Be respectful**: Treat everyone with respect
- **Be helpful**: Help others learn and grow
- **Be patient**: Everyone learns at their own pace
- **Be constructive**: Provide helpful feedback

### Getting Help

- **GitHub Issues**: For bugs and feature requests
- **GitHub Discussions**: For questions and general discussion
- **Documentation**: Check existing docs first
- **Examples**: Look at the examples directory

### Recognition

Contributors are recognized through:
- GitHub contributors list
- Release notes
- Community acknowledgments
- Contributor badges

## Questions?

If you have questions about contributing:

1. Check the [Development Setup Guide](docs/DEVELOPMENT_SETUP.md)
2. Search existing [GitHub Issues](https://github.com/chytirio/orasi/issues)
3. Start a [GitHub Discussion](https://github.com/chytirio/orasi/discussions)
4. Contact the maintainers directly

---

**Thank you for contributing to Orasi! 🚀**
