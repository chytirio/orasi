//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Utility generators for common functionality
//!
//! This module provides utility generators for ID generation, time generation,
//! distribution sampling, and correlation management that are used by other generators.

use crate::config::GeneratorConfig;
use anyhow::Result;
use chrono::{DateTime, Duration, Timelike, Utc};
use rand::prelude::*;
use rand_distr::{Distribution, LogNormal, Normal, Uniform};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Generator for unique identifiers
pub struct IdGenerator {
    config: Arc<GeneratorConfig>,
    rng: Arc<RwLock<StdRng>>,
    counter: Arc<RwLock<u64>>,
}

impl IdGenerator {
    /// Create a new ID generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let seed = config.seed.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        let rng = Arc::new(RwLock::new(StdRng::seed_from_u64(seed)));
        let counter = Arc::new(RwLock::new(0));

        Ok(Self {
            config,
            rng,
            counter,
        })
    }

    /// Generate a UUID
    pub async fn generate_uuid(&self) -> Uuid {
        let mut rng = self.rng.write().await;
        Uuid::new_v4()
    }

    /// Generate a sequential ID
    pub async fn generate_sequential_id(&self) -> u64 {
        let mut counter = self.counter.write().await;
        *counter += 1;
        *counter
    }

    /// Generate a correlation ID
    pub async fn generate_correlation_id(&self) -> String {
        let uuid = self.generate_uuid().await;
        format!("corr-{}", uuid.simple())
    }

    /// Generate a trace ID
    pub async fn generate_trace_id(&self) -> String {
        let uuid = self.generate_uuid().await;
        format!("trace-{}", uuid.simple())
    }

    /// Generate a span ID
    pub async fn generate_span_id(&self) -> String {
        let uuid = self.generate_uuid().await;
        format!("span-{}", uuid.simple())
    }
}

/// Generator for realistic time patterns
pub struct TimeGenerator {
    config: Arc<GeneratorConfig>,
    rng: Arc<RwLock<StdRng>>,
    current_time: Arc<RwLock<DateTime<Utc>>>,
}

impl TimeGenerator {
    /// Create a new time generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let seed = config.seed.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        let rng = Arc::new(RwLock::new(StdRng::seed_from_u64(seed)));
        let current_time = Arc::new(RwLock::new(config.scale.temporal.start_time));

        Ok(Self {
            config,
            rng,
            current_time,
        })
    }

    /// Generate a timestamp within the configured time range
    pub async fn generate_timestamp(&self) -> DateTime<Utc> {
        let mut rng = self.rng.write().await;
        let start = self.config.scale.temporal.start_time;
        let end = self.config.scale.temporal.end_time;
        let duration = end - start;
        let random_seconds = rng.gen_range(0..duration.num_seconds() as u64);
        start + Duration::seconds(random_seconds as i64)
    }

    /// Generate a timestamp with business hours consideration
    pub async fn generate_business_hours_timestamp(&self) -> DateTime<Utc> {
        let mut rng = self.rng.write().await;
        let start = self.config.scale.temporal.start_time;
        let end = self.config.scale.temporal.end_time;

        if !self.config.scale.temporal.business_hours.enabled {
            return self.generate_timestamp().await;
        }

        // Generate timestamp within business hours
        let business_start = self.config.scale.temporal.business_hours.start_hour;
        let business_end = self.config.scale.temporal.business_hours.end_hour;

        let mut timestamp = start;
        while timestamp < end {
            let hour = timestamp.hour() as u8;
            if hour >= business_start && hour < business_end {
                let random_seconds = rng.gen_range(0..3600);
                return timestamp + Duration::seconds(random_seconds as i64);
            }
            timestamp += Duration::hours(1);
        }

        // Fallback to regular timestamp if no business hours found
        self.generate_timestamp().await
    }

    /// Generate a duration based on configuration
    pub async fn generate_duration(
        &self,
        distribution: &crate::config::DurationDistribution,
    ) -> Duration {
        let mut rng = self.rng.write().await;

        match distribution.distribution_type {
            crate::config::DistributionType::Normal => {
                let normal = Normal::new(
                    distribution.mean_seconds as f64,
                    distribution.std_dev_seconds as f64,
                )
                .unwrap();
                let seconds = normal
                    .sample(&mut *rng)
                    .max(distribution.min_seconds as f64)
                    .min(distribution.max_seconds as f64);
                Duration::seconds(seconds as i64)
            }
            crate::config::DistributionType::Exponential => {
                // Use uniform distribution as fallback for exponential
                let uniform = Uniform::new(distribution.min_seconds, distribution.max_seconds);
                let seconds = uniform.sample(&mut *rng);
                Duration::seconds(seconds as i64)
            }
            crate::config::DistributionType::LogNormal => {
                let log_normal = LogNormal::new(
                    (distribution.mean_seconds as f64).ln(),
                    (distribution.std_dev_seconds as f64 / distribution.mean_seconds as f64).ln(),
                )
                .unwrap();
                let seconds = log_normal
                    .sample(&mut *rng)
                    .max(distribution.min_seconds as f64)
                    .min(distribution.max_seconds as f64);
                Duration::seconds(seconds as i64)
            }
            crate::config::DistributionType::Uniform => {
                let uniform = Uniform::new(distribution.min_seconds, distribution.max_seconds);
                let seconds = uniform.sample(&mut *rng);
                Duration::seconds(seconds as i64)
            }
            crate::config::DistributionType::PowerLaw => {
                // Power law distribution: P(x) ∝ x^(-α)
                let alpha = 2.0; // Typical value for many real-world distributions
                let min = distribution.min_seconds as f64;
                let max = distribution.max_seconds as f64;
                let u = rng.gen::<f64>();
                let seconds = min * (1.0_f64 - u).powf(-1.0 / alpha);
                Duration::seconds(seconds.min(max) as i64)
            }
        }
    }

    /// Advance the current time
    pub async fn advance_time(&self, duration: Duration) {
        let mut current = self.current_time.write().await;
        *current += duration;
    }

    /// Get the current time
    pub async fn current_time(&self) -> DateTime<Utc> {
        *self.current_time.read().await
    }
}

