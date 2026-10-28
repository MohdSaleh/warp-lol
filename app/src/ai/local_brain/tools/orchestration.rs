use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use warp_multi_agent_api::message::tool_call as tc;
use warp_multi_agent_api::{harness, run_agents, skill_ref};

use super::availability::ToolAvailability;
use super::context::ToolRuntimeContext;
use super::errors::{ToolArgumentError, ToolEncodingError};
use super::registry::{
    ContextRetentionPolicy, FingerprintSpec, LocalToolDefinition, LocalToolId,
    ProviderNeutralToolCall, ToolExecutionPolicy, ToolLoopSemantics,
};
use super::results::{ToolResultEnvelope, ToolResultRenderResult};

pub const RUN_AGENTS: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::RunAgents,
    provider_name: "run_agents",
    execution_policy: ToolExecutionPolicy::Orchestration,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: run_agents_available,
    schema: run_agents_schema,
    decode: decode_run_agents,
    encode_warp_tool: encode_run_agents,
    encode_history: history_run_agents,
    render_result: render_run_agents_result,
};

pub const SEND_MESSAGE_TO_AGENT: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::SendMessageToAgent,
    provider_name: "send_message_to_agent",
    execution_policy: ToolExecutionPolicy::Orchestration,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: send_message_to_agent_available,
    schema: send_message_to_agent_schema,
    decode: decode_send_message_to_agent,
    encode_warp_tool: encode_send_message_to_agent,
    encode_history: history_send_message_to_agent,
    render_result: render_send_message_to_agent_result,
};

pub const WAIT_FOR_EVENTS: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::WaitForEvents,
    provider_name: "wait_for_events",
    execution_policy: ToolExecutionPolicy::Orchestration,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: wait_for_events_available,
    schema: wait_for_events_schema,
    decode: decode_wait_for_events,
    encode_warp_tool: encode_wait_for_events,
    encode_history: history_wait_for_events,
    render_result: render_wait_for_events_result,
};

pub const SUBAGENT: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::Subagent,
    provider_name: "subagent",
    execution_policy: ToolExecutionPolicy::Orchestration,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: subagent_available,
    schema: subagent_schema,
    decode: decode_subagent,
    encode_warp_tool: encode_subagent,
    encode_history: history_subagent,
    render_result: render_subagent_result,
};

