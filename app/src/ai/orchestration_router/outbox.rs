use serde::{Deserialize, Serialize};

use crate::ai::orchestration_router::command::{ChildTaskSpec, RuntimeKind};
use crate::ai::orchestration_router::ids::RunId;
use crate::AIConversationId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrchestrationOutboxEffect {
    StartChildRun {
        parent_task_id: AIConversationId, // Ensure it's AIConversationId
        child_run_id: String,
        runtime: RuntimeKind,
        task: ChildTaskSpec,
    },
    ResumeParent {
        parent_task_id: AIConversationId,
        child_results: Vec<crate::ai::orchestration_router::child_result::ChildRunResult>,
    },
    CancelChildRun {
        child_run_id: RunId,
        reason: String,
    },
}
