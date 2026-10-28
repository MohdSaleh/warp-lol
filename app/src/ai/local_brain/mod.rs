//! The local agent "brain": a real, multi-turn agentic loop that runs entirely
//! on the user's machine against a BYO OpenAI-compatible endpoint, replacing
//! Warp's closed backend for the request → response-event contract.
//!
//! # How the loop works
//!
//! This module produces one *turn* per request. It does NOT run an in-process
//! `while` loop; instead it rides Warp's existing tool-execution + follow-up
//! machinery, exactly like the real backend:
//!
//! 1. Rebuild the OpenAI chat transcript from conversation history
//!    (`history::build_messages`).
//! 2. Call the model with native tool/function calling, streamed over SSE.
//! 3. As tokens arrive, forward reasoning and answer deltas incrementally via
//!    `AppendToMessageContent` so the terminal renders them live.
//! 4. If the model requested a tool, emit a single `ToolCall` message and finish
//!    the stream. The Warp client executes the tool, appends a `ToolCallResult`
//!    to the task, and issues a follow-up request — which re-enters this module
//!    with the result now in history, continuing the loop.
//! 5. If the model returned a final answer, the streamed `AgentOutput` is
//!    finished and the turn ends.
//!
//! State lives in the conversation/task history that the client round-trips, so
//! the brain itself is stateless per request. This is the same seam Warp's
//! server uses, which is why the terminal UI renders the local brain's output
//! identically.
//!
//! Phase coverage: P0 native tool-calling, P1 history reconstruction, P2 core
//! tools + MCP, P3 live token streaming, P4 plan-then-act (prompt-driven),
//! P5 environment-aware prompt.

mod context;
mod control;
pub mod custom_agents;
pub mod grounding;
pub mod harness;
mod history;
pub mod orchestrator;
mod prompt;
pub mod repair;
mod streaming;
pub(crate) mod tools;

use std::sync::Arc;

use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use warp_multi_agent_api as api;

use crate::ai::agent::api::{ConvertToAPITypeError, RequestParams, ResponseStream};
use crate::server::server_api::AIApiError;
use harness::events::HarnessStopReason;
use harness::executor::{HarnessToolCall, ToolContext, ToolOutcome, ToolOutcomeStatus};
use harness::phase::HarnessPhase;
use harness::{AgentHarness, HarnessConfig, TurnDirective};

/// Appended as the final user message when the working budget is exhausted, so
/// the model's last turn is a text-only summary instead of a dead-end stop.
const WRAP_UP_INSTRUCTION: &str = "The tool and model-turn budget for this request is exhausted. Do not call any tools. Reply with a concise summary of what you completed, what remains unfinished, and the recommended next step for the user.";

fn log_harness_event(event: &harness::events::HarnessEvent) {
    log::debug!("[HARNESS] {event:?}");
}

/// Observation content fed back to the model for a harness-executed call.
fn render_harness_outcome(outcome: &ToolOutcome) -> String {
    match outcome.status {
        ToolOutcomeStatus::Success => outcome.content.clone(),
        ToolOutcomeStatus::Error => format!("Error: {}", outcome.content),
        ToolOutcomeStatus::Rejected => format!("Rejected: {}", outcome.content),
    }
}

/// The custom-endpoint-backed model the current selection resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalModel {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    /// The provider-facing model slug sent on the wire.
    pub(crate) slug: String,
    /// The stable `config_key` used for token-usage reporting.
    pub(crate) config_key: String,
}

