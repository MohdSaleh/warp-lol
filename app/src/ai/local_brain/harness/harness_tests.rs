use super::*;
use crate::ai::local_brain::harness::executor::ToolOutcome;

fn harness() -> AgentHarness {
    AgentHarness::new(HarnessConfig::default(), Ledger::default())
}

#[test]
fn full_tool_loop_reaches_completed() {
    let mut harness = harness();

    harness.enter(HarnessPhase::ContextAssembly).unwrap();
    harness.enter(HarnessPhase::ModelTurn).unwrap();
    harness.record_model_turn(1);
    harness.enter(HarnessPhase::ToolBatch).unwrap();
    harness.record_tool_batch(&[ToolOutcome::success("ok")]);
    harness.enter(HarnessPhase::Observation).unwrap();

    harness.enter(HarnessPhase::ModelTurn).unwrap();
    harness.record_model_turn(0);
    harness.enter(HarnessPhase::Finalizing).unwrap();
    let event = harness.finish(HarnessStopReason::Completed);

    assert_eq!(harness.phase(), HarnessPhase::Completed);
    assert!(matches!(
        event,
        HarnessEvent::Stopped {
            reason: HarnessStopReason::Completed
        }
    ));
    assert!(matches!(
        harness.ledger().status,
        LocalAgentRunStatus::Terminated
    ));
    assert!(matches!(
        harness.ledger().terminal_reason,
        Some(LocalAgentTerminationReason::CompletedSuccessfully)
    ));
}

#[test]
fn invalid_transition_is_rejected_and_state_is_preserved() {
    let mut harness = harness();

    let error = harness.enter(HarnessPhase::ModelTurn).unwrap_err();

    assert_eq!(error.from, HarnessPhase::Intake);
    assert_eq!(error.to, HarnessPhase::ModelTurn);
    assert_eq!(harness.phase(), HarnessPhase::Intake);
}

#[test]
fn budget_stop_maps_to_harness_reason() {
    let mut harness = harness();
    for _ in 0..6 {
        harness.record_model_turn(0);
    }

    let stop = harness.check_budget().unwrap_err();
    let reason = HarnessStopReason::from(stop);

    assert!(matches!(
        reason,
        HarnessStopReason::ModelTurnLimit {
            observed: 6,
            limit: 6
        }
    ));
    assert_eq!(reason.terminal_phase(), HarnessPhase::Failed);
    assert!(!reason.is_success());
}

#[test]
fn budget_allows_normal_turns() {
    let mut harness = harness();
    assert_eq!(harness.begin_turn(), TurnDirective::Proceed);
}

#[test]
fn exhausted_budget_grants_one_wrap_up_turn() {
    let mut harness = harness();
    for _ in 0..6 {
        harness.record_model_turn(0);
    }

    assert!(matches!(
        harness.begin_turn(),
        TurnDirective::WrapUp(LoopLimitStop {
            kind: LoopLimitKind::ModelTurns,
            ..
        })
    ));
    assert_eq!(harness.ledger().model_turns, 6);
    assert!(matches!(
        harness.begin_turn(),
        TurnDirective::Stop(LoopLimitStop {
            kind: LoopLimitKind::ModelTurns,
            ..
        })
    ));
}

#[test]
fn wrap_up_can_be_disabled() {
    let mut config = HarnessConfig::default();
    config.profile.wrap_up_model_turns = 0;
    let mut harness = AgentHarness::new(config, Ledger::default());
    for _ in 0..6 {
        harness.record_model_turn(0);
    }

    assert!(matches!(harness.begin_turn(), TurnDirective::Stop(_)));
}

#[test]
fn tool_batch_progress_resets_the_no_progress_streak() {
    let mut harness = harness();

    harness.record_tool_batch(&[ToolOutcome::error("boom")]);
    harness.record_tool_batch(&[ToolOutcome::rejected("blocked")]);
    assert_eq!(harness.ledger().consecutive_no_progress_batches, 2);

    harness.record_tool_batch(&[ToolOutcome::success("ok")]);
    assert_eq!(harness.ledger().consecutive_no_progress_batches, 0);
}

#[test]
fn model_turn_record_counts_calls_and_batches() {
    let mut harness = harness();

    harness.record_model_turn(2);

    assert_eq!(harness.ledger().model_turns, 1);
    assert_eq!(harness.ledger().tool_calls, 2);
    assert_eq!(harness.ledger().tool_batches, 1);

    harness.record_model_turn(0);
    assert_eq!(harness.ledger().model_turns, 2);
    assert_eq!(harness.ledger().tool_batches, 1);
}

#[test]
fn failure_finishes_through_finalizing() {
    let mut harness = harness();
    harness.enter(HarnessPhase::ContextAssembly).unwrap();
    harness.enter(HarnessPhase::ModelTurn).unwrap();

    let event = harness.finish(HarnessStopReason::Failed {
        message: "provider exploded".to_string(),
    });

    assert_eq!(harness.phase(), HarnessPhase::Failed);
    assert!(matches!(
        event,
        HarnessEvent::Stopped {
            reason: HarnessStopReason::Failed { .. }
        }
    ));
}

#[test]
fn cancellation_skips_finalizing() {
    let mut harness = harness();
    harness.enter(HarnessPhase::ContextAssembly).unwrap();
    harness.enter(HarnessPhase::ModelTurn).unwrap();

    harness.finish(HarnessStopReason::Cancelled);

    assert_eq!(harness.phase(), HarnessPhase::Cancelled);
}
