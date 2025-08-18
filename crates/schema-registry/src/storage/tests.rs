//! Storage tests

use super::*;
use crate::schema::{Schema, SchemaFormat, SchemaSearchCriteria, SchemaType, SchemaVersion};

#[cfg(test)]
mod memory_tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage_creation() {
        let storage = MemoryStorage::new();
        assert!(storage.is_ok());
    }

    #[tokio::test]
    async fn test_store_and_retrieve_schema() {
        let storage = MemoryStorage::new().unwrap();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        let fingerprint = schema.fingerprint.clone();
        let version = storage.store_schema(schema).await.unwrap();
        assert_eq!(version.to_string(), "1.0.0");

        let retrieved = storage.get_schema(&fingerprint).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-schema");
    }

    #[tokio::test]
    async fn test_list_schemas() {
        let storage = MemoryStorage::new().unwrap();

        let schema1 = Schema::new(
            "schema1".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        let schema2 = Schema::new(
            "schema2".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Trace,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        storage.store_schema(schema1).await.unwrap();
        storage.store_schema(schema2).await.unwrap();

        let schemas = storage.list_schemas().await.unwrap();
        assert_eq!(schemas.len(), 1);
    }

    #[tokio::test]
    async fn test_search_schemas() {
        let storage = MemoryStorage::new().unwrap();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        storage.store_schema(schema).await.unwrap();

        let criteria = SchemaSearchCriteria {
            name: Some("test".to_string()),
            ..Default::default()
        };

        let results = storage.search_schemas(&criteria).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test-schema");
    }

    #[tokio::test]
    async fn test_delete_schema() {
        let storage = MemoryStorage::new().unwrap();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        let fingerprint = schema.fingerprint.clone();
        storage.store_schema(schema).await.unwrap();

        let deleted = storage.delete_schema(&fingerprint).await.unwrap();
        assert!(deleted);

        let retrieved = storage.get_schema(&fingerprint).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_health_check() {
        let storage = MemoryStorage::new().unwrap();
        let healthy = storage.health_check().await.unwrap();
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let storage = MemoryStorage::new().unwrap();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        storage.store_schema(schema).await.unwrap();

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_schemas, 1);
        assert_eq!(stats.total_versions, 1);
        assert!(stats.total_size_bytes > 0);
    }
}

// Note: Redis tests are not included here because they require a Redis server
// In a real CI/CD environment, you would use test containers or mock Redis
// For now, the Redis storage implementation is tested through integration tests
