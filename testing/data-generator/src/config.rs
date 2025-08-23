//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Configuration management for test data generation
//!
//! This module provides comprehensive configuration options for generating
//! realistic test data for the OpenTelemetry Data Lake Bridge.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration for test data generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Generator name and version
    pub name: String,
    pub version: String,

    /// Random seed for reproducible generation
    pub seed: Option<u64>,

    /// Output configuration
    pub output: OutputConfig,

    /// Workflow configuration
    pub workflow: WorkflowConfig,

    /// Scale configuration
    pub scale: ScaleConfig,

    /// Quality configuration
    pub quality: QualityConfig,

    /// Test scenarios to execute
    pub scenarios: Vec<TestScenario>,

    /// Additional data generation configuration
    pub additional_data: Option<AdditionalDataConfig>,

    /// Validation configuration
    pub validation: ValidationConfig,

    /// Export configuration
    pub export: ExportConfig,
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output directory
    pub directory: PathBuf,

    /// File format for output
    pub format: OutputFormat,

    /// Compression settings
    pub compression: CompressionConfig,

    /// Batch size for writing
    pub batch_size: usize,

    /// Maximum file size
    pub max_file_size: usize,
}

/// Output format options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Parquet,
    Avro,
    Csv,
    Delta,
    Iceberg,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub algorithm: CompressionAlgorithm,
    pub level: u32,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Gzip,
    Snappy,
    Zstd,
    Lz4,
}

/// Workflow configuration for agentic development patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Research workflow settings
    pub research: ResearchConfig,

    /// Coordination workflow settings
    pub coordination: CoordinationConfig,

    /// Implementation workflow settings
    pub implementation: ImplementationConfig,

    /// Agent interaction patterns
    pub agent_interactions: AgentInteractionConfig,

    /// Repository operations
    pub repository_operations: RepositoryOperationConfig,

    /// User behavior patterns
    pub user_behavior: UserBehaviorConfig,
}

/// Research workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConfig {
    /// Research session duration distribution
    pub session_duration: DurationDistribution,

    /// Research query patterns
    pub query_patterns: Vec<QueryPattern>,

    /// Knowledge graph interactions
    pub knowledge_graph_interactions: KnowledgeGraphConfig,

    /// Citation tracking patterns
    pub citation_tracking: CitationTrackingConfig,
}

/// Duration distribution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationDistribution {
    pub min_seconds: u64,
    pub max_seconds: u64,
    pub mean_seconds: u64,
    pub std_dev_seconds: u64,
    pub distribution_type: DistributionType,
}

/// Distribution types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributionType {
    Normal,
    Exponential,
    LogNormal,
    Uniform,
    PowerLaw,
}

/// Query pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPattern {
    pub name: String,
    pub frequency: f64,
    pub complexity: QueryComplexity,
    pub attributes: HashMap<String, String>,
}

/// Query complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Knowledge graph configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphConfig {
    pub node_count: usize,
    pub edge_count: usize,
    pub query_frequency: f64,
    pub traversal_depth: usize,
}

/// Citation tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationTrackingConfig {
    pub citation_count: usize,
    pub source_diversity: f64,
    pub tracking_frequency: f64,
}

/// Coordination workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationConfig {
    /// Multi-repo coordination patterns
    pub multi_repo_coordination: MultiRepoConfig,

    /// Agent coordination patterns
    pub agent_coordination: AgentCoordinationConfig,

    /// Conflict resolution patterns
    pub conflict_resolution: ConflictResolutionConfig,

    /// Decision making patterns
    pub decision_making: DecisionMakingConfig,
}

/// Multi-repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiRepoConfig {
    pub repository_count: usize,
    pub dependency_depth: usize,
    pub coordination_frequency: f64,
    pub merge_conflict_rate: f64,
}

/// Agent coordination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCoordinationConfig {
    pub agent_count: usize,
    pub coordination_patterns: Vec<CoordinationPattern>,
    pub handoff_frequency: f64,
    pub escalation_rate: f64,
}

/// Coordination pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationPattern {
    pub name: String,
    pub frequency: f64,
    pub complexity: CoordinationComplexity,
    pub attributes: HashMap<String, String>,
}

/// Coordination complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Conflict resolution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionConfig {
    pub conflict_rate: f64,
    pub resolution_time: DurationDistribution,
    pub resolution_strategies: Vec<ResolutionStrategy>,
}

/// Resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionStrategy {
    pub name: String,
    pub frequency: f64,
    pub success_rate: f64,
}

/// Decision making configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionMakingConfig {
    pub decision_frequency: f64,
    pub participant_count: usize,
    pub decision_time: DurationDistribution,
    pub consensus_rate: f64,
}

/// Implementation workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationConfig {
    /// Code generation patterns
    pub code_generation: CodeGenerationConfig,

    /// Testing patterns
    pub testing: TestingConfig,

    /// Deployment patterns
    pub deployment: DeploymentConfig,

    /// Code review patterns
    pub code_review: CodeReviewConfig,
}

/// Code generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenerationConfig {
    pub generation_frequency: f64,
    pub code_complexity: CodeComplexity,
    pub language_distribution: HashMap<String, f64>,
    pub generation_time: DurationDistribution,
}

/// Code complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingConfig {
    pub test_coverage: f64,
    pub test_types: Vec<TestType>,
    pub test_execution_time: DurationDistribution,
    pub failure_rate: f64,
}

/// Test types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestType {
    Unit,
    Integration,
    EndToEnd,
    Performance,
    Security,
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    pub deployment_frequency: f64,
    pub deployment_time: DurationDistribution,
    pub rollback_rate: f64,
    pub environment_count: usize,
}

/// Code review configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReviewConfig {
    pub review_frequency: f64,
    pub reviewer_count: usize,
    pub review_time: DurationDistribution,
    pub approval_rate: f64,
}

/// Agent interaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInteractionConfig {
    pub agent_types: Vec<AgentType>,
    pub interaction_patterns: Vec<InteractionPattern>,
    pub escalation_patterns: Vec<EscalationPattern>,
    pub collaboration_patterns: Vec<CollaborationPattern>,
}

/// Agent types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    ResearchAgent,
    CodeAgent,
    TestAgent,
    ReviewAgent,
    CoordinationAgent,
    SecurityAgent,
}

/// Interaction pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionPattern {
    pub name: String,
    pub frequency: f64,
    pub duration: DurationDistribution,
    pub complexity: InteractionComplexity,
}

/// Interaction complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Escalation pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPattern {
    pub name: String,
    pub frequency: f64,
    pub escalation_reasons: Vec<String>,
    pub resolution_time: DurationDistribution,
}

/// Collaboration pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationPattern {
    pub name: String,
    pub frequency: f64,
    pub participant_count: usize,
    pub collaboration_time: DurationDistribution,
}

/// Repository operation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryOperationConfig {
    pub operation_types: Vec<RepositoryOperationType>,
    pub operation_frequency: f64,
    pub operation_time: DurationDistribution,
    pub error_rate: f64,
}

/// Repository operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepositoryOperationType {
    Clone,
    Pull,
    Push,
    Commit,
    Merge,
    Branch,
    Tag,
    Rebase,
}

/// User behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehaviorConfig {
    pub user_personas: Vec<UserPersona>,
    pub activity_patterns: Vec<ActivityPattern>,
    pub session_patterns: Vec<SessionPattern>,
    pub interaction_patterns: Vec<UserInteractionPattern>,
}

/// User persona
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPersona {
    pub name: String,
    pub frequency: f64,
    pub characteristics: HashMap<String, String>,
    pub behavior_patterns: Vec<String>,
}

/// Activity pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityPattern {
    pub name: String,
    pub frequency: f64,
    pub duration: DurationDistribution,
    pub time_of_day: TimeOfDayPattern,
}

/// Time of day pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeOfDayPattern {
    pub start_hour: u8,
    pub end_hour: u8,
    pub peak_hours: Vec<u8>,
    pub timezone: String,
}

/// Session pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPattern {
    pub name: String,
    pub frequency: f64,
    pub session_duration: DurationDistribution,
    pub session_activities: Vec<String>,
}

/// User interaction pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteractionPattern {
    pub name: String,
    pub frequency: f64,
    pub interaction_type: InteractionType,
    pub interaction_time: DurationDistribution,
}

/// Interaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    Click,
    Type,
    Scroll,
    Navigate,
    Search,
    Command,
}

