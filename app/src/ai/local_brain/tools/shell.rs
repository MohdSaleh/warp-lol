use serde_json::{json, Value};
use warp_multi_agent_api::message::tool_call as tc;

use super::availability::ToolAvailability;
use super::context::ToolRuntimeContext;
use super::errors::{ToolArgumentError, ToolEncodingError};
use super::registry::{
    ContextRetentionPolicy, FingerprintSpec, LocalToolDefinition, LocalToolId,
    ProviderNeutralToolCall, ToolExecutionPolicy, ToolLoopSemantics,
};
use super::results::{ToolResultEnvelope, ToolResultRenderResult, ToolResultStatus};

pub const RUN_SHELL_COMMAND: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::RunShellCommand,
    provider_name: "run_command",
    execution_policy: ToolExecutionPolicy::SerialMutating,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::ReplaceWithSummary,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: run_command_available,
    schema: run_command_schema,
    decode: decode_run_command,
    encode_warp_tool: encode_run_command,
    encode_history: history_run_command,
    render_result: render_run_command_result,
};

fn run_command_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}

fn run_command_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "run_command",
            "description": "Run a non-interactive shell command in the working directory and return its output and exit code.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The shell command to run." },
                    "is_read_only": { "type": "boolean", "description": "True if the command only reads state and makes no changes." }
                },
                "required": ["command"]
            }
        }
    })
}

fn decode_run_command(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    if !args.is_object() {
        return Err(ToolArgumentError::InvalidSchema("Expected object".into()));
    }
    let cmd = args
        .get("command")
        .and_then(Value::as_str)
        .ok_or(ToolArgumentError::MissingField("command"))?;
    if cmd.trim().is_empty() {
        return Err(ToolArgumentError::InvalidSchema(
            "command cannot be empty".into(),
        ));
    }
    Ok(args)
}

fn encode_run_command(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    let command = args
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let is_read_only = args
        .get("is_read_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(tc::Tool::RunShellCommand(tc::RunShellCommand {
        command,
        is_read_only,
        uses_pager: false,
        citations: vec![],
        is_risky: false,
        risk_category: 0,
        wait_until_complete_value: None,
    }))
}

fn history_run_command(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::RunShellCommand(c) = tool {
        Some((
            "run_command".to_string(),
            json!({ "command": c.command, "is_read_only": c.is_read_only }).to_string(),
        ))
    } else {
        None
    }
}

fn render_run_command_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::run_shell_command_result::Result as Sr;

    match &tcr.result {
        Some(R::RunShellCommand(r)) => match &r.result {
            Some(Sr::CommandFinished(finished)) => ToolResultEnvelope::success(
                LocalToolId::RunShellCommand,
                json!({
                    "exit_code": finished.exit_code,
                    "output": finished.output
                }),
            ),
            Some(Sr::LongRunningCommandSnapshot(snapshot)) => ToolResultEnvelope::success(
                LocalToolId::RunShellCommand,
                json!({
                    "snapshot": snapshot.output
                }),
            ),
            Some(_) => {
                let mut env = ToolResultEnvelope::error(
                    LocalToolId::RunShellCommand,
                    super::results::ToolResultError {
                        kind: super::results::ToolErrorKind::ExecutionFailure,
                        code: "RUNNING_OR_NOT_PERMITTED",
                        message: "command is still running or was not permitted".to_string(),
                        retryable: false,
                        field: None,
                    },
                );
                env.status = ToolResultStatus::Error;
                Ok(env)
            }
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::RunShellCommand,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "NO_RESULT",
                    message: "no command result".to_string(),
                    retryable: false,
                    field: None,
                },
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::RunShellCommand,
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
