# Data Processing Submodule

This directory contains the refactored data processing functionality, broken down into logical submodules for better organization and maintainability.

## Structure

```
data/
├── mod.rs              # Main module entry point
├── utils.rs            # Common utilities and shared functionality
├── transformation.rs   # Data transformation processing
├── aggregation.rs      # Data aggregation processing
├── filtering.rs        # Data filtering processing
├── enrichment.rs       # Data enrichment processing
├── validation.rs       # Data validation processing
└── ml.rs              # Machine learning processing
```

## Modules

### `mod.rs` - Main Module
- Contains the main `DataProcessingProcessor` struct
- Handles task routing to appropriate sub-processors
- Provides the public API for data processing

### `utils.rs` - Common Utilities
- `current_timestamp()` - Get current timestamp in milliseconds
- `BaseProcessor` trait - Common interface for all processors
- `DataIO` - Common data I/O operations (read/write simulation)
- `ConfigParser` - Configuration parsing utilities

### `transformation.rs` - Data Transformation
- `TransformationProcessor` - Handles data transformation tasks
- Supports field renaming, adding, removing, and type conversion
- Processes transformation steps sequentially

### `aggregation.rs` - Data Aggregation
- `AggregationProcessor` - Handles data aggregation tasks
- Supports grouping and various aggregation functions (sum, avg, min, max, etc.)
- Generates aggregated results by group

### `filtering.rs` - Data Filtering
- `FilteringProcessor` - Handles data filtering tasks
- Supports various filter operators (equals, greater_than, contains, etc.)
- Processes filter rules sequentially

### `enrichment.rs` - Data Enrichment
- `EnrichmentProcessor` - Handles data enrichment tasks
- Supports timestamp, service, environment, host, geolocation enrichment
- Includes lookup tables, computed values, and external API integration

### `validation.rs` - Data Validation
- `ValidationProcessor` - Handles data validation tasks
- Supports various validation rules (required, string_length, numeric_range, etc.)
- Returns validation results with error details

### `ml.rs` - Machine Learning
- `MLProcessor` - Handles machine learning tasks
- Supports classification, regression, clustering, anomaly detection
- Includes recommendation and forecasting capabilities

## Usage

The main entry point remains the same:

```rust
use crate::processing::data::DataProcessingProcessor;

let processor = DataProcessingProcessor::new(&config, state).await?;
let result = processor.process_data(task).await?;
```

## Benefits of Refactoring

1. **Modularity**: Each processing type is now in its own module
2. **Maintainability**: Easier to find and modify specific functionality
3. **Testability**: Each module can be tested independently
4. **Reusability**: Common utilities are shared across modules
5. **Scalability**: New processing types can be added easily
6. **Code Organization**: Related functionality is grouped together

## Adding New Processors

To add a new processor type:

1. Create a new module file (e.g., `new_processor.rs`)
2. Implement the `BaseProcessor` trait
3. Add the module to `mod.rs`
4. Add the processor type to the main `DataProcessingProcessor::process_data` method

## Common Patterns

All processors follow these common patterns:

- Implement `BaseProcessor` trait for common functionality
- Use `DataIO` for data I/O operations
- Use `ConfigParser` for configuration parsing
- Return structured JSON results
- Include proper error handling and logging