/// Scale configuration for performance testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleConfig {
    /// Data volume settings
    pub volume: VolumeConfig,

    /// Temporal distribution settings
    pub temporal: TemporalConfig,

    /// Geographic distribution settings
    pub geographic: GeographicConfig,

    /// Multi-tenant settings
    pub multi_tenant: MultiTenantConfig,

    /// Concurrency settings
    pub concurrency: ConcurrencyConfig,
}

/// Volume configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    pub total_events: usize,
    pub events_per_second: usize,
    pub burst_multiplier: f64,
    pub data_retention_days: u32,
}

/// Temporal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalConfig {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub business_hours: BusinessHoursConfig,
    pub seasonal_patterns: Vec<SeasonalPattern>,
    pub event_clustering: EventClusteringConfig,
}

/// Business hours configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessHoursConfig {
    pub enabled: bool,
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: String,
    pub activity_multiplier: f64,
}

/// Seasonal pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPattern {
    pub name: String,
    pub start_month: u8,
    pub end_month: u8,
    pub activity_multiplier: f64,
}

/// Event clustering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventClusteringConfig {
    pub enabled: bool,
    pub cluster_size: usize,
    pub cluster_frequency: f64,
    pub cluster_duration: DurationDistribution,
}

/// Geographic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicConfig {
    pub regions: Vec<RegionConfig>,
    pub latency_distribution: LatencyDistribution,
    pub bandwidth_distribution: BandwidthDistribution,
}

/// Region configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    pub name: String,
    pub frequency: f64,
    pub timezone: String,
    pub compliance_requirements: Vec<String>,
}

/// Latency distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDistribution {
    pub min_ms: u64,
    pub max_ms: u64,
    pub mean_ms: u64,
    pub std_dev_ms: u64,
}

/// Bandwidth distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthDistribution {
    pub min_mbps: f64,
    pub max_mbps: f64,
    pub mean_mbps: f64,
    pub std_dev_mbps: f64,
}

/// Multi-tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiTenantConfig {
    pub tenant_count: usize,
    pub tenant_size_distribution: TenantSizeDistribution,
    pub isolation_level: IsolationLevel,
    pub cross_tenant_analytics: bool,
}

/// Tenant size distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSizeDistribution {
    pub small_tenants: f64,
    pub medium_tenants: f64,
    pub large_tenants: f64,
    pub enterprise_tenants: f64,
}

/// Isolation levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsolationLevel {
    Strict,
    Moderate,
    Relaxed,
}

/// Concurrency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    pub concurrent_users: usize,
    pub parallel_workflows: usize,
    pub batch_processing_size: usize,
    pub streaming_velocity: usize,
}

/// Quality configuration for data validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfig {
    /// Schema compliance settings
    pub schema_compliance: SchemaComplianceConfig,

    /// Data consistency settings
    pub data_consistency: DataConsistencyConfig,

    /// Privacy and compliance settings
    pub privacy_compliance: PrivacyComplianceConfig,

    /// Realism settings
    pub realism: RealismConfig,
}

/// Schema compliance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaComplianceConfig {
    pub opentelemetry_compliance: bool,
    pub lakehouse_schema_mapping: bool,
    pub custom_attributes: bool,
    pub schema_migration: bool,
}

/// Data consistency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConsistencyConfig {
    pub cross_reference_validation: bool,
    pub temporal_consistency: bool,
    pub aggregation_consistency: bool,
    pub cross_service_consistency: bool,
}

/// Privacy compliance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyComplianceConfig {
    pub pii_scrubbing: bool,
    pub gdpr_compliance: bool,
    pub ccpa_compliance: bool,
    pub data_classification: bool,
    pub audit_trail: bool,
}

/// Realism configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealismConfig {
    pub workflow_realism: f64,
    pub timing_realism: f64,
    pub data_distribution_realism: f64,
    pub error_pattern_realism: f64,
}

/// Test scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    pub name: String,
    pub description: String,
    pub scenario_type: ScenarioType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub enabled: bool,
}

/// Scenario types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScenarioType {
    Development,
    Failure,
    Scale,
    Evolution,
    Compliance,
    Performance,
    Security,
}

/// Additional data configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditionalDataConfig {
    pub metrics_data: Option<MetricsDataConfig>,
    pub traces_data: Option<TracesDataConfig>,
    pub logs_data: Option<LogsDataConfig>,
    pub events_data: Option<EventsDataConfig>,
}

