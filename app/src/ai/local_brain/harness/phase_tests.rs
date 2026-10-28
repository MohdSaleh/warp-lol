use super::*;

const LIVE_PHASES: [HarnessPhase; 6] = [
    HarnessPhase::Intake,
    HarnessPhase::ContextAssembly,
    HarnessPhase::ModelTurn,
    HarnessPhase::ToolBatch,
    HarnessPhase::Observation,
    HarnessPhase::Finalizing,
];

const ALL_PHASES: [HarnessPhase; 9] = [
    HarnessPhase::Intake,
    HarnessPhase::ContextAssembly,
    HarnessPhase::ModelTurn,
    HarnessPhase::ToolBatch,
    HarnessPhase::Observation,
    HarnessPhase::Finalizing,
    HarnessPhase::Completed,
    HarnessPhase::Failed,
    HarnessPhase::Cancelled,
];

#[test]
fn happy_path_transitions_are_allowed() {
    use HarnessPhase::*;
    let path = [
        (Intake, ContextAssembly),
        (ContextAssembly, ModelTurn),
        (ModelTurn, ToolBatch),
        (ToolBatch, Observation),
        (Observation, ModelTurn),
        (ModelTurn, Finalizing),
        (Finalizing, Completed),
    ];
    for (from, to) in path {
        assert!(from.allows(to), "{from} should allow {to}");
    }
}

#[test]
fn a_turn_without_tools_goes_straight_to_finalizing() {
    assert!(HarnessPhase::ModelTurn.allows(HarnessPhase::Finalizing));
}

#[test]
fn terminal_phases_absorb_every_transition() {
    for terminal in [
        HarnessPhase::Completed,
        HarnessPhase::Failed,
        HarnessPhase::Cancelled,
    ] {
        assert!(terminal.is_terminal());
        for next in ALL_PHASES {
            assert!(
                !terminal.allows(next),
                "{terminal} must not allow {next}"
            );
        }
    }
}

#[test]
fn live_phases_can_fail_or_cancel() {
    for phase in LIVE_PHASES {
        assert!(phase.allows(HarnessPhase::Failed), "{phase} should allow Failed");
        assert!(
            phase.allows(HarnessPhase::Cancelled),
            "{phase} should allow Cancelled"
        );
    }
}

#[test]
fn skipping_phases_is_rejected() {
    let invalid = [
        (HarnessPhase::Intake, HarnessPhase::ModelTurn),
        (HarnessPhase::Intake, HarnessPhase::Completed),
        (HarnessPhase::ContextAssembly, HarnessPhase::ToolBatch),
        (HarnessPhase::ToolBatch, HarnessPhase::ModelTurn),
        (HarnessPhase::Observation, HarnessPhase::ToolBatch),
        (HarnessPhase::Finalizing, HarnessPhase::ModelTurn),
    ];
    for (from, to) in invalid {
        assert!(!from.allows(to), "{from} must not allow {to}");
    }
}

#[test]
fn phase_labels_are_stable() {
    assert_eq!(HarnessPhase::Intake.as_str(), "intake");
    assert_eq!(HarnessPhase::ContextAssembly.as_str(), "context_assembly");
    assert_eq!(HarnessPhase::ModelTurn.as_str(), "model_turn");
    assert_eq!(HarnessPhase::ToolBatch.as_str(), "tool_batch");
    assert_eq!(HarnessPhase::Observation.as_str(), "observation");
    assert_eq!(HarnessPhase::Finalizing.as_str(), "finalizing");
    assert_eq!(HarnessPhase::Completed.as_str(), "completed");
    assert_eq!(HarnessPhase::Failed.as_str(), "failed");
    assert_eq!(HarnessPhase::Cancelled.as_str(), "cancelled");
}

#[test]
fn transition_error_describes_both_phases() {
    let error = InvalidTransition {
        from: HarnessPhase::Intake,
        to: HarnessPhase::Completed,
    };
    assert_eq!(
        error.to_string(),
        "invalid harness phase transition: intake -> completed"
    );
}
