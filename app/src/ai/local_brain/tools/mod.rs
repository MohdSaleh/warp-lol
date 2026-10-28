#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
pub mod availability;
pub mod code_editing;
pub mod computer;
pub mod context;
pub mod documents;
pub mod errors;
pub mod files;
pub mod interaction;
pub mod mcp;
pub mod orchestration;
pub mod registry;
pub mod results;
pub mod search;
pub mod server_tool;
pub mod shell;
pub mod terminal;
pub mod todos;
pub mod utils;
pub mod web_fetch;
pub mod web_research_config;
pub mod web_search;
pub mod workspace;

use std::collections::HashSet;

use context::ToolRuntimeContext;
use registry::LocalToolRegistry;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use crate::ai::agent::api::RequestParams;

/// Max characters of a tool result fed back into the model, to bound context.
const MAX_RESULT_CHARS: usize = 24_000;

const MCP_PREFIX: &str = "mcp__";

pub(super) fn tool_schemas(params: &RequestParams) -> Vec<Value> {
    let empty_set = HashSet::new();
    let ctx = ToolRuntimeContext::new(params, &empty_set, &empty_set, &empty_set);

    let mut schemas = Vec::new();

    // Todo updates are rendered directly as structured task messages rather
    // than executed as client-side actions.
    if params.planning_enabled {
        schemas.push(todos::schema());
    }

    // Built-in schemas
    for def in registry::BUILTIN_TOOL_REGISTRY {
        // Skip MCP adapter, dynamic schemas are generated below
        if def.id == registry::LocalToolId::CallMcpTool {
            continue;
        }

        if matches!(
            (def.availability)(&ctx),
            availability::ToolAvailability::Available
        ) {
            schemas.push((def.schema)(&ctx));
        }
    }

    // Dynamic MCP schemas
    if let Some(mcp) = &params.mcp_context {
        for server in &mcp.servers {
            for tool in &server.tools {
                let name = format!("{MCP_PREFIX}{}__{}", server.id, tool.name);
                let description = tool
                    .description
                    .as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_default();
                let parameters: Value = tool.input_schema.as_ref().clone().into();
                let parameters = if parameters.is_object() {
                    parameters
                } else {
                    json!({ "type": "object" })
                };
                schemas.push(json!({
                    "type": "function",
                    "function": {
                        "name": name,
                        "description": description,
                        "parameters": parameters,
                    }
                }));
            }
        }
    }

    schemas
}

pub(super) fn openai_to_api_tool(
    name: &str,
    args: Value,
) -> anyhow::Result<api::message::tool_call::Tool> {
    let def = LocalToolRegistry::find_by_provider_name(name)
        .ok_or_else(|| anyhow::anyhow!("unsupported tool: {}", name))?;

    let decoded =
        (def.decode)(name, args).map_err(|e| anyhow::anyhow!("tool decode error: {}", e))?;
    (def.encode_warp_tool)(decoded).map_err(|e| anyhow::anyhow!("tool encode error: {}", e))
}

pub(super) fn api_tool_to_openai(tool: &Option<api::message::tool_call::Tool>) -> (String, String) {
    let Some(tool) = tool else {
        return ("noop".to_string(), "{}".to_string());
    };

    if let Some(decoded) = server_tool::decode_call(tool) {
        return decoded;
    }

    for def in registry::BUILTIN_TOOL_REGISTRY {
        if let Some((name, args)) = (def.encode_history)(tool) {
            return (name, args);
        }
    }

    ("unsupported_tool".to_string(), "{}".to_string())
}

