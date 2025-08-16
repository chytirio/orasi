# OpenTelemetry Data Lake Bridge - Test Data Generator

A comprehensive test data generation framework for the OpenTelemetry Data Lake Bridge, designed to create realistic agentic IDE telemetry patterns for thorough testing of the bridge's capabilities.

## 🎯 Overview

This test data generator creates sophisticated, realistic telemetry data that accurately represents the complex, multi-dimensional nature of agentic IDE workflows. It supports comprehensive testing scenarios including:

- **Agentic Development Workflows**: Research, coordination, and implementation patterns
- **Multi-Agent Coordination**: Complex traces spanning multiple AI agents and human interactions
- **Schema Evolution Testing**: Support for testing schema migration and compatibility
- **Scale and Performance Testing**: High-volume data generation for performance validation
- **Error Condition Simulation**: Realistic failure scenarios and edge cases
- **Multi-Tenant Scenarios**: Realistic multi-tenant patterns with proper isolation
- **Compliance Testing**: GDPR, CCPA compliance scenarios with privacy-preserving data

## 🏗️ Architecture

### Core Components

```
test-data-generator/
├── config/              # Configuration management
├── generators/          # Data generation engines
├── models/             # Data models and structures
├── scenarios/          # Test scenario definitions
├── validators/         # Data validation framework
├── exporters/          # Data export capabilities
├── utils/              # Utility functions
└── cli/                # Command-line interface
```

### Data Flow

```
Configuration → Workflow Models → Telemetry Generation → 
Validation → Export → Lakehouse Integration
```

## 🚀 Features

### Realistic Workflow Modeling

- **Research Workflows**: Parameterizable templates for different research patterns
- **Coordination Workflows**: Multi-repo coordination patterns with configurable complexity
- **Agent Interactions**: Realistic agent collaboration and escalation patterns
- **User Behavior**: Human interaction patterns with agents and coordination workflows

### Advanced Data Generation

- **Probabilistic Generation**: Markov chain models for realistic workflow progression
- **Temporal Patterns**: Business hours, seasonal patterns, and realistic timing distributions
- **Geographic Distribution**: Multi-timezone development with coordination across regions
- **Multi-Tenant Support**: Realistic tenant isolation with proper organization structures

### Comprehensive Testing Support

- **Schema Evolution**: Testing bridge behavior during OpenTelemetry schema updates
- **Scale Testing**: 100K+ events per second with realistic burst patterns
- **Error Simulation**: Realistic failure scenarios with proper error propagation
- **Performance Benchmarking**: Representative queries with realistic complexity

## 📦 Installation

### Prerequisites

- Rust 1.70+ with Cargo
- Access to the OpenTelemetry Data Lake Bridge workspace

### From Source

```bash
# Navigate to the test-data-generator directory
cd testing/data-generator

# Build the project
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

## ⚙️ Configuration

### Basic Configuration

Create a `config/test-data-generator.toml` file:

```toml
name = "test-data-generator"
version = "0.1.0"
seed = 42

[output]
directory = "test-data"
format = "Json"
batch_size = 1000

[workflow.research]
session_duration = { min_seconds = 300, max_seconds = 3600, mean_seconds = 1800, std_dev_seconds = 600, distribution_type = "Normal" }

[workflow.coordination]
multi_repo_coordination = { repository_count = 5, dependency_depth = 3, coordination_frequency = 0.2, merge_conflict_rate = 0.1 }

[scale.volume]
total_events = 1_000_000
events_per_second = 1000
burst_multiplier = 2.0

[scale.temporal]
start_time = "2024-01-01T00:00:00Z"
end_time = "2024-01-08T00:00:00Z"

[quality.schema_compliance]
opentelemetry_compliance = true
lakehouse_schema_mapping = true
custom_attributes = true
```

### Advanced Configuration

```toml
[workflow.agent_interactions]
agent_types = ["ResearchAgent", "CodeAgent", "TestAgent"]
interaction_patterns = [
    { name = "collaboration", frequency = 0.3, duration = { min_seconds = 60, max_seconds = 300 } },
    { name = "handoff", frequency = 0.15, duration = { min_seconds = 10, max_seconds = 60 } }
]

[scale.multi_tenant]
tenant_count = 10
tenant_size_distribution = { small_tenants = 0.4, medium_tenants = 0.3, large_tenants = 0.2, enterprise_tenants = 0.1 }
isolation_level = "Strict"

[quality.privacy_compliance]
pii_scrubbing = true
gdpr_compliance = true
data_classification = true
audit_trail = true
```

## 🎮 Usage

### Command Line Interface

```bash
# Generate test data with default configuration
cargo run -- generate

# Generate test data with custom configuration
cargo run -- generate --config config/custom.toml

# Generate specific scenario
cargo run -- generate --scenario development-workflow

