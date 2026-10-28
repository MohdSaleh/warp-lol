use std::fmt;

use serde::{Deserialize, Serialize};

/// Explicit execution phases of the local agent harness.
///
/// Every step of a run is a named phase, so run state can be inspected,
/// persisted, and resumed without inferring it from event ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarnessPhase {
    /// Accepting the user request and selecting the model / agent route.
    Intake,
    /// Assembling the model-visible transcript (history, rules, compaction).
    ContextAssembly,
    /// Streaming one assistant turn from the model endpoint.
    ModelTurn,
    /// Executing the tool calls requested by the previous assistant turn.
    ToolBatch,
    /// Feeding tool outcomes back into the transcript.
    Observation,
    /// Producing the final answer or stop reason.
    Finalizing,
    Completed,
    Failed,
    Cancelled,
}

impl HarnessPhase {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Whether the harness may move from `self` to `next`.
    ///
    /// The loop is `ModelTurn -> ToolBatch -> Observation -> ModelTurn`; a turn
    /// that requests no tools goes straight to `Finalizing`. Terminal phases
    /// absorb by design: a finished run cannot be revived.
    pub fn allows(self, next: Self) -> bool {
        use HarnessPhase::*;
        matches!(
            (self, next),
            (Intake, ContextAssembly | Failed | Cancelled)
                | (ContextAssembly, ModelTurn | Failed | Cancelled)
                | (ModelTurn, ToolBatch | Finalizing | Failed | Cancelled)
                | (ToolBatch, Observation | Failed | Cancelled)
                | (Observation, ModelTurn | Finalizing | Failed | Cancelled)
                | (Finalizing, Completed | Failed | Cancelled)
        )
    }

    /// Stable label for logs and persisted run records.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Intake => "intake",
            Self::ContextAssembly => "context_assembly",
            Self::ModelTurn => "model_turn",
            Self::ToolBatch => "tool_batch",
            Self::Observation => "observation",
            Self::Finalizing => "finalizing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for HarnessPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTransition {
    pub from: HarnessPhase,
    pub to: HarnessPhase,
}

impl fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid harness phase transition: {} -> {}",
            self.from, self.to
        )
    }
}

impl std::error::Error for InvalidTransition {}

#[cfg(test)]
#[path = "phase_tests.rs"]
mod tests;
