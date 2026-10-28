use super::ledger::LocalAgentRunLedger;

pub enum RuntimeEvent {
    ModelTurnStarted,
    ToolCallStarted { call_id: String },
    ToolCallFinished { call_id: String, success: bool },
    ProgressEvaluated { progressed: bool, batch_id: String },
    CompactionTriggered,
}

pub fn apply_event(ledger: &mut LocalAgentRunLedger, event_id: &str, event: RuntimeEvent) {
    if ledger
        .applied_runtime_event_ids
        .contains(&event_id.to_string())
    {
        // Idempotent recovery
        return;
    }

    match event {
        RuntimeEvent::ModelTurnStarted => {
            ledger.model_turns += 1;
        }
        RuntimeEvent::ToolCallStarted { .. } => {
            ledger.tool_calls += 1;
        }
        RuntimeEvent::ToolCallFinished { success, .. } => {
            if !success {
                ledger.consecutive_deterministic_failures += 1;
            } else {
                ledger.consecutive_deterministic_failures = 0;
            }
        }
        RuntimeEvent::ProgressEvaluated { progressed, .. } => {
            ledger.tool_batches += 1;
            if !progressed {
                ledger.consecutive_no_progress_batches += 1;
            } else {
                ledger.consecutive_no_progress_batches = 0;
            }
        }
        RuntimeEvent::CompactionTriggered => {
            ledger.compaction_count += 1;
        }
    }

    ledger
        .applied_runtime_event_ids
        .push_back(event_id.to_string());
    if ledger.applied_runtime_event_ids.len() > 100 {
        ledger.applied_runtime_event_ids.pop_front();
    }
}
