use uuid::Uuid;

use super::*;
use crate::ai::orchestration_router::event::{DurabilityClass, RuntimeEventSource};
use crate::ai::orchestration_router::ids::{CorrelationId, EventId, RunId};
use crate::AIConversationId;

fn create_test_event(payload: OrchestrationEvent) -> RuntimeEventEnvelope<OrchestrationEvent> {
    RuntimeEventEnvelope {
        event_id: EventId("e1".into()),
        schema_version: 1,
        source: RuntimeEventSource::LocalBrain,
        event_type: "test".into(),
        root_run_id: RunId("root".into()),
        run_id: RunId("run1".into()),
        parent_run_id: None,
        conversation_id: AIConversationId::new(),
        tool_call_id: None,
        source_sequence: Some(1),
        ingest_sequence: Some(1),
        correlation_id: CorrelationId("c1".into()),
        causation_id: None,
        traceparent: None,
        occurred_at: 0,
        recorded_at: 0,
        durability: DurabilityClass::Durable,
        payload,
    }
}

#[test]
fn test_register_root() {
    let mut state = RunGraphState::default();
    let event = create_test_event(OrchestrationEvent::RootRunRegistered);
    let changes = apply_event(&mut state, &event).unwrap();
    assert!(matches!(changes[0], DerivedStateChange::RunAdded(_)));
}

#[test]
fn test_duplicate_event() {
    let mut state = RunGraphState::default();
    let event = create_test_event(OrchestrationEvent::RootRunRegistered);
    apply_event(&mut state, &event).unwrap();

    // Applying the same event should yield an error (duplicate ingest sequence)
    let err = apply_event(&mut state, &event).unwrap_err();
    assert!(matches!(err, OrchestrationError::DuplicateEvent(_)));
}

#[test]
fn test_scenario_a_root_only_run() {
    let mut state = RunGraphState::default();

    // 1. Create root run
    let event_register = create_test_event(OrchestrationEvent::RootRunRegistered);
    apply_event(&mut state, &event_register).unwrap();

    // 2. Start root run
    let mut event_start = create_test_event(OrchestrationEvent::RunStarted);
    event_start.event_id = EventId("e2".into());
    event_start.ingest_sequence = Some(2);
    event_start.source_sequence = Some(2);
    apply_event(&mut state, &event_start).unwrap();

    // 3. Complete successfully
    let mut event_complete = create_test_event(OrchestrationEvent::RunCompleted);
    event_complete.event_id = EventId("e3".into());
    event_complete.ingest_sequence = Some(3);
    event_complete.source_sequence = Some(3);
    apply_event(&mut state, &event_complete).unwrap();

    assert_eq!(state.runs.len(), 1, "exactly one root");
    assert_eq!(state.children_by_parent.len(), 0, "zero children");

    let run = state.runs.get(&RunId("run1".into())).unwrap();
    assert!(
        matches!(
            run.lifecycle.phase,
            crate::ai::orchestration_router::lifecycle::RunPhase::Terminal
        ),
        "phase == Terminal"
    );
    assert!(
        matches!(
            run.lifecycle.terminal_outcome,
            Some(crate::ai::orchestration_router::lifecycle::TerminalOutcome::Completed)
        ),
        "terminal_outcome == Completed"
    );
}

#[test]
fn test_scenario_b_tool_execution_loop() {
    let mut state = RunGraphState::default();

    // 1. RootRunRegistered
    let event_register = create_test_event(OrchestrationEvent::RootRunRegistered);
    apply_event(&mut state, &event_register).unwrap();

    // 2. RunStarted
    let mut event_start = create_test_event(OrchestrationEvent::RunStarted);
    event_start.event_id = EventId("e2".into());
    event_start.ingest_sequence = Some(2);
    event_start.source_sequence = Some(2);
    apply_event(&mut state, &event_start).unwrap();

    // 3. ToolCallRequested
    let mut event_tool_req = create_test_event(OrchestrationEvent::ToolCallRequested);
    event_tool_req.event_id = EventId("e3".into());
    event_tool_req.ingest_sequence = Some(3);
    event_tool_req.source_sequence = Some(3);
    event_tool_req.tool_call_id = Some(crate::ai::orchestration_router::ids::ToolCallId(
        "tool1".into(),
    ));
    apply_event(&mut state, &event_tool_req).unwrap();

    // 4. ToolExecutionStarted
    let mut event_tool_start = create_test_event(OrchestrationEvent::ToolExecutionStarted);
    event_tool_start.event_id = EventId("e4".into());
    event_tool_start.ingest_sequence = Some(4);
    event_tool_start.source_sequence = Some(4);
    event_tool_start.tool_call_id = Some(crate::ai::orchestration_router::ids::ToolCallId(
        "tool1".into(),
    ));
    apply_event(&mut state, &event_tool_start).unwrap();

    // 5. ToolExecutionFinished
    let mut event_tool_fin = create_test_event(OrchestrationEvent::ToolExecutionFinished);
    event_tool_fin.event_id = EventId("e5".into());
    event_tool_fin.ingest_sequence = Some(5);
    event_tool_fin.source_sequence = Some(5);
    event_tool_fin.tool_call_id = Some(crate::ai::orchestration_router::ids::ToolCallId(
        "tool1".into(),
    ));
    apply_event(&mut state, &event_tool_fin).unwrap();

    // 6. RunCompleted
    let mut event_complete = create_test_event(OrchestrationEvent::RunCompleted);
    event_complete.event_id = EventId("e6".into());
    event_complete.ingest_sequence = Some(6);
    event_complete.source_sequence = Some(6);
    apply_event(&mut state, &event_complete).unwrap();

    // Asserts
    let run = state.runs.get(&RunId("run1".into())).unwrap();
    assert!(
        matches!(
            run.lifecycle.phase,
            crate::ai::orchestration_router::lifecycle::RunPhase::Terminal
        ),
        "phase == Terminal"
    );
    assert!(
        matches!(
            run.lifecycle.terminal_outcome,
            Some(crate::ai::orchestration_router::lifecycle::TerminalOutcome::Completed)
        ),
        "terminal_outcome == Completed"
    );
}

