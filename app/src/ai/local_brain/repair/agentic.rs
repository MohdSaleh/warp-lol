//! Agentic workflow error diagnosis & on-the-fly self-reflection.
//!
//! When a tool fails (non-zero shell exit code, syntax/compiler errors, missing files,
//! unparseable arguments, or permission errors), this module:
//! 1. Diagnoses the exact failure where execution stopped.
//! 2. Synthesizes a structured Self-Reflection directive.
//! 3. Enables unbounded on-the-fly repairs without artificial turn/tool-call caps.

use serde_json::Value;
use ai::agent::action_result::{AIAgentActionResultType, RequestCommandOutputResult};
use warp_multi_agent_api as api;

use crate::ai::agent::AIAgentInput;
use crate::ai::local_brain::tools;
use crate::ai::local_brain::tools::registry::LocalToolRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgenticFailureKind {
    NonZeroExitCode,
    ExecutionError,
    SchemaValidationError,
    FileNotFoundOrMissing,
    PermissionDenied,
    ToolExecutionDenied,
    ToolUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgenticErrorDiagnosis {
    pub tool_name: String,
    pub tool_call_id: String,
    pub input_summary: String,
    pub error_output: String,
    pub exit_code: Option<i32>,
    pub failure_kind: AgenticFailureKind,
}

/// Diagnoses whether the most recent action/tool observation in history or pending inputs
/// ended in an agentic error.
pub fn diagnose_agentic_error(
    tasks: &[api::Task],
    inputs: &[AIAgentInput],
) -> Option<AgenticErrorDiagnosis> {
    // 1. Check pending input action results first (freshest results from this turn's execution)
    for input in inputs.iter().rev() {
        if let AIAgentInput::ActionResult { result, .. } = input {
            if let Some(diagnosis) = diagnose_from_action_result(&result.id.to_string(), &result.result) {
                return Some(diagnosis);
            }
        }
    }

    // 2. Check task messages (reconstructed history across turns)
    for task in tasks.iter().rev() {
        let mut calls_by_id = std::collections::HashMap::new();
        for msg in &task.messages {
            if let Some(api::message::Message::ToolCall(call)) = &msg.message {
                let (name, args) = tools::api_tool_to_openai(&call.tool);
                calls_by_id.insert(call.tool_call_id.clone(), (name, args));
            }
        }

        for msg in task.messages.iter().rev() {
            if let Some(api::message::Message::ToolCallResult(result)) = &msg.message {
                let (call_name, args) = calls_by_id
                    .get(&result.tool_call_id)
                    .cloned()
                    .unwrap_or_else(|| ("unknown_tool".to_string(), "{}".to_string()));

                if let Some(diagnosis) = diagnose_from_tool_call_result(result, &call_name, &args) {
                    return Some(diagnosis);
                }
            }
        }
    }

    None
}

/// Checks whether the conversation is currently in an agentic error repair state.
pub fn is_repairing_agentic_error(
    tasks: &[api::Task],
    inputs: &[AIAgentInput],
) -> bool {
    diagnose_agentic_error(tasks, inputs).is_some()
}

/// Formulates a structured Self-Reflection & On-The-Fly Auto-Repair prompt block.
pub fn generate_self_reflection_prompt(diagnosis: &AgenticErrorDiagnosis) -> String {
    let exit_code_str = match diagnosis.exit_code {
        Some(code) => format!("Exit Code {code}"),
        None => "Execution Error".to_string(),
    };

    let truncated_error = if diagnosis.error_output.len() > 4000 {
        format!("{}...\n[Error output truncated]", &diagnosis.error_output[..4000])
    } else {
        diagnosis.error_output.clone()
    };

    format!(
        "\n\
        ================================================================================\n\
        ⚡ AGENTIC SELF-REFLECTION & ON-THE-FLY REPAIR ACTIVE\n\
        ================================================================================\n\
        The previous action stopped due to an error:\n\
        - Failed Tool: `{tool}`\n\
        - Status / Code: {status}\n\
        - Input / Command: {input}\n\
        - Observed Failure:\n\
        ```\n\
        {error}\n\
        ```\n\n\
        **MANDATORY SELF-REFLECTION PROTOCOL:**\n\
        1. **Analyze Failure**: Identify the exact root cause of the error above (e.g. wrong directory, missing flag, syntax mistake, uninstalled dependency, or incorrect file path).\n\
        2. **Formulate Repair**: Do NOT blindly repeat the identical command or arguments. Formulate an adjusted strategy, inspect needed files with `read_files` or `grep`, or use an alternate tool.\n\
        3. **Autonomous Repair Without Limit**: You are authorized to perform repairs on the fly without limit. Continue executing now until the objective is accomplished.\n\
        ================================================================================\n",
        tool = diagnosis.tool_name,
        status = exit_code_str,
        input = diagnosis.input_summary,
        error = truncated_error.trim(),
    )
}

fn diagnose_from_action_result(
    id: &str,
    result: &AIAgentActionResultType,
) -> Option<AgenticErrorDiagnosis> {
    match result {
        AIAgentActionResultType::RequestCommandOutput(res) => match res {
            RequestCommandOutputResult::Completed {
                command,
                output,
                exit_code,
                ..
            } => {
                if !exit_code.was_successful() {
                    Some(AgenticErrorDiagnosis {
                        tool_name: "run_command".to_string(),
                        tool_call_id: id.to_string(),
                        input_summary: command.clone(),
                        error_output: output.clone(),
                        exit_code: Some(exit_code.value()),
                        failure_kind: AgenticFailureKind::NonZeroExitCode,
                    })
                } else {
                    None
                }
            }
            RequestCommandOutputResult::Denylisted { command } => Some(AgenticErrorDiagnosis {
                tool_name: "run_command".to_string(),
                tool_call_id: id.to_string(),
                input_summary: command.clone(),
                error_output: "Command is on the denylist and was denied".to_string(),
                exit_code: None,
                failure_kind: AgenticFailureKind::ToolExecutionDenied,
            }),
            RequestCommandOutputResult::CancelledBeforeExecution => Some(AgenticErrorDiagnosis {
                tool_name: "run_command".to_string(),
                tool_call_id: id.to_string(),
                input_summary: "shell command".to_string(),
                error_output: "Command was cancelled before execution".to_string(),
                exit_code: None,
                failure_kind: AgenticFailureKind::ExecutionError,
            }),
            _ => None,
        },
        _ => None,
    }
}

fn diagnose_from_tool_call_result(
    result: &api::message::ToolCallResult,
    tool_name: &str,
    args: &str,
) -> Option<AgenticErrorDiagnosis> {
    use api::message::tool_call_result::Result as R;
    use api::run_shell_command_result::Result as Sr;

    if let Some(res) = &result.result {
        match res {
            R::RunShellCommand(r) => match &r.result {
                Some(Sr::CommandFinished(finished)) => {
                    if finished.exit_code != 0 {
                        let effective_tool = if tool_name != "unknown_tool" {
                            tool_name
                        } else {
                            "run_command"
                        };
                        return Some(AgenticErrorDiagnosis {
                            tool_name: effective_tool.to_string(),
                            tool_call_id: result.tool_call_id.clone(),
                            input_summary: args.to_string(),
                            error_output: finished.output.clone(),
                            exit_code: Some(finished.exit_code),
                            failure_kind: AgenticFailureKind::NonZeroExitCode,
                        });
                    }
                }
                Some(_) => {}
                None => {
                    let effective_tool = if tool_name != "unknown_tool" {
                        tool_name
                    } else {
                        "run_command"
                    };
                    return Some(AgenticErrorDiagnosis {
                        tool_name: effective_tool.to_string(),
                        tool_call_id: result.tool_call_id.clone(),
                        input_summary: args.to_string(),
                        error_output: "No shell command result produced".to_string(),
                        exit_code: None,
                        failure_kind: AgenticFailureKind::ExecutionError,
                    });
                }
            },
            R::ApplyFileDiffs(diff_res) => {
                if let Some(api::apply_file_diffs_result::Result::Error(err)) = &diff_res.result {
                    return Some(AgenticErrorDiagnosis {
                        tool_name: "apply_file_diffs".to_string(),
                        tool_call_id: result.tool_call_id.clone(),
                        input_summary: args.to_string(),
                        error_output: err.message.clone(),
                        exit_code: None,
                        failure_kind: AgenticFailureKind::ExecutionError,
                    });
                }
            }
            R::ReadFiles(rf_res) => {
                if let Some(api::read_files_result::Result::Error(err)) = &rf_res.result {
                    return Some(AgenticErrorDiagnosis {
                        tool_name: "read_files".to_string(),
                        tool_call_id: result.tool_call_id.clone(),
                        input_summary: args.to_string(),
                        error_output: err.message.clone(),
                        exit_code: None,
                        failure_kind: AgenticFailureKind::FileNotFoundOrMissing,
                    });
                }
            }
            R::SearchCodebase(sc_res) => {
                if let Some(api::search_codebase_result::Result::Error(err)) = &sc_res.result {
                    return Some(AgenticErrorDiagnosis {
                        tool_name: "search_codebase".to_string(),
                        tool_call_id: result.tool_call_id.clone(),
                        input_summary: args.to_string(),
                        error_output: err.message.clone(),
                        exit_code: None,
                        failure_kind: AgenticFailureKind::ExecutionError,
                    });
                }
            }
            _ => {}
        }
    }

    // Inspect serialized envelope error if present
    let rendered = tools::render_tool_result(result);
    if let Ok(val) = serde_json::from_str::<Value>(&rendered) {
        let status = val.get("status").and_then(Value::as_str).unwrap_or("");
        if status == "error" || status == "denied" || status == "unavailable" {
            let error_msg = val
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Tool execution failed");

            return Some(AgenticErrorDiagnosis {
                tool_name: tool_name.to_string(),
                tool_call_id: result.tool_call_id.clone(),
                input_summary: args.to_string(),
                error_output: error_msg.to_string(),
                exit_code: None,
                failure_kind: match status {
                    "denied" => AgenticFailureKind::ToolExecutionDenied,
                    "unavailable" => AgenticFailureKind::ToolUnavailable,
                    _ => AgenticFailureKind::ExecutionError,
                },
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_reflection_prompt() {
        let diagnosis = AgenticErrorDiagnosis {
            tool_name: "run_command".to_string(),
            tool_call_id: "call_123".to_string(),
            input_summary: "cargo build --bin non_existent".to_string(),
            error_output: "error: no bin target `non_existent`".to_string(),
            exit_code: Some(101),
            failure_kind: AgenticFailureKind::NonZeroExitCode,
        };

        let prompt = generate_self_reflection_prompt(&diagnosis);
        assert!(prompt.contains("⚡ AGENTIC SELF-REFLECTION"));
        assert!(prompt.contains("run_command"));
        assert!(prompt.contains("Exit Code 101"));
        assert!(prompt.contains("no bin target `non_existent`"));
        assert!(prompt.contains("MANDATORY SELF-REFLECTION PROTOCOL"));
    }

    #[test]
    fn test_diagnose_from_task_messages_non_zero_exit() {
        let mut task = api::Task::default();
        task.messages.push(api::Message {
            id: "msg_1".to_string(),
            message: Some(api::message::Message::ToolCall(api::message::ToolCall {
                tool_call_id: "call_456".to_string(),
                tool: Some(api::message::tool_call::Tool::RunShellCommand(api::message::tool_call::RunShellCommand {
                    command: "cargo check".to_string(),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        });

        task.messages.push(api::Message {
            id: "msg_2".to_string(),
            message: Some(api::message::Message::ToolCallResult(api::message::ToolCallResult {
                tool_call_id: "call_456".to_string(),
                result: Some(api::message::tool_call_result::Result::RunShellCommand(api::RunShellCommandResult {
                    result: Some(api::run_shell_command_result::Result::CommandFinished(api::ShellCommandFinished {
                        output: "error[E0425]: cannot find value `x`".to_string(),
                        exit_code: 1,
                        ..Default::default()
                    })),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        });

        let diagnosis = diagnose_agentic_error(&[task], &[]).expect("Should detect agentic error");
        assert_eq!(diagnosis.tool_name, "run_command");
        assert_eq!(diagnosis.exit_code, Some(1));
        assert!(diagnosis.error_output.contains("cannot find value"));
        assert_eq!(diagnosis.failure_kind, AgenticFailureKind::NonZeroExitCode);
    }

    #[test]
    fn test_no_diagnosis_on_success() {
        let mut task = api::Task::default();
        task.messages.push(api::Message {
            id: "msg_1".to_string(),
            message: Some(api::message::Message::ToolCall(api::message::ToolCall {
                tool_call_id: "call_789".to_string(),
                tool: Some(api::message::tool_call::Tool::RunShellCommand(api::message::tool_call::RunShellCommand {
                    command: "echo hello".to_string(),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        });

        task.messages.push(api::Message {
            id: "msg_2".to_string(),
            message: Some(api::message::Message::ToolCallResult(api::message::ToolCallResult {
                tool_call_id: "call_789".to_string(),
                result: Some(api::message::tool_call_result::Result::RunShellCommand(api::RunShellCommandResult {
                    result: Some(api::run_shell_command_result::Result::CommandFinished(api::ShellCommandFinished {
                        output: "hello".to_string(),
                        exit_code: 0,
                        ..Default::default()
                    })),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        });

        let diagnosis = diagnose_agentic_error(&[task], &[]);
        assert!(diagnosis.is_none(), "Exit code 0 should not be diagnosed as error");
    }
}
