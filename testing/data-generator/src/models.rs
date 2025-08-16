//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Data models for agentic development workflows and telemetry patterns
//!
//! This module provides comprehensive data structures that model realistic
//! agentic IDE workflows, including research sessions, coordination patterns,
//! and implementation workflows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Agentic development workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgenticWorkflow {
    /// Workflow identifier
    pub id: Uuid,

    /// Workflow type
    pub workflow_type: WorkflowType,

    /// Workflow status
    pub status: WorkflowStatus,

    /// Workflow phases
    pub phases: Vec<WorkflowPhase>,

    /// Participating agents
    pub agents: Vec<Agent>,

    /// Workflow metadata
    pub metadata: HashMap<String, String>,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time (if completed)
    pub end_time: Option<DateTime<Utc>>,

    /// Total duration
    pub duration: Option<chrono::Duration>,
}

/// Workflow types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowType {
    Research,
    Coordination,
    Implementation,
    Review,
    Deployment,
    Maintenance,
}

/// Workflow status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowStatus {
    Planning,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Workflow phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPhase {
    /// Phase identifier
    pub id: Uuid,

    /// Phase name
    pub name: String,

    /// Phase type
    pub phase_type: PhaseType,

    /// Phase status
    pub status: PhaseStatus,

    /// Phase activities
    pub activities: Vec<Activity>,

    /// Phase dependencies
    pub dependencies: Vec<Uuid>,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time (if completed)
    pub end_time: Option<DateTime<Utc>>,

    /// Duration
    pub duration: Option<chrono::Duration>,
}

/// Phase types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PhaseType {
    IntentDeclaration,
    Research,
    Analysis,
    Planning,
    Implementation,
    Testing,
    Review,
    Deployment,
    Monitoring,
}

/// Phase status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

/// Activity within a workflow phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Activity identifier
    pub id: Uuid,

    /// Activity name
    pub name: String,

    /// Activity type
    pub activity_type: ActivityType,

    /// Activity status
    pub status: ActivityStatus,

    /// Responsible agent
    pub agent_id: Uuid,

    /// Activity parameters
    pub parameters: HashMap<String, String>,

    /// Activity results
    pub results: Option<ActivityResult>,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time (if completed)
    pub end_time: Option<DateTime<Utc>>,

    /// Duration
    pub duration: Option<chrono::Duration>,
}

/// Activity types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActivityType {
    Query,
    Analysis,
    CodeGeneration,
    Testing,
    Review,
    Deployment,
    Monitoring,
    Coordination,
    Decision,
    Research,
    Planning,
}

/// Activity status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Activity result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityResult {
    /// Success status
    pub success: bool,

    /// Result data
    pub data: HashMap<String, serde_json::Value>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Performance metrics
    pub metrics: HashMap<String, f64>,
}

/// Agent participating in workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Agent identifier
    pub id: Uuid,

    /// Agent name
    pub name: String,

    /// Agent type
    pub agent_type: AgentType,

    /// Agent capabilities
    pub capabilities: Vec<AgentCapability>,

    /// Agent status
    pub status: AgentStatus,

    /// Agent metadata
    pub metadata: HashMap<String, String>,

    /// Creation time
    pub created_at: DateTime<Utc>,
}

/// Agent types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentType {
    ResearchAgent,
    CodeAgent,
    TestAgent,
    ReviewAgent,
    CoordinationAgent,
    SecurityAgent,
    DocumentationAgent,
    DeploymentAgent,
}

/// Agent capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentCapability {
    CodeGeneration,
    CodeReview,
    Testing,
    Research,
    Coordination,
    Security,
    Documentation,
    Deployment,
    Monitoring,
    Analysis,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Available,
    Busy,
    Offline,
    Error,
}

/// Research session within a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSession {
    /// Session identifier
    pub id: Uuid,

    /// Research topic
    pub topic: String,

    /// Research queries
    pub queries: Vec<ResearchQuery>,

    /// Research results
    pub results: Vec<ResearchResult>,

    /// Knowledge graph interactions
    pub knowledge_graph_interactions: Vec<KnowledgeGraphInteraction>,

    /// Citations and references
    pub citations: Vec<Citation>,

    /// Session metadata
    pub metadata: HashMap<String, String>,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: Option<DateTime<Utc>>,

    /// Duration
    pub duration: Option<chrono::Duration>,
}

/// Research query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuery {
    /// Query identifier
    pub id: Uuid,

    /// Query text
    pub query: String,

    /// Query type
    pub query_type: QueryType,

    /// Query complexity
    pub complexity: QueryComplexity,

    /// Query parameters
    pub parameters: HashMap<String, String>,

    /// Query results
    pub results: Vec<QueryResult>,

    /// Execution time
    pub execution_time: chrono::Duration,

    /// Success status
    pub success: bool,
}

