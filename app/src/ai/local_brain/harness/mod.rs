//! Agent harness: the end-to-end execution core for the local brain.
//!
//! A harness run owns one root user request from intake to completion:
//! context assembly, model turns, in-process tool execution, observations,
//! and the final answer. The UI observes [`events::HarnessEvent`]s; it is not
//! a participant in the loop.
//!
//! The phase machine lives in [`phase`], tool execution in [`executor`], and
//! loop-control state is the existing [`control::ledger::Ledger`] so budget
//! enforcement stays consistent with the guard rails.

pub mod events;
pub mod executor;
pub mod executors;
mod output;
pub mod phase;

use super::control::guard::{
    check_preflight_guards, check_preflight_guards_with_repair_state, LoopLimitKind,
    LoopLimitStop,
};
use super::control::ledger::{Ledger, LocalAgentRunStatus, LocalAgentTerminationReason};
use super::control::profile::LoopControlProfile;

use events::{HarnessEvent, HarnessStopReason};
use phase::{HarnessPhase, InvalidTransition};

#[derive(Debug, Clone)]
pub struct HarnessConfig {
    pub profile: LoopControlProfile,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            profile: LoopControlProfile::contract(),
        }
    }
}

/// What the runner may do at the next turn boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum TurnDirective {
    /// Budget remains: run a normal model turn with tools.
    Proceed,
    /// The working budget is exhausted: run one final text-only turn with tools
    /// disabled, so the user gets a summary instead of a dead-end stop.
    WrapUp(LoopLimitStop),
    /// Stop now: no wrap-up applies, or it was already used.
    Stop(LoopLimitStop),
}

/// One harness run, owned by a single root user request.
pub struct AgentHarness {
    config: HarnessConfig,
    ledger: Ledger,
    phase: HarnessPhase,
    wrap_up_turns_used: u32,
}

impl AgentHarness {
    pub fn new(config: HarnessConfig, ledger: Ledger) -> Self {
        Self {
            config,
            ledger,
            phase: HarnessPhase::Intake,
            wrap_up_turns_used: 0,
        }
    }

    pub fn phase(&self) -> HarnessPhase {
        self.phase
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    pub fn config(&self) -> &HarnessConfig {
        &self.config
    }

    /// Advance to `next` and report the transition as an event.
    pub fn enter(&mut self, next: HarnessPhase) -> Result<HarnessEvent, InvalidTransition> {
        if !self.phase.allows(next) {
            return Err(InvalidTransition {
                from: self.phase,
                to: next,
            });
        }
        let from = self.phase;
        self.phase = next;
        Ok(HarnessEvent::PhaseChanged { from, to: next })
    }

    /// Check the loop budget before issuing another model turn.
    pub fn check_budget(&self) -> Result<(), LoopLimitStop> {
        check_preflight_guards(&self.ledger, &self.config.profile)
    }

    /// Decide the next turn. When the working budget is exhausted, one
    /// text-only wrap-up turn is granted (up to `wrap_up_model_turns`), so the
    /// run ends with the model's summary rather than a bare limit message. A
    /// wrap-up turn is not recorded with `record_model_turn`; the caller sends
    /// it with tools disabled and then calls `finish`.
    ///
    /// Safety trips (repeated identical calls, no measurable progress) stop
    /// immediately: an extra call cannot help a model that is already stuck.
    pub fn begin_turn(&mut self) -> TurnDirective {
        self.begin_turn_with_repair(false)
    }

    /// `begin_turn` with the active-repair exemption, so agentic repair keeps
    /// running past the budget exactly as the preflight guard allows.
    pub fn begin_turn_with_repair(&mut self, is_actively_repairing: bool) -> TurnDirective {
        match check_preflight_guards_with_repair_state(
            &self.ledger,
            &self.config.profile,
            is_actively_repairing,
        ) {
            Ok(()) => TurnDirective::Proceed,
            Err(stop) => {
                let budget_exhausted = matches!(
                    stop.kind,
                    LoopLimitKind::ModelTurns | LoopLimitKind::TotalToolCalls
                );
                if budget_exhausted
                    && self.wrap_up_turns_used < self.config.profile.wrap_up_model_turns
                {
                    self.wrap_up_turns_used = self.wrap_up_turns_used.saturating_add(1);
                    TurnDirective::WrapUp(stop)
                } else {
                    TurnDirective::Stop(stop)
                }
            }
        }
    }

    /// Mark the run as continuing because the client will execute another tool
    /// batch and re-enter with its results.
    pub fn mark_active(&mut self) {
        self.ledger.status = LocalAgentRunStatus::Active;
        self.ledger.terminal_reason = None;
    }

    /// Record one assistant turn and the tool calls it requested.
    pub fn record_model_turn(&mut self, requested_tool_calls: u32) {
        self.ledger.model_turns = self.ledger.model_turns.saturating_add(1);
        self.ledger.tool_calls = self.ledger.tool_calls.saturating_add(requested_tool_calls);
        if requested_tool_calls > 0 {
            self.ledger.tool_batches = self.ledger.tool_batches.saturating_add(1);
        }
    }

    /// Record the outcome of a tool batch. A batch in which nothing succeeded
    /// counts toward the no-progress limit; any success resets the streak.
    pub fn record_tool_batch(&mut self, outcomes: &[executor::ToolOutcome]) {
        if outcomes.iter().any(executor::ToolOutcome::is_success) {
            self.ledger.consecutive_no_progress_batches = 0;
        } else {
            self.ledger.consecutive_no_progress_batches = self
                .ledger
                .consecutive_no_progress_batches
                .saturating_add(1);
        }
    }

    /// Record the calls an in-process batch executed so repeated-loop detection
    /// covers harness-owned tools, not only calls rebuilt from task history.
    pub fn record_tool_call_fingerprints<I>(&mut self, calls: I)
    where
        I: IntoIterator<Item = (String, String)>,
    {
        self.ledger.record_tool_calls(calls);
    }

    /// Terminate the run: drive the phase machine to its terminal state and
    /// record the reason in the ledger.
    pub fn finish(&mut self, reason: HarnessStopReason) -> HarnessEvent {
        let target = reason.terminal_phase();
        if !self.phase.is_terminal() && self.phase != target {
            if target != HarnessPhase::Cancelled && self.phase != HarnessPhase::Finalizing {
                let _ = self.enter(HarnessPhase::Finalizing);
            }
            let _ = self.enter(target);
        }

        self.ledger.status = LocalAgentRunStatus::Terminated;
        if reason.is_success() {
            self.ledger.terminal_reason = Some(LocalAgentTerminationReason::CompletedSuccessfully);
        }

        HarnessEvent::Stopped { reason }
    }
}

impl From<LoopLimitStop> for HarnessStopReason {
    fn from(stop: LoopLimitStop) -> Self {
        match stop.kind {
            LoopLimitKind::ModelTurns => Self::ModelTurnLimit {
                observed: stop.observed,
                limit: stop.limit,
            },
            LoopLimitKind::TotalToolCalls => Self::ToolCallLimit {
                observed: stop.observed,
                limit: stop.limit,
            },
            LoopLimitKind::NoMeasurableProgress => Self::NoMeasurableProgress {
                observed: stop.observed,
                limit: stop.limit,
            },
            LoopLimitKind::RepeatedToolLoop => Self::RepeatedToolLoop,
        }
    }
}

#[cfg(test)]
#[path = "harness_tests.rs"]
mod tests;
