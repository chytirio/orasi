//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Repository generators for creating realistic repository operation patterns
//!
//! This module provides generators for creating realistic repository operations
//! and user behavior patterns that simulate real development workflows.

use super::utils::{DistributionGenerator, IdGenerator, TimeGenerator};
use crate::config::GeneratorConfig;
use crate::models::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Generator for repository operations
pub struct RepositoryGenerator {
    config: Arc<GeneratorConfig>,
    id_generator: IdGenerator,
    time_generator: TimeGenerator,
    distribution_generator: DistributionGenerator,
}

impl RepositoryGenerator {
    /// Create a new repository generator
    pub fn new(config: Arc<GeneratorConfig>) -> Result<Self> {
        let id_generator = IdGenerator::new(Arc::clone(&config))?;
        let time_generator = TimeGenerator::new(Arc::clone(&config))?;
        let distribution_generator = DistributionGenerator::new(Arc::clone(&config))?;

        Ok(Self {
            config,
            id_generator,
            time_generator,
            distribution_generator,
        })
    }

    /// Generate repository operations
    pub async fn generate_operations(&self) -> Result<Vec<RepositoryOperation>> {
        let mut operations = Vec::new();

        // Generate pull operations
        for _ in 0..5 {
            operations.push(self.generate_pull_operation().await?);
        }

        // Generate commit operations
        for _ in 0..10 {
            operations.push(self.generate_commit_operation().await?);
        }

        // Generate push operations
        for _ in 0..8 {
            operations.push(self.generate_push_operation().await?);
        }

        // Generate merge operations
        for _ in 0..3 {
            operations.push(self.generate_merge_operation().await?);
        }

        Ok(operations)
    }

    /// Generate development scenario operations
    pub async fn generate_development_operations(&self) -> Result<Vec<RepositoryOperation>> {
        let mut operations = Vec::new();

        // Focus on development-related operations
        for _ in 0..15 {
            operations.push(self.generate_commit_operation().await?);
        }

        for _ in 0..10 {
            operations.push(self.generate_push_operation().await?);
        }

        for _ in 0..5 {
            operations.push(self.generate_pull_operation().await?);
        }

        Ok(operations)
    }

    /// Generate failure scenario operations
    pub async fn generate_failure_operations(&self) -> Result<Vec<RepositoryOperation>> {
        let mut operations = Vec::new();

        // Generate some successful operations
        for _ in 0..5 {
            operations.push(self.generate_commit_operation().await?);
        }

        // Generate failed operations
        for _ in 0..3 {
            operations.push(self.generate_failed_push_operation().await?);
        }

        for _ in 0..2 {
            operations.push(self.generate_failed_merge_operation().await?);
        }

        Ok(operations)
    }

    /// Generate scale scenario operations
    pub async fn generate_scale_operations(&self) -> Result<Vec<RepositoryOperation>> {
        let mut operations = Vec::new();

        // Generate many operations for scale testing
        for _ in 0..50 {
            operations.push(self.generate_commit_operation().await?);
        }

        for _ in 0..30 {
            operations.push(self.generate_push_operation().await?);
        }

        for _ in 0..20 {
            operations.push(self.generate_pull_operation().await?);
        }

        for _ in 0..10 {
            operations.push(self.generate_merge_operation().await?);
        }

        Ok(operations)
    }

    /// Generate evolution scenario operations
    pub async fn generate_evolution_operations(&self) -> Result<Vec<RepositoryOperation>> {
        let mut operations = Vec::new();

        // Generate operations related to schema evolution
        for _ in 0..10 {
            operations.push(self.generate_schema_evolution_commit().await?);
        }

        for _ in 0..5 {
            operations.push(self.generate_migration_commit().await?);
        }

        for _ in 0..3 {
            operations.push(self.generate_rollback_operation().await?);
        }

        Ok(operations)
    }

    /// Generate a pull operation
    async fn generate_pull_operation(&self) -> Result<RepositoryOperation> {
        let mut parameters = HashMap::new();
        parameters.insert("branch".to_string(), "main".to_string());
        parameters.insert("remote".to_string(), "origin".to_string());

        let mut result_data = HashMap::new();
        result_data.insert(
            "files_updated".to_string(),
            serde_json::Value::Number(5.into()),
        );
        result_data.insert("conflicts".to_string(), serde_json::Value::Number(0.into()));

        let mut metrics = HashMap::new();
        metrics.insert("duration_ms".to_string(), 1500.0);
        metrics.insert("bytes_transferred".to_string(), 1024000.0);

        Ok(RepositoryOperation {
            id: self.id_generator.generate_uuid().await,
            operation_type: RepositoryOperationType::Pull,
            repository: self.generate_repository_info().await?,
            parameters,
            result: OperationResult {
                success: true,
                data: result_data,
                error: None,
                metrics,
            },
            timestamp: self.time_generator.generate_timestamp().await,
            duration: chrono::Duration::milliseconds(1500),
        })
    }

