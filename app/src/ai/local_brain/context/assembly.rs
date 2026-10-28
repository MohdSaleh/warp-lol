use warp_multi_agent_api as api;

/// Reconstructs the internal task representation into the provider API format
pub fn assemble_provider_payload(_tasks: &[api::Task]) -> Vec<api::Request> {
    // A real implementation transforms internal history into the actual LLM payload.
    // It would map api::Task structure into the necessary array of requests.
    Vec::new()
}
