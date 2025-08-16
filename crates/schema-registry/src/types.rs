//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Common types for the Schema Registry
//!
//! This module provides common types and utilities used throughout
//! the schema registry.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Component schema for OpenTelemetry components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSchema {
    /// Component name
    pub name: String,

    /// Component version
    pub version: String,

    /// Component type
    pub component_type: String,

    /// Schema content
    pub schema: serde_json::Value,

    /// Schema format
    pub format: String,

    /// Component description
    pub description: Option<String>,

    /// Component tags
    pub tags: Vec<String>,
}

/// Schema registry response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,

    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Response metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Request ID
    pub request_id: Uuid,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,

    /// Response status
    pub status: String,

    /// Additional metadata
    pub additional: HashMap<String, String>,
}

impl<T> ApiResponse<T> {
    /// Create a new API response
    pub fn new(data: T) -> Self {
        Self {
            data,
            metadata: ResponseMetadata {
                request_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                status: "success".to_string(),
                additional: HashMap::new(),
            },
        }
    }

    /// Create a new API response with custom metadata
    pub fn with_metadata(data: T, metadata: ResponseMetadata) -> Self {
        Self { data, metadata }
    }
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-based)
    pub page: Option<u32>,

    /// Page size
    pub page_size: Option<u32>,

    /// Total count
    pub total_count: Option<u64>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(100),
            total_count: None,
        }
    }
}

/// Paginated response
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Response data
    pub data: Vec<T>,

    /// Pagination information
    pub pagination: PaginationInfo,
}

/// Pagination information
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// Current page
    pub current_page: u32,

    /// Page size
    pub page_size: u32,

    /// Total pages
    pub total_pages: u32,

    /// Total count
    pub total_count: u64,

    /// Has next page
    pub has_next: bool,

    /// Has previous page
    pub has_previous: bool,
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(data: Vec<T>, pagination: PaginationInfo) -> Self {
        Self { data, pagination }
    }

    /// Create from data with pagination parameters
    pub fn from_data(data: Vec<T>, params: &PaginationParams, total_count: u64) -> Self {
        let page = params.page.unwrap_or(1);
        let page_size = params.page_size.unwrap_or(100);
        let total_pages = (total_count as f64 / page_size as f64).ceil() as u32;

        let pagination = PaginationInfo {
            current_page: page,
            page_size,
            total_pages,
            total_count,
            has_next: page < total_pages,
            has_previous: page > 1,
        };

        Self { data, pagination }
    }
}

/// Sort parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortParams {
    /// Sort field
    pub field: String,

    /// Sort direction
    pub direction: SortDirection,
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    /// Ascending order
    Asc,

    /// Descending order
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Desc
    }
}

/// Filter parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterParams {
    /// Filter field
    pub field: String,

    /// Filter operator
    pub operator: FilterOperator,

    /// Filter value
    pub value: String,
}

/// Filter operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    /// Equals
    Eq,

    /// Not equals
    Ne,

    /// Greater than
    Gt,

    /// Greater than or equal
    Gte,

    /// Less than
    Lt,

    /// Less than or equal
    Lte,

    /// Contains
    Contains,

    /// Starts with
    StartsWith,

    /// Ends with
    EndsWith,

    /// In list
    In,

    /// Not in list
    NotIn,
}

/// Query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    /// Pagination parameters
    pub pagination: Option<PaginationParams>,

    /// Sort parameters
    pub sort: Option<SortParams>,

    /// Filter parameters
    pub filters: Vec<FilterParams>,

    /// Search query
    pub search: Option<String>,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            pagination: Some(PaginationParams::default()),
            sort: None,
            filters: Vec::new(),
            search: None,
        }
    }
}

/// Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error details
    pub details: Option<String>,

    /// Error timestamp
    pub timestamp: DateTime<Utc>,

    /// Request ID
    pub request_id: Option<Uuid>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: String, message: String) -> Self {
        Self {
            code,
            message,
            details: None,
            timestamp: Utc::now(),
            request_id: None,
        }
    }

    /// Create with details
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Create with request ID
    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

/// Success response
#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessResponse<T> {
    /// Success status
    pub success: bool,

    /// Response data
    pub data: T,

    /// Response message
    pub message: Option<String>,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,
}

impl<T> SuccessResponse<T> {
    /// Create a new success response
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            data,
            message: None,
            timestamp: Utc::now(),
        }
    }

    /// Create with message
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Health status
    pub status: HealthStatus,

    /// Service name
    pub service: String,

    /// Service version
    pub version: String,

    /// Check timestamp
    pub timestamp: DateTime<Utc>,

    /// Component health checks
    pub components: HashMap<String, ComponentHealth>,
}

/// Health status
#[derive(Debug, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Healthy
    Healthy,

    /// Unhealthy
    Unhealthy,

    /// Degraded
    Degraded,
}

/// Component health
#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component status
    pub status: HealthStatus,

    /// Component message
    pub message: Option<String>,

    /// Component details
    pub details: Option<HashMap<String, String>>,
}

impl ComponentHealth {
    /// Create a healthy component
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: None,
            details: None,
        }
    }

    /// Create an unhealthy component
    pub fn unhealthy(message: String) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            message: Some(message),
            details: None,
        }
    }

    /// Create a degraded component
    pub fn degraded(message: String) -> Self {
        Self {
            status: HealthStatus::Degraded,
            message: Some(message),
            details: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_creation() {
        let data = "test data";
        let response = ApiResponse::new(data);

        assert_eq!(response.data, "test data");
        assert_eq!(response.metadata.status, "success");
        assert!(response.metadata.additional.is_empty());
    }

    #[test]
    fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.page, Some(1));
        assert_eq!(params.page_size, Some(100));
        assert_eq!(params.total_count, None);
    }

    #[test]
    fn test_paginated_response_creation() {
        let data = vec!["item1", "item2", "item3"];
        let params = PaginationParams::default();
        let response = PaginatedResponse::from_data(data, &params, 100);

        assert_eq!(response.data.len(), 3);
        assert_eq!(response.pagination.current_page, 1);
        assert_eq!(response.pagination.page_size, 100);
        assert_eq!(response.pagination.total_count, 100);
        assert!(!response.pagination.has_next);
        assert!(!response.pagination.has_previous);
    }

    #[test]
    fn test_error_response_creation() {
        let error = ErrorResponse::new("TEST_ERROR".to_string(), "Test error message".to_string());

        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test error message");
        assert!(error.details.is_none());
        assert!(error.request_id.is_none());
    }

    #[test]
    fn test_success_response_creation() {
        let data = "test data";
        let response = SuccessResponse::new(data);

        assert!(response.success);
        assert_eq!(response.data, "test data");
        assert!(response.message.is_none());
    }

    #[test]
    fn test_component_health_creation() {
        let healthy = ComponentHealth::healthy();
        assert!(matches!(healthy.status, HealthStatus::Healthy));

        let unhealthy = ComponentHealth::unhealthy("Test error".to_string());
        assert!(matches!(unhealthy.status, HealthStatus::Unhealthy));
        assert_eq!(unhealthy.message, Some("Test error".to_string()));

        let degraded = ComponentHealth::degraded("Test warning".to_string());
        assert!(matches!(degraded.status, HealthStatus::Degraded));
        assert_eq!(degraded.message, Some("Test warning".to_string()));
    }
}