#[test]
fn test_scenario_c_multiple_child_runs() {
    let mut state = RunGraphState::default();

    // 1. RootRunRegistered
    let event_register = create_test_event(OrchestrationEvent::RootRunRegistered);
    apply_event(&mut state, &event_register).unwrap();

    // 2. Child A Spawned
    let mut event_child_a = create_test_event(OrchestrationEvent::ChildRunSpawned);
    event_child_a.event_id = EventId("e_ca".into());
    event_child_a.ingest_sequence = Some(10);
    event_child_a.source_sequence = Some(1);
    event_child_a.run_id = RunId("child_a".into());
    event_child_a.parent_run_id = Some(RunId("run1".into()));
    apply_event(&mut state, &event_child_a).unwrap();

    // 3. Child B Spawned
    let mut event_child_b = create_test_event(OrchestrationEvent::ChildRunSpawned);
    event_child_b.event_id = EventId("e_cb".into());
    event_child_b.ingest_sequence = Some(11);
    event_child_b.source_sequence = Some(2);
    event_child_b.run_id = RunId("child_b".into());
    event_child_b.parent_run_id = Some(RunId("run1".into()));
    apply_event(&mut state, &event_child_b).unwrap();

    // 4. Child C Spawned
    let mut event_child_c = create_test_event(OrchestrationEvent::ChildRunSpawned);
    event_child_c.event_id = EventId("e_cc".into());
    event_child_c.ingest_sequence = Some(12);
    event_child_c.source_sequence = Some(3);
    event_child_c.run_id = RunId("child_c".into());
    event_child_c.parent_run_id = Some(RunId("run1".into()));
    apply_event(&mut state, &event_child_c).unwrap();

    // 5. Child A Completed
    let mut event_child_a_fin = create_test_event(OrchestrationEvent::RunCompleted);
    event_child_a_fin.event_id = EventId("e_cafin".into());
    event_child_a_fin.ingest_sequence = Some(13);
    event_child_a_fin.source_sequence = Some(4);
    event_child_a_fin.run_id = RunId("child_a".into());
    event_child_a_fin.parent_run_id = Some(RunId("run1".into()));
    apply_event(&mut state, &event_child_a_fin).unwrap();

    // Asserts
    assert_eq!(state.runs.len(), 4, "One root and three children");

    let children = state.children_by_parent.get(&RunId("run1".into())).unwrap();
    assert_eq!(children.len(), 3, "Root has 3 children");
    assert!(children.contains(&RunId("child_a".into())));
    assert!(children.contains(&RunId("child_b".into())));
    assert!(children.contains(&RunId("child_c".into())));

    let root = state.runs.get(&RunId("run1".into())).unwrap();
    assert!(
        !matches!(
            root.lifecycle.phase,
            crate::ai::orchestration_router::lifecycle::RunPhase::Terminal
        ),
        "Root not terminal"
    );

    let child_a = state.runs.get(&RunId("child_a".into())).unwrap();
    assert!(
        matches!(
            child_a.lifecycle.phase,
            crate::ai::orchestration_router::lifecycle::RunPhase::Terminal
        ),
        "Child A is terminal"
    );

    let child_b = state.runs.get(&RunId("child_b".into())).unwrap();
    assert!(
        !matches!(
            child_b.lifecycle.phase,
            crate::ai::orchestration_router::lifecycle::RunPhase::Terminal
        ),
        "Child B not terminal"
    );
}

#[test]
fn test_scenario_d_duplicate_event() {
    let mut state = RunGraphState::default();
    let event = create_test_event(OrchestrationEvent::RootRunRegistered);

    let res1 = apply_event(&mut state, &event);
    assert!(res1.is_ok());
    assert_eq!(state.runs.len(), 1);

    let res2 = apply_event(&mut state, &event);
    assert!(res2.is_err());
    assert!(matches!(
        res2.unwrap_err(),
        OrchestrationError::DuplicateEvent(_)
    ));

    // State should not have changed
    assert_eq!(state.runs.len(), 1);
}