fn run_agents_available(context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if context.orchestration_enabled {
        ToolAvailability::Available
    } else {
        ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::FeatureDisabled,
        }
    }
}
fn run_agents_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "run_agents",
            "description": "Run one or more local child agents in parallel. Give every child a specific, self-contained prompt with the task context it needs.",
            "parameters": {
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "Short description of why the agents are being launched."
                    },
                    "base_prompt": {
                        "type": "string",
                        "description": "Context and instructions shared by every child agent."
                    },
                    "harness": {
                        "type": "string",
                        "enum": ["oz", "claude", "opencode", "gemini", "codex"]
                    },
                    "model_id": { "type": "string" },
                    "skills": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "kind": {
                                    "type": "string",
                                    "enum": ["path", "bundled_skill_id"]
                                },
                                "value": { "type": "string" }
                            },
                            "required": ["kind", "value"],
                            "additionalProperties": false
                        }
                    },
                    "agent_run_configs": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "prompt": {
                                    "type": "string",
                                    "description": "The child's exact delegated task and relevant context. Do not use /init."
                                },
                                "title": { "type": "string" }
                            },
                            "required": ["name", "prompt"],
                            "additionalProperties": false
                        }
                    },
                    "plan_id": { "type": "string" }
                },
                "required": ["agent_run_configs"],
                "additionalProperties": false
            }
        }
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RunAgentsArgs {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    base_prompt: String,
    #[serde(default)]
    skills: Vec<RunAgentsSkillArg>,
    #[serde(default)]
    model_id: String,
    harness: Option<String>,
    agent_run_configs: Vec<RunAgentsAgentArg>,
    #[serde(default)]
    plan_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RunAgentsSkillArg {
    kind: String,
    value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RunAgentsAgentArg {
    name: String,
    prompt: String,
    #[serde(default)]
    title: String,
}

fn decode_run_agents(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    let args: RunAgentsArgs = serde_json::from_value(args)
        .map_err(|error| ToolArgumentError::InvalidSchema(error.to_string()))?;
    validate_run_agents_args(&args).map_err(ToolArgumentError::InvalidSchema)?;
    serde_json::to_value(args).map_err(|error| ToolArgumentError::InvalidSchema(error.to_string()))
}

fn validate_run_agents_args(args: &RunAgentsArgs) -> Result<(), String> {
    if args.agent_run_configs.is_empty() {
        return Err("agent_run_configs must contain at least one child agent".into());
    }

    let mut names = HashSet::new();
    for config in &args.agent_run_configs {
        let name = config.name.trim();
        if name.is_empty() {
            return Err("agent names must be non-empty".into());
        }
        if !names.insert(name) {
            return Err(format!("duplicate agent name: {name}"));
        }
    }

    for skill in &args.skills {
        if skill.value.trim().is_empty() {
            return Err("skill values must be non-empty".into());
        }
        if !matches!(skill.kind.as_str(), "path" | "bundled_skill_id") {
            return Err(format!("unsupported skill kind: {}", skill.kind));
        }
    }

    if let Some(harness) = &args.harness {
        if !matches!(
            harness.as_str(),
            "oz" | "claude" | "opencode" | "gemini" | "codex"
        ) {
            return Err(format!("unsupported harness: {harness}"));
        }
    }

    Ok(())
}

fn encode_run_agents(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    let args: RunAgentsArgs = serde_json::from_value(args)
        .map_err(|error| ToolEncodingError::ConversionFailed(error.to_string()))?;
    validate_run_agents_args(&args).map_err(ToolEncodingError::ConversionFailed)?;

    let harness = args
        .harness
        .as_deref()
        .map(|name| warp_multi_agent_api::Harness {
            variant: Some(match name {
                "oz" => harness::Variant::Oz(harness::Oz {}),
                "claude" => harness::Variant::ClaudeCode(harness::ClaudeCode {}),
                "opencode" => harness::Variant::OpenCode(harness::OpenCode {}),
                "gemini" => harness::Variant::Gemini(harness::Gemini {}),
                "codex" => harness::Variant::Codex(harness::Codex {}),
                _ => unreachable!("validated harness"),
            }),
        });

    Ok(tc::Tool::RunAgents(warp_multi_agent_api::RunAgents {
        summary: args.summary,
        base_prompt: args.base_prompt,
        skills: args
            .skills
            .into_iter()
            .map(|skill| warp_multi_agent_api::SkillRef {
                skill_reference: Some(match skill.kind.as_str() {
                    "path" => skill_ref::SkillReference::Path(skill.value),
                    "bundled_skill_id" => skill_ref::SkillReference::BundledSkillId(skill.value),
                    _ => unreachable!("validated skill kind"),
                }),
            })
            .collect(),
        model_id: args.model_id,
        harness,
        agent_run_configs: args
            .agent_run_configs
            .into_iter()
            .map(|config| run_agents::AgentRunConfig {
                name: config.name,
                prompt: config.prompt,
                title: config.title,
            })
            .collect(),
        plan_id: args.plan_id,
        // Local Brain must never turn orchestration into a Warp backend request.
        execution_mode: Some(run_agents::ExecutionMode::Local(run_agents::Local {})),
    }))
}
fn history_run_agents(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::RunAgents(call) = tool {
        let harness = call.harness.as_ref().and_then(|harness| {
            harness.variant.as_ref().map(|variant| match variant {
                harness::Variant::Oz(_) => "oz",
                harness::Variant::ClaudeCode(_) => "claude",
                harness::Variant::OpenCode(_) => "opencode",
                harness::Variant::Gemini(_) => "gemini",
                harness::Variant::Codex(_) => "codex",
            })
        });
        let skills: Vec<_> = call
            .skills
            .iter()
            .filter_map(|skill| match skill.skill_reference.as_ref()? {
                skill_ref::SkillReference::Path(value) => {
                    Some(json!({ "kind": "path", "value": value }))
                }
                skill_ref::SkillReference::BundledSkillId(value) => {
                    Some(json!({ "kind": "bundled_skill_id", "value": value }))
                }
            })
            .collect();
        let agent_run_configs: Vec<_> = call
            .agent_run_configs
            .iter()
            .map(|config| {
                json!({
                    "name": config.name,
                    "prompt": config.prompt,
                    "title": config.title,
                })
            })
            .collect();
        Some((
            "run_agents".into(),
            json!({
                "summary": call.summary,
                "base_prompt": call.base_prompt,
                "skills": skills,
                "model_id": call.model_id,
                "harness": harness,
                "agent_run_configs": agent_run_configs,
                "plan_id": call.plan_id,
                "execution_mode": "local",
            })
            .to_string(),
        ))
    } else {
        None
    }
}
fn render_run_agents_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::run_agents_result::Outcome as Sr;
    match &tcr.result {
        Some(R::RunAgentsResult(r)) => match &r.outcome {
            Some(Sr::Launched(s)) => ToolResultEnvelope::success(
                LocalToolId::RunAgents,
                json!({
                    "agents": s.agents.iter().map(|a| {
                        let (status, error, agent_id) = match &a.result {
                            Some(warp_multi_agent_api::run_agents_result::agent_outcome::Result::Launched(suc)) => ("success", None, Some(suc.agent_id.clone())),
                            Some(warp_multi_agent_api::run_agents_result::agent_outcome::Result::Failed(err)) => ("error", Some(err.error.clone()), None),
                            None => ("unknown", None, None),
                        };
                        json!({
                            "name": a.name,
                            "status": status,
                            "agent_id": agent_id,
                            "error": error,
                        })
                    }).collect::<Vec<_>>()
                }),
            ),
            Some(Sr::Failure(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::RunAgents,
                e.error.clone().into(),
            )),
            _ => Ok(ToolResultEnvelope::error(
                LocalToolId::RunAgents,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::RunAgents,
            "Unexpected result type".into(),
        )),
    }
}

fn send_message_to_agent_available(context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if context.is_local_profile {
        // send_message_to_agent routes through the Warp server API; forbidden in local-only mode.
        return ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::LocalOnlyViolation,
        };
    }
    ToolAvailability::Unavailable {
        reason: super::availability::ToolUnavailableReason::FeatureDisabled,
    }
}
fn send_message_to_agent_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "send_message_to_agent",
            "description": "Send a message to an agent.",
            "parameters": {
                "type": "object",
                "properties": {
                    "addresses": { "type": "array", "items": { "type": "string" } },
                    "subject": { "type": "string" },
                    "message": { "type": "string" }
                },
                "required": ["addresses", "message"]
            }
        }
    })
}
fn decode_send_message_to_agent(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_send_message_to_agent(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::SendMessageToAgent(
        warp_multi_agent_api::SendMessageToAgent {
            addresses: args
                .get("addresses")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            subject: args
                .get("subject")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            message: args
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
        },
    ))
}
fn history_send_message_to_agent(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::SendMessageToAgent(c) = tool {
        Some((
            "send_message_to_agent".into(),
            json!({"addresses": c.addresses, "subject": c.subject}).to_string(),
        ))
    } else {
        None
    }
}
fn render_send_message_to_agent_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::send_message_to_agent_result::Result as Sr;
    match &tcr.result {
        Some(R::SendMessageToAgent(r)) => match &r.result {
            Some(Sr::Success(_s)) => {
                ToolResultEnvelope::success(LocalToolId::SendMessageToAgent, json!({}))
            }
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::SendMessageToAgent,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::SendMessageToAgent,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::SendMessageToAgent,
            "Unexpected result type".into(),
        )),
    }
}

