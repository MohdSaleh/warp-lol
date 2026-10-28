use std::time::{Duration, SystemTime};

use super::durability::EventDurability;
use super::source::{AgentId, LocalToolCallId, ProviderId, ProviderModelRef, RichEventSource};
use super::visibility::EventVisibility;
use crate::ai::agent::conversation::AIConversationId;

pub type RichEventId = String;
pub type RichEventType = String;
pub type RichEventStreamId = String;
pub type RunId = String;
pub type TurnId = String;
pub type TaskId = String;
pub type AIAgentExchangeId = String;
pub type CorrelationId = String;
pub type ContentEntityId = String;
pub type PlanId = String;
pub type PlanItemId = String;
pub type RetrievalId = String;
pub type GroundingSourceId = String;
pub type CitationId = String;
pub type ArtifactId = String;
pub type SuggestionId = String;
pub type CheckpointId = String;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RichAgentEvent {
    pub schema_version: u32,
    pub event_id: RichEventId,
    pub event_type: RichEventType,
    pub stream_id: RichEventStreamId,
    pub sequence: u64,
    pub entity_revision: u64,
    pub run_id: RunId,
    pub turn_id: TurnId,
    pub conversation_id: AIConversationId,
    pub task_id: Option<TaskId>,
    pub exchange_id: Option<AIAgentExchangeId>,
    pub parent_event_id: Option<RichEventId>,
    pub correlation_id: Option<CorrelationId>,
    pub occurred_at: SystemTime,
    pub source: RichEventSource,
    pub visibility: EventVisibility,
    pub durability: EventDurability,
    pub payload: RichEventPayload,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RichEventPayload {
    Run(RunLifecycleEvent),
    Status(StatusEvent),
    ReasoningSummary(ReasoningSummaryEvent),
    Content(ContentEvent),
    Tool(ToolLifecycleEvent),
    Plan(PlanEvent),
    Retrieval(RetrievalEvent),
    Source(SourceEvent),
    Citation(CitationEvent),
    Memory(MemoryEvent),
    Artifact(ArtifactEvent),
    Diff(DiffEvent),
    ChildAgent(ChildAgentEvent),
    Routing(RoutingEvent),
    Context(ContextEvent),
    Usage(UsageEvent),
    Suggestion(SuggestionEvent),
    Warning(WarningEvent),
    Terminal(TerminalEvent),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RunLifecycleState {
    Queued,
    Preparing,
    ResolvingContext,
    WaitingForProvider,
    Generating,
    WaitingForApproval,
    ExecutingTools,
    WaitingForTools,
    WaitingForChildAgents,
    Recovering,
    CompactingContext,
    Finalizing,
    Completed,
    Partial,
    Blocked,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunLifecycleEvent {
    pub state: RunLifecycleState,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatusEvent {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReasoningSummaryEvent {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ContentKind {
    Text,
    // Add other content kinds
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContentAnnotation {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ContentEvent {
    Started {
        content_id: ContentEntityId,
        kind: ContentKind,
    },
    Delta {
        content_id: ContentEntityId,
        text: String,
    },
    Replaced {
        content_id: ContentEntityId,
        complete_text: String,
        revision: u64,
    },
    AnnotationAdded {
        content_id: ContentEntityId,
        annotation: ContentAnnotation,
    },
    Completed {
        content_id: ContentEntityId,
    },
    Refused {
        content_id: ContentEntityId,
        safe_reason: Option<String>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanonicalToolCallSummary {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PermissionSummary {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SafeToolOutputPreview {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolResultSummary {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SafeToolError {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCancellationReason {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ToolLifecycleEvent {
    Proposed {
        tool_call: CanonicalToolCallSummary,
    },
    PermissionRequired {
        summary: PermissionSummary,
    },
    Started,
    Progress {
        completed: Option<f64>,
        total: Option<f64>,
        message: Option<String>,
    },
    OutputPreview {
        preview: SafeToolOutputPreview,
    },
    Completed {
        result_summary: ToolResultSummary,
    },
    Failed {
        error: SafeToolError,
    },
    Cancelled {
        reason: ToolCancellationReason,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceEvent {
    pub source_id: GroundingSourceId,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CitationEvent {
    pub citation_id: CitationId,
    pub document_type: i32,
    pub document_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanItem {
    pub id: PlanItemId,
    pub title: String,
    pub completed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlanEvent {
    PlanCreated {
        plan_id: PlanId,
        items: Vec<PlanItem>,
    },
    ItemAdded {
        plan_id: PlanId,
        item: PlanItem,
    },
    ItemUpdated {
        plan_id: PlanId,
        item: PlanItem,
    },
    ItemCompleted {
        plan_id: PlanId,
        item_id: PlanItemId,
    },
    PlanCompleted {
        plan_id: PlanId,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RetrievalKind {
    Lexical,
    Semantic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroundedSourceReference {
    pub source_id: GroundingSourceId,
    pub title: String,
    pub score: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetrievalQuality {
    pub score: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SafeRetrievalError {
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RetrievalEvent {
    Started {
        retrieval_id: RetrievalId,
        kind: RetrievalKind,
        safe_query_summary: Option<String>,
    },
    Progress {
        retrieval_id: RetrievalId,
        sources_examined: Option<u32>,
    },
    ResultsAvailable {
        retrieval_id: RetrievalId,
        sources: Vec<GroundedSourceReference>,
        quality: RetrievalQuality,
    },
    Completed {
        retrieval_id: RetrievalId,
    },
    Failed {
        retrieval_id: RetrievalId,
        error: SafeRetrievalError,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MemoryUnavailableReason {
    Unavailable,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MemoryEvent {
    MemoryUsed {
        source_id: GroundingSourceId,
        memory_store_id: String,
        safe_label: String,
    },
    MemoryUnavailable {
        reason: MemoryUnavailableReason,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactEvent {
    pub artifact_id: ArtifactId,
    pub name: String,
    pub path: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffEvent {
    pub path: String,
    pub diff: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChildAgentEvent {
    pub agent_id: AgentId,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderRouteSummary {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetryReason {
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FallbackReason {
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RoutingEvent {
    RouteSelected {
        provider: ProviderId,
        model: ProviderModelRef,
    },
    RetryScheduled {
        safe_reason: RetryReason,
        delay: Option<Duration>,
    },
    FallbackStarted {
        from: ProviderRouteSummary,
        to: ProviderRouteSummary,
        reason: FallbackReason,
    },
    FallbackCompleted {
        active: ProviderRouteSummary,
    },
    AuthenticationRequired {
        provider: ProviderId,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ContextCompactionStrategy {
    Summarize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewChainReason {
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ContextEvent {
    NearLimit {
        utilization: Option<f32>,
    },
    CompactionStarted {
        strategy: ContextCompactionStrategy,
    },
    CompactionCompleted {
        removed_tokens: Option<u64>,
        retained_tokens: Option<u64>,
        checkpoint_id: Option<CheckpointId>,
    },
    NewProviderChainStarted {
        reason: NewChainReason,
    },
    ContextLimitExceeded {
        required_tokens: Option<u64>,
        available_tokens: Option<u64>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageEvent {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SuggestionKind {
    FollowUp,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SuggestionSource {
    Model,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RichSuggestion {
    pub suggestion_id: SuggestionId,
    pub label: String,
    pub prompt: String,
    pub kind: SuggestionKind,
    pub source: SuggestionSource,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SuggestionEvent {
    pub suggestion: RichSuggestion,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WarningEvent {
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletedWorkSummary {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IncompleteWorkSummary {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserResolvableBlocker {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CancellationReason {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RichUserError {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetryAvailability {
    // TBD
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RichTerminalOutcome {
    Success,
    Partial {
        completed: Vec<CompletedWorkSummary>,
        incomplete: Vec<IncompleteWorkSummary>,
    },
    Blocked {
        blocker: UserResolvableBlocker,
    },
    Cancelled {
        reason: CancellationReason,
    },
    Failed {
        error: RichUserError,
        retry: RetryAvailability,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminalEvent {
    pub outcome: RichTerminalOutcome,
}