/// Query types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Semantic,
    Keyword,
    Structured,
    NaturalLanguage,
    Code,
}

/// Query complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Result identifier
    pub id: Uuid,

    /// Result content
    pub content: String,

    /// Result type
    pub result_type: ResultType,

    /// Relevance score
    pub relevance_score: f64,

    /// Source information
    pub source: Option<String>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultType {
    Text,
    Code,
    Documentation,
    Example,
    Reference,
}

/// Research result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    /// Result identifier
    pub id: Uuid,

    /// Result title
    pub title: String,

    /// Result summary
    pub summary: String,

    /// Result content
    pub content: String,

    /// Result type
    pub result_type: ResearchResultType,

    /// Confidence score
    pub confidence_score: f64,

    /// Related queries
    pub related_queries: Vec<Uuid>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Research result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResearchResultType {
    Solution,
    Approach,
    Pattern,
    Reference,
    Example,
    Warning,
}

/// Knowledge graph interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphInteraction {
    /// Interaction identifier
    pub id: Uuid,

    /// Interaction type
    pub interaction_type: KnowledgeGraphInteractionType,

    /// Node identifiers
    pub node_ids: Vec<String>,

    /// Edge identifiers
    pub edge_ids: Vec<String>,

    /// Traversal path
    pub traversal_path: Vec<String>,

    /// Interaction result
    pub result: HashMap<String, serde_json::Value>,

    /// Execution time
    pub execution_time: chrono::Duration,
}

/// Knowledge graph interaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeGraphInteractionType {
    Query,
    Traverse,
    Update,
    Create,
    Delete,
}

/// Citation reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Citation identifier
    pub id: Uuid,

    /// Citation type
    pub citation_type: CitationType,

    /// Source information
    pub source: String,

    /// Reference information
    pub reference: String,

    /// Citation text
    pub text: String,

    /// Relevance score
    pub relevance_score: f64,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Citation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CitationType {
    Paper,
    Documentation,
    Code,
    Blog,
    Forum,
    Book,
}

/// Coordination workflow between multiple agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationWorkflow {
    /// Workflow identifier
    pub id: Uuid,

    /// Coordination type
    pub coordination_type: CoordinationType,

    /// Participating agents
    pub agents: Vec<Uuid>,

    /// Coordination events
    pub events: Vec<CoordinationEvent>,

    /// Handoffs between agents
    pub handoffs: Vec<AgentHandoff>,

    /// Escalations
    pub escalations: Vec<Escalation>,

    /// Decisions made
    pub decisions: Vec<Decision>,

    /// Workflow status
    pub status: CoordinationStatus,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: Option<DateTime<Utc>>,
}

/// Coordination types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationType {
    Sequential,
    Parallel,
    Hierarchical,
    Peer,
    Hybrid,
}

/// Coordination event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationEvent {
    /// Event identifier
    pub id: Uuid,

    /// Event type
    pub event_type: CoordinationEventType,

    /// Initiating agent
    pub initiator: Uuid,

    /// Target agents
    pub targets: Vec<Uuid>,

    /// Event data
    pub data: HashMap<String, serde_json::Value>,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Event status
    pub status: EventStatus,
}

/// Coordination event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationEventType {
    Request,
    Response,
    Notification,
    Update,
    Error,
    Completion,
}

/// Event status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Agent handoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandoff {
    /// Handoff identifier
    pub id: Uuid,

    /// Source agent
    pub source_agent: Uuid,

    /// Target agent
    pub target_agent: Uuid,

    /// Handoff reason
    pub reason: String,

    /// Handoff data
    pub data: HashMap<String, serde_json::Value>,

    /// Handoff timestamp
    pub timestamp: DateTime<Utc>,

    /// Handoff status
    pub status: HandoffStatus,
}

/// Handoff status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandoffStatus {
    Initiated,
    Accepted,
    Rejected,
    Completed,
    Failed,
}

/// Escalation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Escalation {
    /// Escalation identifier
    pub id: Uuid,

    /// Escalation reason
    pub reason: String,

    /// Escalation level
    pub level: EscalationLevel,

    /// Initiating agent
    pub initiator: Uuid,

    /// Target agents
    pub targets: Vec<Uuid>,

    /// Escalation data
    pub data: HashMap<String, serde_json::Value>,

    /// Escalation timestamp
    pub timestamp: DateTime<Utc>,

    /// Resolution timestamp
    pub resolution_timestamp: Option<DateTime<Utc>>,

    /// Resolution status
    pub resolution_status: EscalationResolutionStatus,
}

/// Escalation levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Escalation resolution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationResolutionStatus {
    Pending,
    InProgress,
    Resolved,
    Failed,
}

