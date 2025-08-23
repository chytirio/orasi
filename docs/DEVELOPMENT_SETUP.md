# Development Setup Guide

This guide will help you set up your development environment for contributing to Orasi.

## Prerequisites

### Required Software
- **Rust 1.70+**: [Install Rust](https://rustup.rs/)
- **Git**: [Install Git](https://git-scm.com/)
- **Docker** (optional): [Install Docker](https://docs.docker.com/get-docker/)
- **Kubernetes** (optional): [Install kubectl](https://kubernetes.io/docs/tasks/tools/)

### Recommended Tools
- **VS Code** with Rust extensions
- **cargo-watch**: `cargo install cargo-watch`
- **cargo-tarpaulin**: `cargo install cargo-tarpaulin`
- **cargo-husky**: `cargo install cargo-husky`

## Quick Start

### 1. Clone the Repository
```bash
git clone https://github.com/chytirio/orasi.git
cd orasi
```

### 2. Install Development Dependencies
```bash
# Install Rust components
rustup component add rustfmt clippy

# Install development tools
cargo install cargo-tarpaulin cargo-husky cargo-watch

# Setup pre-commit hooks
cargo husky install
```

### 3. Build the Project
```bash
# Build all crates
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test
```

### 4. Verify Setup
```bash
# Run all checks
just check

# Run examples
just examples

# Check documentation
just doc-check
```

## Development Workflow

### Using Just Commands
Orasi uses [Just](https://github.com/casey/just) for development automation:

```bash
# Show available commands
just

# Build everything
just build-all

# Run all tests
just test-all

# Run clippy on all modules
just clippy-all

# Generate documentation
just doc-open
```

### Code Quality Checks
```bash
# Format code
just fmt-fix

# Run linter
just clippy

# Run security audit
just audit

# Generate coverage report
just coverage
```

### Running Examples
```bash
# Run basic usage example
just example-basic

# Run all examples
just examples

# Run specific example
cargo run --example basic_usage
```

## Project Structure

```
orasi/
├── crates/                    # Core libraries
│   ├── bridge-core/          # Core types and utilities
│   ├── bridge-api/           # REST/gRPC API server
│   ├── ingestion/            # Data ingestion pipeline
│   ├── streaming-processor/  # Real-time processing
│   ├── query-engine/         # SQL query processing
│   └── schema-registry/      # Schema management
├── app/                      # Applications
│   ├── orasi-agent/          # Agent runtime
│   ├── orasi-cli/            # Management CLI
│   ├── orasi-gateway/        # Gateway runtime
│   └── orasi-web/            # Web portal
├── connectors/               # Data connectors
│   ├── deltalake/           # Delta Lake connector
│   ├── iceberg/             # Apache Iceberg connector
│   ├── snowflake/           # Snowflake connector
│   └── s3-parquet/          # S3 Parquet connector
├── examples/                 # Usage examples
├── docs/                     # Documentation
├── testing/                  # Testing utilities
└── infra/                    # Infrastructure
```

## Development Environment Setup

### VS Code Configuration
Create `.vscode/settings.json`:
```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.buildScripts.enable": true,
    "rust-analyzer.procMacro.enable": true,
    "editor.formatOnSave": true,
    "editor.codeActionsOnSave": {
        "source.fixAll": true
    }
}
```

### Recommended VS Code Extensions
- **rust-analyzer**: Rust language support
- **crates**: Cargo.toml dependency management
- **Even Better TOML**: TOML file support
- **Docker**: Docker support
- **YAML**: YAML file support

### Git Hooks
Pre-commit hooks are automatically installed with cargo-husky:
- Code formatting with rustfmt
- Linting with clippy
- Basic tests

## Testing

### Running Tests
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p bridge-core

# Run integration tests
cargo test --test integration

# Run tests with output
cargo test -- --nocapture

# Run benchmarks
cargo bench
```

### Test Coverage
```bash
# Generate coverage report
cargo tarpaulin --out Html

# View coverage report
open target/tarpaulin/report.html
```

## Documentation

### Building Documentation
```bash
# Generate API documentation
cargo doc --no-deps

# Open documentation in browser
cargo doc --no-deps --open

# Check documentation
cargo doc --no-deps --document-private-items
```

### Documentation Structure
- **API Reference**: `docs/API_REFERENCE.md`
- **Deployment Guide**: `docs/DEPLOYMENT_GUIDE.md`
- **Configuration Guide**: `docs/CONFIGURATION.md`
- **Examples**: `examples/` directory

## Debugging

### Logging
```bash
# Run with debug logging
RUST_LOG=debug cargo run --bin orasi-bridge

# Run with specific log level
RUST_LOG=orasi_bridge=debug cargo run --bin orasi-bridge
```

### Common Issues

#### Build Issues
```bash
# Clean and rebuild
cargo clean && cargo build

# Update dependencies
cargo update

# Check for outdated dependencies
cargo outdated
```

#### Test Issues
```bash
# Run tests with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in specific module
cargo test module_name
```

## Performance Profiling

### Using cargo-instruments (macOS)
```bash
# Install cargo-instruments
cargo install cargo-instruments

# Profile application
cargo instruments --open
```

### Using perf (Linux)
```bash
# Profile with perf
perf record --call-graph=dwarf cargo run --release
perf report
```

## Contributing

### Before Submitting
1. **Run all checks**: `just check`
2. **Run tests**: `cargo test`
3. **Update documentation**: `cargo doc`
4. **Check formatting**: `cargo fmt --check`
5. **Run clippy**: `cargo clippy`

### Commit Guidelines
- Use conventional commit messages
- Include tests for new features
- Update documentation as needed
- Keep commits focused and atomic

## Getting Help

### Resources
- **Rust Book**: https://doc.rust-lang.org/book/
- **Rust Reference**: https://doc.rust-lang.org/reference/
- **Cargo Book**: https://doc.rust-lang.org/cargo/
- **Tokio Tutorial**: https://tokio.rs/tokio/tutorial

### Community
- **GitHub Issues**: Report bugs and request features
- **GitHub Discussions**: Ask questions and share ideas
- **Discord**: Join our community server

## Troubleshooting

### Common Problems

#### "No such file or directory" errors
```bash
# Ensure you're in the project root
pwd  # Should show /path/to/orasi

# Check if submodules are initialized
git submodule update --init --recursive
```

#### Build failures
```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean && cargo build

# Check for system dependencies
# (varies by platform)
```

#### Test failures
```bash
# Check if external services are running
# (Kafka, PostgreSQL, etc.)

# Run tests with more verbose output
cargo test -- --nocapture --test-threads=1
```

#### Performance issues
```bash
# Build in release mode
cargo build --release

# Profile the application
cargo install flamegraph
cargo flamegraph
```

## Next Steps

1. **Read the Architecture Guide**: Understand the system design
2. **Explore Examples**: Run and study the examples
3. **Pick a Good First Issue**: Look for issues labeled "good first issue"
4. **Join Discussions**: Participate in GitHub discussions
5. **Start Small**: Begin with documentation or test improvements

## Support

If you encounter issues not covered in this guide:

1. Check the [GitHub Issues](https://github.com/chytirio/orasi/issues)
2. Search [GitHub Discussions](https://github.com/chytirio/orasi/discussions)
3. Create a new issue with detailed information
4. Join our community channels

---

**Happy coding! 🦀**
