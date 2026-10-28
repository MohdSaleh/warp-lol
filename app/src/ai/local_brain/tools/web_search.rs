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

pub const WEB_SEARCH: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::WebSearch,
    provider_name: "web_search",
    execution_policy: ToolExecutionPolicy::ParallelReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::ReplaceWithSummary,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: web_search_available,
    schema: web_search_schema,
    decode: decode_web_search,
    encode_warp_tool: encode_web_search,
    encode_history: history_web_search,
    render_result: render_web_search_result,
};

fn web_search_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    // Always available via DuckDuckGo
    ToolAvailability::Available
}

fn web_search_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "web_search",
            "description": "Deep web research tool. Use this tool to query a web search engine and retrieve highly relevant links and snippets.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search query." },
                    "num_results": { "type": "integer", "description": "Number of results to return (default 10)." }
                },
                "required": ["query"]
            }
        }
    })
}

fn decode_web_search(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    if !args.is_object() {
        return Err(ToolArgumentError::InvalidSchema("Expected object".into()));
    }
    if args.get("query").and_then(Value::as_str).is_none() {
        return Err(ToolArgumentError::MissingField("query"));
    }
    Ok(args)
}

fn encode_web_search(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    use crate::ai::local_brain::tools::utils::json_object_to_struct;
    // Intercepted via CallMcpTool transport in the execution layer
    Ok(tc::Tool::CallMcpTool(tc::CallMcpTool {
        server_id: "warp_internal".to_string(),
        name: "web_search".to_string(),
        args: json_object_to_struct(&args),
    }))
}

fn history_web_search(_tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    None
}

fn render_web_search_result(
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
    ToolResultEnvelope::success(LocalToolId::WebSearch, json_val)
}
