//! Schema metadata and search functionality
//!
//! This module contains structures for schema metadata and search criteria.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{types::*, version::SchemaVersion};

/// Schema metadata for listing and searching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Schema ID
    pub id: Uuid,

    /// Schema name
    pub name: String,

    /// Schema version
    pub version: SchemaVersion,

    /// Schema type
    pub schema_type: SchemaType,

    /// Schema format
    pub format: SchemaFormat,

    /// Schema fingerprint
    pub fingerprint: String,

    /// Schema description
    pub description: Option<String>,

    /// Schema tags
    pub tags: Vec<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,

    /// Schema owner
    pub owner: Option<String>,

    /// Schema visibility
    pub visibility: SchemaVisibility,

    /// Number of versions
    pub version_count: u64,

    /// Schema size in bytes
    pub size: usize,
}

/// Schema search criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSearchCriteria {
    /// Search by name (partial match)
    pub name: Option<String>,

    /// Search by type
    pub schema_type: Option<SchemaType>,

    /// Search by format
    pub format: Option<SchemaFormat>,

    /// Search by tags
    pub tags: Vec<String>,

    /// Search by owner
    pub owner: Option<String>,

    /// Search by visibility
    pub visibility: Option<SchemaVisibility>,

    /// Created after timestamp
    pub created_after: Option<DateTime<Utc>>,

    /// Created before timestamp
    pub created_before: Option<DateTime<Utc>>,

    /// Updated after timestamp
    pub updated_after: Option<DateTime<Utc>>,

    /// Updated before timestamp
    pub updated_before: Option<DateTime<Utc>>,

    /// Maximum number of results
    pub limit: Option<usize>,

    /// Offset for pagination
    pub offset: Option<usize>,
}

impl Default for SchemaSearchCriteria {
    fn default() -> Self {
        Self {
            name: None,
            schema_type: None,
            format: None,
            tags: Vec::new(),
            owner: None,
            visibility: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            limit: Some(100),
            offset: Some(0),
        }
    }
}
