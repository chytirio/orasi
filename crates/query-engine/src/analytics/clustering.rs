//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Clustering for time series data
//!
//! This module provides clustering capabilities for time series data.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{ClusterResult, ClusterType, TimeSeriesData, TimeSeriesPoint};

/// Clustering algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusteringAlgorithm {
    /// K-means clustering
    KMeans,
    /// DBSCAN clustering
    DBSCAN,
    /// Hierarchical clustering
    Hierarchical,
    /// Spectral clustering
    Spectral,
    /// Auto-select best algorithm
    Auto,
}

impl Default for ClusteringAlgorithm {
    fn default() -> Self {
        Self::KMeans
    }
}

/// Cluster analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterAnalyzerConfig {
    /// Analyzer name
    pub name: String,

    /// Analyzer version
    pub version: String,

    /// Enable clustering
    pub enable_clustering: bool,

    /// Number of clusters
    pub num_clusters: usize,

    /// Clustering algorithm to use
    pub algorithm: ClusteringAlgorithm,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for ClusterAnalyzerConfig {
    fn default() -> Self {
        Self {
            name: "default_cluster_analyzer".to_string(),
            version: "1.0.0".to_string(),
            enable_clustering: true,
            num_clusters: 3,
            algorithm: ClusteringAlgorithm::default(),
            additional_config: HashMap::new(),
        }
    }
}

/// Cluster analyzer trait
#[async_trait]
pub trait ClusterAnalyzer: Send + Sync {
    /// Perform clustering
    async fn cluster(&self, data: &TimeSeriesData) -> BridgeResult<Vec<ClusterResult>>;
    
    /// Perform clustering on raw data with labels
    async fn cluster_data(&self, data: &[Vec<f64>], labels: &[String]) -> BridgeResult<Vec<ClusterResult>>;
}

/// Default cluster analyzer implementation
pub struct DefaultClusterAnalyzer {
    config: ClusterAnalyzerConfig,
}

impl DefaultClusterAnalyzer {
    /// Create a new cluster analyzer
    pub fn new(config: ClusterAnalyzerConfig) -> Self {
        Self { config }
    }

    /// Get the algorithm to use
    fn get_algorithm(&self) -> &ClusteringAlgorithm {
        &self.config.algorithm
    }
}

#[async_trait]
impl ClusterAnalyzer for DefaultClusterAnalyzer {
    async fn cluster(&self, data: &TimeSeriesData) -> BridgeResult<Vec<ClusterResult>> {
        info!("Performing clustering on series '{}' with {} points", data.name, data.points.len());

        // Validate input data
        if data.points.len() < 2 {
            return Err(bridge_core::BridgeError::internal(
                "Insufficient data points for clustering (minimum 2 required)",
            ));
        }

        // Extract features from time series data
        let features = self.extract_features(data)?;

        // Perform clustering based on selected algorithm
        let cluster_labels = match self.get_algorithm() {
            ClusteringAlgorithm::KMeans => {
                self.kmeans_clustering(&features, self.config.num_clusters).await?
            }
            ClusteringAlgorithm::DBSCAN => {
                self.dbscan_clustering(&features).await?
            }
            ClusteringAlgorithm::Hierarchical => {
                self.hierarchical_clustering(&features, self.config.num_clusters).await?
            }
            ClusteringAlgorithm::Spectral => {
                self.spectral_clustering(&features, self.config.num_clusters).await?
            }
            ClusteringAlgorithm::Auto => {
                self.auto_clustering(&features).await?
            }
        };

        // Generate cluster results
        let results = self.generate_cluster_results(&features, &cluster_labels, data).await?;

        debug!("Generated {} clusters", results.len());
        Ok(results)
    }

