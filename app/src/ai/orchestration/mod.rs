pub mod pipeline_tracker;

use pipeline_tracker::PipelineTracker;
use warpui_core::Entity;

#[derive(Debug, Clone)]
pub struct OrchestrationPipeline {
    pub tracker: PipelineTracker,
    pub active_conversation_id: Option<crate::AIConversationId>,
    pub parent_task: String,
}

impl OrchestrationPipeline {
    pub fn new(tracker: PipelineTracker, parent_task: String) -> Self {
        Self {
            tracker,
            active_conversation_id: None,
            parent_task,
        }
    }
}

pub fn pipeline_handoff_prompt(
    parent_task: &str,
    agent_name: &str,
    agent_index: usize,
    agent_count: usize,
) -> String {
    format!(
        "You are {agent_name}, agent {agent_index} of {agent_count} in this pipeline. Continue the parent task in this same conversation, using the complete conversation history and existing workspace state. Do not run /init and do not ask the user to restate the task.\n\nParent task:\n{}",
        parent_task.trim()
    )
}

#[cfg(test)]
#[path = "orchestration_tests.rs"]
mod tests;

impl Entity for OrchestrationPipeline {
    type Event = ();
}
