use anyhow::Result;

use crate::ai::local_brain::custom_agents::LocalCustomAgentStore;
use crate::server::server_api::ai::AgentResponse;

pub mod graph;

pub struct SwarmOrchestrator;

impl SwarmOrchestrator {
    /// Intercepts tasks routed to custom agents.
    /// Returns the matched AgentResponse and the rewritten query text if a custom agent was routed to.
    pub async fn intercept_task(
        query: &str,
        active_agent_id: Option<&str>,
    ) -> Result<Option<(AgentResponse, String)>> {
        let agents = LocalCustomAgentStore::list_agents().await?;

        Ok(Self::resolve_agent(&agents, query, active_agent_id))
    }

    fn resolve_agent(
        agents: &[AgentResponse],
        query: &str,
        active_agent_id: Option<&str>,
    ) -> Option<(AgentResponse, String)> {
        for agent in agents {
            let prefix = format!("/{} ", agent.name);
            if query.starts_with(&prefix) {
                let rewritten_query = query[prefix.len()..].trim().to_string();
                return Some((agent.clone(), rewritten_query));
            }
        }

        active_agent_id
            .and_then(|agent_id| agents.iter().find(|agent| agent.uid == agent_id))
            .map(|agent| (agent.clone(), query.to_string()))
    }

    /// Prepares or restores a GraphEngine for multi-node agent orchestration workflows.
    pub fn prepare_graph_engine(
        query: &str,
        active_agent_id: Option<&str>,
    ) -> Option<graph::GraphEngine> {
        if query.starts_with("/graph ") || query.starts_with("/workflow ") {
            return Some(graph::GraphEngine::new(graph::NodeId::FileIndexer, 10));
        }

        if let Some(agent_id) = active_agent_id {
            if !agent_id.is_empty() {
                return Some(graph::GraphEngine::new(
                    graph::NodeId::Custom(agent_id.to_string()),
                    10,
                ));
            }
        }

        None
    }
}

#[cfg(test)]
#[path = "orchestrator_tests.rs"]
mod tests;
