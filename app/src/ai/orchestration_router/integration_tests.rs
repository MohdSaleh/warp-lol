#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use diesel::sqlite::SqliteConnection;
    use diesel::Connection;
    use diesel_migrations::MigrationHarness;

    use crate::ai::orchestration_router::event::*;
    use crate::ai::orchestration_router::ids::RunId;
    use crate::ai::orchestration_router::lifecycle::RunPhase;
    use crate::ai::orchestration_router::model::OrchestrationRouterModel;
    use crate::ai::orchestration_router::persistence::{
        load_all_edges, load_all_runs, persist_edge, persist_run,
    };
    use crate::ai::orchestration_router::state::RunGraphState;
    use crate::persistence::model::{OrchestrationRouterEdgeRecord, OrchestrationRouterRunRecord};

    fn test_connection() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:")
            .expect("in-memory sqlite connection should open");
        conn.run_pending_migrations(::persistence::MIGRATIONS)
            .expect("migrations should run");
        conn
    }

    #[test]
    fn test_scenario_g_restart_recovery() {
        let mut conn = test_connection();
        let record_a = OrchestrationRouterRunRecord {
            run_id: "root_1".to_string(),
            root_run_id: "root_1".to_string(),
            parent_run_id: None,
            conversation_id: uuid::Uuid::new_v4().to_string(),
            runtime_kind: "local".to_string(),
            agent_role: None,
            lifecycle_state: "\"Working\"".to_string(),
            last_source_sequence: Some(2),
            last_ingest_sequence: 1,
            created_at: chrono::Local::now().naive_local(),
            updated_at: chrono::Local::now().naive_local(),
            terminal_reason: None,
        };
        let record_b = OrchestrationRouterRunRecord {
            run_id: "child_1".to_string(),
            root_run_id: "root_1".to_string(),
            parent_run_id: Some("root_1".to_string()),
            conversation_id: uuid::Uuid::new_v4().to_string(),
            runtime_kind: "cloud".to_string(),
            agent_role: None,
            lifecycle_state: "\"Terminal\"".to_string(),
            last_source_sequence: Some(5),
            last_ingest_sequence: 2,
            created_at: chrono::Local::now().naive_local(),
            updated_at: chrono::Local::now().naive_local(),
            terminal_reason: Some("Completed".to_string()),
        };
        persist_run(&mut conn, record_a).unwrap();
        persist_run(&mut conn, record_b).unwrap();

        let edge = OrchestrationRouterEdgeRecord {
            parent_run_id: "root_1".to_string(),
            child_run_id: "child_1".to_string(),
        };
        persist_edge(&mut conn, edge).unwrap();

        let runs = load_all_runs(&mut conn).unwrap();
        let edges = load_all_edges(&mut conn).unwrap();
        let state = RunGraphState::restore(runs, edges);

        assert_eq!(state.runs.len(), 2);
        let root = state.runs.get(&RunId("root_1".to_string())).unwrap();
        assert!(matches!(root.lifecycle.phase, RunPhase::Working));
        let children = state
            .children_by_parent
            .get(&RunId("root_1".to_string()))
            .unwrap();
        assert!(children.contains(&RunId("child_1".to_string())));

        let child = state.runs.get(&RunId("child_1".to_string())).unwrap();
        assert!(matches!(child.lifecycle.phase, RunPhase::Terminal));
        assert_eq!(child.parent_run_id, Some(RunId("root_1".to_string())));
    }

    #[test]
    fn test_scenario_h_transactional_failure() {
        let mut conn = test_connection();
        let result: std::result::Result<(), diesel::result::Error> = conn.transaction(|conn| {
            let record = OrchestrationRouterRunRecord {
                run_id: "trans_1".to_string(),
                root_run_id: "trans_1".to_string(),
                parent_run_id: None,
                conversation_id: uuid::Uuid::new_v4().to_string(),
                runtime_kind: "local".to_string(),
                agent_role: None,
                lifecycle_state: "\"Working\"".to_string(),
                last_source_sequence: Some(1),
                last_ingest_sequence: 1,
                created_at: chrono::Local::now().naive_local(),
                updated_at: chrono::Local::now().naive_local(),
                terminal_reason: None,
            };
            persist_run(conn, record).unwrap();
            Err(diesel::result::Error::RollbackTransaction)
        });
        assert!(result.is_err());
        let runs = load_all_runs(&mut conn).unwrap();
        assert_eq!(runs.len(), 0, "No partial records should remain");
    }
}

#[cfg(test)]
mod side_effect_tests {
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::ai::orchestration_router::event::{
        DurabilityClass, OrchestrationEvent, RuntimeEventEnvelope, RuntimeEventSource,
    };
    use crate::ai::orchestration_router::ids::{EventId, RunId};
    use crate::ai::orchestration_router::lifecycle::RunPhase;
    use crate::ai::orchestration_router::reducer::apply_event;
    use crate::ai::orchestration_router::state::RunGraphState;
    use crate::AIConversationId;

