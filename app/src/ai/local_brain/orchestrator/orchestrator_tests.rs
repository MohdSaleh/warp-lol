use chrono::Utc;

use super::*;

fn agent(uid: &str, name: &str) -> AgentResponse {
    AgentResponse {
        uid: uid.to_string(),
        name: name.to_string(),
        description: None,
        instructions: Some(format!("Instructions for {name}")),
        available: true,
        created_at: Utc::now(),
        secrets: Vec::new(),
        skills: Vec::new(),
        base_model: None,
        environment_id: None,
    }
}

#[test]
fn slash_invocation_selects_agent_and_strips_prefix() {
    let agents = vec![agent("reviewer-id", "Reviewer")];
    let (selected, query) =
        SwarmOrchestrator::resolve_agent(&agents, "/Reviewer inspect this diff", None)
            .expect("slash invocation should resolve");

    assert_eq!(selected.uid, "reviewer-id");
    assert_eq!(query, "inspect this diff");
}

#[test]
fn active_agent_applies_to_follow_up_queries() {
    let agents = vec![agent("reviewer-id", "Reviewer")];
    let (selected, query) =
        SwarmOrchestrator::resolve_agent(&agents, "Now check the tests", Some("reviewer-id"))
            .expect("active conversation agent should resolve");

    assert_eq!(selected.uid, "reviewer-id");
    assert_eq!(query, "Now check the tests");
}

#[test]
fn prepare_graph_engine_prepares_workflow_and_active_agent_nodes() {
    let engine = SwarmOrchestrator::prepare_graph_engine("/graph analyze code", None)
        .expect("graph workflow prefix should initialize engine");
    assert_eq!(engine.state.current_node, graph::NodeId::FileIndexer);

    let custom_engine =
        SwarmOrchestrator::prepare_graph_engine("inspect code", Some("custom-agent-123"))
            .expect("active custom agent should initialize engine");
    assert_eq!(
        custom_engine.state.current_node,
        graph::NodeId::Custom("custom-agent-123".to_string())
    );
}
