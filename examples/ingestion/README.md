# Ingestion Examples

Data ingestion and receiver examples using various protocols and patterns.

## Examples

### `otap_example.rs`
OpenTelemetry Arrow Protocol examples showing how to work with OTAP data.

### `otap_receiver_example.rs`
OTAP receiver implementation demonstrating how to receive and process OTAP data.

### `minimal_otap_example.rs`
Minimal OTAP setup for quick testing and development.

### `receiver_example.rs`
General receiver patterns and implementations for various data sources.

### `simple_receiver_example.rs`
Basic receiver implementation showing fundamental receiver concepts.

### `streaming_exporter_example.rs`
Streaming data export patterns and implementations.

## Running

```bash
cargo run --example otap_example
cargo run --example otap_receiver_example
cargo run --example minimal_otap_example
cargo run --example receiver_example
cargo run --example simple_receiver_example
cargo run --example streaming_exporter_example
```

## Prerequisites

Some examples may require:
- OpenTelemetry Collector
- Network access for receiving data
- Proper configuration for data sources