/// Generator for statistical distributions
pub struct DistributionGenerator {
    config: Arc<GeneratorConfig>,
    rng: Arc<RwLock<StdRng>>,
}

impl DistributionGenerator {
    /// Create a new distribution generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let seed = config.seed.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        let rng = Arc::new(RwLock::new(StdRng::seed_from_u64(seed)));

        Ok(Self { config, rng })
    }

    /// Generate a value from a normal distribution
    pub async fn normal(&self, mean: f64, std_dev: f64) -> f64 {
        let mut rng = self.rng.write().await;
        let normal = Normal::new(mean, std_dev).unwrap();
        normal.sample(&mut *rng)
    }

    /// Generate a value from an exponential distribution
    pub async fn exponential(&self, lambda: f64) -> f64 {
        let mut rng = self.rng.write().await;
        // Use uniform distribution as fallback for exponential
        let uniform = Uniform::new(0.0, 1.0);
        let u: f64 = uniform.sample(&mut *rng);
        -u.ln() / lambda
    }

    /// Generate a value from a log-normal distribution
    pub async fn log_normal(&self, mean: f64, std_dev: f64) -> f64 {
        let mut rng = self.rng.write().await;
        let log_normal = LogNormal::new(mean.ln(), std_dev.ln()).unwrap();
        log_normal.sample(&mut *rng)
    }

    /// Generate a value from a uniform distribution
    pub async fn uniform(&self, min: f64, max: f64) -> f64 {
        let mut rng = self.rng.write().await;
        let uniform = Uniform::new(min, max);
        uniform.sample(&mut *rng)
    }

    /// Generate a value from a power law distribution
    pub async fn power_law(&self, alpha: f64, min: f64, max: f64) -> f64 {
        let mut rng = self.rng.write().await;
        let u = rng.gen::<f64>();
        let value = min * (1.0_f64 - u).powf(-1.0 / alpha);
        value.min(max)
    }

    /// Generate a random boolean with given probability
    pub async fn bernoulli(&self, probability: f64) -> bool {
        let mut rng = self.rng.write().await;
        rng.gen::<f64>() < probability
    }

    /// Choose a random item from a weighted list
    pub async fn weighted_choice<'a, T>(&self, items: &'a [(T, f64)]) -> Option<&'a T>
    where
        T: Clone,
    {
        let total_weight: f64 = items.iter().map(|(_, weight)| weight).sum();
        if total_weight <= 0.0 {
            return None;
        }

        let mut rng = self.rng.write().await;
        let random_value = rng.gen::<f64>() * total_weight;

        let mut cumulative_weight = 0.0;
        for (item, weight) in items {
            cumulative_weight += weight;
            if random_value <= cumulative_weight {
                return Some(item);
            }
        }

        items.last().map(|(item, _)| item)
    }
}

