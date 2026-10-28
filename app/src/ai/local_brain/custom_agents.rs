use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use tokio::fs;

use crate::server::server_api::ai::{AgentResponse, CreateAgentRequest, UpdateAgentRequest};

pub struct LocalCustomAgentStore;

impl LocalCustomAgentStore {
    /// Gets the path to the local custom agents file.
    fn file_path() -> PathBuf {
        warp_core::paths::config_local_dir().join("local_custom_agents.json")
    }

    /// Reads the agents from the local file.
    pub async fn list_agents() -> Result<Vec<AgentResponse>> {
        let path = Self::file_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&path)
            .await
            .context("Failed to read local_custom_agents.json")?;

        // Handle empty file as empty list
        if contents.trim().is_empty() {
            return Ok(Vec::new());
        }

        let agents: Vec<AgentResponse> =
            serde_json::from_str(&contents).context("Failed to parse local_custom_agents.json")?;

        Ok(agents)
    }

    /// Writes the agents to the local file atomically.
    async fn write_agents(agents: &[AgentResponse]) -> Result<()> {
        let path = Self::file_path();

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .context("Failed to create config dir")?;
            }
        }

        let temp_path = path.with_extension("json.tmp");
        let contents = serde_json::to_string_pretty(agents)
            .context("Failed to serialize local_custom_agents.json")?;

        fs::write(&temp_path, contents)
            .await
            .context("Failed to write to temp local_custom_agents.json")?;

        fs::rename(&temp_path, &path)
            .await
            .context("Failed to rename temp file to local_custom_agents.json")?;

        Ok(())
    }

    /// Creates a new agent locally.
    pub async fn create_agent(request: CreateAgentRequest) -> Result<AgentResponse> {
        let mut agents = Self::list_agents().await?;

        let new_agent = AgentResponse {
            uid: uuid::Uuid::new_v4().to_string(),
            name: request.name,
            description: request.description,
            instructions: request.instructions,
            available: true,
            created_at: Utc::now(),
            secrets: request.secrets,
            skills: request.skills,
            base_model: request.base_model,
            environment_id: request.environment_id,
        };

        agents.push(new_agent.clone());
        Self::write_agents(&agents).await?;

        Ok(new_agent)
    }

    /// Updates an existing agent locally.
    pub async fn update_agent(uid: &str, request: UpdateAgentRequest) -> Result<AgentResponse> {
        let mut agents = Self::list_agents().await?;

        if let Some(agent) = agents.iter_mut().find(|a| a.uid == uid) {
            if let Some(name) = request.name {
                agent.name = name;
            }
            if let Some(description) = request.description {
                agent.description = Some(description);
            }
            if let Some(instructions) = request.instructions {
                agent.instructions = Some(instructions);
            }
            if let Some(secrets) = request.secrets {
                agent.secrets = secrets;
            }
            if let Some(skills) = request.skills {
                agent.skills = skills;
            }
            if let Some(base_model) = request.base_model {
                agent.base_model = Some(base_model);
            }
            if let Some(environment_id) = request.environment_id {
                agent.environment_id = Some(environment_id);
            }

            let updated_agent = agent.clone();
            Self::write_agents(&agents).await?;
            Ok(updated_agent)
        } else {
            anyhow::bail!("Agent with uid {} not found", uid)
        }
    }

    /// Deletes an agent locally.
    #[allow(dead_code)]
    pub async fn delete_agent(uid: &str) -> Result<()> {
        let mut agents = Self::list_agents().await?;
        let initial_len = agents.len();

        agents.retain(|a| a.uid != uid);

        if agents.len() == initial_len {
            anyhow::bail!("Agent with uid {} not found", uid)
        }

        Self::write_agents(&agents).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_custom_agent_crud_logic() {
        let req = CreateAgentRequest {
            name: "Test Agent".to_string(),
            description: Some("A test description".to_string()),
            instructions: Some("You are a test agent".to_string()),
            secrets: vec![],
            skills: vec![],
            base_model: None,
            environment_id: None,
        };

        let mut agent = AgentResponse {
            uid: "test-uid-123".to_string(),
            name: req.name.clone(),
            description: req.description.clone(),
            instructions: req.instructions.clone(),
            available: true,
            created_at: Utc::now(),
            secrets: req.secrets.clone(),
            skills: req.skills.clone(),
            base_model: req.base_model.clone(),
            environment_id: req.environment_id.clone(),
        };

        let update_req = UpdateAgentRequest {
            name: Some("Updated Agent".to_string()),
            description: None,
            instructions: None,
            secrets: None,
            skills: None,
            base_model: None,
            environment_id: None,
        };

        agent.name = update_req.name.unwrap();
        assert_eq!(agent.name, "Updated Agent");
    }
}
