# Orasi Modular Justfile Structure

This document describes the modular justfile structure used in the Orasi project to organize development tasks by domain.

## Overview

The Orasi project uses a modular justfile approach to organize development tasks closer to their actual implementations. This makes it easier to:

- Find relevant commands for specific parts of the codebase
- Maintain and update commands as the codebase evolves
- Provide domain-specific help and information
- Run commands on specific modules without affecting others

## Justfile Structure

### Root Justfile (`./justfile`)
The main justfile contains:
- **Core Development Commands**: Build, test, lint, format, etc.
- **Application Commands**: Run applications, health checks
- **Example Commands**: Run various examples
- **Connector Testing Commands**: Test data lakehouse connectors
- **Docker Commands**: Basic Docker operations
- **Docker Bake Targets**: Advanced Docker build system
- **Docker Compose Integration**: Multi-service orchestration
- **Delta Lake Testing Platform**: Local testing environment
- **Kind Cluster Commands**: Kubernetes local development
- **Modular Justfile Commands**: Integration with domain-specific justfiles

### Domain-Specific Justfiles

#### Crates Justfile (`./crates/justfile`)
Focused on core library development:
- Build, test, and lint all crates
- Run crate-specific examples
- Generate documentation for crates
- Manage bridge-core, bridge-api, bridge-auth, ingestion, query-engine, schema-registry, streaming-processor

#### Connectors Justfile (`./connectors/justfile`)
Focused on data lakehouse connectors:
- Build, test, and lint all connectors
- Run connector-specific examples
- Test connector integrations
- Manage deltalake, hudi, iceberg, kafka, s3-parquet, snowflake connectors

#### Infrastructure Justfile (`./infra/justfile`)
Focused on infrastructure and deployment:
- Docker Bake build system
- Docker Compose orchestration
- Kind cluster management
- Service deployment and management

## Usage

### Using the Root Justfile

```bash
# Show project status
just status

# Run development workflow
just dev-workflow

# Run key examples
just examples-key

# Show modular justfile information
just modular-info
```

### Using Domain-Specific Justfiles

#### Direct Usage
```bash
# Work with crates
cd crates
just build
just test
just info

# Work with connectors
cd connectors
just build
just test-connectors
just info

# Work with infrastructure
cd infra
just docker-bake-all
just kind-setup
just info
```

#### Using Modular Commands from Root
```bash
# Show information about specific modules
just crates-info
just connectors-info
just infra-info

# Build all modules
just build-all

# Test all modules
just test-all

# Lint all modules
just clippy-all
```

## Key Commands by Domain

### Core Development (Root)
- `just build` - Build entire project
- `just test` - Run all tests
- `just dev-workflow` - Format, lint, test, build
- `just examples-key` - Run key examples

### Crates
- `just build` - Build all crates
- `just test` - Test all crates
- `just examples-key` - Run key crate examples
- `just info` - Show crate information

### Connectors
- `just build` - Build all connectors
- `just test-connectors` - Run connector tests
- `just examples-key` - Run key connector examples
- `just info` - Show connector information

### Infrastructure
- `just docker-bake-all` - Build all Docker services
- `just kind-setup` - Setup Kind cluster
- `just compose-bake-up` - Start all services
- `just info` - Show infrastructure information

## Benefits of Modular Structure

1. **Organization**: Commands are grouped by domain and located near their implementations
2. **Maintainability**: Easier to update commands when code changes
3. **Discoverability**: Domain-specific help and information
4. **Isolation**: Run commands on specific modules without affecting others
5. **Flexibility**: Multiple ways to interact with the system (direct, modular commands from root)

## Adding New Justfiles

To add a new domain-specific justfile:

1. Create the justfile in the appropriate directory
2. Add an `info` recipe to provide domain-specific help
3. Add modular commands to the root justfile if needed
4. Update this documentation

## Best Practices

1. **Consistent Naming**: Use consistent recipe names across justfiles (build, test, clippy, info)
2. **Domain-Specific Help**: Always include an `info` recipe that explains the domain and available commands
3. **Clear Documentation**: Use descriptive comments for all recipes
4. **Integration**: Provide ways to run commands across multiple justfiles when needed
5. **Error Handling**: Include appropriate error handling and user feedback

## Troubleshooting

### Justfile Not Found
If a justfile is not found, check:
- The file exists in the expected location
- You're in the right directory when running commands

### Command Not Found
If a command is not found in a justfile:
- Check if the recipe exists using `just --list`
- Verify the recipe name is correct
- Check if the recipe is available in the specific justfile

### Permission Issues
If you encounter permission issues:
- Check file permissions on justfiles
- Ensure you have the necessary permissions to run the commands

### Docker Build Issues
If you encounter Docker build issues:
- Ensure Docker is running (`docker ps`)
- Check Docker daemon connectivity
- Verify Docker buildx is available (`docker buildx version`)
- The bake file path issues have been resolved - the build script now correctly locates `docker-bake.hcl`