    /// Generate a commit operation
    async fn generate_commit_operation(&self) -> Result<RepositoryOperation> {
        let mut parameters = HashMap::new();
        parameters.insert(
            "message".to_string(),
            "Update feature implementation".to_string(),
        );
        parameters.insert("author".to_string(), "developer@example.com".to_string());

        let mut result_data = HashMap::new();
        result_data.insert(
            "commit_hash".to_string(),
            serde_json::Value::String("abc123def456".to_string()),
        );
        result_data.insert(
            "files_changed".to_string(),
            serde_json::Value::Number(3.into()),
        );
        result_data.insert(
            "lines_added".to_string(),
            serde_json::Value::Number(45.into()),
        );
        result_data.insert(
            "lines_deleted".to_string(),
            serde_json::Value::Number(12.into()),
        );

        let mut metrics = HashMap::new();
        metrics.insert("duration_ms".to_string(), 500.0);
        metrics.insert("commit_size_bytes".to_string(), 2048.0);

        Ok(RepositoryOperation {
            id: self.id_generator.generate_uuid().await,
            operation_type: RepositoryOperationType::Commit,
            repository: self.generate_repository_info().await?,
            parameters,
            result: OperationResult {
                success: true,
                data: result_data,
                error: None,
                metrics,
            },
            timestamp: self.time_generator.generate_timestamp().await,
            duration: chrono::Duration::milliseconds(500),
        })
    }

    /// Generate a push operation
    async fn generate_push_operation(&self) -> Result<RepositoryOperation> {
        let mut parameters = HashMap::new();
        parameters.insert("branch".to_string(), "main".to_string());
        parameters.insert("remote".to_string(), "origin".to_string());

        let mut result_data = HashMap::new();
        result_data.insert(
            "commits_pushed".to_string(),
            serde_json::Value::Number(2.into()),
        );
        result_data.insert(
            "refs_updated".to_string(),
            serde_json::Value::Number(1.into()),
        );

        let mut metrics = HashMap::new();
        metrics.insert("duration_ms".to_string(), 2000.0);
        metrics.insert("bytes_transferred".to_string(), 2048000.0);

        Ok(RepositoryOperation {
            id: self.id_generator.generate_uuid().await,
            operation_type: RepositoryOperationType::Push,
            repository: self.generate_repository_info().await?,
            parameters,
            result: OperationResult {
                success: true,
                data: result_data,
                error: None,
                metrics,
            },
            timestamp: self.time_generator.generate_timestamp().await,
            duration: chrono::Duration::milliseconds(2000),
        })
    }

    /// Generate a merge operation
    async fn generate_merge_operation(&self) -> Result<RepositoryOperation> {
        let mut parameters = HashMap::new();
        parameters.insert(
            "source_branch".to_string(),
            "feature/new-feature".to_string(),
        );
        parameters.insert("target_branch".to_string(), "main".to_string());
        parameters.insert("strategy".to_string(), "recursive".to_string());

        let mut result_data = HashMap::new();
        result_data.insert(
            "merge_commit_hash".to_string(),
            serde_json::Value::String("def456ghi789".to_string()),
        );
        result_data.insert(
            "conflicts_resolved".to_string(),
            serde_json::Value::Number(0.into()),
        );

        let mut metrics = HashMap::new();
        metrics.insert("duration_ms".to_string(), 3000.0);
        metrics.insert("files_merged".to_string(), 15.0);

        Ok(RepositoryOperation {
            id: self.id_generator.generate_uuid().await,
            operation_type: RepositoryOperationType::Merge,
            repository: self.generate_repository_info().await?,
            parameters,
            result: OperationResult {
                success: true,
                data: result_data,
                error: None,
                metrics,
            },
            timestamp: self.time_generator.generate_timestamp().await,
            duration: chrono::Duration::milliseconds(3000),
        })
    }

    /// Generate a failed push operation
    async fn generate_failed_push_operation(&self) -> Result<RepositoryOperation> {
        let mut parameters = HashMap::new();
        parameters.insert("branch".to_string(), "main".to_string());
        parameters.insert("remote".to_string(), "origin".to_string());

        let mut result_data = HashMap::new();
        result_data.insert(
            "commits_pushed".to_string(),
            serde_json::Value::Number(0.into()),
        );

        let mut metrics = HashMap::new();
        metrics.insert("duration_ms".to_string(), 5000.0);
        metrics.insert("retry_count".to_string(), 3.0);

        Ok(RepositoryOperation {
            id: self.id_generator.generate_uuid().await,
            operation_type: RepositoryOperationType::Push,
            repository: self.generate_repository_info().await?,
            parameters,
            result: OperationResult {
                success: false,
                data: result_data,
                error: Some("Remote repository rejected push due to conflicts".to_string()),
                metrics,
            },
            timestamp: self.time_generator.generate_timestamp().await,
            duration: chrono::Duration::milliseconds(5000),
        })
    }