/// Metrics data configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDataConfig {
    pub metric_types: Vec<MetricType>,
    pub cardinality: usize,
    pub sampling_rate: f64,
}

/// Metric types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Traces data configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracesDataConfig {
    pub trace_depth: usize,
    pub span_count: usize,
    pub error_rate: f64,
    pub baggage_items: usize,
}

/// Logs data configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsDataConfig {
    pub log_levels: Vec<String>,
    pub structured_logs: bool,
    pub log_volume: usize,
}

/// Events data configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsDataConfig {
    pub event_types: Vec<String>,
    pub event_volume: usize,
    pub event_complexity: EventComplexity,
}

/// Event complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub schema_validation: bool,
    pub data_validation: bool,
    pub consistency_validation: bool,
    pub quality_validation: bool,
    pub performance_validation: bool,
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub formats: Vec<ExportFormat>,
    pub destinations: Vec<ExportDestination>,
    pub compression: bool,
    pub encryption: bool,
}

/// Export formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Parquet,
    Avro,
    Csv,
    Delta,
    Iceberg,
}

/// Export destinations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportDestination {
    File(PathBuf),
    S3(String),
    Lakehouse(String),
    Stream(String),
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            name: "test-data-generator".to_string(),
            version: "0.1.0".to_string(),
            seed: Some(42),
            output: OutputConfig::default(),
            workflow: WorkflowConfig::default(),
            scale: ScaleConfig::default(),
            quality: QualityConfig::default(),
            scenarios: vec![],
            additional_data: None,
            validation: ValidationConfig::default(),
            export: ExportConfig::default(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("test-data"),
            format: OutputFormat::Json,
            compression: CompressionConfig::default(),
            batch_size: 1000,
            max_file_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: CompressionAlgorithm::Gzip,
            level: 6,
        }
    }
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            research: ResearchConfig::default(),
            coordination: CoordinationConfig::default(),
            implementation: ImplementationConfig::default(),
            agent_interactions: AgentInteractionConfig::default(),
            repository_operations: RepositoryOperationConfig::default(),
            user_behavior: UserBehaviorConfig::default(),
        }
    }
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            session_duration: DurationDistribution::default(),
            query_patterns: vec![],
            knowledge_graph_interactions: KnowledgeGraphConfig::default(),
            citation_tracking: CitationTrackingConfig::default(),
        }
    }
}

impl Default for DurationDistribution {
    fn default() -> Self {
        Self {
            min_seconds: 60,
            max_seconds: 3600,
            mean_seconds: 1800,
            std_dev_seconds: 600,
            distribution_type: DistributionType::Normal,
        }
    }
}

impl Default for KnowledgeGraphConfig {
    fn default() -> Self {
        Self {
            node_count: 1000,
            edge_count: 5000,
            query_frequency: 0.1,
            traversal_depth: 3,
        }
    }
}

impl Default for CitationTrackingConfig {
    fn default() -> Self {
        Self {
            citation_count: 10,
            source_diversity: 0.8,
            tracking_frequency: 0.05,
        }
    }
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        Self {
            multi_repo_coordination: MultiRepoConfig::default(),
            agent_coordination: AgentCoordinationConfig::default(),
            conflict_resolution: ConflictResolutionConfig::default(),
            decision_making: DecisionMakingConfig::default(),
        }
    }
}

impl Default for MultiRepoConfig {
    fn default() -> Self {
        Self {
            repository_count: 5,
            dependency_depth: 3,
            coordination_frequency: 0.2,
            merge_conflict_rate: 0.1,
        }
    }
}

impl Default for AgentCoordinationConfig {
    fn default() -> Self {
        Self {
            agent_count: 3,
            coordination_patterns: vec![],
            handoff_frequency: 0.15,
            escalation_rate: 0.05,
        }
    }
}

impl Default for ConflictResolutionConfig {
    fn default() -> Self {
        Self {
            conflict_rate: 0.1,
            resolution_time: DurationDistribution::default(),
            resolution_strategies: vec![],
        }
    }
}

impl Default for DecisionMakingConfig {
    fn default() -> Self {
        Self {
            decision_frequency: 0.1,
            participant_count: 3,
            decision_time: DurationDistribution::default(),
            consensus_rate: 0.8,
        }
    }
}

