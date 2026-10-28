use serde_json::{json, Value};
use warp_multi_agent_api::message::tool_call as tc;

use super::availability::ToolAvailability;
use super::context::ToolRuntimeContext;
use super::errors::{ToolArgumentError, ToolEncodingError, ToolResultRenderError};
use super::registry::{
    ContextRetentionPolicy, FingerprintSpec, LocalToolDefinition, LocalToolId,
    ProviderNeutralToolCall, ToolExecutionPolicy, ToolLoopSemantics,
};
use super::results::{ToolResultEnvelope, ToolResultRenderResult};

pub const WEB_FETCH: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::WebFetch,
    provider_name: "web_fetch",
    execution_policy: ToolExecutionPolicy::ParallelReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::ReplaceWithSummary,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: web_fetch_available,
    schema: web_fetch_schema,
    decode: decode_web_fetch,
    encode_warp_tool: encode_web_fetch,
    encode_history: history_web_fetch,
    render_result: render_web_fetch_result,
};

fn web_fetch_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    // Always available via internal reqwest client
    ToolAvailability::Available
}

fn web_fetch_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "web_fetch",
            "description": "Fetch the content of multiple URLs. Used by the deep web research agent to verify claims and extract content.",
            "parameters": {
                "type": "object",
                "properties": {
                    "urls": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of URLs to fetch."
                    },
                    "extract_mode": {
                        "type": "string",
                        "description": "Extraction mode (full or summary)."
                    }
                },
                "required": ["urls"]
            }
        }
    })
}

fn decode_web_fetch(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    if !args.is_object() {
        return Err(ToolArgumentError::InvalidSchema("Expected object".into()));
    }
    if args.get("urls").and_then(Value::as_array).is_none() {
        return Err(ToolArgumentError::MissingField("urls"));
    }
    Ok(args)
}

fn encode_web_fetch(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    use crate::ai::local_brain::tools::utils::json_object_to_struct;
    // Intercepted via CallMcpTool transport in the execution layer
    Ok(tc::Tool::CallMcpTool(tc::CallMcpTool {
        server_id: "warp_internal".to_string(),
        name: "web_fetch".to_string(),
        args: json_object_to_struct(&args),
    }))
}

fn history_web_fetch(_tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    None
}

fn render_web_fetch_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    let result = match &tcr.result {
        Some(warp_multi_agent_api::message::tool_call_result::Result::CallMcpTool(r)) => r,
        _ => {
            return Err(ToolResultRenderError::ExtractionFailed(
                "Expected MCP result".into(),
            ))
        }
    };

    let success = match &result.result {
        Some(warp_multi_agent_api::call_mcp_tool_result::Result::Success(s)) => s,
        Some(warp_multi_agent_api::call_mcp_tool_result::Result::Error(e)) => {
            return Err(ToolResultRenderError::ExtractionFailed(e.message.clone()))
        }
        _ => {
            return Err(ToolResultRenderError::ExtractionFailed(
                "Expected success result".into(),
            ))
        }
    };

    let text = success
        .results
        .first()
        .and_then(|r| match &r.result {
            Some(warp_multi_agent_api::call_mcp_tool_result::success::result::Result::Text(t)) => {
                Some(t.text.clone())
            }
            _ => None,
        })
        .ok_or_else(|| ToolResultRenderError::ExtractionFailed("Missing text content".into()))?;

    let json_val: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "text": text }));
    ToolResultEnvelope::success(LocalToolId::WebFetch, json_val)
}