/// Decision made during coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Decision identifier
    pub id: Uuid,

    /// Decision topic
    pub topic: String,

    /// Decision options
    pub options: Vec<DecisionOption>,

    /// Selected option
    pub selected_option: Option<Uuid>,

    /// Decision participants
    pub participants: Vec<Uuid>,

    /// Decision timestamp
    pub timestamp: DateTime<Utc>,

    /// Decision status
    pub status: DecisionStatus,

    /// Decision rationale
    pub rationale: Option<String>,
}

/// Decision option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOption {
    /// Option identifier
    pub id: Uuid,

    /// Option description
    pub description: String,

    /// Option pros
    pub pros: Vec<String>,

    /// Option cons
    pub cons: Vec<String>,

    /// Option votes
    pub votes: HashMap<Uuid, bool>,
}

/// Decision status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionStatus {
    Pending,
    InProgress,
    Decided,
    Implemented,
    Rejected,
}

/// Coordination status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStatus {
    Planning,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Implementation workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationWorkflow {
    /// Workflow identifier
    pub id: Uuid,

    /// Implementation type
    pub implementation_type: ImplementationType,

    /// Code generation activities
    pub code_generation: Vec<CodeGenerationActivity>,

    /// Testing activities
    pub testing: Vec<TestingActivity>,

    /// Review activities
    pub review: Vec<ReviewActivity>,

    /// Deployment activities
    pub deployment: Vec<DeploymentActivity>,

    /// Implementation status
    pub status: ImplementationStatus,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: Option<DateTime<Utc>>,
}

/// Implementation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationType {
    NewFeature,
    BugFix,
    Refactoring,
    Optimization,
    Security,
    Documentation,
}

/// Code generation activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenerationActivity {
    /// Activity identifier
    pub id: Uuid,

    /// Generated code
    pub code: String,

    /// Programming language
    pub language: String,

    /// Code complexity
    pub complexity: CodeComplexity,

    /// Code metrics
    pub metrics: CodeMetrics,

    /// Generation parameters
    pub parameters: HashMap<String, String>,

    /// Generation timestamp
    pub timestamp: DateTime<Utc>,

    /// Generation duration
    pub duration: chrono::Duration,

    /// Success status
    pub success: bool,
}

/// Code complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Code metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    /// Lines of code
    pub lines_of_code: usize,

    /// Cyclomatic complexity
    pub cyclomatic_complexity: usize,

    /// Cognitive complexity
    pub cognitive_complexity: usize,

    /// Maintainability index
    pub maintainability_index: f64,

    /// Test coverage
    pub test_coverage: f64,
}

/// Testing activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingActivity {
    /// Activity identifier
    pub id: Uuid,

    /// Test type
    pub test_type: TestType,

    /// Test results
    pub results: TestResults,

    /// Test duration
    pub duration: chrono::Duration,

    /// Test timestamp
    pub timestamp: DateTime<Utc>,

    /// Success status
    pub success: bool,
}

/// Test types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestType {
    Unit,
    Integration,
    EndToEnd,
    Performance,
    Security,
    Regression,
}

/// Test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    /// Total tests
    pub total_tests: usize,

    /// Passed tests
    pub passed_tests: usize,

    /// Failed tests
    pub failed_tests: usize,

    /// Skipped tests
    pub skipped_tests: usize,

    /// Test details
    pub details: Vec<TestDetail>,
}

/// Test detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDetail {
    /// Test name
    pub name: String,

    /// Test status
    pub status: TestStatus,

    /// Test duration
    pub duration: chrono::Duration,

    /// Error message (if failed)
    pub error: Option<String>,
}

/// Test status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

/// Review activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewActivity {
    /// Activity identifier
    pub id: Uuid,

    /// Review type
    pub review_type: ReviewType,

    /// Review comments
    pub comments: Vec<ReviewComment>,

    /// Review decision
    pub decision: ReviewDecision,

    /// Review duration
    pub duration: chrono::Duration,

    /// Review timestamp
    pub timestamp: DateTime<Utc>,
}

/// Review types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewType {
    CodeReview,
    DesignReview,
    SecurityReview,
    PerformanceReview,
}

/// Review comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    /// Comment identifier
    pub id: Uuid,

    /// Comment text
    pub text: String,

    /// Comment type
    pub comment_type: CommentType,

    /// Comment severity
    pub severity: CommentSeverity,

    /// Comment location
    pub location: Option<String>,

    /// Comment timestamp
    pub timestamp: DateTime<Utc>,
}

/// Comment types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommentType {
    Suggestion,
    Question,
    Issue,
    Praise,
    Nitpick,
}

/// Comment severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommentSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Review decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewDecision {
    Approved,
    ApprovedWithChanges,
    Rejected,
    NeedsMoreWork,
}