impl Default for ImplementationConfig {
    fn default() -> Self {
        Self {
            code_generation: CodeGenerationConfig::default(),
            testing: TestingConfig::default(),
            deployment: DeploymentConfig::default(),
            code_review: CodeReviewConfig::default(),
        }
    }
}

impl Default for CodeGenerationConfig {
    fn default() -> Self {
        Self {
            generation_frequency: 0.3,
            code_complexity: CodeComplexity::Moderate,
            language_distribution: HashMap::new(),
            generation_time: DurationDistribution::default(),
        }
    }
}

impl Default for TestingConfig {
    fn default() -> Self {
        Self {
            test_coverage: 0.8,
            test_types: vec![TestType::Unit, TestType::Integration],
            test_execution_time: DurationDistribution::default(),
            failure_rate: 0.05,
        }
    }
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            deployment_frequency: 0.1,
            deployment_time: DurationDistribution::default(),
            rollback_rate: 0.02,
            environment_count: 3,
        }
    }
}

impl Default for CodeReviewConfig {
    fn default() -> Self {
        Self {
            review_frequency: 0.2,
            reviewer_count: 2,
            review_time: DurationDistribution::default(),
            approval_rate: 0.9,
        }
    }
}

impl Default for AgentInteractionConfig {
    fn default() -> Self {
        Self {
            agent_types: vec![
                AgentType::ResearchAgent,
                AgentType::CodeAgent,
                AgentType::TestAgent,
            ],
            interaction_patterns: vec![],
            escalation_patterns: vec![],
            collaboration_patterns: vec![],
        }
    }
}

impl Default for RepositoryOperationConfig {
    fn default() -> Self {
        Self {
            operation_types: vec![
                RepositoryOperationType::Pull,
                RepositoryOperationType::Push,
                RepositoryOperationType::Commit,
                RepositoryOperationType::Merge,
            ],
            operation_frequency: 0.4,
            operation_time: DurationDistribution::default(),
            error_rate: 0.02,
        }
    }
}

impl Default for UserBehaviorConfig {
    fn default() -> Self {
        Self {
            user_personas: vec![],
            activity_patterns: vec![],
            session_patterns: vec![],
            interaction_patterns: vec![],
        }
    }
}

impl Default for ScaleConfig {
    fn default() -> Self {
        Self {
            volume: VolumeConfig::default(),
            temporal: TemporalConfig::default(),
            geographic: GeographicConfig::default(),
            multi_tenant: MultiTenantConfig::default(),
            concurrency: ConcurrencyConfig::default(),
        }
    }
}

impl Default for VolumeConfig {
    fn default() -> Self {
        Self {
            total_events: 1_000_000,
            events_per_second: 1000,
            burst_multiplier: 2.0,
            data_retention_days: 30,
        }
    }
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            start_time: Utc::now() - chrono::Duration::days(7),
            end_time: Utc::now(),
            business_hours: BusinessHoursConfig::default(),
            seasonal_patterns: vec![],
            event_clustering: EventClusteringConfig::default(),
        }
    }
}

impl Default for BusinessHoursConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            start_hour: 9,
            end_hour: 17,
            timezone: "UTC".to_string(),
            activity_multiplier: 2.0,
        }
    }
}

impl Default for EventClusteringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cluster_size: 10,
            cluster_frequency: 0.1,
            cluster_duration: DurationDistribution::default(),
        }
    }
}

impl Default for GeographicConfig {
    fn default() -> Self {
        Self {
            regions: vec![RegionConfig::default()],
            latency_distribution: LatencyDistribution::default(),
            bandwidth_distribution: BandwidthDistribution::default(),
        }
    }
}

impl Default for RegionConfig {
    fn default() -> Self {
        Self {
            name: "us-west-1".to_string(),
            frequency: 1.0,
            timezone: "America/Los_Angeles".to_string(),
            compliance_requirements: vec![],
        }
    }
}

impl Default for LatencyDistribution {
    fn default() -> Self {
        Self {
            min_ms: 10,
            max_ms: 200,
            mean_ms: 50,
            std_dev_ms: 30,
        }
    }
}

impl Default for BandwidthDistribution {
    fn default() -> Self {
        Self {
            min_mbps: 10.0,
            max_mbps: 1000.0,
            mean_mbps: 100.0,
            std_dev_mbps: 50.0,
        }
    }
}

