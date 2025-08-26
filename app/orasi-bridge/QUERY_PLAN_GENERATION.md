# Query Plan Generation

The Bridge API provides sophisticated query plan generation capabilities that analyze, optimize, and plan the execution of telemetry data queries. This system helps optimize query performance and provides detailed insights into how queries will be executed.

## Overview

The query planner is a comprehensive system that:

- **Analyzes Query Characteristics**: Examines query type, filters, aggregations, and time ranges
- **Selects Execution Strategies**: Chooses the most efficient execution approach
- **Estimates Costs**: Calculates resource usage including I/O, CPU, and network costs
- **Identifies Optimizations**: Finds opportunities to improve query performance
- **Plans Execution Steps**: Creates a detailed step-by-step execution plan
- **Determines Parallelism**: Identifies opportunities for parallel execution

## Execution Strategies

The query planner supports several execution strategies, automatically selected based on query characteristics:

### 1. Time Series Scan
**Best for**: Simple queries with minimal filtering
- Direct sequential scan through time-series data
- Optimal for recent data access patterns
- Supports both forward and backward scanning

### 2. Index Lookup
**Best for**: Queries with high-selectivity filters
- Leverages indexes for efficient data retrieval
- Supports time indexes, field indexes, and composite indexes
- Significantly reduces data scanning overhead

### 3. Aggregation Push
**Best for**: Queries with large time ranges and aggregations
- Pushes aggregations down to the storage layer
- Uses pre-aggregated data when available
- Supports multiple aggregation levels (row, minute, hour, day)

### 4. Multi-Stage Execution
**Best for**: Complex analytics queries
- Breaks complex queries into manageable stages
- Optimizes each stage independently
- Provides fine-grained resource control

## Query Optimizations

The planner automatically identifies and applies various optimizations:

### Filter Pushdown
- Moves filter operations closer to data sources
- Reduces the amount of data transferred
- Improves overall query performance

### Aggregation Pushdown
- Executes aggregations at the storage layer
- Minimizes data movement
- Leverages pre-computed aggregations when available

### Index Selection
- Automatically selects the most efficient indexes
- Considers filter selectivity and query patterns
- Balances index overhead with scan reduction

### Time Range Optimization
- Optimizes large time range queries
- Suggests pre-aggregated data usage
- Partitions data access for efficiency

### Caching Strategy
- Identifies cacheable query patterns
- Determines optimal cache TTL values
- Balances cache hits with data freshness

## Cost Estimation

The query planner provides detailed cost estimates:

### Resource Usage
- **Estimated Rows Scanned**: Number of rows that need to be examined
- **Estimated Rows Returned**: Number of rows in the final result
- **Memory Usage**: Estimated memory consumption in MB
- **Execution Time**: Predicted query execution time

### Cost Components
- **I/O Cost**: Cost of data access operations
- **CPU Cost**: Computational overhead (especially for aggregations)
- **Network Cost**: Data transfer costs

## Execution Steps

Each query plan includes detailed execution steps:

### Step Types
1. **Scan**: Data source scanning with conditions
2. **Filter**: Apply filter conditions
3. **Aggregate**: Execute aggregation functions
4. **Sort**: Sort results by specified columns
5. **Limit**: Apply result limits and offsets

### Step Metadata
- **Cost Estimation**: Individual step costs
- **Row Estimates**: Expected rows processed
- **Dependencies**: Inter-step dependencies
- **Parallelizability**: Whether the step can run in parallel

## Parallelism Planning

The planner determines optimal parallelism strategies:

### Parallel Execution
- Identifies steps that can run concurrently
- Considers data partitioning strategies
- Balances parallelism with resource constraints

### Sequential Dependencies
- Maps out required execution order
- Ensures data consistency
- Optimizes resource utilization

## Data Source Analysis

The planner analyzes available data sources:

### Source Types
- **Time Series**: Metrics and continuous data
- **Logs**: Event and log data
- **Traces**: Distributed tracing data
- **Analytics**: Pre-computed analytics data

### Partitioning Strategy
- Identifies relevant data partitions
- Estimates partition sizes
- Plans partition-parallel execution

## Query Plan Output

The query planner generates comprehensive, human-readable execution plans:

```
=== Query Execution Plan (ID: example-id) ===
Query Type: Metrics

📋 Execution Strategy: Time Series Scan (Index: Partial, Direction: Backward)

💰 Cost Estimation:
  • Estimated rows scanned: 1,000,000
  • Estimated rows returned: 1,000
  • Estimated memory usage: 95.37 MB
  • Estimated execution time: 250 ms
  • Total cost: 1000.15 (IO: 1000.00, CPU: 0.10, Network: 0.05)

🚀 Applied Optimizations:
  1. Filter Pushdown: 3 filters
  2. Index Selection: time_index (selectivity: 0.001)
  3. Caching Strategy: query_abc123 (TTL: 300s)

⚡ Execution Steps:
  || Step 0: Scan metrics_data (4 conditions) (Cost: 800.00, Rows: 1000000)
  || Step 1: Filter (3 conditions) (Cost: 100.00, Rows: 1000)
     Dependencies: [0]
  -- Step 2: Sort timestamp (Descending) (Cost: 75.00, Rows: 1000)
     Dependencies: [1]
  || Step 3: Limit 1000 offset 0 (Cost: 25.00, Rows: 1000)
     Dependencies: [2]

💾 Data Sources:
  • metrics_timeseries (Metrics): 1000.00 MB across 5 partitions

🔀 Parallelism:
  • Max parallelism: 5
  • Parallel stages: [0, 1, 3]
  • Sequential dependencies: [(1, 2), (2, 3)]

=== End Query Plan ===
```