# Validate generated data
cargo run -- validate --input test-data/

# Export data to different formats
cargo run -- export --format parquet --output test-data-parquet/

# Run performance benchmarks
cargo run -- benchmark --duration 300
```

### Programmatic Usage

```rust
use test_data_generator::{GeneratorConfig, generate_test_data, validate_test_data};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = GeneratorConfig::load_from_file("config/test-data-generator.toml")?;
    
    // Generate test data
    generate_test_data(&config).await?;
    
    // Validate generated data
    let report = validate_test_data(&config).await?;
    println!("Validation report: {:?}", report);
    
    Ok(())
}
```

## 📊 Test Scenarios

### Development Workflow Scenarios

#### Research-Driven Development
```toml
[[scenarios]]
name = "research-driven-development"
description = "Complete research-driven development workflow"
scenario_type = "Development"
enabled = true
parameters = { research_duration = "2h", coordination_complexity = "high", implementation_scope = "medium" }
```

#### Multi-Repo Coordination
```toml
[[scenarios]]
name = "multi-repo-coordination"
description = "Complex multi-repository coordination workflow"
scenario_type = "Development"
enabled = true
parameters = { repo_count = 5, dependency_depth = 3, conflict_rate = 0.15 }
```

### Failure Scenarios

#### Agent Failures
```toml
[[scenarios]]
name = "agent-failures"
description = "Realistic agent failure and recovery patterns"
scenario_type = "Failure"
enabled = true
parameters = { failure_rate = 0.05, recovery_time = "5m", escalation_rate = 0.3 }
```

#### Network Partitions
```toml
[[scenarios]]
name = "network-partitions"
description = "Desktop-cloud connectivity issues with offline operation"
scenario_type = "Failure"
enabled = true
parameters = { partition_duration = "10m", sync_delay = "2m", data_loss_rate = 0.01 }
```

### Scale Scenarios

#### High-Volume Load
```toml
[[scenarios]]
name = "high-volume-load"
description = "100K+ events per second with realistic burst patterns"
scenario_type = "Scale"
enabled = true
parameters = { events_per_second = 100000, burst_multiplier = 3.0, duration = "1h" }
```

#### Multi-Tenant Scale
```toml
[[scenarios]]
name = "multi-tenant-scale"
description = "Large-scale multi-tenant scenario with proper isolation"
scenario_type = "Scale"
enabled = true
parameters = { tenant_count = 100, tenant_size = "large", isolation_level = "strict" }
```

## 🔍 Data Validation

### Schema Compliance

The generator ensures 100% compliance with OpenTelemetry specifications:

- **Semantic Conventions**: Proper use of OpenTelemetry semantic conventions
- **Schema Mapping**: Correct mapping to different lakehouse formats
- **Custom Attributes**: Support for organization-specific attributes
- **Schema Evolution**: Testing schema migration and backward compatibility

### Data Consistency

Comprehensive validation of data relationships:

- **Cross-Reference Validation**: Ensuring related data maintains referential integrity
- **Temporal Consistency**: Validating proper temporal ordering and causality
- **Aggregation Consistency**: Ensuring aggregated data matches detailed data
- **Cross-Service Consistency**: Validating data consistency across service boundaries

### Quality Metrics

Quantitative measures of data quality:

- **Schema Compliance**: 100% compliance with OpenTelemetry specifications
- **Relationship Accuracy**: 99.9% accuracy in cross-reference and temporal relationships
- **Distribution Realism**: Statistical validation of realistic data distributions
- **Workflow Completeness**: Complete coverage of identified workflow patterns

## 📈 Performance Characteristics

### Generation Performance

- **Throughput**: Generate test data at 10x production ingestion rates
- **Scalability**: Linear scaling with configurable parallelism
- **Resource Efficiency**: Minimal resource usage for generation and storage
- **Reproducibility**: Deterministic generation with seed-based reproducibility

### Data Characteristics

- **Volume**: Support for millions of events with realistic distributions
- **Variety**: Complete coverage of metrics, traces, and logs
- **Velocity**: High-velocity streaming with backpressure testing
- **Veracity**: Realistic data quality with configurable error rates

## 🔧 Customization

### Custom Workflow Templates

```rust
use test_data_generator::models::{AgenticWorkflow, WorkflowType};

// Create custom workflow template
let custom_workflow = AgenticWorkflow {
    id: Uuid::new_v4(),
    workflow_type: WorkflowType::Research,
    // ... custom configuration
};
```

### Custom Data Generators

```rust
use test_data_generator::generators::TelemetryGenerator;

// Extend telemetry generator with custom logic
struct CustomTelemetryGenerator {
    base_generator: TelemetryGenerator,
    custom_config: CustomConfig,
}