fn wait_for_events_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Unavailable {
        reason: super::availability::ToolUnavailableReason::FeatureDisabled,
    }
}
fn wait_for_events_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "wait_for_events",
            "description": "Wait for events.",
            "parameters": {
                "type": "object",
                "properties": {
                    "idle_timeout_seconds": { "type": "number" }
                }
            }
        }
    })
}
fn decode_wait_for_events(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_wait_for_events(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::WaitForEvents(tc::WaitForEvents {
        idle_timeout_seconds: args
            .get("idle_timeout_seconds")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32,
    }))
}
fn history_wait_for_events(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::WaitForEvents(c) = tool {
        Some((
            "wait_for_events".into(),
            json!({"idle_timeout_seconds": c.idle_timeout_seconds}).to_string(),
        ))
    } else {
        None
    }
}
fn render_wait_for_events_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    match &tcr.result {
        Some(R::WaitForEvents(_r)) => {
            ToolResultEnvelope::success(LocalToolId::WaitForEvents, json!({}))
        }
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::WaitForEvents,
            "Unexpected result type".into(),
        )),
    }
}

fn subagent_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Unavailable {
        reason: super::availability::ToolUnavailableReason::FeatureDisabled,
    }
}
fn subagent_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "subagent",
            "description": "Spawn a subagent.",
            "parameters": {
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "payload": { "type": "string" }
                },
                "required": ["task_id", "payload"]
            }
        }
    })
}
fn decode_subagent(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_subagent(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::Subagent(tc::Subagent {
        task_id: args
            .get("task_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        payload: args
            .get("payload")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        metadata: None,
    }))
}
fn history_subagent(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::Subagent(c) = tool {
        Some(("subagent".into(), json!({"task_id": c.task_id}).to_string()))
    } else {
        None
    }
}
fn render_subagent_result(
    _tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    // Subagent tools typically do not return directly in the same way, but we will return success.
    ToolResultEnvelope::success(LocalToolId::Subagent, json!({}))
}

pub const YIELD_STATE: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::YieldState,
    provider_name: "yield_state",
    execution_policy: ToolExecutionPolicy::Orchestration,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: yield_state_available,
    schema: yield_state_schema,
    decode: decode_yield_state,
    encode_warp_tool: encode_yield_state,
    encode_history: history_yield_state,
    render_result: render_yield_state_result,
};