    /// Generate a failed merge operation
    async fn generate_failed_merge_operation(&self) -> Result<RepositoryOperation> {
        let mut parameters = HashMap::new();
        parameters.insert(
            "source_branch".to_string(),
            "feature/conflicting-feature".to_string(),
        );
        parameters.insert("target_branch".to_string(), "main".to_string());
        parameters.insert("strategy".to_string(), "recursive".to_string());

        let mut result_data = HashMap::new();
        result_data.insert(
            "conflicts_found".to_string(),
            serde_json::Value::Number(3.into()),
        );
        result_data.insert(
            "files_with_conflicts".to_string(),
            serde_json::Value::Array(vec![
                serde_json::Value::String("src/main.rs".to_string()),
                serde_json::Value::String("src/lib.rs".to_string()),
                serde_json::Value::String("Cargo.toml".to_string()),
            ]),
        );

        let mut metrics = HashMap::new();
        metrics.insert("duration_ms".to_string(), 4000.0);
        metrics.insert("conflict_count".to_string(), 3.0);

        Ok(RepositoryOperation {
            id: self.id_generator.generate_uuid().await,
            operation_type: RepositoryOperationType::Merge,
            repository: self.generate_repository_info().await?,
            parameters,
            result: OperationResult {
                success: false,
                data: result_data,
                error: Some("Merge conflicts detected, manual resolution required".to_string()),
                metrics,
            },
            timestamp: self.time_generator.generate_timestamp().await,
            duration: chrono::Duration::milliseconds(4000),
        })
    }

    /// Generate a schema evolution commit
    async fn generate_schema_evolution_commit(&self) -> Result<RepositoryOperation> {
        let mut parameters = HashMap::new();
        parameters.insert(
            "message".to_string(),
            "feat: Add new schema fields for backward compatibility".to_string(),
        );
        parameters.insert(
            "author".to_string(),
            "schema-evolution@example.com".to_string(),
        );
        parameters.insert(
            "evolution_type".to_string(),
            "backward_compatible".to_string(),
        );

        let mut result_data = HashMap::new();
        result_data.insert(
            "commit_hash".to_string(),
            serde_json::Value::String("schema123evol456".to_string()),
        );
        result_data.insert(
            "files_changed".to_string(),
            serde_json::Value::Number(5.into()),
        );
        result_data.insert(
            "schema_version".to_string(),
            serde_json::Value::String("v2.1.0".to_string()),
        );
        result_data.insert(
            "migration_files".to_string(),
            serde_json::Value::Number(2.into()),
        );

        let mut metrics = HashMap::new();
        metrics.insert("duration_ms".to_string(), 800.0);
        metrics.insert("schema_complexity".to_string(), 0.7);

        Ok(RepositoryOperation {
            id: self.id_generator.generate_uuid().await,
            operation_type: RepositoryOperationType::Commit,
            repository: self.generate_repository_info().await?,
            parameters,
            result: OperationResult {
                success: true,
                data: result_data,
                error: None,
                metrics,
            },
            timestamp: self.time_generator.generate_timestamp().await,
            duration: chrono::Duration::milliseconds(800),
        })
    }

    /// Generate a migration commit
    async fn generate_migration_commit(&self) -> Result<RepositoryOperation> {
        let mut parameters = HashMap::new();
        parameters.insert(
            "message".to_string(),
            "feat: Add data migration for schema v2.1.0".to_string(),
        );
        parameters.insert("author".to_string(), "migration@example.com".to_string());
        parameters.insert("migration_type".to_string(), "data_migration".to_string());

        let mut result_data = HashMap::new();
        result_data.insert(
            "commit_hash".to_string(),
            serde_json::Value::String("mig789data012".to_string()),
        );
        result_data.insert(
            "migration_script".to_string(),
            serde_json::Value::String("migrations/001_schema_v2_1_0.sql".to_string()),
        );
        result_data.insert(
            "affected_tables".to_string(),
            serde_json::Value::Number(3.into()),
        );
        result_data.insert(
            "estimated_rows".to_string(),
            serde_json::Value::Number(10000.into()),
        );

        let mut metrics = HashMap::new();
        metrics.insert("duration_ms".to_string(), 1200.0);
        metrics.insert("migration_complexity".to_string(), 0.8);

        Ok(RepositoryOperation {
            id: self.id_generator.generate_uuid().await,
            operation_type: RepositoryOperationType::Commit,
            repository: self.generate_repository_info().await?,
            parameters,
            result: OperationResult {
                success: true,
                data: result_data,
                error: None,
                metrics,
            },
            timestamp: self.time_generator.generate_timestamp().await,
            duration: chrono::Duration::milliseconds(1200),
        })
    }