// ── OpenAI Chat Completions wire types ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    fn system(text: &str) -> Self {
        Self {
            role: "system".into(),
            content: Some(text.to_string()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    fn user(text: String) -> Self {
        Self {
            role: "user".into(),
            content: Some(text),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    fn assistant_text(text: String) -> Self {
        Self {
            role: "assistant".into(),
            content: Some(text),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    fn assistant_tool_call(call: ChatToolCall) -> Self {
        Self {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(vec![call]),
            tool_call_id: None,
        }
    }
    fn tool_result(tool_call_id: String, content: String) -> Self {
        Self {
            role: "tool".into(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ChatToolCall {
    pub id: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    pub function: ChatFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ChatFunctionCall {
    pub name: String,
    /// JSON-encoded arguments, per the OpenAI tool-calling contract.
    pub arguments: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'static str>,
    stream: bool,
}

// ── Public entry point (called by `local_byok::generate_output`) ────────────

/// Runs one agent turn as a *live* response-event stream: the model is streamed
/// over SSE and its reasoning / answer tokens are forwarded incrementally as
/// `AppendToMessageContent` events, so the terminal renders them as they arrive
/// (rather than all at once when the turn completes).
pub(crate) async fn generate_output(
    params: RequestParams,
    cancellation_rx: futures::channel::oneshot::Receiver<()>,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    let streaming_config = streaming::config::LocalStreamingConfig::production();

    let stream = async_stream::stream! {
        // ── Per-turn setup ─────────────────────────────────────────────
        let mut params = params;
        let routed_agent = match history::latest_user_text(&params) {
            Some(user_text) => orchestrator::SwarmOrchestrator::intercept_task(
                &user_text,
                Some(params.model.as_str()),
            )
            .await
            .ok()
            .flatten(),
            None => None,
        };
        let selected_model = routed_agent
            .as_ref()
            .and_then(|(agent, _)| agent.base_model.as_deref())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| params.model.as_str());
        let model = match select_model_by_id(&params, selected_model) {
            Ok(model) => model,
            Err(error) => {
                yield Err(Arc::new(AIApiError::Other(error)));
                return;
            }
        };

        let ambient_id = params.ambient_agent_task_id.as_ref().map(ToString::to_string);
        let graph_engine = history::latest_user_text(&params).and_then(|user_text| {
            orchestrator::SwarmOrchestrator::prepare_graph_engine(
                &user_text,
                ambient_id.as_deref(),
            )
        });

        let mut system_prompt = prompt::system_prompt(&params);

        if let Some(engine) = &graph_engine {
            system_prompt.push_str(&engine.get_prompt_extension());
        }

        if let Some((agent, rewritten_query)) = routed_agent {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(&format!("You are acting as the custom agent '{}'.\n\nAgent Instructions:\n{}", agent.name, agent.instructions.as_deref().unwrap_or("")));

            if !agent.skills.is_empty() {
                system_prompt.push_str("\n\nAssigned Skills (available via `read_skill` tool):\n");
                for skill in &agent.skills {
                    system_prompt.push_str(&format!("- {}\n", skill));
                }
            }

            if let Some(env) = &agent.environment_id {
                system_prompt.push_str(&format!("\n\nAssigned Environment ID: {}\n", env));
            }

            for input in params.input.iter_mut().rev() {
                if let crate::ai::agent::AIAgentInput::UserQuery { query, .. } = input {
                    *query = rewritten_query.clone();
                    break;
                }
            }
        }

        let history::BuiltHistory { messages, mut ledger, compacted, active_repair_diagnosis } =
            history::build_messages(&params, &system_prompt);
        let is_actively_repairing = active_repair_diagnosis.is_some();
        let tool_schemas = tools::tool_schemas(&params);

        let conversation_id = params
            .conversation_token
            .as_ref()
            .map(|token| token.as_str().to_string())
            .unwrap_or_else(streaming::warp_events::new_uuid);
        let run_id = params
            .ambient_agent_task_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();

        if let Some(db_url) = crate::persistence::database_file_path_for_scope(
            &crate::persistence::PersistenceScope::App,
        ).to_str() {
            if let Ok(mut conn) = crate::persistence::establish_rw_connection(db_url) {
                if let Ok(Some(saved_record)) = control::ledger::load_runtime_record(&mut conn, &conversation_id, &run_id) {
                    if let Ok(deserialized_ledger) = serde_json::from_str::<control::ledger::Ledger>(&saved_record.ledger_json) {
                        ledger.request_epoch = deserialized_ledger.request_epoch + 1;
                        ledger.accumulated_active_duration = deserialized_ledger.accumulated_active_duration;
                    }
                }
            }
        }
        let request_id = streaming::warp_events::new_uuid();
        let has_existing_task = !params.tasks.is_empty();
        let task_id = params
            .tasks
            .first()
            .map(|task| task.id.clone())
            .filter(|id| !id.is_empty())
            .unwrap_or_else(streaming::warp_events::new_uuid);

        log::info!(
            "[PROMPT_PIPELINE] generate_output started: request_id={}, has_user_text={}, input_count={}, task_count={}, provider_request_started=true",
            request_id,
            history::latest_user_text(&params).is_some(),
            params.input.len(),
            params.tasks.len()
        );

        yield Ok(streaming::warp_events::init_event(conversation_id.clone(), request_id.clone(), run_id.clone()));
        if !has_existing_task {
            let description = history::latest_user_text(&params).unwrap_or_default();
            yield Ok(streaming::warp_events::create_task_event(task_id.clone(), description));
        }

        if compacted {
            yield Ok(streaming::warp_events::add_messages_event(
                task_id.clone(),
                vec![streaming::warp_events::output_message(
                    &task_id,
                    &request_id,
                    "Context was compacted to fit the model window\n",
                )]
            ));
        }

        let loop_profile = control::profile::LoopControlProfile::contract();
        let mut harness = AgentHarness::new(
            HarnessConfig {
                profile: loop_profile.clone(),
            },
            ledger,
        );
        if let Ok(event) = harness.enter(HarnessPhase::ContextAssembly) {
            log_harness_event(&event);
        }

        // In-process execution is only safe when the session works on a local
        // directory: remote sessions must execute filesystem tools on the host,
        // which only the client can reach.
        let harness_working_directory: Option<std::path::PathBuf> = params
            .session_context
            .current_working_directory()
            .as_deref()
            .filter(|directory| !directory.trim().is_empty())
            .filter(|_| !params.session_context.is_remote())
            .map(std::path::PathBuf::from);
        let harness_tools = harness_working_directory
            .as_ref()
            .map(|_| harness::executors::filesystem_registry());

        let mut wrap_up = match harness.begin_turn_with_repair(is_actively_repairing) {
            TurnDirective::Proceed => None,
            TurnDirective::WrapUp(stop) => Some(stop),
            TurnDirective::Stop(stop) => {
                yield Ok(streaming::warp_events::add_messages_event(
                    task_id.clone(),
                    vec![streaming::warp_events::output_message(
                        &task_id,
                        &request_id,
                        &stop.user_message(),
                    )],
                ));
                yield Ok(streaming::warp_events::finished_event(
                    model.config_key.clone(),
                    0,
                    0,
                    0,
                ));
                let stopped = harness.finish(HarnessStopReason::from(stop));
                log_harness_event(&stopped);
                persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                return;
            }
        };
        if let Some(stop) = wrap_up.as_ref() {
            log::info!("[HARNESS] budget exhausted ({stop:?}); requesting one text-only wrap-up turn");
        }

        // ── Open the provider SSE stream with systematic auto-recall (up to 3 times) ──
        let url = match chat_completions_url(&model.base_url) {
            Ok(url) => url,
            Err(error) => {
                yield Err(Arc::new(AIApiError::Other(error)));
                return;
            }
        };
        let mut messages = messages;
        if wrap_up.is_some() {
            messages.push(ChatMessage::user(WRAP_UP_INSTRUCTION.to_string()));
        }
        let mut systematic_attempt = 0usize;
        let mut rate_limit_attempt = 0usize;
        let mut context_repair_attempt = 0usize;

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let token_clone = cancellation_token.clone();
        tokio::spawn(async move {
            let _ = cancellation_rx.await;
            token_clone.cancel();
        });

        if let Ok(event) = harness.enter(HarnessPhase::ModelTurn) {
            log_harness_event(&event);
        }

        // Set after an in-process tool batch: the next loop iteration must
        // re-check the budget before issuing another provider turn.
        let mut check_budget_before_turn = false;

        // Rich-event sequences are unique per (stream_id, sequence) in sqlite;
        // retries and follow-up turns reuse the request id, so numbering must
        // continue where the previous attempt stopped.
        let mut rich_sequence: u64 = 0;

        loop {
            if cancellation_token.is_cancelled() {
                let stopped = harness.finish(HarnessStopReason::Cancelled);
                log_harness_event(&stopped);
                persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                return;
            }

            if check_budget_before_turn {
                check_budget_before_turn = false;
                match harness.begin_turn_with_repair(is_actively_repairing) {
                    TurnDirective::Proceed => {}
                    TurnDirective::WrapUp(stop) => {
                        log::info!(
                            "[HARNESS] budget exhausted mid-run ({stop:?}); requesting one text-only wrap-up turn"
                        );
                        messages.push(ChatMessage::user(WRAP_UP_INSTRUCTION.to_string()));
                        wrap_up = Some(stop);
                    }
                    TurnDirective::Stop(stop) => {
                        yield Ok(streaming::warp_events::add_messages_event(
                            task_id.clone(),
                            vec![streaming::warp_events::output_message(
                                &task_id,
                                &request_id,
                                &stop.user_message(),
                            )],
                        ));
                        yield Ok(streaming::warp_events::finished_event(
                            model.config_key.clone(),
                            0,
                            0,
                            0,
                        ));
                        let stopped = harness.finish(HarnessStopReason::from(stop));
                        log_harness_event(&stopped);
                        persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                        return;
                    }
                }
                if let Ok(event) = harness.enter(HarnessPhase::ModelTurn) {
                    log_harness_event(&event);
                }
            }

            let has_tools = !tool_schemas.is_empty() && wrap_up.is_none();
            let body = ChatRequest {
                model: &model.slug,
                messages: &messages,
                tools: if has_tools {
                    tool_schemas.clone()
                } else {
                    Vec::new()
                },
                tool_choice: has_tools.then_some("auto"),
                stream: true,
            };

            let client = http_client::Client::new();
            let req = client
                .post(&url)
                .bearer_auth(model.api_key.trim())
                .header("Accept", "text/event-stream")
                .json(&body);

            let sse = req.eventsource();

            let decoder = Box::new(streaming::openai_chat::OpenAiChatCompletionsDecoder::default());

            let driver = streaming::driver::LocalStreamDriver::new(
                sse,
                decoder,
                streaming_config.timeouts.clone(),
            );

            let ai_conversation_id = crate::ai::agent::conversation::AIConversationId::new();
            let mut emitter = streaming::emitter::WarpStreamEmitter::new(
                streaming_config.emission.clone(),
                streaming_config.limits.clone(),
                task_id.clone(),
                request_id.clone(),
                ai_conversation_id,
                model.config_key.clone(),
                loop_profile.max_tool_calls_per_turn,
                rich_sequence,
            );

            let (tx, mut rx) = tokio::sync::mpsc::channel(streaming_config.emission.canonical_channel_capacity);
            let attempt_token = cancellation_token.clone();

            // Spawn driver in background
            let driver_task = tokio::spawn(async move {
                driver.run(tx, attempt_token).await
            });

            let mut stream_failure = None;

            while let Some(event) = rx.recv().await {
                match emitter.apply_event(event) {
                    Ok((warp_events, rich_events)) => {
                        persist_rich_events(&rich_events);
                        for warp_event in warp_events {
                            yield Ok(warp_event);
                        }
                    }
                    Err(e) => {
                        stream_failure = Some(e);
                        break;
                    }
                }
            }

            let exit_reason = match stream_failure {
                Some(e) => streaming::driver::StreamExit::Failed(e),
                None => driver_task.await.unwrap_or(streaming::driver::StreamExit::Cancelled),
            };

            match exit_reason {
                streaming::driver::StreamExit::Cancelled => {
                    let stopped = harness.finish(HarnessStopReason::Cancelled);
                    log_harness_event(&stopped);
                    persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                    return;
                }
                streaming::driver::StreamExit::Failed(ref e) => {
                    // 1. Context limit exceeded auto-repair loop:
                    // Even if self-reflection is unavailable, detect context limit rejection,
                    // automatically compact the context, and re-issue the request in the loop.
                    if let streaming::error::LocalStreamError::ContextLimitExceeded(ref reason) = e {
                        if !emitter.has_started_emitting()
                            && context_repair_attempt < repair::MAX_CONTEXT_REPAIR_AUTO_RECALLS
                        {
                            context_repair_attempt += 1;
                            let current_tokens = context::token_counter::estimate_tokens_for_messages(&messages);
                            let new_budget = (current_tokens * 2 / 3).max(1000);
                            log::warn!(
                                "[CONTEXT_REPAIR_LOOP] Model context limit exceeded ({reason}). Auto-repairing context (attempt {}/{}) by compacting to budget {new_budget}...",
                                context_repair_attempt,
                                repair::MAX_CONTEXT_REPAIR_AUTO_RECALLS
                            );
                            let _ = context::compaction::compact_context(&mut messages, new_budget);
                            rich_sequence = emitter.sequence();
                            continue;
                        }
                    }

                    // 2. Rate limit auto-recall loop:
                    // Back off with retry-after header or progressive exponential backoff and continue loop without stopping.
                    if let streaming::error::LocalStreamError::RateLimitExceeded { retry_after_secs, status } = e {
                        if !emitter.has_started_emitting()
                            && rate_limit_attempt < repair::MAX_RATE_LIMIT_AUTO_RECALLS
                        {
                            rate_limit_attempt += 1;
                            let backoff = retry_after_secs
                                .map(std::time::Duration::from_secs)
                                .unwrap_or_else(|| repair::rate_limit_backoff_duration(rate_limit_attempt));
                            log::warn!(
                                "[RATE_LIMIT_REPAIR_LOOP] Rate limit encountered (HTTP {status}). Auto-backing off for {backoff:?} (attempt {}/{}) and continuing loop without stopping...",
                                rate_limit_attempt,
                                repair::MAX_RATE_LIMIT_AUTO_RECALLS
                            );
                            tokio::select! {
                                _ = cancellation_token.cancelled() => {
                                    let stopped = harness.finish(HarnessStopReason::Cancelled);
                                    log_harness_event(&stopped);
                                    persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                                    return;
                                }
                                _ = tokio::time::sleep(backoff) => {}
                            }
                            rich_sequence = emitter.sequence();
                            continue;
                        }
                    }

                    // 3. Systematic transport/server error recall loop (up to 3 attempts with backoff)
                    if repair::is_systematic_stream_error(e)
                        && !emitter.has_started_emitting()
                        && systematic_attempt < repair::MAX_SYSTEMATIC_AUTO_RECALLS
                    {
                        systematic_attempt += 1;
                        let backoff = repair::systematic_backoff_duration(systematic_attempt);
                        log::warn!(
                            "[SYSTEMATIC_REPAIR_LOOP] Systematic model error ({e:?}); auto-recalling attempt {}/{} after {backoff:?} backoff...",
                            systematic_attempt,
                            repair::MAX_SYSTEMATIC_AUTO_RECALLS
                        );
                        tokio::select! {
                            _ = cancellation_token.cancelled() => {
                                let stopped = harness.finish(HarnessStopReason::Cancelled);
                                log_harness_event(&stopped);
                                persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                                return;
                            }
                            _ = tokio::time::sleep(backoff) => {}
                        }
                        continue;
                    }

                    let stopped = harness.finish(HarnessStopReason::Failed {
                        message: e.to_string(),
                    });
                    log_harness_event(&stopped);
                    persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                    yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!("stream failed: {e}"))));
                    return;
                }
                streaming::driver::StreamExit::Complete => {
                    // Split this turn's calls: harness-owned calls run here,
                    // everything else keeps the client execution path.
                    let mut turn_calls = streaming::emitter::TurnToolCalls::default();
                    if wrap_up.is_none() && emitter.has_tool_calls() {
                        turn_calls = match harness_tools.as_ref() {
                            Some(registry) => emitter.take_in_process_tool_calls(|name| {
                                registry.contains(name)
                                    || name == tools::todos::UPDATE_TODOS_NAME
                            }),
                            None => streaming::emitter::TurnToolCalls {
                                in_process: Vec::new(),
                                client_count: emitter.tool_call_count(),
                                dropped: 0,
                            },
                        };
                    }

                    if wrap_up.is_some() {
                        // The final summary turn must not hand work back to the
                        // client loop; any requested calls are dropped.
                        emitter.suppress_tool_calls();
                        turn_calls = streaming::emitter::TurnToolCalls::default();
                    }

                    let requested_tool_calls =
                        turn_calls.client_count + turn_calls.in_process.len() as u32;
                    if requested_tool_calls > 0 {
                        harness.record_model_turn(requested_tool_calls);
                        harness.mark_active();
                        if let Ok(event) = harness.enter(HarnessPhase::ToolBatch) {
                            log_harness_event(&event);
                        }
                    }

                    if turn_calls.dropped > 0 {
                        yield Ok(streaming::warp_events::add_messages_event(
                            task_id.clone(),
                            vec![streaming::warp_events::output_message(
                                &task_id,
                                &request_id,
                                &format!(
                                    "Local brain dropped {} extra tool call(s).",
                                    turn_calls.dropped
                                ),
                            )],
                        ));
                    }

                    if let Some(stop) = wrap_up.as_ref() {
                        if !emitter.has_output_text() {
                            yield Ok(streaming::warp_events::add_messages_event(
                                task_id.clone(),
                                vec![streaming::warp_events::output_message(
                                    &task_id,
                                    &request_id,
                                    &stop.user_message(),
                                )],
                            ));
                        }
                    }

                    // The request keeps running in-process only when every
                    // call in this turn is harness-owned; otherwise the stream
                    // ends and the client executes the rest, then re-enters.
                    let continues_in_process =
                        !turn_calls.in_process.is_empty() && turn_calls.client_count == 0;
                    let turn_request_id = if continues_in_process {
                        streaming::warp_events::new_uuid()
                    } else {
                        request_id.clone()
                    };

                    if !turn_calls.in_process.is_empty() {
                        let registry = harness_tools
                            .as_ref()
                            .expect("in-process calls require a tool registry");
                        let assistant_text = emitter.output_text();

                        let mut call_ids = Vec::with_capacity(turn_calls.in_process.len());
                        let mut call_arguments = Vec::with_capacity(turn_calls.in_process.len());
                        for call in &turn_calls.in_process {
                            call_ids
                                .push(call.id.clone().unwrap_or_else(streaming::warp_events::new_uuid));
                            let arguments =
                                serde_json::from_str::<serde_json::Value>(&call.arguments)
                                    .unwrap_or_else(|error| {
                                        log::warn!(
                                            "LocalBrain: harness tool '{}' had invalid json args: {error}; falling back to empty object",
                                            call.name
                                        );
                                        serde_json::json!({})
                                    });
                            call_arguments.push(arguments);
                        }

                        // Todo updates change UI state rather than running a
                        // tool; everything else is executed by the registry.
                        let mut batch_calls = Vec::new();
                        let mut batch_positions = Vec::new();
                        for (position, call) in turn_calls.in_process.iter().enumerate() {
                            if call.name == tools::todos::UPDATE_TODOS_NAME {
                                continue;
                            }
                            batch_calls.push(HarnessToolCall {
                                call_id: call_ids[position].clone(),
                                name: call.name.clone(),
                                arguments: call_arguments[position].clone(),
                            });
                            batch_positions.push(position);
                        }

                        let tool_context = ToolContext {
                            working_directory: harness_working_directory.clone(),
                            turn: harness.ledger().model_turns,
                        };
                        let batch_outcomes = registry
                            .execute_batch(
                                &batch_calls,
                                &tool_context,
                                loop_profile.max_tool_calls_per_turn,
                            )
                            .await;
                        let mut outcomes: Vec<Option<ToolOutcome>> =
                            (0..turn_calls.in_process.len()).map(|_| None).collect();
                        for (outcome, position) in batch_outcomes.into_iter().zip(batch_positions)
                        {
                            outcomes[position] = Some(outcome);
                        }

                        let mut chat_tool_calls = Vec::with_capacity(turn_calls.in_process.len());
                        let mut tool_messages = Vec::with_capacity(turn_calls.in_process.len());
                        let mut replay_messages = Vec::new();
                        let mut progress_outcomes = Vec::new();
                        let mut ledger_calls = Vec::new();

                        for (position, call) in turn_calls.in_process.iter().enumerate() {
                            let call_id = call_ids[position].clone();
                            let arguments = call_arguments[position].clone();

                            let outcome = if call.name == tools::todos::UPDATE_TODOS_NAME {
                                match tools::todos::decode_update(arguments.clone()) {
                                    Ok(update) => {
                                        replay_messages.push(
                                            streaming::warp_events::update_todos_message(
                                                &task_id,
                                                &turn_request_id,
                                                update,
                                            ),
                                        );
                                        ToolOutcome::success(tools::todos::TODO_UPDATE_RESULT)
                                    }
                                    Err(error) => ToolOutcome::error(error.to_string()),
                                }
                            } else {
                                outcomes[position].take().unwrap_or_else(|| {
                                    ToolOutcome::error(format!(
                                        "tool was not executed: {}",
                                        call.name
                                    ))
                                })
                            };
                            let content = render_harness_outcome(&outcome);

                            chat_tool_calls.push(ChatToolCall {
                                id: call_id.clone(),
                                kind: "function".to_string(),
                                function: ChatFunctionCall {
                                    name: call.name.clone(),
                                    arguments: arguments.to_string(),
                                },
                            });
                            tool_messages.push(ChatMessage::tool_result(
                                call_id.clone(),
                                content.clone(),
                            ));

                            if call.name != tools::todos::UPDATE_TODOS_NAME {
                                let call_message = streaming::warp_events::api_message_with_id(
                                    &streaming::warp_events::new_uuid(),
                                    &task_id,
                                    &turn_request_id,
                                    api::message::Message::ToolCall(api::message::ToolCall {
                                        tool_call_id: call_id.clone(),
                                        tool: Some(api::message::tool_call::Tool::Server(
                                            api::message::tool_call::Server {
                                                payload: tools::server_tool::encode_call(
                                                    &call.name,
                                                    &arguments,
                                                ),
                                            },
                                        )),
                                    }),
                                );
                                if let Some(api::message::Message::ToolCall(tool_call)) =
                                    call_message.message.as_ref()
                                {
                                    ledger_calls.push((
                                        call_id.clone(),
                                        control::fingerprint::fingerprint_tool_call(tool_call),
                                    ));
                                }
                                replay_messages.push(call_message);
                                replay_messages.push(
                                    streaming::warp_events::api_message_with_id(
                                        &streaming::warp_events::new_uuid(),
                                        &task_id,
                                        &turn_request_id,
                                        api::message::Message::ToolCallResult(
                                            api::message::ToolCallResult {
                                                tool_call_id: call_id,
                                                context: None,
                                                result: Some(
                                                    api::message::tool_call_result::Result::Server(
                                                        api::message::tool_call_result::ServerResult {
                                                            serialized_result: content,
                                                        },
                                                    ),
                                                ),
                                            },
                                        ),
                                    ),
                                );
                                progress_outcomes.push(outcome);
                            }
                        }

                        messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: assistant_text,
                            tool_calls: (!chat_tool_calls.is_empty()).then_some(chat_tool_calls),
                            tool_call_id: None,
                        });
                        messages.extend(tool_messages);

                        if !replay_messages.is_empty() {
                            yield Ok(streaming::warp_events::add_messages_event(
                                task_id.clone(),
                                replay_messages,
                            ));
                        }

                        if !progress_outcomes.is_empty() {
                            harness.record_tool_batch(&progress_outcomes);
                        }
                        harness.record_tool_call_fingerprints(ledger_calls);
                        if let Ok(event) = harness.enter(HarnessPhase::Observation) {
                            log_harness_event(&event);
                        }
                    }

                    match emitter.finalize_turn(!continues_in_process) {
                        Ok((warp_events, rich_events)) => {
                            persist_rich_events(&rich_events);
                            if let Some(usage) = emitter.canonical_usage() {
                                persist_usage(
                                    &usage,
                                    &conversation_id.to_string(),
                                    &turn_request_id,
                                    &model.config_key,
                                );
                            }
                            let mut finished = false;
                            for warp_event in warp_events {
                                finished |= matches!(
                                    &warp_event.r#type,
                                    Some(warp_multi_agent_api::response_event::Type::Finished(_))
                                );
                                yield Ok(warp_event);
                            }
                            if wrap_up.is_some() && !finished {
                                // The wrap-up turn ends the run, so close the
                                // stream even if the provider only sent [DONE].
                                yield Ok(streaming::warp_events::finished_event(
                                    model.config_key.clone(),
                                    0,
                                    0,
                                    0,
                                ));
                            }
                        }
                        Err(e) => {
                            let stopped = harness.finish(HarnessStopReason::Failed {
                                message: e.to_string(),
                            });
                            log_harness_event(&stopped);
                            persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                            yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!("finalize error: {e}"))));
                            return;
                        }
                    }

                    if let Err(e) = emitter.completion_outcome() {
                        let stopped = harness.finish(HarnessStopReason::Failed {
                            message: e.to_string(),
                        });
                        log_harness_event(&stopped);
                        persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                        let err = match e {
                            streaming::error::LocalStreamError::UnexpectedEof => AIApiError::UnexpectedEof,
                            _ => AIApiError::Other(anyhow::anyhow!("stream incomplete: {e}")),
                        };
                        yield Err(Arc::new(err));
                        return;
                    }

                    if continues_in_process {
                        persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                        rich_sequence = emitter.sequence();
                        check_budget_before_turn = true;
                        continue;
                    }

                    // Tool calls were emitted: the client executes them and
                    // re-enters for the next turn, so the run stays Active.
                    if turn_calls.client_count > 0 {
                        persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                        break;
                    }

                    let reason = match wrap_up.as_ref() {
                        Some(stop) => HarnessStopReason::from(stop.clone()),
                        None => HarnessStopReason::Completed,
                    };
                    let stopped = harness.finish(reason);
                    log_harness_event(&stopped);
                    persist_ledger_state(harness.ledger(), &conversation_id, &run_id);
                    break;
                }
            }
        }
    };

    Ok(Box::pin(stream))
}

fn persist_ledger_state(ledger: &control::ledger::Ledger, conversation_id: &str, run_id: &str) {
    let db_path = crate::persistence::database_file_path_for_scope(
        &crate::persistence::PersistenceScope::App,
    );
    let Some(db_url) = db_path.to_str() else {
        return;
    };
    let Ok(mut conn) = crate::persistence::establish_rw_connection(db_url) else {
        return;
    };
    // Runtime records reference agent_conversations, and local-brain runs do
    // not always persist a conversation row (e.g. when session restore is
    // unavailable). Skip instead of tripping the foreign key every request.
    if !agent_conversation_exists(&mut conn, conversation_id) {
        return;
    }
    let record = ledger.to_runtime_record(conversation_id, run_id);
    if let Err(error) = control::ledger::save_runtime_record(&mut conn, &record) {
        log::error!("Failed to persist local ledger runtime record: {error:?}");
    }
}

fn agent_conversation_exists(
    conn: &mut diesel::SqliteConnection,
    conversation_id: &str,
) -> bool {
    use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

    use crate::persistence::schema::agent_conversations;

    agent_conversations::table
        .filter(agent_conversations::conversation_id.eq(conversation_id))
        .count()
        .get_result::<i64>(conn)
        .map(|count| count > 0)
        .unwrap_or(false)
}

fn persist_usage(
    usage: &crate::ai::observability::usage::entry::CanonicalTokenUsage,
    conversation_id: &str,
    request_id: &str,
    model_config_key: &str,
) {
    let db_path = crate::persistence::database_file_path_for_scope(
        &crate::persistence::PersistenceScope::App,
    );
    let Some(db_url) = db_path.to_str() else {
        return;
    };
    let Ok(mut conn) = crate::persistence::establish_rw_connection(db_url) else {
        return;
    };

    let now = std::time::SystemTime::now();
    let catalog = crate::ai::observability::cost::catalog::PricingCatalog {
        schema_version: 1,
        catalog_version: 0,
        generated_at: now,
        effective_at: now,
        expires_at: now + std::time::Duration::from_secs(365 * 24 * 60 * 60),
        entries: vec![],
        key_id: "local-empty".to_string(),
        signature: String::new(),
    };
    let privacy_policy = crate::ai::observability::privacy::AgentTelemetryPrivacyPolicy {
        export_enabled: false,
        content_export_enabled: false,
        metadata_export: crate::ai::observability::privacy::ExportDecision::DeniedByDefault,
        crash_diagnostics_export:
            crate::ai::observability::privacy::ExportDecision::DeniedByDefault,
        ai_content_export: crate::ai::observability::privacy::ExportDecision::DeniedByDefault,
        allow_prompt_content: false,
        allow_response_content: false,
        allow_tool_arguments: false,
        allow_tool_results: false,
        allow_source_excerpts: false,
        identifier_policy: crate::ai::observability::privacy::IdentifierPolicy::Keep,
        enterprise_zero_retention: true,
        secret_redaction_required: true,
        policy_generation: 1,
    };

    if let Err(error) = crate::ai::observability::usage::writer::UsageWriter::write_usage_and_cost(
        &mut conn,
        usage,
        "custom_endpoint",
        model_config_key,
        &catalog,
        &privacy_policy,
        conversation_id,
        request_id,
        request_id,
        request_id,
        request_id,
        "primary",
        "provider_reported",
        "authoritative",
    ) {
        log::error!("Failed to persist local usage: {error:?}");
    }
}

fn persist_rich_events(rich_events: &[crate::ai::response_experience::RichAgentEvent]) {
    if rich_events.is_empty() {
        return;
    }

    let db_path = crate::persistence::database_file_path_for_scope(
        &crate::persistence::PersistenceScope::App,
    );
    let Some(db_url) = db_path.to_str() else {
        return;
    };
    let Ok(mut conn) = crate::persistence::establish_rw_connection(db_url) else {
        return;
    };
    if let Err(error) =
        crate::ai::response_experience::persistence::LocalAgentHistoryModel::insert_rich_events(
            &mut conn,
            rich_events,
        )
    {
        log::error!("Failed to persist rich events: {error:?}");
    }
}

pub(crate) fn chat_completions_url(base_url: &str) -> anyhow::Result<String> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        bail!("configure a custom inference endpoint URL first");
    }
    if base.ends_with("/chat/completions") {
        return Ok(base.to_string());
    }
    for suffix in ["/models", "/responses"] {
        if let Some(stripped) = base.strip_suffix(suffix) {
            return Ok(format!("{stripped}/chat/completions"));
        }
    }
    Ok(format!("{base}/chat/completions"))
}

