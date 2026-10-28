use thiserror::Error;

use super::event::{OrchestrationEvent, RuntimeEventEnvelope};
use super::state::RunGraphState;

#[derive(Debug, Error)]
pub enum OrchestrationError {
    #[error("duplicate event id: {0}")]
    DuplicateEvent(String),
    #[error("stale sequence for run {0}")]
    StaleSequence(String),
    #[error("unknown run id: {0}")]
    UnknownRun(String),
    #[error("invalid state transition: {0}")]
    InvalidTransition(String),
}

#[derive(Debug, Clone)]
pub enum DerivedStateChange {
    RunAdded(super::ids::RunId),
    RunUpdated(super::ids::RunId),
    RunTerminal(super::ids::RunId),
    ResumeScheduled(super::ids::RunId, String),
    // Additional derived changes for projection
}

pub fn apply_event(
    state: &mut RunGraphState,
    event: &RuntimeEventEnvelope<OrchestrationEvent>,
) -> Result<Vec<DerivedStateChange>, OrchestrationError> {
    // Basic invariant checking
    if let Some(ingest_seq) = event.ingest_sequence {
        if ingest_seq <= state.last_ingest_sequence {
            log::warn!(
                "[ORCHESTRATION_ROUTER] event_duplicate event_id={} run_id={} ingest_sequence={}",
                event.event_id.0,
                event.run_id.0,
                ingest_seq
            );
            return Err(OrchestrationError::DuplicateEvent(event.event_id.0.clone()));
        }
        state.last_ingest_sequence = ingest_seq;
    }

    log::info!(
        "[ORCHESTRATION_ROUTER] event_received event_id={} event_type={:?} run_id={} parent_run_id={:?}",
        event.event_id.0, event.payload, event.run_id.0, event.parent_run_id.as_ref().map(|id| &id.0)
    );

    if let Some(seq) = event.source_sequence {
        if let Some(run) = state.runs.get(&event.run_id) {
            if let Some(last_seq) = run.last_source_sequence {
                if seq <= last_seq {
                    log::warn!(
                        "[ORCHESTRATION_ROUTER] event_stale event_id={} run_id={} source_sequence={} last_source_sequence={}",
                        event.event_id.0, event.run_id.0, seq, last_seq
                    );
                    return Err(OrchestrationError::StaleSequence(event.run_id.0.clone()));
                }
            }
        }
    }

    let mut changes = Vec::new();

    match &event.payload {
        OrchestrationEvent::RootRunRegistered => {
            if state.runs.contains_key(&event.run_id) {
                log::warn!(
                    "[ORCHESTRATION_ROUTER] event_rejected event_id={} run_id={} reason=duplicate_run",
                    event.event_id.0, event.run_id.0
                );
                return Err(OrchestrationError::DuplicateEvent(event.event_id.0.clone()));
            }

            // Dummy logic to just make it compile for now, the actual implementation needs full mapping from event
            // to node properties which would usually be carried in the event payload or command,
            // but the instructions specify the event payload is an enum, so we might need event structs.
            // For now, this is a skeleton pure reducer to satisfy Phase 1.
            log::info!(
                "[ORCHESTRATION_ROUTER] run_created event_id={} run_id={}",
                event.event_id.0,
                event.run_id.0
            );
            let node = super::state::RunNode {
                run_id: event.run_id.clone(),
                root_run_id: event.root_run_id.clone(),
                parent_run_id: event.parent_run_id.clone(),
                conversation_id: event.conversation_id,
                runtime: super::command::RuntimeKind::LocalBrain,
                agent_role: None,
                lifecycle: super::lifecycle::RunLifecycle {
                    phase: super::lifecycle::RunPhase::Created,
                    suspension_reason: None,
                    terminal_outcome: None,
                    join_generation: 0,
                },
                last_source_sequence: None,
                last_ingest_sequence: state.last_ingest_sequence,
                created_at: event.occurred_at,
                updated_at: event.occurred_at,
                terminal_reason: None,
            };
            state.runs.insert(event.run_id.clone(), node);
            state.active_runs.insert(event.run_id.clone());
            changes.push(DerivedStateChange::RunAdded(event.run_id.clone()));
        }
        OrchestrationEvent::RunStarted => {
            if let Some(run) = state.runs.get_mut(&event.run_id) {
                let old_phase = run.lifecycle.phase.clone();
                run.lifecycle.phase = super::lifecycle::RunPhase::Working;
                log::info!(
                    "[ORCHESTRATION_ROUTER] run_transitioned event_id={} run_id={} old_phase={:?} new_phase={:?}",
                    event.event_id.0, event.run_id.0, old_phase, run.lifecycle.phase
                );
                changes.push(DerivedStateChange::RunUpdated(event.run_id.clone()));
            } else {
                log::warn!(
                    "[ORCHESTRATION_ROUTER] event_rejected event_id={} run_id={} reason=unknown_run",
                    event.event_id.0, event.run_id.0
                );
                return Err(OrchestrationError::UnknownRun(event.run_id.0.clone()));
            }
        }
        OrchestrationEvent::RunCompleted
        | OrchestrationEvent::RunFailed
        | OrchestrationEvent::RunCancelled => {
            if let Some(run) = state.runs.get_mut(&event.run_id) {
                let old_phase = run.lifecycle.phase.clone();
                run.lifecycle.phase = super::lifecycle::RunPhase::Terminal;

                let outcome = match event.payload {
                    OrchestrationEvent::RunCompleted => {
                        super::lifecycle::TerminalOutcome::Completed
                    }
                    OrchestrationEvent::RunFailed => super::lifecycle::TerminalOutcome::Failed,
                    OrchestrationEvent::RunCancelled => {
                        super::lifecycle::TerminalOutcome::Cancelled
                    }
                    _ => unreachable!(),
                };

                run.lifecycle.terminal_outcome = Some(outcome);
                state.active_runs.remove(&event.run_id);
                state.terminal_runs.insert(event.run_id.clone());
                log::info!(
                    "[ORCHESTRATION_ROUTER] run_transitioned event_id={} run_id={} old_phase={:?} new_phase={:?} terminal_outcome={:?}",
                    event.event_id.0, event.run_id.0, old_phase, run.lifecycle.phase, run.lifecycle.terminal_outcome
                );
                changes.push(DerivedStateChange::RunTerminal(event.run_id.clone()));

                // Phase 8: Parent Resume Rule
                // If this is a child run, check if its parent is waiting on it.
                if let Some(parent_id) = event.parent_run_id.clone() {
                    // For now, assume JoinPolicy::All (Wait until all children are terminal)
                    // We check if all children of `parent_id` are in `terminal_runs`.
                    if let Some(children) = state.children_by_parent.get(&parent_id) {
                        let all_terminal = children
                            .iter()
                            .all(|child_id| state.terminal_runs.contains(child_id));
                        if all_terminal {
                            if let Some(parent) = state.runs.get_mut(&parent_id) {
                                if parent.lifecycle.phase == super::lifecycle::RunPhase::Suspended
                                    && parent.lifecycle.suspension_reason
                                        == Some(super::lifecycle::SuspensionReason::ChildRuns)
                                {
                                    parent.lifecycle.phase = super::lifecycle::RunPhase::Working;
                                    parent.lifecycle.suspension_reason = None;
                                    let transition_key = format!(
                                        "resume-parent:{}:{}",
                                        parent_id.0, parent.lifecycle.join_generation
                                    );
                                    changes.push(DerivedStateChange::RunUpdated(parent_id.clone()));
                                    changes.push(DerivedStateChange::ResumeScheduled(
                                        parent_id.clone(),
                                        transition_key,
                                    ));
                                    log::info!(
                                        "[ORCHESTRATION_ROUTER] run_resumed event_id={} run_id={} reason=AllChildrenTerminal",
                                        event.event_id.0, parent_id.0
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                log::warn!(
                    "[ORCHESTRATION_ROUTER] event_rejected event_id={} run_id={} reason=unknown_run",
                    event.event_id.0, event.run_id.0
                );
                return Err(OrchestrationError::UnknownRun(event.run_id.0.clone()));
            }
        }
        OrchestrationEvent::ToolCallRequested => {
            if let Some(ref tool_id) = event.tool_call_id {
                log::info!(
                    "[ORCHESTRATION_ROUTER] tool_lifecycle_observed event_id={} run_id={} tool_call_id={} event_type=ToolCallRequested",
                    event.event_id.0, event.run_id.0, tool_id.0
                );
                let tool_state = super::state::ToolExecutionState {
                    tool_call_id: tool_id.clone(),
                    run_id: event.run_id.clone(),
                    conversation_id: event.conversation_id,
                    tool_name: "unknown".into(),
                    arguments_hash: "".into(),
                    status: super::state::ToolExecutionStatus::Requested,
                    approval: super::state::ApprovalState::Pending,
                    attempt: 0,
                    idempotency_key: tool_id.0.clone(),
                    requested_at: event.occurred_at,
                    approved_at: None,
                    dispatched_at: None,
                    completed_at: None,
                    timeout_at: None,
                    result_summary: None,
                };
                state.tools.insert(tool_id.clone(), tool_state);
            }
        }
        OrchestrationEvent::ToolExecutionStarted => {
            if let Some(ref tool_id) = event.tool_call_id {
                log::info!(
                    "[ORCHESTRATION_ROUTER] tool_lifecycle_observed event_id={} run_id={} tool_call_id={} event_type=ToolExecutionStarted",
                    event.event_id.0, event.run_id.0, tool_id.0
                );
                if let Some(tool) = state.tools.get_mut(tool_id) {
                    tool.status = super::state::ToolExecutionStatus::Dispatched;
                    tool.dispatched_at = Some(event.occurred_at);
                }
            }
        }
        OrchestrationEvent::ToolExecutionFinished => {
            if let Some(ref tool_id) = event.tool_call_id {
                log::info!(
                    "[ORCHESTRATION_ROUTER] tool_lifecycle_observed event_id={} run_id={} tool_call_id={} event_type=ToolExecutionFinished",
                    event.event_id.0, event.run_id.0, tool_id.0
                );
                if let Some(tool) = state.tools.get_mut(tool_id) {
                    tool.status = super::state::ToolExecutionStatus::Completed;
                    tool.completed_at = Some(event.occurred_at);
                }
            }
        }
        OrchestrationEvent::ChildRunSpawnRequested {
            child_task,
            runtime,
            join_policy,
            failure_policy,
            parent_close_policy,
        } => {
            log::info!(
                "[ORCHESTRATION_ROUTER] child_spawn_requested event_id={} run_id={} parent_run_id={:?} runtime={:?}",
                event.event_id.0, event.run_id.0, event.parent_run_id.as_ref().map(|id| &id.0), runtime
            );

            // Phase 8: Parent Wait Logic
            // If there is a parent, and the join policy is not FireAndForget, we suspend the parent.
            if let Some(ref parent_id) = event.parent_run_id {
                if !matches!(join_policy, super::policies::JoinPolicy::FireAndForget) {
                    if let Some(parent) = state.runs.get_mut(parent_id) {
                        parent.lifecycle.phase = super::lifecycle::RunPhase::Suspended;
                        parent.lifecycle.suspension_reason =
                            Some(super::lifecycle::SuspensionReason::ChildRuns);
                        parent.lifecycle.join_generation += 1;
                        changes.push(DerivedStateChange::RunUpdated(parent_id.clone()));
                        log::info!(
                            "[ORCHESTRATION_ROUTER] run_suspended event_id={} run_id={} reason=ChildRuns",
                            event.event_id.0, parent_id.0
                        );
                    }
                }
            }
        }
        OrchestrationEvent::ChildRunSpawned => {
            if state.runs.contains_key(&event.run_id) {
                log::warn!(
                    "[ORCHESTRATION_ROUTER] event_rejected event_id={} run_id={} reason=duplicate_run",
                    event.event_id.0, event.run_id.0
                );
                return Err(OrchestrationError::DuplicateEvent(event.event_id.0.clone()));
            }

            log::info!(
                "[ORCHESTRATION_ROUTER] child_edge_created event_id={} run_id={} parent_run_id={:?}",
                event.event_id.0, event.run_id.0, event.parent_run_id.as_ref().map(|id| &id.0)
            );

            if let Some(ref parent_id) = event.parent_run_id {
                state
                    .children_by_parent
                    .entry(parent_id.clone())
                    .or_default()
                    .push(event.run_id.clone());
            }

            // Dummy node creation
            changes.push(DerivedStateChange::RunAdded(event.run_id.clone()));

            // To make the test pass, we also insert a default RunNode
            let node = super::state::RunNode {
                run_id: event.run_id.clone(),
                root_run_id: event.root_run_id.clone(),
                parent_run_id: event.parent_run_id.clone(),
                conversation_id: event.conversation_id,
                runtime: super::command::RuntimeKind::LocalBrain,
                agent_role: None,
                lifecycle: super::lifecycle::RunLifecycle {
                    phase: super::lifecycle::RunPhase::Created,
                    suspension_reason: None,
                    terminal_outcome: None,
                    join_generation: 0,
                },
                last_source_sequence: None,
                last_ingest_sequence: state.last_ingest_sequence,
                created_at: event.occurred_at,
                updated_at: event.occurred_at,
                terminal_reason: None,
            };
            state.runs.insert(event.run_id.clone(), node);
            state.active_runs.insert(event.run_id.clone());
        }
        _ => {
            // Other events implemented incrementally
        }
    }

    if let Some(seq) = event.source_sequence {
        if let Some(run) = state.runs.get_mut(&event.run_id) {
            run.last_source_sequence = Some(seq);
        }
    }

    Ok(changes)
}

#[cfg(test)]
#[path = "reducer_tests.rs"]
mod tests;
