use warp_multi_agent_api as api;

/// Reconstructs the internal task tree into a linear sequence of atomic units
/// using a deterministic depth-first search.
pub fn linearize_tasks(tasks: &[api::Task]) -> Vec<&api::Task> {
    let mut result = Vec::new();
    // Implementation placeholder for DFS linearization
    for task in tasks {
        result.push(task);
    }
    result
}