#[test]
fn test_scenario_e_stale_sequence() {
    let mut state = RunGraphState::default();

    // 1. RootRunRegistered
    let event_register = create_test_event(OrchestrationEvent::RootRunRegistered);
    apply_event(&mut state, &event_register).unwrap();

    // 2. Event with sequence 5
    let mut event_5 = create_test_event(OrchestrationEvent::RunStarted);
    event_5.event_id = EventId("e5".into());
    event_5.ingest_sequence = Some(5);
    event_5.source_sequence = Some(5);
    let res_5 = apply_event(&mut state, &event_5);
    assert!(res_5.is_ok());

    // 3. Event with sequence 4
    let mut event_4 = create_test_event(OrchestrationEvent::RunCompleted);
    event_4.event_id = EventId("e4".into());
    event_4.ingest_sequence = Some(6); // different ingest to pass ingest check
    event_4.source_sequence = Some(4);

    let res_4 = apply_event(&mut state, &event_4);
    assert!(res_4.is_err());
    assert!(matches!(
        res_4.unwrap_err(),
        OrchestrationError::StaleSequence(_)
    ));

    // State should not roll back to phase Terminal
    let run = state.runs.get(&RunId("run1".into())).unwrap();
    assert!(
        matches!(
            run.lifecycle.phase,
            crate::ai::orchestration_router::lifecycle::RunPhase::Working
        ),
        "Phase is not completed because event 4 was stale"
    );
}

#[test]
fn test_scenario_f_out_of_order_independent_children() {
    let mut state = RunGraphState::default();

    // 1. Child A seq 3
    let mut event_a3 = create_test_event(OrchestrationEvent::ChildRunSpawned);
    event_a3.event_id = EventId("ea3".into());
    event_a3.ingest_sequence = Some(1);
    event_a3.source_sequence = Some(3);
    event_a3.run_id = RunId("child_a".into());
    apply_event(&mut state, &event_a3).unwrap();

    // 2. Child B seq 7
    let mut event_b7 = create_test_event(OrchestrationEvent::ChildRunSpawned);
    event_b7.event_id = EventId("eb7".into());
    event_b7.ingest_sequence = Some(2);
    event_b7.source_sequence = Some(7);
    event_b7.run_id = RunId("child_b".into());
    apply_event(&mut state, &event_b7).unwrap();

    // 3. Child A seq 4
    let mut event_a4 = create_test_event(OrchestrationEvent::RunStarted);
    event_a4.event_id = EventId("ea4".into());
    event_a4.ingest_sequence = Some(3);
    event_a4.source_sequence = Some(4);
    event_a4.run_id = RunId("child_a".into());
    apply_event(&mut state, &event_a4).unwrap();

    // 4. Child B seq 8
    let mut event_b8 = create_test_event(OrchestrationEvent::RunStarted);
    event_b8.event_id = EventId("eb8".into());
    event_b8.ingest_sequence = Some(4);
    event_b8.source_sequence = Some(8);
    event_b8.run_id = RunId("child_b".into());
    apply_event(&mut state, &event_b8).unwrap();

    // Asserts
    let child_a = state.runs.get(&RunId("child_a".into())).unwrap();
    assert_eq!(child_a.last_source_sequence, Some(4));

    let child_b = state.runs.get(&RunId("child_b".into())).unwrap();
    assert_eq!(child_b.last_source_sequence, Some(8));
}

#[test]
fn test_scenario_k_spawn_requested() {
    let mut state = RunGraphState::default();

    // 1. RootRunRegistered
    let event_register = create_test_event(OrchestrationEvent::RootRunRegistered);
    apply_event(&mut state, &event_register).unwrap();

    // 2. ChildRunSpawnRequested
    let mut event_spawn_req = create_test_event(OrchestrationEvent::ChildRunSpawnRequested {
        child_task: crate::ai::orchestration_router::command::ChildTaskSpec {
            summary: "Test Task".into(),
            context_refs: vec![],
            pwd: None,
            os: None,
            shell: None,
        },
        runtime: crate::ai::orchestration_router::command::RuntimeKind::LocalBrain,
        join_policy: crate::ai::orchestration_router::policies::JoinPolicy::All,
        failure_policy: crate::ai::orchestration_router::policies::ChildFailurePolicy::FailParent,
        parent_close_policy:
            crate::ai::orchestration_router::policies::ParentClosePolicy::RequestCancellation,
    });
    event_spawn_req.event_id = EventId("e_spawn_req".into());
    event_spawn_req.ingest_sequence = Some(2);
    event_spawn_req.source_sequence = Some(2);
    event_spawn_req.run_id = RunId("child_x".into());
    event_spawn_req.parent_run_id = Some(RunId("run1".into()));

    // Assert applies without error
    let changes = apply_event(&mut state, &event_spawn_req).unwrap();
    assert_eq!(
        changes.len(),
        1,
        "Spawn requested should output parent RunUpdated change"
    );
    assert!(matches!(changes[0], DerivedStateChange::RunUpdated(_)));
}