    /// Generate a rollback operation
    async fn generate_rollback_operation(&self) -> Result<RepositoryOperation> {
        let mut parameters = HashMap::new();
        parameters.insert("target_commit".to_string(), "abc123def456".to_string());
        parameters.insert(
            "reason".to_string(),
            "Schema migration caused performance issues".to_string(),
        );

        let mut result_data = HashMap::new();
        result_data.insert(
            "rollback_commit_hash".to_string(),
            serde_json::Value::String("roll789back012".to_string()),
        );
        result_data.insert(
            "reverted_files".to_string(),
            serde_json::Value::Number(5.into()),
        );
        result_data.insert(
            "rollback_duration".to_string(),
            serde_json::Value::String("2h 15m".to_string()),
        );

        let mut metrics = HashMap::new();
        metrics.insert("duration_ms".to_string(), 5000.0);
        metrics.insert("downtime_minutes".to_string(), 135.0);

        Ok(RepositoryOperation {
            id: self.id_generator.generate_uuid().await,
            operation_type: RepositoryOperationType::Reset,
            repository: self.generate_repository_info().await?,
            parameters,
            result: OperationResult {
                success: true,
                data: result_data,
                error: None,
                metrics,
            },
            timestamp: self.time_generator.generate_timestamp().await,
            duration: chrono::Duration::milliseconds(5000),
        })
    }

    /// Generate repository information
    async fn generate_repository_info(&self) -> Result<RepositoryInfo> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "project_name".to_string(),
            "test-data-generator".to_string(),
        );
        metadata.insert("repository_type".to_string(), "git".to_string());
        metadata.insert("default_branch".to_string(), "main".to_string());

        Ok(RepositoryInfo {
            url: "https://github.com/example/test-data-generator.git".to_string(),
            name: "test-data-generator".to_string(),
            branch: "main".to_string(),
            commit: Some("abc123def456789".to_string()),
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GeneratorConfig;

    #[tokio::test]
    async fn test_repository_generator_creation() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = RepositoryGenerator::new(config);
        assert!(generator.is_ok());
    }

    #[tokio::test]
    async fn test_operation_generation() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = RepositoryGenerator::new(config).unwrap();

        let operations = generator.generate_operations().await.unwrap();
        assert!(!operations.is_empty());

        // Check that we have different operation types
        let operation_types: std::collections::HashSet<_> =
            operations.iter().map(|o| &o.operation_type).collect();
        assert!(operation_types.len() > 1);
    }

    #[tokio::test]
    async fn test_development_operations() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = RepositoryGenerator::new(config).unwrap();

        let operations = generator.generate_development_operations().await.unwrap();
        assert!(!operations.is_empty());

        // Should have more commits and pushes for development
        let commits = operations
            .iter()
            .filter(|o| matches!(o.operation_type, RepositoryOperationType::Commit))
            .count();
        let pushes = operations
            .iter()
            .filter(|o| matches!(o.operation_type, RepositoryOperationType::Push))
            .count();

        assert!(commits > 0);
        assert!(pushes > 0);
    }

    #[tokio::test]
    async fn test_failure_operations() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = RepositoryGenerator::new(config).unwrap();

        let operations = generator.generate_failure_operations().await.unwrap();
        assert!(!operations.is_empty());

        // Should have some failed operations
        let failed_operations = operations.iter().filter(|o| !o.result.success).count();

        assert!(failed_operations > 0);
    }

    #[tokio::test]
    async fn test_scale_operations() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = RepositoryGenerator::new(config).unwrap();

        let operations = generator.generate_scale_operations().await.unwrap();
        assert!(operations.len() > 50); // Should have many operations for scale testing
    }

    #[tokio::test]
    async fn test_evolution_operations() {
        let config = Arc::new(GeneratorConfig::default());
        let generator = RepositoryGenerator::new(config).unwrap();

        let operations = generator.generate_evolution_operations().await.unwrap();
        assert!(!operations.is_empty());

        // Should have schema evolution related operations
        let evolution_commits = operations
            .iter()
            .filter(|o| {
                matches!(o.operation_type, RepositoryOperationType::Commit)
                    && o.parameters.get("evolution_type").is_some()
            })
            .count();

        assert!(evolution_commits > 0);
    }
}
