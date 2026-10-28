//! Conversation-history reconstruction (P1).
//!
//! Warp keeps the full conversation as `api::Message`s inside
//! `params.tasks[*].messages`, accumulating across turns (the client appends
//! tool-call results and re-requests). To make the local brain a real
//! multi-turn agent — rather than the single-shot responder it replaces — we
//! rebuild the OpenAI chat `messages` array from that history on every request:
//!
//!   * `UserQuery`        → `user`
//!   * `AgentOutput`      → `assistant` (text)
//!   * `ToolCall`         → `assistant` with a `tool_calls` entry
//!   * `ToolCallResult`   → `tool` (observation fed back to the model)
//!
//! Agent reasoning and other bookkeeping messages are skipped — they aren't
//! part of the model-visible transcript.
//!
//! Each persisted `ToolCall` maps to its own single-call `assistant` message
//! immediately followed by its `tool` result. Structured todo updates are
//! reconstructed as an answered provider-only tool call so the model retains
//! its plan without leaving a dangling call.

use std::collections::HashSet;

use warp_multi_agent_api as api;

use super::control::ledger::Ledger;
use super::{tools, ChatFunctionCall, ChatMessage, ChatToolCall};
use crate::ai::agent::api::RequestParams;
use crate::ai::agent::AIAgentInput;

/// Rebuilds the full OpenAI `messages` array (system prompt first, then the
/// reconstructed transcript, then the just-submitted user input if it isn't
/// already the trailing message).
pub(super) struct BuiltHistory {
    pub messages: Vec<ChatMessage>,
    pub ledger: Ledger,
    pub compacted: bool,
    pub active_repair_diagnosis: Option<super::repair::AgenticErrorDiagnosis>,
}