fn tool_id_from_result(
    result: &api::message::tool_call_result::Result,
) -> Option<registry::LocalToolId> {
    use api::message::tool_call_result::Result as R;
    use registry::LocalToolId;
    match result {
        R::RunShellCommand(_) => Some(LocalToolId::RunShellCommand),
        R::ReadShellCommandOutput(_) => Some(LocalToolId::ReadShellCommandOutput),
        R::WriteToLongRunningShellCommand(_) => Some(LocalToolId::WriteToLongRunningShellCommand),
        R::ReadFiles(_) => Some(LocalToolId::ReadFiles),
        R::Grep(_) => Some(LocalToolId::Grep),
        #[allow(deprecated)]
        R::FileGlob(_) => Some(LocalToolId::FileGlobV2),
        R::SearchCodebase(_) => Some(LocalToolId::SearchCodebase),
        R::ApplyFileDiffs(_) => Some(LocalToolId::ApplyFileDiffs),
        R::InitProject(_) => Some(LocalToolId::InitProject),
        R::OpenCodeReview(_) => Some(LocalToolId::OpenCodeReview),
        R::InsertReviewComments(_) => Some(LocalToolId::InsertReviewComments),
        R::ReadDocuments(_) => Some(LocalToolId::ReadDocuments),
        R::CreateDocuments(_) => Some(LocalToolId::CreateDocuments),
        R::EditDocuments(_) => Some(LocalToolId::EditDocuments),
        R::UploadFileArtifact(_) => Some(LocalToolId::UploadFileArtifact),
        R::AskUserQuestion(_) => Some(LocalToolId::AskUserQuestion),
        R::SuggestPrompt(_) => Some(LocalToolId::SuggestPrompt),
        R::SuggestNewConversation(_) => Some(LocalToolId::SuggestNewConversation),
        R::RunAgentsResult(_) => Some(LocalToolId::RunAgents),
        R::Subagent(_) => Some(LocalToolId::Subagent),
        R::SendMessageToAgent(_) => Some(LocalToolId::SendMessageToAgent),
        R::WaitForEvents(_) => Some(LocalToolId::WaitForEvents),
        R::RequestComputerUseResult(_) => Some(LocalToolId::RequestComputerUse),
        R::UseComputer(_) => Some(LocalToolId::UseComputer),
        R::ReadSkill(_) => Some(LocalToolId::ReadSkill),
        R::FetchConversation(_) => Some(LocalToolId::FetchConversation),
        R::CallMcpTool(_) => None,
        _ => None,
    }
}

pub(super) fn render_tool_result(tcr: &api::message::ToolCallResult) -> String {
    if let Some(content) = server_tool::decode_result(tcr) {
        return content;
    }

    // Attempt exact match by Result variant first
    if let Some(res) = &tcr.result {
        if let Some(tool_id) = tool_id_from_result(res) {
            if let Some(def) = registry::BUILTIN_TOOL_REGISTRY
                .iter()
                .find(|d| d.id == tool_id)
            {
                if let Ok(mut envelope) = (def.render_result)(tcr) {
                    return serialize_tool_result(&mut envelope);
                }
            }
        }
    }

    // Fallback to iterating for any legacy or CallMcpTool results
    for def in registry::BUILTIN_TOOL_REGISTRY {
        if let Ok(mut envelope) = (def.render_result)(tcr) {
            if envelope.status as u8 == results::ToolResultStatus::Error as u8 {
                if let Some(err) = &envelope.error {
                    if err.code == "INVALID_RESULT_TYPE" {
                        continue;
                    }
                }
            }
            return serialize_tool_result(&mut envelope);
        }
    }

    "[no result]".to_string()
}

pub(super) fn serialize_tool_result(env: &mut results::ToolResultEnvelope) -> String {
    if let Some(data) = &mut env.data {
        utils::truncate_json_leaves(data, MAX_RESULT_CHARS);
    }
    if let Some(error) = &mut env.error {
        if error.message.chars().count() > MAX_RESULT_CHARS {
            let truncated: String = error.message.chars().take(MAX_RESULT_CHARS).collect();
            error.message = format!(
                "{truncated}\n… [truncated {} more chars]",
                error.message.chars().count() - MAX_RESULT_CHARS
            );
        }
    }

    serde_json::to_string_pretty(env).unwrap_or_else(|_| "{}".to_string())
}