    async fn cluster_data(&self, data: &[Vec<f64>], labels: &[String]) -> BridgeResult<Vec<ClusterResult>> {
        info!("Performing clustering on raw data with {} points", data.len());

        // Validate input data
        if data.is_empty() {
            return Err(bridge_core::BridgeError::internal(
                "No data provided for clustering",
            ));
        }

        // Perform clustering based on selected algorithm
        let cluster_labels = match self.get_algorithm() {
            ClusteringAlgorithm::KMeans => {
                self.kmeans_clustering(data, self.config.num_clusters).await?
            }
            ClusteringAlgorithm::DBSCAN => {
                self.dbscan_clustering(data).await?
            }
            ClusteringAlgorithm::Hierarchical => {
                self.hierarchical_clustering(data, self.config.num_clusters).await?
            }
            ClusteringAlgorithm::Spectral => {
                self.spectral_clustering(data, self.config.num_clusters).await?
            }
            ClusteringAlgorithm::Auto => {
                self.auto_clustering(data).await?
            }
        };

        // Generate cluster results from raw data
        let results = self.generate_cluster_results_from_raw_data(data, &cluster_labels, labels).await?;

        debug!("Generated {} clusters", results.len());
        Ok(results)
    }
}

impl DefaultClusterAnalyzer {
    /// Extract features from time series data
    fn extract_features(&self, data: &TimeSeriesData) -> BridgeResult<Vec<Vec<f64>>> {
        let mut features = Vec::new();

        // Sort points by timestamp
        let mut sorted_points = data.points.clone();
        sorted_points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Extract statistical features from sliding windows
        let window_size = (sorted_points.len() / 10).max(5).min(50);
        
        for window in sorted_points.windows(window_size) {
            let values: Vec<f64> = window.iter().map(|p| p.value).collect();
            
            // Calculate statistical features
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = self.calculate_variance(&values);
            let std_dev = variance.sqrt();
            let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let range = max - min;
            
            // Calculate trend (simple linear regression slope)
            let trend = self.calculate_trend(&values);
            
            // Calculate seasonality (autocorrelation at lag 1)
            let seasonality = self.calculate_autocorrelation(&values, 1);
            
            features.push(vec![mean, std_dev, range, trend, seasonality]);
        }

        Ok(features)
    }

    /// K-means clustering
    async fn kmeans_clustering(&self, features: &[Vec<f64>], k: usize) -> BridgeResult<Vec<i32>> {
        if features.is_empty() {
            return Ok(vec![]);
        }

        let feature_dim = features[0].len();
        let mut centroids = self.initialize_centroids(features, k);
        let mut labels = vec![0; features.len()];
        let max_iterations = 100;
        let mut converged = false;

        for iteration in 0..max_iterations {
            let mut new_labels = vec![0; features.len()];
            let mut cluster_sums = vec![vec![0.0; feature_dim]; k];
            let mut cluster_counts = vec![0; k];

            // Assign points to nearest centroid
            for (i, feature) in features.iter().enumerate() {
                let mut min_distance = f64::INFINITY;
                let mut best_cluster = 0;

                for (j, centroid) in centroids.iter().enumerate() {
                    let distance = self.euclidean_distance(feature, centroid);
                    if distance < min_distance {
                        min_distance = distance;
                        best_cluster = j;
                    }
                }

                new_labels[i] = best_cluster as i32;
                for (dim, &val) in feature.iter().enumerate() {
                    cluster_sums[best_cluster][dim] += val;
                }
                cluster_counts[best_cluster] += 1;
            }

            // Update centroids
            for cluster in 0..k {
                if cluster_counts[cluster] > 0 {
                    for dim in 0..feature_dim {
                        centroids[cluster][dim] = cluster_sums[cluster][dim] / cluster_counts[cluster] as f64;
                    }
                }
            }

            // Check convergence
            if new_labels == labels {
                converged = true;
                break;
            }

            labels = new_labels;
        }

        if !converged {
            warn!("K-means clustering did not converge after {} iterations", max_iterations);
        }

        Ok(labels)
    }