impl Default for MultiTenantConfig {
    fn default() -> Self {
        Self {
            tenant_count: 10,
            tenant_size_distribution: TenantSizeDistribution::default(),
            isolation_level: IsolationLevel::Strict,
            cross_tenant_analytics: false,
        }
    }
}

impl Default for TenantSizeDistribution {
    fn default() -> Self {
        Self {
            small_tenants: 0.4,
            medium_tenants: 0.3,
            large_tenants: 0.2,
            enterprise_tenants: 0.1,
        }
    }
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 100,
            parallel_workflows: 10,
            batch_processing_size: 1000,
            streaming_velocity: 10000,
        }
    }
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            schema_compliance: SchemaComplianceConfig::default(),
            data_consistency: DataConsistencyConfig::default(),
            privacy_compliance: PrivacyComplianceConfig::default(),
            realism: RealismConfig::default(),
        }
    }
}

impl Default for SchemaComplianceConfig {
    fn default() -> Self {
        Self {
            opentelemetry_compliance: true,
            lakehouse_schema_mapping: true,
            custom_attributes: true,
            schema_migration: false,
        }
    }
}

impl Default for DataConsistencyConfig {
    fn default() -> Self {
        Self {
            cross_reference_validation: true,
            temporal_consistency: true,
            aggregation_consistency: true,
            cross_service_consistency: true,
        }
    }
}

impl Default for PrivacyComplianceConfig {
    fn default() -> Self {
        Self {
            pii_scrubbing: true,
            gdpr_compliance: false,
            ccpa_compliance: false,
            data_classification: true,
            audit_trail: true,
        }
    }
}

impl Default for RealismConfig {
    fn default() -> Self {
        Self {
            workflow_realism: 0.9,
            timing_realism: 0.8,
            data_distribution_realism: 0.85,
            error_pattern_realism: 0.7,
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            schema_validation: true,
            data_validation: true,
            consistency_validation: true,
            quality_validation: true,
            performance_validation: false,
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            formats: vec![ExportFormat::Json],
            destinations: vec![ExportDestination::File(PathBuf::from("test-data"))],
            compression: true,
            encryption: false,
        }
    }
}

/// Load configuration from file
pub fn load_config(path: &PathBuf) -> anyhow::Result<GeneratorConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: GeneratorConfig = toml::from_str(&content)?;
    Ok(config)
}

/// Save configuration to file
pub fn save_config(config: &GeneratorConfig, path: &PathBuf) -> anyhow::Result<()> {
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Validate configuration
pub fn validate_config(config: &GeneratorConfig) -> anyhow::Result<()> {
    // Validate required fields
    if config.name.is_empty() {
        return Err(anyhow::anyhow!("Generator name cannot be empty"));
    }

    if config.scale.volume.total_events == 0 {
        return Err(anyhow::anyhow!("Total events must be greater than 0"));
    }

    if config.scale.volume.events_per_second == 0 {
        return Err(anyhow::anyhow!("Events per second must be greater than 0"));
    }

    // Validate time ranges
    if config.scale.temporal.start_time >= config.scale.temporal.end_time {
        return Err(anyhow::anyhow!("Start time must be before end time"));
    }

    // Validate probabilities with floating-point tolerance
    let sum = config
        .scale
        .multi_tenant
        .tenant_size_distribution
        .small_tenants
        + config
            .scale
            .multi_tenant
            .tenant_size_distribution
            .medium_tenants
        + config
            .scale
            .multi_tenant
            .tenant_size_distribution
            .large_tenants
        + config
            .scale
            .multi_tenant
            .tenant_size_distribution
            .enterprise_tenants;

    if (sum - 1.0).abs() > f64::EPSILON {
        return Err(anyhow::anyhow!(
            "Tenant size distribution probabilities must sum to 1.0 (got {})",
            sum
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GeneratorConfig::default();
        assert_eq!(config.name, "test-data-generator");
        assert_eq!(config.version, "0.1.0");
        assert!(config.seed.is_some());
    }

    #[test]
    fn test_config_validation() {
        let mut config = GeneratorConfig::default();
        assert!(validate_config(&config).is_ok());

        config.scale.volume.total_events = 0;
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = GeneratorConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: GeneratorConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config.name, deserialized.name);
    }
}
