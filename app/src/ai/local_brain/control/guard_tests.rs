use super::*;
use crate::ai::local_brain::control::ledger::Ledger;

#[test]
fn allows_counts_below_contract_limits() {
    let ledger = Ledger {
        model_turns: 5,
        tool_calls: 11,
        ..Default::default()
    };

    assert!(check_preflight_guards(&ledger, &LoopControlProfile::contract()).is_ok());
}

#[test]
fn stops_when_model_turn_limit_is_reached() {
    let ledger = Ledger {
        model_turns: 6,
        ..Default::default()
    };

    let stop = check_preflight_guards(&ledger, &LoopControlProfile::contract()).unwrap_err();
    assert_eq!(stop.kind, LoopLimitKind::ModelTurns);
    assert_eq!(stop.observed, 6);
    assert_eq!(
        stop.user_message(),
        "Local brain stopped: model turn limit reached (6). Send a new message to continue."
    );
}

#[test]
fn stops_when_total_tool_call_limit_is_reached() {
    let ledger = Ledger {
        tool_calls: 12,
        ..Default::default()
    };

    let stop = check_preflight_guards(&ledger, &LoopControlProfile::contract()).unwrap_err();
    assert_eq!(stop.kind, LoopLimitKind::TotalToolCalls);
    assert_eq!(stop.observed, 12);
    assert_eq!(
        stop.user_message(),
        "Local brain stopped: total tool-call limit reached (12). Send a new message to continue."
    );
}

#[test]
fn allows_unbounded_turns_when_repairing() {
    let ledger = Ledger {
        model_turns: 20,
        tool_calls: 50,
        consecutive_no_progress_batches: 10,
        ..Default::default()
    };

    assert!(check_preflight_guards_with_repair_state(
        &ledger,
        &LoopControlProfile::contract(),
        true
    ).is_ok());
}

#[test]
fn allows_unbounded_turns_with_unbounded_profile() {
    let ledger = Ledger {
        model_turns: 20,
        tool_calls: 50,
        consecutive_no_progress_batches: 10,
        ..Default::default()
    };

    assert!(check_preflight_guards(
        &ledger,
        &LoopControlProfile::contract_unbounded()
    ).is_ok());
}