/// Resolves the currently selected coding model through the same custom
/// endpoint configuration used by normal local-brain requests.
pub(crate) fn resolve_active_coding_model(app: &warpui::AppContext) -> anyhow::Result<LocalModel> {
    use ai::api_keys::ApiKeyManager;
    use warpui::SingletonEntity;

    let selected = crate::ai::llms::LLMPreferences::as_ref(app)
        .get_active_coding_model(app, None)
        .id
        .to_string();
    let endpoints = &ApiKeyManager::as_ref(app).keys().custom_endpoints;

    endpoints
        .iter()
        .filter(|endpoint| !endpoint.url.trim().is_empty())
        .find_map(|endpoint| {
            endpoint
                .models
                .iter()
                .find(|model| model.config_key == selected)
                .map(|model| LocalModel {
                    base_url: endpoint.url.clone(),
                    api_key: endpoint.api_key.clone(),
                    slug: model.name.clone(),
                    config_key: model.config_key.clone(),
                })
        })
        .or_else(|| {
            endpoints
                .iter()
                .filter(|endpoint| !endpoint.url.trim().is_empty())
                .flat_map(|endpoint| {
                    endpoint.models.iter().map(move |model| LocalModel {
                        base_url: endpoint.url.clone(),
                        api_key: endpoint.api_key.clone(),
                        slug: model.name.clone(),
                        config_key: model.config_key.clone(),
                    })
                })
                .next()
        })
        .ok_or_else(|| {
            anyhow!("Select a custom-inference coding model before creating a local coding lab.")
        })
}