    #[test]
    fn test_scenario_i_observation_only_isolation() {
        let mut state = RunGraphState::default();
        let event = OrchestrationEvent::RunStarted;
        let envelope = RuntimeEventEnvelope {
            event_id: EventId("e1".to_string()),
            schema_version: 1,
            source: RuntimeEventSource::LocalBrain,
            event_type: "RunStarted".to_string(),
            root_run_id: RunId("run_iso".to_string()),
            run_id: RunId("run_iso".to_string()),
            parent_run_id: None,
            conversation_id: AIConversationId::new(),
            tool_call_id: None,
            source_sequence: Some(1),
            ingest_sequence: Some(1),
            correlation_id: crate::ai::orchestration_router::ids::CorrelationId(
                "corr1".to_string(),
            ),
            causation_id: None,
            traceparent: None,
            occurred_at: 0,
            recorded_at: 0,
            durability: DurabilityClass::Ephemeral,
            payload: event,
        };

        let initial_dispatch =
            crate::ai::orchestration_router::test_spies::TEST_ACTION_DISPATCH_COUNT
                .load(Ordering::SeqCst);
        let initial_exec =
            crate::ai::orchestration_router::test_spies::TEST_EXECUTOR_COUNT.load(Ordering::SeqCst);
        let initial_followup =
            crate::ai::orchestration_router::test_spies::TEST_FOLLOWUP_REQUEST_COUNT
                .load(Ordering::SeqCst);
        let initial_child = crate::ai::orchestration_router::test_spies::TEST_CHILD_START_COUNT
            .load(Ordering::SeqCst);
        let initial_router = crate::ai::orchestration_router::test_spies::TEST_ROUTER_EFFECT_COUNT
            .load(Ordering::SeqCst);

        // We process an event via the reducer
        let _ = apply_event(&mut state, &envelope);

        let final_dispatch =
            crate::ai::orchestration_router::test_spies::TEST_ACTION_DISPATCH_COUNT
                .load(Ordering::SeqCst);
        let final_exec =
            crate::ai::orchestration_router::test_spies::TEST_EXECUTOR_COUNT.load(Ordering::SeqCst);
        let final_followup =
            crate::ai::orchestration_router::test_spies::TEST_FOLLOWUP_REQUEST_COUNT
                .load(Ordering::SeqCst);
        let final_child = crate::ai::orchestration_router::test_spies::TEST_CHILD_START_COUNT
            .load(Ordering::SeqCst);
        let final_router = crate::ai::orchestration_router::test_spies::TEST_ROUTER_EFFECT_COUNT
            .load(Ordering::SeqCst);

        assert_eq!(
            initial_dispatch, final_dispatch,
            "Router should not dispatch actions"
        );
        assert_eq!(
            initial_exec, final_exec,
            "Router should not execute actions directly"
        );
        assert_eq!(
            initial_followup, final_followup,
            "Router should not request follow-ups"
        );
        assert_eq!(
            initial_child, final_child,
            "Router should not start child agents directly"
        );
        assert_eq!(
            initial_router, final_router,
            "Router should not emit side effects"
        );
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    use crate::ai::orchestration_router::ids::RunId;
    use crate::ai::orchestration_router::projection::descendant_conversation_ids_in_spawn_order;
    use crate::ai::orchestration_router::state::RunGraphState;
    use crate::AIConversationId;

    #[test]
    fn test_scenario_j_pill_bar_projection_parity() {
        let mut state = RunGraphState::default();
        let conv_1 = AIConversationId::new();
        let conv_2 = AIConversationId::new();
        let conv_3 = AIConversationId::new();

        let root = crate::ai::orchestration_router::state::RunNode {
            run_id: RunId("root_1".to_string()),
            root_run_id: RunId("root_1".to_string()),
            parent_run_id: None,
            conversation_id: conv_1,
            runtime: crate::ai::orchestration_router::command::RuntimeKind::LocalBrain,
            agent_role: None,
            lifecycle: crate::ai::orchestration_router::lifecycle::RunLifecycle {
                phase: crate::ai::orchestration_router::lifecycle::RunPhase::Working,
                suspension_reason: None,
                terminal_outcome: None,
                join_generation: 0,
            },
            last_source_sequence: None,
            last_ingest_sequence: 1,
            created_at: 0,
            updated_at: 0,
            terminal_reason: None,
        };
        state.runs.insert(RunId("root_1".to_string()), root);

        let child1 = crate::ai::orchestration_router::state::RunNode {
            run_id: RunId("child_1".to_string()),
            root_run_id: RunId("root_1".to_string()),
            parent_run_id: Some(RunId("root_1".to_string())),
            conversation_id: conv_2,
            runtime: crate::ai::orchestration_router::command::RuntimeKind::LocalBrain,
            agent_role: None,
            lifecycle: crate::ai::orchestration_router::lifecycle::RunLifecycle {
                phase: crate::ai::orchestration_router::lifecycle::RunPhase::Working,
                suspension_reason: None,
                terminal_outcome: None,
                join_generation: 0,
            },
            last_source_sequence: None,
            last_ingest_sequence: 2,
            created_at: 0,
            updated_at: 0,
            terminal_reason: None,
        };
        state.runs.insert(RunId("child_1".to_string()), child1);

        let child2 = crate::ai::orchestration_router::state::RunNode {
            run_id: RunId("child_2".to_string()),
            root_run_id: RunId("root_1".to_string()),
            parent_run_id: Some(RunId("root_1".to_string())),
            conversation_id: conv_3,
            runtime: crate::ai::orchestration_router::command::RuntimeKind::LocalBrain,
            agent_role: None,
            lifecycle: crate::ai::orchestration_router::lifecycle::RunLifecycle {
                phase: crate::ai::orchestration_router::lifecycle::RunPhase::Working,
                suspension_reason: None,
                terminal_outcome: None,
                join_generation: 0,
            },
            last_source_sequence: None,
            last_ingest_sequence: 3,
            created_at: 0,
            updated_at: 0,
            terminal_reason: None,
        };
        state.runs.insert(RunId("child_2".to_string()), child2);

        state.children_by_parent.insert(
            RunId("root_1".to_string()),
            vec![RunId("child_1".to_string()), RunId("child_2".to_string())],
        );

        let descendants = descendant_conversation_ids_in_spawn_order(&state, conv_1);
        assert_eq!(descendants, vec![conv_2, conv_3]);
    }
}