/// Deployment activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentActivity {
    /// Activity identifier
    pub id: Uuid,

    /// Deployment environment
    pub environment: String,

    /// Deployment status
    pub status: DeploymentStatus,

    /// Deployment metrics
    pub metrics: DeploymentMetrics,

    /// Deployment duration
    pub duration: chrono::Duration,

    /// Deployment timestamp
    pub timestamp: DateTime<Utc>,

    /// Rollback information
    pub rollback: Option<RollbackInfo>,
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    RolledBack,
}

/// Deployment metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentMetrics {
    /// Deployment time
    pub deployment_time: chrono::Duration,

    /// Downtime
    pub downtime: chrono::Duration,

    /// Error rate
    pub error_rate: f64,

    /// Performance impact
    pub performance_impact: f64,
}

/// Rollback information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackInfo {
    /// Rollback reason
    pub reason: String,

    /// Rollback timestamp
    pub timestamp: DateTime<Utc>,

    /// Rollback duration
    pub duration: chrono::Duration,
}

/// Implementation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationStatus {
    Planning,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Agent interaction pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInteraction {
    /// Interaction identifier
    pub id: Uuid,

    /// Interaction type
    pub interaction_type: InteractionType,

    /// Initiating agent
    pub initiator: Uuid,

    /// Target agent
    pub target: Uuid,

    /// Interaction data
    pub data: HashMap<String, serde_json::Value>,

    /// Interaction timestamp
    pub timestamp: DateTime<Utc>,

    /// Interaction duration
    pub duration: chrono::Duration,

    /// Interaction status
    pub status: InteractionStatus,
}

/// Interaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    Request,
    Response,
    Notification,
    Collaboration,
    Handoff,
    Escalation,
}

/// Interaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionStatus {
    Initiated,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Repository operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryOperation {
    /// Operation identifier
    pub id: Uuid,

    /// Operation type
    pub operation_type: RepositoryOperationType,

    /// Repository information
    pub repository: RepositoryInfo,

    /// Operation parameters
    pub parameters: HashMap<String, String>,

    /// Operation result
    pub result: OperationResult,

    /// Operation timestamp
    pub timestamp: DateTime<Utc>,

    /// Operation duration
    pub duration: chrono::Duration,
}

/// Repository operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RepositoryOperationType {
    Clone,
    Pull,
    Push,
    Commit,
    Merge,
    Branch,
    Tag,
    Rebase,
    CherryPick,
    Reset,
}

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// Repository URL
    pub url: String,

    /// Repository name
    pub name: String,

    /// Branch
    pub branch: String,

    /// Commit hash
    pub commit: Option<String>,

    /// Repository metadata
    pub metadata: HashMap<String, String>,
}

/// Operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Success status
    pub success: bool,

    /// Result data
    pub data: HashMap<String, serde_json::Value>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Performance metrics
    pub metrics: HashMap<String, f64>,
}

/// User behavior pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehavior {
    /// Behavior identifier
    pub id: Uuid,

    /// User identifier
    pub user_id: String,

    /// Behavior type
    pub behavior_type: BehaviorType,

    /// Behavior data
    pub data: HashMap<String, serde_json::Value>,

    /// Behavior timestamp
    pub timestamp: DateTime<Utc>,

    /// Session information
    pub session: Option<SessionInfo>,
}

/// Behavior types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BehaviorType {
    Click,
    Type,
    Scroll,
    Navigate,
    Search,
    Command,
    Focus,
    Blur,
    Hover,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session identifier
    pub session_id: String,

    /// Session start time
    pub start_time: DateTime<Utc>,

    /// Session duration
    pub duration: chrono::Duration,

    /// Session activities
    pub activities: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agentic_workflow_creation() {
        let workflow = AgenticWorkflow {
            id: Uuid::new_v4(),
            workflow_type: WorkflowType::Research,
            status: WorkflowStatus::InProgress,
            phases: vec![],
            agents: vec![],
            metadata: HashMap::new(),
            start_time: Utc::now(),
            end_time: None,
            duration: None,
        };

        assert_eq!(workflow.workflow_type, WorkflowType::Research);
        assert_eq!(workflow.status, WorkflowStatus::InProgress);
    }

    #[test]
    fn test_research_session_creation() {
        let session = ResearchSession {
            id: Uuid::new_v4(),
            topic: "Test Research Topic".to_string(),
            queries: vec![],
            results: vec![],
            knowledge_graph_interactions: vec![],
            citations: vec![],
            metadata: HashMap::new(),
            start_time: Utc::now(),
            end_time: None,
            duration: None,
        };

        assert_eq!(session.topic, "Test Research Topic");
    }

    #[test]
    fn test_agent_creation() {
        let agent = Agent {
            id: Uuid::new_v4(),
            name: "Test Agent".to_string(),
            agent_type: AgentType::ResearchAgent,
            capabilities: vec![AgentCapability::Research],
            status: AgentStatus::Available,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };

        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.agent_type, AgentType::ResearchAgent);
    }
}
