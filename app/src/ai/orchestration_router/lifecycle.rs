use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunPhase {
    Created,
    Submitted,
    Working,
    Suspended,
    Cancelling,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuspensionReason {
    UserInput,
    ToolApproval,
    ToolExecution,
    ChildRuns,
    Authentication,
    RetryBackoff,
    ResourceUnavailable,
    Backpressure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalOutcome {
    Completed,
    Failed,
    Cancelled,
    Rejected,
    Abandoned,
    GuardStopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunLifecycle {
    pub phase: RunPhase,
    pub suspension_reason: Option<SuspensionReason>,
    pub terminal_outcome: Option<TerminalOutcome>,
    #[serde(default)]
    pub join_generation: u32,
}

impl RunLifecycle {
    pub fn new() -> Self {
        Self {
            phase: RunPhase::Created,
            suspension_reason: None,
            terminal_outcome: None,
            join_generation: 0,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.phase != RunPhase::Suspended && self.suspension_reason.is_some() {
            return Err("suspension_reason must be None when phase is not Suspended");
        }
        if self.phase != RunPhase::Terminal && self.terminal_outcome.is_some() {
            return Err("terminal_outcome must be None when phase is not Terminal");
        }
        if self.phase == RunPhase::Terminal && self.terminal_outcome.is_none() {
            return Err("terminal_outcome must be Some when phase is Terminal");
        }
        Ok(())
    }
}
