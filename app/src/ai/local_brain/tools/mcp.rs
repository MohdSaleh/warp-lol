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
use super::utils::{json_object_to_struct, struct_to_json};

pub const CALL_MCP_TOOL_ADAPTER: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::CallMcpTool,
    provider_name: "mcp__adapter", // This is not directly matched; lookup handles MCP_PREFIX
    execution_policy: ToolExecutionPolicy::SerialMutating,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch, // Default policy for unknown MCP tools
    availability: call_mcp_available,
    schema: call_mcp_schema,
    decode: decode_call_mcp,
    encode_warp_tool: encode_call_mcp,
    encode_history: history_call_mcp,
    render_result: render_call_mcp_result,
};

const MCP_PREFIX: &str = "mcp__";

fn call_mcp_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}

fn call_mcp_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    // The adapter itself doesn't advertise a single schema.
    // Dynamic MCP schemas are generated during `available_tools()` resolution.
    json!({})
}

fn decode_call_mcp(name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    if !args.is_object() {
        return Err(ToolArgumentError::InvalidSchema("Expected object".into()));
    }

    let rest = name.strip_prefix(MCP_PREFIX).ok_or_else(|| {
        ToolArgumentError::InvalidSchema(format!("malformed MCP tool name: {}", name))
    })?;

    let (server_id, tool_name) = rest.split_once("__").ok_or_else(|| {
        ToolArgumentError::InvalidSchema(format!("malformed MCP tool name: {}", name))
    })?;

    // Inject the server_id and tool_name into the args for encode_warp_tool
    let mut mapped_args = args.clone();
    if let Value::Object(ref mut map) = mapped_args {
        map.insert(
            "__server_id".to_string(),
            Value::String(server_id.to_string()),
        );
        map.insert(
            "__tool_name".to_string(),
            Value::String(tool_name.to_string()),
        );
    }

    Ok(mapped_args)
}

fn encode_call_mcp(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    let server_id = args
        .get("__server_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let tool_name = args
        .get("__tool_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let mut clean_args = args.clone();
    if let Value::Object(ref mut map) = clean_args {
        map.remove("__server_id");
        map.remove("__tool_name");
    }

    let args_struct = json_object_to_struct(&clean_args);

    Ok(tc::Tool::CallMcpTool(tc::CallMcpTool {
        name: tool_name,
        args: args_struct,
        server_id,
    }))
}

fn history_call_mcp(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::CallMcpTool(m) = tool {
        let name = format!("{MCP_PREFIX}{}__{}", m.server_id, m.name);
        let args = m
            .args
            .as_ref()
            .map(struct_to_json)
            .unwrap_or_else(|| json!({}));
        Some((name, args.to_string()))
    } else {
        None
    }
}

fn render_call_mcp_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::call_mcp_tool_result::Result as Sr;
    use warp_multi_agent_api::message::tool_call_result::Result as R;

    match &tcr.result {
        Some(R::CallMcpTool(r)) => match &r.result {
            Some(Sr::Success(s)) => {
                let items: Vec<serde_json::Value> = s
                    .results
                    .iter()
                    .map(|item| match &item.result {
                        Some(warp_multi_agent_api::call_mcp_tool_result::success::result::Result::Text(t)) => {
                            serde_json::json!({ "type": "text", "text": t.text })
                        }
                        Some(warp_multi_agent_api::call_mcp_tool_result::success::result::Result::Image(img)) => {
                            serde_json::json!({ "type": "image", "mime_type": img.mime_type, "data": img.data })
                        }
                        Some(warp_multi_agent_api::call_mcp_tool_result::success::result::Result::Resource(res)) => {
                            serde_json::json!({ "type": "resource", "uri": res.uri })
                        }
                        None => serde_json::json!({}),
                    })
                    .collect();
                ToolResultEnvelope::success(
                    LocalToolId::CallMcpTool,
                    serde_json::json!({ "results": items }),
                )
            }
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::CallMcpTool,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "MCP_TOOL_ERROR",
                    message: e.message.clone(),
                    retryable: true,
                    field: None,
                },
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::CallMcpTool,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "NO_RESULT",
                    message: "no MCP tool result".to_string(),
                    retryable: false,
                    field: None,
                },
            )),
        },
        // Any non-MCP result must fall through so the probe loop reaches the
        // correct typed codec. Returning INVALID_RESULT_TYPE signals that.
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::CallMcpTool,
            super::results::ToolResultError {
                kind: super::results::ToolErrorKind::ExecutionFailure,
                code: "INVALID_RESULT_TYPE",
                message: "no result or wrong result type".to_string(),
                retryable: false,
                field: None,
            },
        )),
    }
}