pub(super) fn build_messages(params: &RequestParams, system_prompt: &str) -> BuiltHistory {
    let current_user_text = latest_user_text(params);
    let ledger = Ledger::for_request(&params.tasks, current_user_text.as_deref());
    let mut messages = vec![ChatMessage::system(system_prompt)];
    // Track which tool calls already have a result in the transcript, so a
    // fresh result carried in `params.input` isn't added twice.
    let mut answered_tool_calls: HashSet<String> = HashSet::new();

    for task in &params.tasks {
        for msg in &task.messages {
            match &msg.message {
                Some(api::message::Message::UserQuery(uq)) => {
                    push_user(&mut messages, uq.query.clone());
                }
                Some(api::message::Message::AgentOutput(out)) => {
                    if !out.text.trim().is_empty() {
                        messages.push(ChatMessage::assistant_text(out.text.clone()));
                    }
                }
                Some(api::message::Message::ToolCall(call)) => {
                    let (name, arguments) = tools::api_tool_to_openai(&call.tool);
                    let id = non_empty_or_uuid(&call.tool_call_id);
                    messages.push(ChatMessage::assistant_tool_call(ChatToolCall {
                        id,
                        kind: "function".to_string(),
                        function: ChatFunctionCall { name, arguments },
                    }));
                }
                Some(api::message::Message::ToolCallResult(result)) => {
                    let content = tools::render_tool_result(result);
                    let id = non_empty_or_uuid(&result.tool_call_id);
                    answered_tool_calls.insert(id.clone());
                    messages.push(ChatMessage::tool_result(id, content));
                }
                Some(api::message::Message::UpdateTodos(update)) => {
                    if let Some((name, arguments)) = tools::todos::encode_history(update) {
                        let id = non_empty_or_uuid(&msg.id);
                        messages.push(ChatMessage::assistant_tool_call(ChatToolCall {
                            id: id.clone(),
                            kind: "function".to_string(),
                            function: ChatFunctionCall { name, arguments },
                        }));
                        answered_tool_calls.insert(id.clone());
                        messages.push(ChatMessage::tool_result(
                            id,
                            tools::todos::TODO_UPDATE_RESULT.to_string(),
                        ));
                    }
                }
                // System queries, reasoning, model-used, etc. are not part of
                // the model-visible transcript.
                _ => {}
            }
        }
    }

    // On a follow-up after tool execution, the just-produced results arrive in
    // `params.input` as `ActionResult`s (keyed by the tool_call_id). Fold in any
    // that aren't already represented as a `ToolCallResult` in task history, so
    // no assistant tool call is left dangling/unanswered for the model.
    for input in &params.input {
        if let AIAgentInput::ActionResult { result, .. } = input {
            let id = result.id.to_string();
            if answered_tool_calls.insert(id.clone()) {
                let content = match &result.result {
                    ai::agent::action_result::AIAgentActionResultType::AskUserQuestion(result) => {
                        match tools::interaction::render_internal_ask_user_question_result(result) {
                            Ok(mut envelope) => tools::serialize_tool_result(&mut envelope),
                            Err(error) => format!("Failed to serialize selected answer: {error}"),
                        }
                    }
                    _ => result.to_string(),
                };
                messages.push(ChatMessage::tool_result(id, content));
            }
        }
    }

    // The freshly submitted user turn arrives in `params.input`; on tool-result
    // follow-ups there is no new user text and the model simply continues.
    if let Some(query) = current_user_text {
        let already_last = matches!(
            messages.last(),
            Some(ChatMessage { role, content: Some(text), .. })
                if role == "user" && text == &query
        );
        if !already_last {
            push_user(&mut messages, query);
        }
    }

    // Agentic Error Auto-Repair & Dynamic Self-Reflection Integration:
    // If the latest tool execution resulted in a failure (exit code != 0, syntax error,
    // missing file, unparseable arguments, etc.), inject a dynamic self-reflection directive
    // so the model immediately reflects on the root cause and repairs it on the fly.
    let active_repair_diagnosis = super::repair::diagnose_agentic_error(&params.tasks, &params.input);
    if let Some(diagnosis) = &active_repair_diagnosis {
        let reflection = super::repair::generate_self_reflection_prompt(diagnosis);
        push_user(&mut messages, reflection);
    }

    let mut estimated_tokens =
        super::context::token_counter::estimate_tokens_for_messages(&messages);
    let budget = params
        .context_window_limit
        .map(|limit| limit as u64)
        .unwrap_or(super::context::DEFAULT_TOKEN_BUDGET);

    let mut compacted = false;
    if estimated_tokens > budget
        && super::context::compaction::compact_context(&mut messages, budget)
    {
        estimated_tokens = super::context::token_counter::estimate_tokens_for_messages(&messages);
        compacted = true;
    }

    tracing::debug!(
        "Built history of {} messages, estimated {} tokens",
        messages.len(),
        estimated_tokens
    );

    let has_user_message = messages.iter().any(|m| m.role == "user");
    let last_message_role = messages.last().map(|m| m.role.as_str()).unwrap_or("none");
    let last_message_char_count = messages
        .last()
        .and_then(|m| m.content.as_ref())
        .map(|c| c.len())
        .unwrap_or(0);
    log::info!(
        "[PROMPT_PIPELINE] history::build_messages output: message_count={}, has_user_message={}, last_message_role={}, last_message_char_count={}, has_active_repair={}",
        messages.len(),
        has_user_message,
        last_message_role,
        last_message_char_count,
        active_repair_diagnosis.is_some()
    );

    BuiltHistory {
        messages,
        ledger,
        compacted,
        active_repair_diagnosis,
    }
}

/// The text of the most recent user query in this request's `input`, if any.
pub(super) fn latest_user_text(params: &RequestParams) -> Option<String> {
    params.input.iter().rev().find_map(model_visible_input_text)
}

fn model_visible_input_text(input: &AIAgentInput) -> Option<String> {
    match input {
        AIAgentInput::UserQuery { query, .. } => Some(query.clone()),
        AIAgentInput::InitProjectRules { display_query, .. } => {
            let display_query = display_query.as_deref().unwrap_or_default().trim();
            let detail = if display_query.is_empty() || display_query.eq_ignore_ascii_case("/init")
            {
                String::new()
            } else {
                format!(" The user supplied this additional request: {display_query}")
            };
            Some(format!(
                "Initialize this project now. Inspect the repository instructions and available project context, then report the relevant setup and be ready to work on the codebase.{detail} Do not ask the user to restate the task."
            ))
        }
        _ => None,
    }
}

fn push_user(messages: &mut Vec<ChatMessage>, query: String) {
    if query.trim().is_empty() {
        log::warn!("[PROMPT_PIPELINE] push_user dropped empty query");
        return;
    }
    messages.push(ChatMessage::user(query));
}

fn non_empty_or_uuid(id: &str) -> String {
    if id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        id.to_string()
    }
}
