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

pub const REQUEST_COMPUTER_USE: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::RequestComputerUse,
    provider_name: "request_computer_use",
    execution_policy: ToolExecutionPolicy::ComputerUse,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::DropOnCompaction,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: request_computer_use_available,
    schema: request_computer_use_schema,
    decode: decode_request_computer_use,
    encode_warp_tool: encode_request_computer_use,
    encode_history: history_request_computer_use,
    render_result: render_request_computer_use_result,
};

pub const USE_COMPUTER: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::UseComputer,
    provider_name: "use_computer",
    execution_policy: ToolExecutionPolicy::ComputerUse,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::DropOnCompaction,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: use_computer_available,
    schema: use_computer_schema,
    decode: decode_use_computer,
    encode_warp_tool: encode_use_computer,
    encode_history: history_use_computer,
    render_result: render_use_computer_result,
};

fn request_computer_use_available(context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if context.computer_use_enabled {
        ToolAvailability::Available
    } else {
        ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::FeatureDisabled,
        }
    }
}
fn request_computer_use_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "request_computer_use",
            "description": "Request a computer use session.",
            "parameters": {
                "type": "object",
                "properties": {
                    "task_summary": { "type": "string" }
                }
            }
        }
    })
}
fn decode_request_computer_use(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_request_computer_use(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::RequestComputerUse(tc::RequestComputerUse {
        task_summary: args
            .get("task_summary")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        screenshot_params: None,
    }))
}
fn history_request_computer_use(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::RequestComputerUse(c) = tool {
        Some((
            "request_computer_use".into(),
            json!({"task_summary": c.task_summary}).to_string(),
        ))
    } else {
        None
    }
}
fn render_request_computer_use_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::request_computer_use_result::Result as Sr;
    match &tcr.result {
        Some(R::RequestComputerUseResult(r)) => match &r.result {
            Some(Sr::Approved(_s)) => ToolResultEnvelope::success(
                LocalToolId::RequestComputerUse,
                json!({ "status": "approved" }),
            ),
            Some(Sr::Rejected(_s)) => ToolResultEnvelope::success(
                LocalToolId::RequestComputerUse,
                json!({ "status": "rejected" }),
            ),
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::RequestComputerUse,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::RequestComputerUse,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::RequestComputerUse,
            "Unexpected result type".into(),
        )),
    }
}

fn use_computer_available(context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if context.computer_use_enabled {
        ToolAvailability::Available
    } else {
        ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::FeatureDisabled,
        }
    }
}
fn use_computer_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "use_computer",
            "description": "Execute actions on a computer.",
            "parameters": {
                "type": "object",
                "properties": {
                    "action_summary": { "type": "string" },
                    "actions": { "type": "array", "items": { "type": "object", "properties": { "type": {"type": "string"} } } }
                }
            }
        }
    })
}
fn decode_use_computer(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_use_computer(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    let action_summary = args
        .get("action_summary")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let raw_actions = args
        .get("actions")
        .map(|a| a.to_string())
        .unwrap_or_default();

    Ok(tc::Tool::UseComputer(tc::UseComputer {
        actions: vec![], // Preserved in action_summary
        post_actions_screenshot_params: None,
        action_summary: format!("{} | Actions: {}", action_summary, raw_actions),
    }))
}
fn history_use_computer(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::UseComputer(c) = tool {
        Some((
            "use_computer".into(),
            json!({
                "action_summary": c.action_summary,
            })
            .to_string(),
        ))
    } else {
        None
    }
}
fn render_use_computer_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::use_computer_result::Result as Sr;
    match &tcr.result {
        Some(R::UseComputer(r)) => match &r.result {
            Some(Sr::Success(_s)) => {
                ToolResultEnvelope::success(LocalToolId::UseComputer, json!({}))
            }
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::UseComputer,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::UseComputer,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::UseComputer,
            "Unexpected result type".into(),
        )),
    }
}