/// Generator for correlation management
pub struct CorrelationGenerator {
    config: Arc<GeneratorConfig>,
    rng: Arc<RwLock<StdRng>>,
    correlations: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl CorrelationGenerator {
    /// Create a new correlation generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let seed = config.seed.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        let rng = Arc::new(RwLock::new(StdRng::seed_from_u64(seed)));
        let correlations = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            config,
            rng,
            correlations,
        })
    }

    /// Create a new correlation group
    pub async fn create_correlation_group(&self, group_id: &str) -> String {
        let correlation_id = format!("corr-{}", Uuid::new_v4().simple());
        let mut correlations = self.correlations.write().await;
        correlations.insert(group_id.to_string(), vec![correlation_id.clone()]);
        correlation_id
    }

    /// Add an item to a correlation group
    pub async fn add_to_correlation_group(&self, group_id: &str, item_id: &str) {
        let mut correlations = self.correlations.write().await;
        correlations
            .entry(group_id.to_string())
            .or_insert_with(Vec::new)
            .push(item_id.to_string());
    }

    /// Get all items in a correlation group
    pub async fn get_correlation_group(&self, group_id: &str) -> Vec<String> {
        let correlations = self.correlations.read().await;
        correlations.get(group_id).cloned().unwrap_or_default()
    }

    /// Generate a trace ID and span ID pair
    pub async fn generate_trace_span_pair(&self) -> (String, String) {
        let trace_id = format!("trace-{}", Uuid::new_v4().simple());
        let span_id = format!("span-{}", Uuid::new_v4().simple());
        (trace_id, span_id)
    }

    /// Generate a parent-child span relationship
    pub async fn generate_span_hierarchy(
        &self,
        depth: usize,
    ) -> Vec<(String, String, Option<String>)> {
        let mut spans = Vec::new();
        let mut parent_stack = Vec::new();

        for _i in 0..depth {
            let (trace_id, span_id) = self.generate_trace_span_pair().await;
            let parent_id = parent_stack.last().cloned();

            spans.push((trace_id, span_id.clone(), parent_id));
            parent_stack.push(span_id);
        }

        spans
    }

    /// Generate correlated attributes across multiple data points
    pub async fn generate_correlated_attributes(
        &self,
        count: usize,
    ) -> Vec<HashMap<String, Value>> {
        let mut attributes = Vec::new();
        let correlation_id = format!("corr-{}", Uuid::new_v4().simple());

        for i in 0..count {
            let mut attr = HashMap::new();
            attr.insert(
                "correlation_id".to_string(),
                Value::String(correlation_id.clone()),
            );
            attr.insert("sequence_number".to_string(), Value::Number(i.into()));
            attr.insert(
                "timestamp".to_string(),
                Value::String(Utc::now().to_rfc3339()),
            );

            attributes.push(attr);
        }

        attributes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GeneratorConfig;

    #[tokio::test]
    async fn test_id_generator() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = IdGenerator::new(config).unwrap();

        let uuid = generator.generate_uuid().await;
        assert_ne!(uuid, Uuid::nil());

        let id1 = generator.generate_sequential_id().await;
        let id2 = generator.generate_sequential_id().await;
        assert_eq!(id2, id1 + 1);
    }

    #[tokio::test]
    async fn test_time_generator() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = TimeGenerator::new(config).unwrap();

        let timestamp = generator.generate_timestamp().await;
        assert!(timestamp >= Utc::now() - Duration::days(7));
        assert!(timestamp <= Utc::now());
    }

    #[tokio::test]
    async fn test_distribution_generator() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = DistributionGenerator::new(config).unwrap();

        let normal_value = generator.normal(0.0, 1.0).await;
        assert!(normal_value.abs() < 10.0); // Very unlikely to be outside this range

        let uniform_value = generator.uniform(0.0, 1.0).await;
        assert!(uniform_value >= 0.0 && uniform_value <= 1.0);
    }

    #[tokio::test]
    async fn test_correlation_generator() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = CorrelationGenerator::new(config).unwrap();

        let group_id = "test-group";
        let correlation_id = generator.create_correlation_group(group_id).await;
        assert!(correlation_id.starts_with("corr-"));

        generator.add_to_correlation_group(group_id, "item1").await;
        let group = generator.get_correlation_group(group_id).await;
        assert_eq!(group.len(), 2); // correlation_id + item1
    }
}
