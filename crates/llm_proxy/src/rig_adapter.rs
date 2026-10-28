//! Adapter module for integrating Rig Framework (`rig-core`) into Warp's LLM Proxy.

use rig::completion::ToolDefinition;
use rig::providers::openai;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RigToolError {
    #[error("{0}")]
    Execution(String),
}

/// Input arguments for Warp's basic tool execution wrapper.
#[derive(Debug, Deserialize, Serialize)]
pub struct DynamicToolArgs {
    #[serde(flatten)]
    pub params: Value,
}

/// Generic wrapper for Warp tools implemented over Rig's `Tool` trait.
pub struct WarpToolWrapper {
    pub name: String,
    pub description: String,
    pub parameters_schema: Value,
}

impl Tool for WarpToolWrapper {
    const NAME: &'static str = "warp_tool";
    type Error = RigToolError;
    type Args = DynamicToolArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters_schema.clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(serde_json::to_string(&args.params).unwrap_or_default())
    }
}

/// Constructs a Rig agent targeting NVIDIA NIM's OpenAI-compatible API base URL.
pub fn create_nim_agent(
    api_key: &str,
    model_id: &str,
    system_prompt: &str,
) -> anyhow::Result<rig::agent::Agent<openai::CompletionModel>> {
    let client = openai::Client::from_url(api_key, "https://integrate.api.nvidia.com/v1");

    let agent = client.agent(model_id).preamble(system_prompt).build();

    Ok(agent)
}

/// Robust ReAct text parser: extracts tool JSON from assistant content when a model
/// lacks native function calling support (e.g. `minimaxai/minimax-m3`).
pub fn extract_tool_calls_from_text(content: &str) -> Option<Vec<Value>> {
    let trimmed = content.trim();
    if let Ok(tool_obj) = serde_json::from_str::<Value>(trimmed) {
        if let Some(name) = tool_obj.get("name").and_then(Value::as_str) {
            let args = tool_obj.get("arguments").cloned().unwrap_or(json!({}));
            return Some(vec![json!({
                "id": format!("call_{}", uuid::Uuid::new_v4()),
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": serde_json::to_string(&args).unwrap_or_default()
                }
            })]);
        }
    }

    // Try extracting JSON inside <tool_call>...</tool_call> tags
    if let Some(start) = trimmed.find("<tool_call>") {
        if let Some(end) = trimmed.find("</tool_call>") {
            let json_str = &trimmed[start + 11..end].trim();
            if let Ok(tool_obj) = serde_json::from_str::<Value>(json_str) {
                if let Some(name) = tool_obj.get("name").and_then(Value::as_str) {
                    let args = tool_obj.get("arguments").cloned().unwrap_or(json!({}));
                    return Some(vec![json!({
                        "id": format!("call_{}", uuid::Uuid::new_v4()),
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": serde_json::to_string(&args).unwrap_or_default()
                        }
                    })]);
                }
            }
        }
    }

    None
}
