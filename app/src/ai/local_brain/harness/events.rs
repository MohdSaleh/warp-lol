use serde::{Deserialize, Serialize};

use super::executor::ToolOutcome;
use super::phase::HarnessPhase;

/// Why a harness run stopped. Mirrors the loop-control and terminal states so
/// the UI and the run record agree on a single vocabulary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HarnessStopReason {
    Completed,
    Cancelled,
    ModelTurnLimit { observed: u32, limit: u32 },
    ToolCallLimit { observed: u32, limit: u32 },
    NoMeasurableProgress { observed: u32, limit: u32 },
    RepeatedToolLoop,
    Failed { message: String },
}

impl HarnessStopReason {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }

    pub fn terminal_phase(&self) -> HarnessPhase {
        match self {
            Self::Completed => HarnessPhase::Completed,
            Self::Cancelled => HarnessPhase::Cancelled,
            Self::ModelTurnLimit { .. }
            | Self::ToolCallLimit { .. }
            | Self::NoMeasurableProgress { .. }
            | Self::RepeatedToolLoop
            | Self::Failed { .. } => HarnessPhase::Failed,
        }
    }
}

/// Everything a harness run emits. This is the only outbound contract: the
/// terminal UI, persistence, and tests all consume the same stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HarnessEvent {
    PhaseChanged {
        from: HarnessPhase,
        to: HarnessPhase,
    },
    /// Incremental assistant text, forwarded as it streams.
    AssistantDelta {
        text: String,
    },
    /// A completed assistant message for the transcript.
    AssistantMessage {
        text: String,
    },
    ToolCallRequested {
        call_id: String,
        name: String,
        arguments: serde_json::Value,
    },
    ToolCallCompleted {
        call_id: String,
        outcome: ToolOutcome,
    },
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    Stopped {
        reason: HarnessStopReason,
    },
}
