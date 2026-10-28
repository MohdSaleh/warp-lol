use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::command::RuntimeKind;
use super::ids::{RunId, ToolCallId};
use super::lifecycle::RunLifecycle;
use crate::AIConversationId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunNode {
    pub run_id: RunId,
    pub root_run_id: RunId,
    pub parent_run_id: Option<RunId>,
    pub conversation_id: AIConversationId,

    pub runtime: RuntimeKind,
    pub agent_role: Option<String>,

    pub lifecycle: RunLifecycle,

    pub last_source_sequence: Option<u64>,
    pub last_ingest_sequence: u64,

    pub created_at: i64,
    pub updated_at: i64,

    pub terminal_reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunGraphState {
    pub runs: HashMap<RunId, RunNode>,
    pub children_by_parent: HashMap<RunId, Vec<RunId>>,
    pub active_runs: HashSet<RunId>,
    pub terminal_runs: HashSet<RunId>,
    pub tools: HashMap<ToolCallId, ToolExecutionState>,
    pub last_ingest_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolExecutionStatus {
    Requested,
    Approved,
    Dispatched,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalState {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResultSummary {
    pub success: bool,
    pub summary_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionState {
    pub tool_call_id: ToolCallId,
    pub run_id: RunId,
    pub conversation_id: AIConversationId,

    pub tool_name: String,
    pub arguments_hash: String,

    pub status: ToolExecutionStatus,
    pub approval: ApprovalState,

    pub attempt: u32,
    pub idempotency_key: String,

    pub requested_at: i64,
    pub approved_at: Option<i64>,
    pub dispatched_at: Option<i64>,
    pub completed_at: Option<i64>,

    pub timeout_at: Option<i64>,
    pub result_summary: Option<ToolResultSummary>,
}

impl RunGraphState {
    pub fn restore(
        run_records: Vec<crate::persistence::model::OrchestrationRouterRunRecord>,
        edge_records: Vec<crate::persistence::model::OrchestrationRouterEdgeRecord>,
    ) -> Self {
        use crate::ai::orchestration_router::lifecycle::RunPhase;
        let mut state = Self::default();
        for record in run_records {
            let phase: RunPhase =
                serde_json::from_str(&record.lifecycle_state).unwrap_or(RunPhase::Created);
            let node = RunNode {
                run_id: RunId(record.run_id),
                root_run_id: RunId(record.root_run_id),
                parent_run_id: record.parent_run_id.map(RunId),
                conversation_id: serde_json::from_value(serde_json::Value::String(
                    record.conversation_id,
                ))
                .unwrap(),
                runtime: match record.runtime_kind.as_str() {
                    "local" => RuntimeKind::LocalBrain,
                    "cloud" => RuntimeKind::CloudAgent,
                    _ => RuntimeKind::LocalBrain,
                },
                agent_role: record.agent_role,
                lifecycle: RunLifecycle {
                    phase,
                    suspension_reason: None,
                    terminal_outcome: None,
                    join_generation: 0,
                },
                last_source_sequence: record.last_source_sequence.map(|v| v as u64),
                last_ingest_sequence: record.last_ingest_sequence as u64,
                created_at: record.created_at.and_utc().timestamp(),
                updated_at: record.updated_at.and_utc().timestamp(),
                terminal_reason: record.terminal_reason,
            };
            if matches!(node.lifecycle.phase, RunPhase::Terminal) {
                state.terminal_runs.insert(node.run_id.clone());
            } else {
                state.active_runs.insert(node.run_id.clone());
            }
            state.runs.insert(node.run_id.clone(), node);
        }
        for edge in edge_records {
            state
                .children_by_parent
                .entry(RunId(edge.parent_run_id.clone()))
                .or_default()
                .push(RunId(edge.child_run_id.clone()));
        }
        state
    }
}