/// Resolves the active model selection to a custom-endpoint-backed model.
#[cfg(test)]
fn select_model(params: &RequestParams) -> anyhow::Result<LocalModel> {
    select_model_by_id(params, params.model.as_str())
}

fn select_model_by_id(params: &RequestParams, selected: &str) -> anyhow::Result<LocalModel> {
    let providers = params.custom_model_providers.as_ref().ok_or_else(|| {
        anyhow!("Configure a custom inference endpoint before using the local agent.")
    })?;

    providers
        .providers
        .iter()
        .filter(|provider| !provider.base_url.trim().is_empty())
        .find_map(|provider| {
            provider
                .models
                .iter()
                .find(|model| model.config_key == selected)
                .map(|model| LocalModel {
                    base_url: provider.base_url.clone(),
                    api_key: provider.api_key.clone(),
                    slug: model.slug.clone(),
                    config_key: model.config_key.clone(),
                })
        })
        .or_else(|| {
            providers
                .providers
                .iter()
                .filter(|provider| !provider.base_url.trim().is_empty())
                .flat_map(|provider| {
                    provider.models.iter().map(move |model| LocalModel {
                        base_url: provider.base_url.clone(),
                        api_key: provider.api_key.clone(),
                        slug: model.slug.clone(),
                        config_key: model.config_key.clone(),
                    })
                })
                .next()
        })
        .ok_or_else(|| {
            anyhow!(
                "Select a custom-inference model before using the local agent. The active model \
                 is not backed by a custom endpoint."
            )
        })
}

// Event / message construction moved to streaming/warp_events.rs

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;

#[cfg(test)]
#[path = "prompt_pipeline_tests.rs"]
mod prompt_pipeline_tests;
