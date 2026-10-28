use std::collections::HashSet;

use crate::ai::orchestration_router::ids::RunId;
use crate::ai::orchestration_router::state::RunGraphState;
use crate::AIConversationId;

/// Given a RunGraphState and a parent conversation id, return all descendant
/// conversation ids in pre-order traversal (spawn order).
pub fn descendant_conversation_ids_in_spawn_order(
    state: &RunGraphState,
    parent_conversation_id: AIConversationId,
) -> Vec<AIConversationId> {
    // Find the run(s) corresponding to this conversation_id.
    // Usually there is 1:1 mapping for roots.
    let mut root_run_ids = vec![];
    for run in state.runs.values() {
        if run.conversation_id == parent_conversation_id {
            root_run_ids.push(run.run_id.clone());
        }
    }

    let mut descendants = Vec::new();
    let mut visited = HashSet::new();

    for run_id in root_run_ids {
        collect_descendants(state, &run_id, &mut descendants, &mut visited);
    }

    descendants
}

fn collect_descendants(
    state: &RunGraphState,
    run_id: &RunId,
    descendants: &mut Vec<AIConversationId>,
    visited: &mut HashSet<RunId>,
) {
    if !visited.insert(run_id.clone()) {
        return;
    }

    if let Some(children) = state.children_by_parent.get(run_id) {
        for child_run_id in children {
            if let Some(child_run) = state.runs.get(child_run_id) {
                descendants.push(child_run.conversation_id);
                collect_descendants(state, child_run_id, descendants, visited);
            }
        }
    }
}