impl CustomTelemetryGenerator {
    pub fn generate_custom_data(&self) -> anyhow::Result<Vec<TelemetryRecord>> {
        // Custom generation logic
    }
}
```

### Custom Validation Rules

```rust
use test_data_generator::validators::{DataValidator, ValidationRule};

// Define custom validation rules
struct CustomValidationRule;

impl ValidationRule for CustomValidationRule {
    fn validate(&self, data: &TelemetryRecord) -> ValidationResult {
        // Custom validation logic
    }
}
```

## 🧪 Testing

### Unit Tests

```bash
# Run all unit tests
cargo test

# Run specific test module
cargo test --test config_tests

# Run tests with coverage
cargo test --coverage
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration

# Run tests with specific features
cargo test --features "lakehouse-integration"
```

### Performance Tests

```bash
# Run performance benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench generation_benchmarks
```

## 📚 Examples

### Basic Research Workflow

```rust
use test_data_generator::{
    GeneratorConfig, 
    models::{AgenticWorkflow, ResearchSession, Agent, AgentType}
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = GeneratorConfig::default();
    
    // Generate research workflow
    let workflow = AgenticWorkflow {
        id: Uuid::new_v4(),
        workflow_type: WorkflowType::Research,
        status: WorkflowStatus::InProgress,
        phases: vec![],
        agents: vec![
            Agent {
                id: Uuid::new_v4(),
                name: "Research Agent".to_string(),
                agent_type: AgentType::ResearchAgent,
                capabilities: vec![AgentCapability::Research],
                status: AgentStatus::Available,
                metadata: HashMap::new(),
                created_at: Utc::now(),
            }
        ],
        metadata: HashMap::new(),
        start_time: Utc::now(),
        end_time: None,
        duration: None,
    };
    
    // Generate telemetry data
    generate_test_data(&config).await?;
    
    Ok(())
}
```

### Multi-Agent Coordination

```rust
use test_data_generator::models::{
    CoordinationWorkflow, 
    CoordinationType, 
    AgentHandoff, 
    Escalation
};

let coordination = CoordinationWorkflow {
    id: Uuid::new_v4(),
    coordination_type: CoordinationType::Parallel,
    agents: vec![agent1_id, agent2_id, agent3_id],
    events: vec![],
    handoffs: vec![
        AgentHandoff {
            id: Uuid::new_v4(),
            source_agent: agent1_id,
            target_agent: agent2_id,
            reason: "Task completion".to_string(),
            data: HashMap::new(),
            timestamp: Utc::now(),
            status: HandoffStatus::Completed,
        }
    ],
    escalations: vec![],
    decisions: vec![],
    status: CoordinationStatus::InProgress,
    start_time: Utc::now(),
    end_time: None,
};
```

## 🤝 Contributing

### Development Setup

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Make your changes**: Follow Rust coding standards
4. **Add tests**: Ensure comprehensive test coverage
5. **Run tests**: `cargo test && cargo clippy`
6. **Submit a pull request**: Include detailed description of changes

### Code Standards

- Follow Rust coding conventions
- Use comprehensive error handling with `anyhow`
- Include documentation for all public APIs
- Maintain high test coverage (>90%)
- Use meaningful commit messages

### Testing Guidelines

- Write unit tests for all new functionality
- Include integration tests for complex workflows
- Add performance benchmarks for critical paths
- Ensure reproducible test results

## 📄 License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

### Documentation

- [API Reference](docs/api.md)
- [Configuration Guide](docs/configuration.md)
- [Scenario Library](docs/scenarios.md)
- [Validation Framework](docs/validation.md)

### Issues and Questions

- **Bug Reports**: Use GitHub issues with detailed reproduction steps
- **Feature Requests**: Submit enhancement proposals with use cases
- **Questions**: Use GitHub discussions for general questions
- **Security**: Report security issues privately to the maintainers

### Community

- **Discussions**: [GitHub Discussions](https://github.com/chytirio/orasi/discussions)
- **Issues**: [GitHub Issues](https://github.com/chytirio/orasi/issues)
- **Contributing**: [Contributing Guide](CONTRIBUTING.md)

## 🗺️ Roadmap

### Short Term (Next 3 Months)

- [ ] Enhanced workflow templates
- [ ] Additional lakehouse format support
- [ ] Improved performance benchmarks
- [ ] Extended validation framework

### Medium Term (3-6 Months)

- [ ] Machine learning-based data generation
- [ ] Real-time data streaming capabilities
- [ ] Advanced privacy compliance features
- [ ] Integration with CI/CD pipelines

### Long Term (6+ Months)

- [ ] AI-powered scenario generation
- [ ] Cross-platform telemetry support
- [ ] Advanced analytics integration
- [ ] Enterprise deployment features

---

**Note**: This test data generator is designed to work seamlessly with the OpenTelemetry Data Lake Bridge. For optimal results, ensure compatibility with the latest bridge version and follow the recommended configuration patterns.