    /// DBSCAN clustering
    async fn dbscan_clustering(&self, features: &[Vec<f64>]) -> BridgeResult<Vec<i32>> {
        if features.is_empty() {
            return Ok(vec![]);
        }

        let eps = self.get_dbscan_eps(features);
        let min_pts = (features.len() / 20).max(2).min(10);
        let mut labels = vec![-1; features.len()]; // -1 means unassigned
        let mut cluster_id = 0;

        for i in 0..features.len() {
            if labels[i] != -1 {
                continue;
            }

            let neighbors = self.find_neighbors(features, i, eps);
            if neighbors.len() < min_pts {
                labels[i] = -1; // Noise point
                continue;
            }

            // Start new cluster
            cluster_id += 1;
            labels[i] = cluster_id;

            let mut seed_set = neighbors;
            let mut j = 0;

            while j < seed_set.len() {
                let q = seed_set[j];
                if labels[q] == -1 {
                    labels[q] = cluster_id;
                }

                if labels[q] != -1 {
                    continue;
                }

                labels[q] = cluster_id;
                let q_neighbors = self.find_neighbors(features, q, eps);
                if q_neighbors.len() >= min_pts {
                    seed_set.extend(q_neighbors);
                }

                j += 1;
            }
        }

        Ok(labels)
    }

    /// Hierarchical clustering
    async fn hierarchical_clustering(&self, features: &[Vec<f64>], k: usize) -> BridgeResult<Vec<i32>> {
        if features.is_empty() {
            return Ok(vec![]);
        }

        let n = features.len();
        if n <= k {
            // If we have fewer points than clusters, assign each point to its own cluster
            return Ok((0..n as i32).collect());
        }

        // Initialize each point as its own cluster
        let mut clusters: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
        let mut labels: Vec<i32> = (0..n as i32).collect();

        // Perform hierarchical clustering until we have k clusters
        while clusters.len() > k {
            let mut min_distance = f64::INFINITY;
            let mut merge_i = 0;
            let mut merge_j = 0;

            // Find the two closest clusters
            for i in 0..clusters.len() {
                for j in (i + 1)..clusters.len() {
                    let distance = self.cluster_distance(&clusters[i], &clusters[j], features);
                    if distance < min_distance {
                        min_distance = distance;
                        merge_i = i;
                        merge_j = j;
                    }
                }
            }

            // Merge clusters
            let cluster_j = clusters.remove(merge_j);
            clusters[merge_i].extend(cluster_j);

            // Update labels
            for &point_idx in &clusters[merge_i] {
                labels[point_idx] = merge_i as i32;
            }

            // Adjust labels for clusters after the removed one
            for cluster_idx in merge_j..clusters.len() {
                for &point_idx in &clusters[cluster_idx] {
                    labels[point_idx] = cluster_idx as i32;
                }
            }
        }

        Ok(labels)
    }