fn yield_state_available(context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if context.orchestration_enabled {
        ToolAvailability::Available
    } else {
        ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::FeatureDisabled,
        }
    }
}

fn yield_state_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "yield_state",
            "description": "Yield state from a discrete subagent node to the multi-agent graph state machine.",
            "parameters": {
                "type": "object",
                "properties": {
                    "next_node": { "type": "string", "description": "The target node ID or transition (e.g. 'TerminalParser', 'CodeGenerator', 'Cycle', 'Terminate')" },
                    "memory_update": { "type": "object", "description": "Key-value pair memory state updates to attach to shared graph memory" }
                },
                "required": ["next_node"]
            }
        }
    })
}

fn decode_yield_state(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}

fn encode_yield_state(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    let payload = serde_json::to_string(&args)
        .map_err(|e| ToolEncodingError::ConversionFailed(e.to_string()))?;
    Ok(tc::Tool::Subagent(tc::Subagent {
        task_id: "yield_state".into(),
        payload,
        metadata: None,
    }))
}

fn history_yield_state(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::Subagent(c) = tool {
        if c.task_id == "yield_state" {
            return Some(("yield_state".into(), c.payload.clone()));
        }
    }
    None
}

fn render_yield_state_result(
    _tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    ToolResultEnvelope::success(LocalToolId::YieldState, json!({"status": "state_yielded"}))
}

#[cfg(test)]
#[path = "orchestration_tests.rs"]
mod tests;
