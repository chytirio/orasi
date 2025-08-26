//! Aggregation function implementations

pub mod count;
pub mod sum;
pub mod average;
pub mod min;
pub mod max;

// Re-export all aggregation functions
pub use count::CountAggregation;
pub use sum::SumAggregation;
pub use average::AverageAggregation;
pub use min::MinAggregation;
pub use max::MaxAggregation;