## API Usage

### Generate Query Plan

The query plan is automatically generated for each query and included in the response metadata:

```json
{
  "data": {
    "results": { ... },
    "metadata": {
      "query_id": "uuid",
      "execution_time_ms": 145,
      "result_count": 1000,
      "cache_hit": false,
      "query_plan": "=== Query Execution Plan... ==="
    }
  }
}
```

### Query Examples

#### Simple Metrics Query
```json
{
  "query_type": "Metrics",
  "parameters": {
    "time_range": {
      "start": "2025-08-23T01:00:00Z",
      "end": "2025-08-23T02:00:00Z"
    },
    "limit": 100
  },
  "options": {
    "enable_cache": true
  }
}
```

#### Complex Filtered Query
```json
{
  "query_type": "Traces",
  "parameters": {
    "time_range": {
      "start": "2025-08-16T00:00:00Z",
      "end": "2025-08-23T00:00:00Z"
    },
    "filters": [
      {
        "field": "service.name",
        "operator": "Equals",
        "value": { "String": "user-service" }
      },
      {
        "field": "duration",
        "operator": "GreaterThan",
        "value": { "Number": 1000.0 }
      }
    ],
    "aggregations": [
      {
        "field": "duration",
        "function": "Average",
        "alias": "avg_duration"
      }
    ],
    "limit": 1000
  }
}
```

## Strategy Selection Logic

The query planner uses the following logic to select execution strategies:

### Large Time Range + Aggregations
- **Condition**: Time range > 1 day AND aggregations present
- **Strategy**: Aggregation Push
- **Benefits**: Leverages pre-aggregated data, reduces scan overhead

### High Filter Selectivity
- **Condition**: Multiple specific filters (> 2 filters)
- **Strategy**: Index Lookup
- **Benefits**: Dramatically reduces data scanning

### Analytics Queries
- **Condition**: Query type is Analytics
- **Strategy**: Multi-Stage Execution
- **Benefits**: Optimizes complex analytical workloads

### Default Case
- **Condition**: Simple queries without complex patterns
- **Strategy**: Time Series Scan
- **Benefits**: Straightforward, predictable performance

## Performance Considerations

### Query Optimization Tips

1. **Use Specific Filters**: Include high-selectivity filters to trigger index usage
2. **Leverage Time Ranges**: Use reasonable time ranges to enable optimizations
3. **Enable Caching**: Use caching for frequently accessed data
4. **Consider Aggregations**: Use aggregations for large datasets
5. **Limit Result Sets**: Apply appropriate limits to control resource usage

### Cost Factors

- **Time Range Size**: Larger ranges increase scanning costs
- **Filter Selectivity**: More selective filters reduce costs
- **Aggregation Complexity**: Complex aggregations increase CPU costs
- **Result Set Size**: Larger results increase network transfer costs

## Monitoring and Debugging

### Query Performance Analysis

The query planner provides detailed information for performance analysis:

- **Cost Breakdown**: Understand where resources are consumed
- **Execution Steps**: Identify bottlenecks in the execution plan
- **Optimization Opportunities**: See applied and potential optimizations
- **Parallelism Usage**: Understand parallel execution potential

### Common Patterns

#### High-Cost Queries
- Large time ranges without filters
- Complex aggregations on unfiltered data
- Queries that don't utilize indexes

#### Optimized Queries
- Time-bound queries with specific filters
- Queries that leverage pre-aggregated data
- Cacheable query patterns

## Best Practices

### Query Design
1. **Start with Time Bounds**: Always specify reasonable time ranges
2. **Add Selective Filters**: Include filters that significantly reduce data
3. **Use Appropriate Aggregations**: Choose aggregations that match your analysis needs
4. **Consider Caching**: Enable caching for repeated queries

### Performance Optimization
1. **Review Query Plans**: Examine generated plans for optimization opportunities
2. **Monitor Costs**: Track query costs to identify expensive operations
3. **Adjust Time Ranges**: Use smaller ranges for interactive queries
4. **Leverage Indexes**: Ensure filters can utilize available indexes

### Development Workflow
1. **Test with Small Data**: Start with limited time ranges during development
2. **Analyze Plans**: Review query plans before deploying to production
3. **Monitor Performance**: Track query performance in production
4. **Iterate and Optimize**: Continuously improve query patterns

## Future Enhancements

The query planner is designed to be extensible and will be enhanced with:

- **Machine Learning Optimization**: Learning from query patterns
- **Dynamic Index Recommendations**: Suggesting new indexes based on usage
- **Cost-Based Optimization**: More sophisticated cost models
- **Real-Time Optimization**: Adaptive optimization based on current system load
- **Query Rewriting**: Automatic query transformation for better performance

## Integration Examples

### Using the Query Planner

```rust
use bridge_api::handlers::query::generate_query_plan;

// Generate a query plan
let plan = generate_query_plan(&query_request);
println!("Generated plan: {}", plan.unwrap_or_default());
```

### Analyzing Query Costs

```rust
// The query plan includes detailed cost information
// that can be used for monitoring and optimization
if plan.contains("High cost detected") {
    // Consider query optimization
}
```

The query plan generation system provides a powerful foundation for understanding and optimizing telemetry data queries, ensuring optimal performance across different query patterns and data volumes.
