use serde::{Deserialize, Serialize};

use super::command::{ChildTaskSpec, RuntimeKind};
use super::ids::{CorrelationId, EventId, RunId, ToolCallId};
use super::policies::{ChildFailurePolicy, JoinPolicy, ParentClosePolicy};
use crate::AIConversationId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEventSource {
    WarpCloud,
    LocalBrain,
    ClientToolExecutor,
    User,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurabilityClass {
    Ephemeral,
    Durable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrchestrationEvent {
    RootRunRegistered,
    RunStarted,
    ChildRunSpawnRequested {
        child_task: ChildTaskSpec,
        runtime: RuntimeKind,
        join_policy: JoinPolicy,
        failure_policy: ChildFailurePolicy,
        parent_close_policy: ParentClosePolicy,
    },
    ChildRunSpawned,
    RunHandoffRequested,
    RunHandedOff,
    RunSuspended,
    RunResumed,
    ToolCallRequested,
    ToolApprovalRequested,
    ToolApproved,
    ToolRejected,
    ToolExecutionDispatched,
    ToolExecutionStarted,
    ToolExecutionFinished,
    ToolExecutionFailed,
    RunCancellationRequested,
    RunCancelled,
    RunCompleted,
    RunFailed,
    RunAbandoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEventEnvelope<T> {
    pub event_id: EventId,
    pub schema_version: u16,

    pub source: RuntimeEventSource,
    pub event_type: String,

    pub root_run_id: RunId,
    pub run_id: RunId,
    pub parent_run_id: Option<RunId>,
    pub conversation_id: AIConversationId,
    pub tool_call_id: Option<ToolCallId>,

    pub source_sequence: Option<u64>,
    pub ingest_sequence: Option<u64>,

    pub correlation_id: CorrelationId,
    pub causation_id: Option<EventId>,

    pub traceparent: Option<String>,

    pub occurred_at: i64, // Epoch millis or similar, adjusting for Warp's timestamp standard
    pub recorded_at: i64,

    pub durability: DurabilityClass,
    pub payload: T,
}
