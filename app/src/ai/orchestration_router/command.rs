use serde::{Deserialize, Serialize};

use super::ids::{CorrelationId, EventId, RunId, ToolCallId};
use super::lifecycle::SuspensionReason;
use super::policies::{ChildFailurePolicy, JoinPolicy, ParentClosePolicy};
use crate::AIConversationId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeKind {
    LocalBrain,
    CloudAgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildTaskSpec {
    pub summary: String,
    pub context_refs: Vec<String>,
    pub pwd: Option<String>,
    pub os: Option<String>,
    pub shell: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTarget {
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffContext {
    pub filter: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CancellationReason {
    UserRequested,
    ParentCancelled,
    SystemError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalToolCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetryReason {
    NetworkFailure,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrchestrationCommand {
    RegisterRootRun {
        run_id: RunId,
        conversation_id: AIConversationId,
        runtime: RuntimeKind,
    },

    SpawnChildRun {
        parent_run_id: RunId,
        child_run_id: RunId,
        agent_role: Option<String>,
        task: ChildTaskSpec,
        runtime: RuntimeKind,
        join_policy: JoinPolicy,
        failure_policy: ChildFailurePolicy,
        parent_close_policy: ParentClosePolicy,
        causation_id: Option<EventId>,
        correlation_id: CorrelationId,
    },

    HandoffRun {
        from_run_id: RunId,
        target_agent: AgentTarget,
        filtered_context: HandoffContext,
    },

    SuspendRun {
        run_id: RunId,
        reason: SuspensionReason,
    },

    ResumeRun {
        run_id: RunId,
    },

    CancelRun {
        run_id: RunId,
        reason: CancellationReason,
        propagate_to_descendants: bool,
    },

    RecordToolRequest {
        run_id: RunId,
        tool_call: CanonicalToolCall,
    },

    RecordToolApproval {
        tool_call_id: ToolCallId,
        decision: ApprovalDecision,
    },

    RetryRun {
        run_id: RunId,
        reason: RetryReason,
    },
}
