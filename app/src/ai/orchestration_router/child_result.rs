use serde::{Deserialize, Serialize};

use crate::ai::orchestration_router::command::ChildTaskSpec;
use crate::ai::orchestration_router::ids::RunId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildRunResult {
    pub run_id: RunId,
    pub task: ChildTaskSpec,
    pub terminal_outcome: super::lifecycle::TerminalOutcome,
    pub final_text: Option<String>,
    pub tool_calls: Vec<super::command::CanonicalToolCall>,
}

impl ChildRunResult {
    pub fn new(
        run_id: RunId,
        task: ChildTaskSpec,
        terminal_outcome: super::lifecycle::TerminalOutcome,
        final_text: Option<String>,
        mut tool_calls: Vec<super::command::CanonicalToolCall>,
    ) -> Self {
        // Enforce bounds to prevent out-of-memory cascades
        let final_text = final_text.map(|mut t| {
            if t.len() > 10_000 {
                t.truncate(10_000);
                t.push_str("...[truncated]");
            }
            t
        });

        if tool_calls.len() > 10 {
            tool_calls.truncate(10);
        }

        Self {
            run_id,
            task,
            terminal_outcome,
            final_text,
            tool_calls,
        }
    }
}
