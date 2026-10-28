use serde_json::{json, Value};
use warp_multi_agent_api::message::tool_call as tc;

use super::availability::ToolAvailability;
use super::context::ToolRuntimeContext;
use super::errors::{ToolArgumentError, ToolEncodingError};
use super::registry::{
    ContextRetentionPolicy, FingerprintSpec, LocalToolDefinition, LocalToolId,
    ProviderNeutralToolCall, ToolExecutionPolicy, ToolLoopSemantics,
};
use super::results::{ToolResultEnvelope, ToolResultRenderResult};

pub const READ_MCP_RESOURCE: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::ReadMcpResource,
    provider_name: "read_mcp_resource",
    execution_policy: ToolExecutionPolicy::SerialReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::ReplaceWithSummary,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: read_mcp_resource_available,
    schema: read_mcp_resource_schema,
    decode: decode_read_mcp_resource,
    encode_warp_tool: encode_read_mcp_resource,
    encode_history: history_read_mcp_resource,
    render_result: render_read_mcp_resource_result,
};

pub const READ_SKILL: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::ReadSkill,
    provider_name: "read_skill",
    execution_policy: ToolExecutionPolicy::SerialReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: read_skill_available,
    schema: read_skill_schema,
    decode: decode_read_skill,
    encode_warp_tool: encode_read_skill,
    encode_history: history_read_skill,
    render_result: render_read_skill_result,
};

pub const FETCH_CONVERSATION: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::FetchConversation,
    provider_name: "fetch_conversation",
    execution_policy: ToolExecutionPolicy::SerialReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: fetch_conversation_available,
    schema: fetch_conversation_schema,
    decode: decode_fetch_conversation,
    encode_warp_tool: encode_fetch_conversation,
    encode_history: history_fetch_conversation,
    render_result: render_fetch_conversation_result,
};

fn read_mcp_resource_available(context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if context.mcp_context.is_some() {
        ToolAvailability::Available
    } else {
        ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::FeatureDisabled,
        }
    }
}
fn read_mcp_resource_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "read_mcp_resource",
            "description": "Read an MCP resource.",
            "parameters": {
                "type": "object",
                "properties": {
                    "uri": { "type": "string" },
                    "server_id": { "type": "string" }
                },
                "required": ["uri", "server_id"]
            }
        }
    })
}
fn decode_read_mcp_resource(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_read_mcp_resource(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::ReadMcpResource(tc::ReadMcpResource {
        uri: args
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        server_id: args
            .get("server_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
    }))
}
fn history_read_mcp_resource(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::ReadMcpResource(c) = tool {
        Some((
            "read_mcp_resource".into(),
            json!({"uri": c.uri, "server_id": c.server_id}).to_string(),
        ))
    } else {
        None
    }
}
fn render_read_mcp_resource_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::read_mcp_resource_result::Result as Sr;
    match &tcr.result {
        Some(R::ReadMcpResource(r)) => match &r.result {
            Some(Sr::Success(s)) => {
                let mut results = vec![];
                for rr in &s.contents {
                    results
                        .push(json!({ "uri": rr.uri, "content_type": rr.content_type.is_some() }));
                }
                ToolResultEnvelope::success(
                    LocalToolId::ReadMcpResource,
                    json!({ "results": results }),
                )
            }
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadMcpResource,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadMcpResource,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::ReadMcpResource,
            "Unexpected result type".into(),
        )),
    }
}

fn read_skill_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn read_skill_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "read_skill",
            "description": "Read a skill's content.",
            "parameters": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }
        }
    })
}
fn decode_read_skill(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_read_skill(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    use warp_multi_agent_api::message::tool_call::read_skill::SkillReference;
    let name = args
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let skill_reference = if name.contains('/') {
        Some(SkillReference::SkillPath(name.clone()))
    } else {
        Some(SkillReference::BundledSkillId(name.clone()))
    };

    Ok(tc::Tool::ReadSkill(tc::ReadSkill {
        name,
        skill_reference,
    }))
}
fn history_read_skill(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::ReadSkill(c) = tool {
        Some(("read_skill".into(), json!({"name": c.name}).to_string()))
    } else {
        None
    }
}
fn render_read_skill_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::read_skill_result::Result as Sr;
    match &tcr.result {
        Some(R::ReadSkill(r)) => match &r.result {
            Some(Sr::Success(s)) => ToolResultEnvelope::success(
                LocalToolId::ReadSkill,
                json!({ "content": s.content.as_ref().map(|c| json!({ "file_path": c.file_path, "content": c.content, "line_range": c.line_range.as_ref().map(|lr| json!({"start_line": lr.start, "end_line": lr.end})) })) }),
            ),
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadSkill,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadSkill,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::ReadSkill,
            "Unexpected result type".into(),
        )),
    }
}

fn fetch_conversation_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn fetch_conversation_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "fetch_conversation",
            "description": "Fetch a conversation's content.",
            "parameters": {
                "type": "object",
                "properties": {
                    "conversation_id": { "type": "string" }
                },
                "required": ["conversation_id"]
            }
        }
    })
}
fn decode_fetch_conversation(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_fetch_conversation(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::FetchConversation(tc::FetchConversation {
        conversation_id: args
            .get("conversation_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
    }))
}
fn history_fetch_conversation(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::FetchConversation(c) = tool {
        Some((
            "fetch_conversation".into(),
            json!({"conversation_id": c.conversation_id}).to_string(),
        ))
    } else {
        None
    }
}
fn render_fetch_conversation_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::fetch_conversation_result::Result as Sr;
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    match &tcr.result {
        Some(R::FetchConversation(r)) => match &r.result {
            Some(Sr::Success(s)) => ToolResultEnvelope::success(
                LocalToolId::FetchConversation,
                json!({ "directory_path": s.directory_path }),
            ),
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::FetchConversation,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::FetchConversation,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::FetchConversation,
            "Unexpected result type".into(),
        )),
    }
}
