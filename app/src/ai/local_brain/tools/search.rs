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
use super::utils::string_array;

pub const GREP: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::Grep,
    provider_name: "grep",
    execution_policy: ToolExecutionPolicy::ParallelReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::ReplaceWithSummary,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: grep_available,
    schema: grep_schema,
    decode: decode_grep,
    encode_warp_tool: encode_grep,
    encode_history: history_grep,
    render_result: render_grep_result,
};

fn grep_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}

fn grep_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "grep",
            "description": "Search file contents for one or more regex/text patterns under a path.",
            "parameters": {
                "type": "object",
                "properties": {
                    "queries": { "type": "array", "items": { "type": "string" }, "description": "Patterns to search for." },
                    "path": { "type": "string", "description": "File or directory to search in (default: working directory)." }
                },
                "required": ["queries"]
            }
        }
    })
}

fn decode_grep(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    if !args.is_object() {
        return Err(ToolArgumentError::InvalidSchema("Expected object".into()));
    }
    let queries = string_array(&args, "queries");
    if queries.is_none() || queries.as_ref().unwrap().is_empty() {
        return Err(ToolArgumentError::MissingField("queries"));
    }
    Ok(args)
}

fn encode_grep(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    let queries = string_array(&args, "queries").unwrap_or_default();
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or(".")
        .to_string();
    Ok(tc::Tool::Grep(tc::Grep { queries, path }))
}

fn history_grep(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::Grep(g) = tool {
        Some((
            "grep".to_string(),
            json!({ "queries": g.queries, "path": g.path }).to_string(),
        ))
    } else {
        None
    }
}

fn render_grep_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::grep_result::Result as Gr;
    use warp_multi_agent_api::message::tool_call_result::Result as R;

    match &tcr.result {
        Some(R::Grep(r)) => match &r.result {
            Some(Gr::Success(success)) => {
                let data = success
                    .matched_files
                    .iter()
                    .map(|hit| {
                        json!({
                            "file_path": hit.file_path,
                            "lines": hit.matched_lines.iter().map(|line| {
                                json!({
                                    "line_number": line.line_number,
                                })
                            }).collect::<Vec<_>>()
                        })
                    })
                    .collect::<Vec<_>>();
                ToolResultEnvelope::success(LocalToolId::Grep, json!({ "hits": data }))
            }
            Some(Gr::Error(error)) => Ok(ToolResultEnvelope::error(
                LocalToolId::Grep,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "GREP_ERROR",
                    message: error.message.clone(),
                    retryable: true,
                    field: None,
                },
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::Grep,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "NO_RESULT",
                    message: "no grep result".to_string(),
                    retryable: false,
                    field: None,
                },
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::Grep,
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
