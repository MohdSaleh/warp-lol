use warpui::{AppContext, Entity, ModelContext, ModelHandle, SingletonEntity};

use crate::ai::blocklist::orchestration_event_streamer::{
    OrchestrationEventStreamer, OrchestrationEventStreamerEvent,
};
use crate::ai::blocklist::{
    BlocklistAIActionEvent, BlocklistAIActionModel, ConversationSelectionEvent,
    ConversationSelectionHandle,
};
use crate::ai::orchestration_router::command::OrchestrationCommand;
use crate::ai::orchestration_router::event::{
    DurabilityClass, OrchestrationEvent, RuntimeEventEnvelope, RuntimeEventSource,
};
use crate::ai::orchestration_router::ids::{EventId, RunId};
use crate::ai::orchestration_router::outbox::OrchestrationOutboxEffect;
use crate::ai::orchestration_router::reducer::{apply_event, OrchestrationError};
use crate::ai::orchestration_router::state::RunGraphState;
use crate::terminal::model::session::active_session::ActiveSession;

pub struct OrchestrationRouterModel {
    pub state: RunGraphState,
}

impl OrchestrationRouterModel {
    pub fn new(
        ctx: &mut ModelContext<Self>,
        action_model: ModelHandle<BlocklistAIActionModel>,
        conversation_selection: ConversationSelectionHandle,
        active_session: ModelHandle<ActiveSession>,
    ) -> Self {
        let streamer = OrchestrationEventStreamer::handle(ctx);
        ctx.subscribe_to_model(&streamer, |me, _, event, ctx| {
            // Shadow-track events for phase 2
            let _ = event;
        });

        ctx.subscribe_to_model(&action_model, |me, _, event, ctx| {
            let _ = event;
        });

        ctx.subscribe_to_model(&conversation_selection, |me, _, event, ctx| {
            let _ = event;
        });

        ctx.subscribe_to_model(&active_session, |me, _, event, ctx| {
            let _ = event;
        });

        Self {
            state: RunGraphState::default(),
        }
    }

    pub fn handle_event(
        &mut self,
        event: RuntimeEventEnvelope<OrchestrationEvent>,
        cx: &mut ModelContext<Self>,
    ) {
        match apply_event(&mut self.state, &event) {
            Ok(changes) => {
                for change in changes {
                    if let crate::ai::orchestration_router::reducer::DerivedStateChange::ResumeScheduled(ref run_id, ref transition_key) = change {
                        if let Some(run) = self.state.runs.get(run_id) {
                            if run.lifecycle.phase == crate::ai::orchestration_router::lifecycle::RunPhase::Working {
                                log::info!("Phase 10: Parent Synthesis Follow-Up for run_id={} with transition_key={}", run_id.0, transition_key);
                                // Here we would generate `ResumeParent` effect and persist it.
                                // let effect = OrchestrationOutboxEffect::ResumeParent { ... };
                                // persist_outbox(effect, transition_key);
                            }
                        }
                    }
                    if let crate::ai::orchestration_router::reducer::DerivedStateChange::RunTerminal(ref run_id) = change {
                        if let Some(run) = self.state.runs.get(run_id) {
                            if run.lifecycle.terminal_outcome == Some(crate::ai::orchestration_router::lifecycle::TerminalOutcome::Cancelled) {
                                log::info!("Phase 11: Cancellation Propagation from parent run_id={}", run_id.0);
                                if let Some(children) = self.state.children_by_parent.get(run_id) {
                                    for child_id in children {
                                        if self.state.active_runs.contains(child_id) {
                                            log::info!("Phase 11: Propagating cancel to child_run_id={}", child_id.0);
                                            // let effect = OrchestrationOutboxEffect::CancelChildRun { ... };
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                cx.notify();
            }
            Err(e) => {
                log::error!("OrchestrationRouter error: {:?}", e);
            }
        }
    }
    pub fn execute_command(
        &mut self,
        command: OrchestrationCommand,
        cx: &mut ModelContext<Self>,
    ) -> Result<Vec<OrchestrationOutboxEffect>, OrchestrationError> {
        match command {
            OrchestrationCommand::SpawnChildRun {
                parent_run_id,
                child_run_id,
                agent_role,
                task,
                runtime,
                join_policy,
                failure_policy,
                parent_close_policy,
                causation_id,
                correlation_id,
            } => {
                // Validate parent
                if !self.state.active_runs.contains(&parent_run_id) {
                    return Err(OrchestrationError::InvalidTransition(
                        "Parent run is not active".into(),
                    ));
                }

                let parent_node = self.state.runs.get(&parent_run_id).unwrap();
                let conversation_id = parent_node.conversation_id;
                let root_run_id = parent_node.root_run_id.clone();

                // child_run_id is now passed in the command for idempotency and durable tracking
                // Let's use the provided child_run_id instead of generating a new one
                // let child_run_id = RunId(uuid::Uuid::new_v4().to_string());

                // TODO: Store child role and target.
                let _ = agent_role;

                let event = RuntimeEventEnvelope {
                    event_id: EventId(uuid::Uuid::new_v4().to_string()),
                    schema_version: 1,
                    source: RuntimeEventSource::LocalBrain, // Or dynamic based on origin
                    event_type: "ChildRunSpawnRequested".to_string(),
                    root_run_id: root_run_id.clone(),
                    run_id: child_run_id.clone(),
                    parent_run_id: Some(parent_run_id),
                    conversation_id,
                    tool_call_id: None,
                    source_sequence: None,
                    ingest_sequence: None,
                    correlation_id,
                    causation_id,
                    traceparent: None,
                    occurred_at: chrono::Utc::now().timestamp_millis(),
                    recorded_at: chrono::Utc::now().timestamp_millis(),
                    durability: DurabilityClass::Durable,
                    payload: OrchestrationEvent::ChildRunSpawnRequested {
                        child_task: task.clone(),
                        runtime: runtime.clone(),
                        join_policy,
                        failure_policy,
                        parent_close_policy,
                    },
                };

                // Appended to state
                self.handle_event(event, cx);

                // Formulate the dispatch effect to the Outbox
                let outbox_effect = OrchestrationOutboxEffect::StartChildRun {
                    parent_task_id: conversation_id,
                    child_run_id: child_run_id.0,
                    runtime,
                    task,
                };

                return Ok(vec![outbox_effect]);
            }
            _ => {
                // Implement other commands
            }
        }
        Ok(vec![])
    }
}

pub enum OrchestrationRouterEvent {}

impl Entity for OrchestrationRouterModel {
    type Event = OrchestrationRouterEvent;
}