    /// Spectral clustering
    async fn spectral_clustering(&self, features: &[Vec<f64>], k: usize) -> BridgeResult<Vec<i32>> {
        if features.is_empty() {
            return Ok(vec![]);
        }

        let n = features.len();
        if n <= k {
            return Ok((0..n as i32).collect());
        }

        // Build similarity matrix
        let mut similarity_matrix = vec![vec![0.0; n]; n];
        let sigma = self.calculate_sigma(features);

        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let distance = self.euclidean_distance(&features[i], &features[j]);
                    similarity_matrix[i][j] = (-distance * distance / (2.0 * sigma * sigma)).exp();
                } else {
                    similarity_matrix[i][j] = 1.0;
                }
            }
        }

        // Build Laplacian matrix
        let mut laplacian = vec![vec![0.0; n]; n];
        let mut degrees = vec![0.0; n];

        // Calculate degrees
        for i in 0..n {
            degrees[i] = similarity_matrix[i].iter().sum();
        }

        // Build normalized Laplacian
        for i in 0..n {
            for j in 0..n {
                if degrees[i] > 0.0 && degrees[j] > 0.0 {
                    laplacian[i][j] = if i == j {
                        1.0 - similarity_matrix[i][j] / degrees[i]
                    } else {
                        -similarity_matrix[i][j] / (degrees[i] * degrees[j]).sqrt()
                    };
                }
            }
        }

        // For simplicity, we'll use a simplified approach
        // In a real implementation, you would compute eigenvectors and use k-means on them
        // For now, we'll use the similarity matrix directly with k-means
        let mut cluster_features = Vec::new();
        for i in 0..n {
            cluster_features.push(similarity_matrix[i].clone());
        }

        // Apply k-means to the similarity features
        self.kmeans_clustering(&cluster_features, k).await
    }

    /// Auto-select best clustering algorithm
    async fn auto_clustering(&self, features: &[Vec<f64>]) -> BridgeResult<Vec<i32>> {
        // Simple heuristic: use DBSCAN for small datasets, K-means for larger ones
        if features.len() < 50 {
            self.dbscan_clustering(features).await
        } else {
            let k = (features.len() as f64).sqrt() as usize;
            let k = k.max(2).min(10);
            self.kmeans_clustering(features, k).await
        }
    }

    /// Generate cluster results from labels
    async fn generate_cluster_results(
        &self,
        features: &[Vec<f64>],
        labels: &[i32],
        data: &TimeSeriesData,
    ) -> BridgeResult<Vec<ClusterResult>> {
        let mut cluster_results = Vec::new();
        let mut cluster_groups: HashMap<i32, Vec<usize>> = HashMap::new();

        // Group points by cluster
        for (i, &label) in labels.iter().enumerate() {
            cluster_groups.entry(label).or_insert_with(Vec::new).push(i);
        }

        // Generate results for each cluster
        for (&cluster_id, indices) in cluster_groups.iter() {
            if cluster_id == -1 {
                continue; // Skip noise points
            }

            let cluster_features: Vec<&Vec<f64>> = indices.iter().map(|&i| &features[i]).collect();
            let center = self.calculate_cluster_center(&cluster_features);
            let score = self.calculate_cluster_score(&cluster_features, &center);

            let cluster_type = match self.get_algorithm() {
                ClusteringAlgorithm::KMeans => ClusterType::KMeans,
                ClusteringAlgorithm::DBSCAN => ClusterType::DBSCAN,
                ClusteringAlgorithm::Hierarchical => ClusterType::Hierarchical,
                ClusteringAlgorithm::Spectral => ClusterType::Spectral,
                ClusteringAlgorithm::Auto => ClusterType::KMeans, // Default for auto
            };

            let result = ClusterResult {
                id: Uuid::new_v4(),
                cluster_type,
                label: cluster_id,
                center,
                size: indices.len(),
                score,
                metadata: HashMap::from([
                    ("algorithm".to_string(), format!("{:?}", self.get_algorithm())),
                    ("cluster_id".to_string(), cluster_id.to_string()),
                    ("feature_dimensions".to_string(), features[0].len().to_string()),
                ]),
            };

            cluster_results.push(result);
        }

        Ok(cluster_results)
    }

    /// Generate cluster results from raw data and labels
    async fn generate_cluster_results_from_raw_data(
        &self,
        data: &[Vec<f64>],
        labels: &[i32],
        original_labels: &[String],
    ) -> BridgeResult<Vec<ClusterResult>> {
        let mut cluster_results = Vec::new();
        let mut cluster_groups: HashMap<i32, Vec<usize>> = HashMap::new();

        // Group points by cluster
        for (i, &label) in labels.iter().enumerate() {
            cluster_groups.entry(label).or_insert_with(Vec::new).push(i);
        }

        // Generate results for each cluster
        for (&cluster_id, indices) in cluster_groups.iter() {
            if cluster_id == -1 {
                continue; // Skip noise points
            }

            let cluster_features: Vec<&Vec<f64>> = indices.iter().map(|&i| &data[i]).collect();
            let center = self.calculate_cluster_center(&cluster_features);
            let score = self.calculate_cluster_score(&cluster_features, &center);

            let cluster_type = match self.get_algorithm() {
                ClusteringAlgorithm::KMeans => ClusterType::KMeans,
                ClusteringAlgorithm::DBSCAN => ClusterType::DBSCAN,
                ClusteringAlgorithm::Hierarchical => ClusterType::Hierarchical,
                ClusteringAlgorithm::Spectral => ClusterType::Spectral,
                ClusteringAlgorithm::Auto => ClusterType::KMeans, // Default for auto
            };

            let result = ClusterResult {
                id: Uuid::new_v4(),
                cluster_type,
                label: cluster_id,
                center,
                size: indices.len(),
                score,
                metadata: HashMap::from([
                    ("algorithm".to_string(), format!("{:?}", self.get_algorithm())),
                    ("cluster_id".to_string(), cluster_id.to_string()),
                    ("feature_dimensions".to_string(), data[0].len().to_string()),
                ]),
            };

            cluster_results.push(result);
        }

        Ok(cluster_results)
    }

    // Helper methods

    /// Initialize centroids for k-means
    fn initialize_centroids(&self, features: &[Vec<f64>], k: usize) -> Vec<Vec<f64>> {
        let mut centroids = Vec::new();
        let feature_dim = features[0].len();

        // Use k-means++ initialization
        let mut rng = fastrand::Rng::new();
        
        // Choose first centroid randomly
        let first_centroid = features[rng.usize(..features.len())].clone();
        centroids.push(first_centroid);

        // Choose remaining centroids
        for _ in 1..k {
            let mut distances = Vec::new();
            let mut total_distance = 0.0;

            for feature in features {
                let min_distance = centroids
                    .iter()
                    .map(|centroid| self.euclidean_distance(feature, centroid))
                    .fold(f64::INFINITY, f64::min);
                distances.push(min_distance);
                total_distance += min_distance;
            }

            // Choose next centroid with probability proportional to distance squared
            let threshold = rng.f64() * total_distance;
            let mut cumulative = 0.0;
            for (i, &distance) in distances.iter().enumerate() {
                cumulative += distance;
                if cumulative >= threshold {
                    centroids.push(features[i].clone());
                    break;
                }
            }
        }

        centroids
    }

    /// Calculate Euclidean distance between two feature vectors
    fn euclidean_distance(&self, a: &[f64], b: &[f64]) -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    /// Calculate variance of a vector
    fn calculate_variance(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (values.len() - 1) as f64
    }

    /// Calculate trend (slope) of a time series
    fn calculate_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let n = values.len() as f64;
        let x_values: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
        
        let sum_x: f64 = x_values.iter().sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = x_values.iter().zip(values.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = x_values.iter().map(|x| x * x).sum();
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        slope
    }

    /// Calculate autocorrelation at given lag
    fn calculate_autocorrelation(&self, values: &[f64], lag: usize) -> f64 {
        if values.len() <= lag {
            return 0.0;
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = self.calculate_variance(values);

        if variance == 0.0 {
            return 0.0;
        }

        let mut autocorr = 0.0;
        for i in 0..(values.len() - lag) {
            autocorr += (values[i] - mean) * (values[i + lag] - mean);
        }

        autocorr / ((values.len() - lag) as f64 * variance)
    }

    /// Find neighbors within epsilon distance
    fn find_neighbors(&self, features: &[Vec<f64>], point_idx: usize, eps: f64) -> Vec<usize> {
        let mut neighbors = Vec::new();
        for (i, feature) in features.iter().enumerate() {
            if i != point_idx && self.euclidean_distance(&features[point_idx], feature) <= eps {
                neighbors.push(i);
            }
        }
        neighbors
    }

    /// Calculate DBSCAN epsilon parameter
    fn get_dbscan_eps(&self, features: &[Vec<f64>]) -> f64 {
        // Simple heuristic: use average distance to 5th nearest neighbor
        let k = 5.min(features.len() - 1);
        let mut distances = Vec::new();

        for i in 0..features.len() {
            let mut point_distances = Vec::new();
            for j in 0..features.len() {
                if i != j {
                    point_distances.push(self.euclidean_distance(&features[i], &features[j]));
                }
            }
            point_distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if point_distances.len() >= k {
                distances.push(point_distances[k - 1]);
            }
        }

        if distances.is_empty() {
            1.0 // Default value
        } else {
            distances.iter().sum::<f64>() / distances.len() as f64
        }
    }

    /// Calculate distance between two clusters
    fn cluster_distance(&self, cluster1: &[usize], cluster2: &[usize], features: &[Vec<f64>]) -> f64 {
        let mut min_distance = f64::INFINITY;
        for &i in cluster1 {
            for &j in cluster2 {
                let distance = self.euclidean_distance(&features[i], &features[j]);
                min_distance = min_distance.min(distance);
            }
        }
        min_distance
    }

    /// Calculate sigma for spectral clustering
    fn calculate_sigma(&self, features: &[Vec<f64>]) -> f64 {
        let mut distances = Vec::new();
        for i in 0..features.len() {
            for j in (i + 1)..features.len() {
                distances.push(self.euclidean_distance(&features[i], &features[j]));
            }
        }
        
        if distances.is_empty() {
            return 1.0;
        }

        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_idx = distances.len() / 2;
        distances[median_idx]
    }

    /// Calculate cluster center
    fn calculate_cluster_center(&self, cluster_features: &[&Vec<f64>]) -> Vec<f64> {
        if cluster_features.is_empty() {
            return vec![];
        }

        let feature_dim = cluster_features[0].len();
        let mut center = vec![0.0; feature_dim];

        for feature in cluster_features {
            for (dim, &value) in feature.iter().enumerate() {
                center[dim] += value;
            }
        }

        for value in &mut center {
            *value /= cluster_features.len() as f64;
        }

        center
    }

    /// Calculate cluster score (silhouette-like)
    fn calculate_cluster_score(&self, cluster_features: &[&Vec<f64>], center: &[f64]) -> f64 {
        if cluster_features.is_empty() {
            return 0.0;
        }

        // Calculate average distance to center
        let avg_distance = cluster_features
            .iter()
            .map(|feature| self.euclidean_distance(feature, center))
            .sum::<f64>() / cluster_features.len() as f64;

        // Convert to a score between 0 and 1 (lower distance = higher score)
        1.0 / (1.0 + avg_distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_clustering_algorithms() {
        // Create sample time series data with different patterns
        let mut points = Vec::new();
        let base_time = Utc::now();
        
        // Create three distinct patterns: increasing, decreasing, and stable
        for i in 0..15 {
            let timestamp = base_time + chrono::Duration::seconds(i as i64);
            let value = match i {
                0..=4 => 100.0 + (i as f64 * 2.0), // Increasing trend
                5..=9 => 200.0 - (i as f64 * 1.5), // Decreasing trend
                _ => 150.0 + (i % 3) as f64, // Stable with small variations
            };
            points.push(TimeSeriesPoint {
                timestamp,
                value,
                metadata: HashMap::new(),
            });
        }

        let data = TimeSeriesData {
            id: Uuid::new_v4(),
            name: "test_clustering_series".to_string(),
            points,
            metadata: HashMap::new(),
        };

        // Test each clustering algorithm
        let algorithms = vec![
            ClusteringAlgorithm::KMeans,
            ClusteringAlgorithm::DBSCAN,
            ClusteringAlgorithm::Hierarchical,
            ClusteringAlgorithm::Auto,
        ];

        for algorithm in algorithms {
            let config = ClusterAnalyzerConfig {
                name: "test_cluster_analyzer".to_string(),
                version: "1.0.0".to_string(),
                enable_clustering: true,
                num_clusters: 3,
                algorithm: algorithm.clone(),
                additional_config: HashMap::new(),
            };

            let analyzer = DefaultClusterAnalyzer::new(config);
            let clusters = analyzer.cluster(&data).await.unwrap();

            // Verify clusters
            assert!(!clusters.is_empty(), "Should generate at least one cluster for algorithm {:?}", algorithm);
            
            for cluster in &clusters {
                assert!(cluster.size > 0, "Cluster should have at least one point");
                assert!(cluster.score >= 0.0 && cluster.score <= 1.0, "Cluster score should be between 0 and 1");
                assert!(!cluster.center.is_empty(), "Cluster center should not be empty");
                assert!(!cluster.metadata.is_empty(), "Cluster metadata should not be empty");
            }

            println!("✅ {:?} algorithm: Generated {} clusters", algorithm, clusters.len());
            
            // Print cluster details
            for cluster in &clusters {
                println!("  - Cluster {}: {} points, score: {:.3}", 
                    cluster.label, cluster.size, cluster.score);
            }
        }
    }

    #[test]
    fn test_feature_extraction() {
        let analyzer = DefaultClusterAnalyzer::new(ClusterAnalyzerConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enable_clustering: true,
            num_clusters: 3,
            algorithm: ClusteringAlgorithm::KMeans,
            additional_config: HashMap::new(),
        });

        // Create test data
        let mut points = Vec::new();
        let base_time = Utc::now();
        
        for i in 0..20 {
            let timestamp = base_time + chrono::Duration::seconds(i as i64);
            let value = 100.0 + (i as f64 * 5.0); // Linear trend
            points.push(TimeSeriesPoint {
                timestamp,
                value,
                metadata: HashMap::new(),
            });
        }

        let data = TimeSeriesData {
            id: Uuid::new_v4(),
            name: "test_series".to_string(),
            points,
            metadata: HashMap::new(),
        };

        let features = analyzer.extract_features(&data).unwrap();
        
        assert!(!features.is_empty(), "Should extract features");
        assert_eq!(features[0].len(), 5, "Should extract 5 features per window");
        
        // Check that features are reasonable
        for feature in &features {
            assert!(feature.len() == 5, "Each feature vector should have 5 dimensions");
            // Mean should be positive for our test data
            assert!(feature[0] > 0.0, "Mean should be positive");
            // Standard deviation should be non-negative
            assert!(feature[1] >= 0.0, "Standard deviation should be non-negative");
        }
    }

    #[test]
    fn test_kmeans_clustering() {
        let analyzer = DefaultClusterAnalyzer::new(ClusterAnalyzerConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enable_clustering: true,
            num_clusters: 3,
            algorithm: ClusteringAlgorithm::KMeans,
            additional_config: HashMap::new(),
        });

        // Create test features with 3 distinct clusters
        let features = vec![
            vec![1.0, 1.0, 1.0, 1.0, 1.0], // Cluster 1
            vec![1.1, 1.1, 1.1, 1.1, 1.1],
            vec![1.2, 1.2, 1.2, 1.2, 1.2],
            vec![10.0, 10.0, 10.0, 10.0, 10.0], // Cluster 2
            vec![10.1, 10.1, 10.1, 10.1, 10.1],
            vec![10.2, 10.2, 10.2, 10.2, 10.2],
            vec![20.0, 20.0, 20.0, 20.0, 20.0], // Cluster 3
            vec![20.1, 20.1, 20.1, 20.1, 20.1],
            vec![20.2, 20.2, 20.2, 20.2, 20.2],
        ];

        let labels = tokio::runtime::Runtime::new().unwrap().block_on(
            analyzer.kmeans_clustering(&features, 3)
        ).unwrap();

        assert_eq!(labels.len(), features.len(), "Should assign a label to each point");
        
        // Check that we have 3 different cluster labels
        let unique_labels: std::collections::HashSet<i32> = labels.iter().cloned().collect();
        assert_eq!(unique_labels.len(), 3, "Should have 3 distinct clusters");
    }

    #[test]
    fn test_euclidean_distance() {
        let analyzer = DefaultClusterAnalyzer::new(ClusterAnalyzerConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enable_clustering: true,
            num_clusters: 3,
            algorithm: ClusteringAlgorithm::KMeans,
            additional_config: HashMap::new(),
        });

        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        
        let distance = analyzer.euclidean_distance(&a, &b);
        let expected = (3.0_f64.powi(2) + 3.0_f64.powi(2) + 3.0_f64.powi(2)).sqrt();
        
        assert!((distance - expected).abs() < 0.001, "Distance calculation should be correct");
    }

    #[test]
    fn test_variance_calculation() {
        let analyzer = DefaultClusterAnalyzer::new(ClusterAnalyzerConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enable_clustering: true,
            num_clusters: 3,
            algorithm: ClusteringAlgorithm::KMeans,
            additional_config: HashMap::new(),
        });

        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let variance = analyzer.calculate_variance(&values);
        
        // Expected variance for [1,2,3,4,5] is 2.5
        assert!((variance - 2.5).abs() < 0.001, "Variance calculation should be correct");
    }

    #[test]
    fn test_trend_calculation() {
        let analyzer = DefaultClusterAnalyzer::new(ClusterAnalyzerConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enable_clustering: true,
            num_clusters: 3,
            algorithm: ClusteringAlgorithm::KMeans,
            additional_config: HashMap::new(),
        });

        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let trend = analyzer.calculate_trend(&values);
        
        // Expected slope for linear trend [1,2,3,4,5] is 1.0
        assert!((trend - 1.0).abs() < 0.001, "Trend calculation should be correct");
    }
}
