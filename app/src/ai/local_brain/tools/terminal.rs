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

pub const READ_SHELL_COMMAND_OUTPUT: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::ReadShellCommandOutput,
    provider_name: "read_command_output",
    execution_policy: ToolExecutionPolicy::SerialReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::DropOnCompaction,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: read_shell_command_output_available,
    schema: read_shell_command_output_schema,
    decode: decode_read_shell_command_output,
    encode_warp_tool: encode_read_shell_command_output,
    encode_history: history_read_shell_command_output,
    render_result: render_read_shell_command_output_result,
};

pub const WRITE_TO_LONG_RUNNING_SHELL_COMMAND: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::WriteToLongRunningShellCommand,
    provider_name: "write_to_command",
    execution_policy: ToolExecutionPolicy::SerialMutating,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: write_to_long_running_shell_command_available,
    schema: write_to_long_running_shell_command_schema,
    decode: decode_write_to_long_running_shell_command,
    encode_warp_tool: encode_write_to_long_running_shell_command,
    encode_history: history_write_to_long_running_shell_command,
    render_result: render_write_to_long_running_shell_command_result,
};

fn read_shell_command_output_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn read_shell_command_output_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "read_command_output",
            "description": "Read the output of a running command.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command_id": { "type": "string" },
                    "delay_seconds": { "type": "number" }
                },
                "required": ["command_id"]
            }
        }
    })
}
fn decode_read_shell_command_output(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_read_shell_command_output(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    let _delay_seconds = args
        .get("delay_seconds")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    Ok(tc::Tool::ReadShellCommandOutput(
        tc::ReadShellCommandOutput {
            command_id: args
                .get("command_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            delay: None,
        },
    ))
}
fn history_read_shell_command_output(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::ReadShellCommandOutput(c) = tool {
        Some((
            "read_command_output".into(),
            json!({"command_id": c.command_id}).to_string(),
        ))
    } else {
        None
    }
}
fn render_read_shell_command_output_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::read_shell_command_output_result::Result as Sr;
    match &tcr.result {
        Some(R::ReadShellCommandOutput(r)) => match &r.result {
            Some(Sr::LongRunningCommandSnapshot(s)) => ToolResultEnvelope::success(
                LocalToolId::ReadShellCommandOutput,
                json!({ "output": s.output }),
            ),
            Some(Sr::CommandFinished(s)) => ToolResultEnvelope::success(
                LocalToolId::ReadShellCommandOutput,
                json!({ "output": s.output }),
            ),
            Some(Sr::Error(_e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadShellCommandOutput,
                "Command error".into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadShellCommandOutput,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::ReadShellCommandOutput,
            "Unexpected result type".into(),
        )),
    }
}

fn write_to_long_running_shell_command_available(
    _context: &ToolRuntimeContext<'_>,
) -> ToolAvailability {
    ToolAvailability::Available
}
fn write_to_long_running_shell_command_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "write_to_command",
            "description": "Write input to a long running command.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command_id": { "type": "string" },
                    "input": { "type": "string" }
                },
                "required": ["command_id", "input"]
            }
        }
    })
}
fn decode_write_to_long_running_shell_command(
    _name: &str,
    args: Value,
) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_write_to_long_running_shell_command(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::WriteToLongRunningShellCommand(
        tc::WriteToLongRunningShellCommand {
            command_id: args
                .get("command_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            input: args
                .get("input")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
            mode: None,
        },
    ))
}
fn history_write_to_long_running_shell_command(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::WriteToLongRunningShellCommand(c) = tool {
        Some((
            "write_to_command".into(),
            json!({"command_id": c.command_id}).to_string(),
        ))
    } else {
        None
    }
}
fn render_write_to_long_running_shell_command_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::write_to_long_running_shell_command_result::Result as Sr;
    match &tcr.result {
        Some(R::WriteToLongRunningShellCommand(r)) => match &r.result {
            Some(Sr::LongRunningCommandSnapshot(_s)) => {
                ToolResultEnvelope::success(LocalToolId::WriteToLongRunningShellCommand, json!({}))
            }
            Some(Sr::CommandFinished(_s)) => {
                ToolResultEnvelope::success(LocalToolId::WriteToLongRunningShellCommand, json!({}))
            }
            Some(Sr::Error(_e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::WriteToLongRunningShellCommand,
                "Command error".into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::WriteToLongRunningShellCommand,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::WriteToLongRunningShellCommand,
            "Unexpected result type".into(),
        )),
    }
}
