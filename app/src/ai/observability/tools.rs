use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use super::errors::NormalizedErrorType;
use super::progress::ToolProgressObservation;
use crate::ai::response_experience::source::{AgentId, LocalToolCallId};

pub type CanonicalToolName = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolClass {
    WarpClient,
    Mcp,
    Server,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolObservationOutcome {
    Completed,
    Failed,
    Denied,
    Cancelled,
    TimedOut,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolProgressSummary {
    pub completed: Option<f64>,
    pub total: Option<f64>,
    pub monotonic: bool,
}

impl From<&ToolProgressObservation> for ToolProgressSummary {
    fn from(obs: &ToolProgressObservation) -> Self {
        Self {
            completed: obs.completed,
            total: obs.total,
            monotonic: obs.monotonic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolObservation {
    pub tool_call_id: LocalToolCallId,
    pub tool_name: CanonicalToolName,
    pub tool_class: ToolClass,

    pub proposed_at: SystemTime,
    pub queued_at: Option<SystemTime>,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,

    pub permission_required: bool,
    pub permission_wait: Option<Duration>,

    pub outcome: Option<ToolObservationOutcome>,
    pub error_type: Option<NormalizedErrorType>,

    pub progress: ToolProgressSummary,
    pub result_size: Option<u64>,

    pub child_agent_id: Option<AgentId>,
}
