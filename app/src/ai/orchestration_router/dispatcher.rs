use anyhow::Result;
use diesel::SqliteConnection;
use persistence::model::OrchestrationRouterOutboxRecord;

use crate::ai::orchestration_router::event::{
    DurabilityClass, OrchestrationEvent, RuntimeEventEnvelope, RuntimeEventSource,
};
use crate::ai::orchestration_router::ids::{EventId, RunId};
use crate::ai::orchestration_router::outbox::OrchestrationOutboxEffect;

pub struct OrchestrationDispatcher;

impl OrchestrationDispatcher {
    pub fn poll_outbox(
        conn: &mut SqliteConnection,
    ) -> Result<Vec<OrchestrationRouterOutboxRecord>> {
        use diesel::prelude::*;
        use persistence::schema::orchestration_router_outbox;

        // In a real implementation we'd lock/claim these rows.
        let records =
            orchestration_router_outbox::table.load::<OrchestrationRouterOutboxRecord>(conn)?;

        Ok(records)
    }

    pub fn process_effect(
        conn: &mut SqliteConnection,
        record: OrchestrationRouterOutboxRecord,
    ) -> Result<Option<RuntimeEventEnvelope<OrchestrationEvent>>> {
        let effect: OrchestrationOutboxEffect = serde_json::from_str(&record.event_payload)?;

        match effect {
            OrchestrationOutboxEffect::StartChildRun {
                parent_task_id,
                child_run_id,
                runtime,
                task,
            } => {
                // TODO: Here we would call AIClient::start_ambient_agent or local brain's `generate_output`
                // using the task spec and runtime settings.
                // For Phase 7 (Local Brain Protocol), we pass `task.pwd`, `task.os`, and `task.shell`
                // to the child's `RequestParams::session_context` or `AgentConfigSnapshot`.
                // Validate parent skill references before hydrating child (Phase 7)
                let mut valid_context_refs = Vec::new();
                for r in task.context_refs {
                    let is_valid = if r.starts_with("@warp-skill:") {
                        true
                    } else {
                        std::path::Path::new(&r).exists()
                    };

                    if is_valid {
                        valid_context_refs.push(r);
                    } else {
                        log::warn!(
                            "Child context hydration downgraded: unresolvable skill reference: {}",
                            r
                        );
                    }
                }

                let _hydrated_task = crate::ai::orchestration_router::command::ChildTaskSpec {
                    context_refs: valid_context_refs,
                    ..task
                };

                // Upon success, fire ChildRunSpawned.
                let event = RuntimeEventEnvelope {
                    event_id: EventId(uuid::Uuid::new_v4().to_string()),
                    schema_version: 1,
                    source: RuntimeEventSource::LocalBrain,
                    event_type: "ChildRunSpawned".to_string(),
                    root_run_id: RunId(parent_task_id.to_string()), // Approximate for root_run_id
                    run_id: RunId(child_run_id),
                    parent_run_id: Some(RunId(parent_task_id.to_string())),
                    conversation_id: crate::AIConversationId::new(),
                    tool_call_id: None,
                    source_sequence: None,
                    ingest_sequence: None,
                    correlation_id: crate::ai::orchestration_router::ids::CorrelationId(
                        uuid::Uuid::new_v4().to_string(),
                    ),
                    causation_id: None,
                    traceparent: None,
                    occurred_at: chrono::Utc::now().timestamp_millis(),
                    recorded_at: chrono::Utc::now().timestamp_millis(),
                    durability: DurabilityClass::Durable,
                    payload: OrchestrationEvent::ChildRunSpawned,
                };

                // Clean up the outbox since dispatch succeeded
                crate::ai::orchestration_router::persistence::delete_outbox(
                    conn,
                    &record.event_id,
                )?;

                Ok(Some(event))
            }
            OrchestrationOutboxEffect::ResumeParent {
                parent_task_id,
                child_results,
            } => {
                // TODO: Dispatch the synthesis follow-up request to the parent's runtime,
                // feeding it the `child_results` as a tool return or synthesized system prompt.

                crate::ai::orchestration_router::persistence::delete_outbox(
                    conn,
                    &record.event_id,
                )?;

                // We don't necessarily emit a new runtime event here unless we want to track
                // that the follow-up was dispatched. For now, we just return Ok(None).
                Ok(None)
            }
            OrchestrationOutboxEffect::CancelChildRun {
                child_run_id,
                reason,
            } => {
                // TODO: Find the runtime tracking the child run and dispatch a cancellation
                // request to it.
                crate::ai::orchestration_router::persistence::delete_outbox(
                    conn,
                    &record.event_id,
                )?;
                Ok(None)
            }
        }
    }
}
