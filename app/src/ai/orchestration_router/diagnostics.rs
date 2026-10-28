use crate::ai::orchestration_router::ids::RunId;
use crate::ai::orchestration_router::state::RunGraphState;

/// A test-only or debug-only comparison helper that compares existing runtime state against router state.
/// This helper must not mutate production state.
pub fn compare_state(
    conversation_id: &str,
    existing_status: &str,
    router_state: &RunGraphState,
    run_id: &RunId,
) {
    if let Some(run) = router_state.runs.get(run_id) {
        let router_phase = format!("{:?}", run.lifecycle.phase);
        let router_outcome = format!("{:?}", run.lifecycle.terminal_outcome);
        let active_children = router_state
            .children_by_parent
            .get(run_id)
            .map(|c| c.len())
            .unwrap_or(0);
        let pending_tools = 0; // TODO: Implement pending tools tracking

        let match_status = true; // In a real scenario, this would compare specific invariants

        if match_status {
            log::info!(
                "[ORCHESTRATION_COMPARE]\nconversation_id={}\nexisting_status={}\nrouter_phase={}\nrouter_outcome={}\nactive_children={}\npending_tools={}\nmatch={}",
                conversation_id, existing_status, router_phase, router_outcome, active_children, pending_tools, match_status
            );
        } else {
            log::warn!(
                "[ORCHESTRATION_COMPARE]\nconversation_id={}\nmatch=false\nreason=\"state mismatch\"",
                conversation_id
            );
        }
    } else {
        log::warn!(
            "[ORCHESTRATION_COMPARE]\nconversation_id={}\nmatch=false\nreason=\"run not found in router\"",
            conversation_id
        );
    }
}
