use super::ledger::LocalAgentRunLedger;
use super::profile::LoopControlProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopLimitKind {
    ModelTurns,
    TotalToolCalls,
    RepeatedToolLoop,
    NoMeasurableProgress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopLimitStop {
    pub kind: LoopLimitKind,
    pub observed: u32,
    pub limit: u32,
}

impl LoopLimitStop {
    pub fn user_message(&self) -> String {
        match self.kind {
            LoopLimitKind::ModelTurns => format!(
                "Local brain stopped: model turn limit reached ({}). Send a new message to continue.",
                self.observed
            ),
            LoopLimitKind::TotalToolCalls => format!(
                "Local brain stopped: total tool-call limit reached ({}). Send a new message to continue.",
                self.observed
            ),
            LoopLimitKind::RepeatedToolLoop => {
                "Local brain stopped: the model repeated the same tool call 3 times.".to_string()
            }
            LoopLimitKind::NoMeasurableProgress => format!(
                "Local brain stopped: no progress detected across {} consecutive tool batches.",
                self.observed
            ),
        }
    }
}

pub fn check_preflight_guards(
    ledger: &LocalAgentRunLedger,
    profile: &LoopControlProfile,
) -> Result<(), LoopLimitStop> {
    check_preflight_guards_with_repair_state(ledger, profile, false)
}

pub fn check_preflight_guards_with_repair_state(
    ledger: &LocalAgentRunLedger,
    profile: &LoopControlProfile,
    is_actively_repairing: bool,
) -> Result<(), LoopLimitStop> {
    let unbounded = profile.unbounded_agentic_repair || is_actively_repairing;

    if !unbounded {
        if ledger.model_turns >= profile.hard_model_turn_limit {
            return Err(LoopLimitStop {
                kind: LoopLimitKind::ModelTurns,
                observed: ledger.model_turns,
                limit: profile.hard_model_turn_limit,
            });
        }

        if ledger.tool_calls >= profile.hard_tool_call_limit {
            return Err(LoopLimitStop {
                kind: LoopLimitKind::TotalToolCalls,
                observed: ledger.tool_calls,
                limit: profile.hard_tool_call_limit,
            });
        }

        if ledger.consecutive_no_progress_batches >= profile.hard_no_progress_batch_limit {
            return Err(LoopLimitStop {
                kind: LoopLimitKind::NoMeasurableProgress,
                observed: ledger.consecutive_no_progress_batches,
                limit: profile.hard_no_progress_batch_limit,
            });
        }
    }

    let n = ledger.recent_tool_calls.len();
    if n >= 3 {
        let last = &ledger.recent_tool_calls[n - 1].fingerprint_hash;
        if ledger.recent_tool_calls[n - 2].fingerprint_hash == *last
            && ledger.recent_tool_calls[n - 3].fingerprint_hash == *last
        {
            return Err(LoopLimitStop {
                kind: LoopLimitKind::RepeatedToolLoop,
                observed: 3,
                limit: 3,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "guard_tests.rs"]
mod tests;
