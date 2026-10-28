use serde::{Deserialize, Serialize};

use super::identifiers::{AgentRunId, AgentTurnId, LogicalRequestId, ProviderAttemptId, TaskId};
use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::AIAgentExchangeId;
use crate::ai::response_experience::source::{AgentId, LocalToolCallId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentObservationContext {
    pub conversation_id: AIConversationId,
    pub run_id: AgentRunId,
    pub turn_id: AgentTurnId,

    pub logical_request_id: Option<LogicalRequestId>,
    pub attempt_id: Option<ProviderAttemptId>,

    pub exchange_id: Option<AIAgentExchangeId>,
    pub task_id: Option<TaskId>,
    pub tool_call_id: Option<LocalToolCallId>,
    pub child_agent_id: Option<AgentId>,

    pub trace_context: TraceContext,
}
