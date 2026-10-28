use std::time::{Duration, SystemTime};

use instant::Instant;
use warp_multi_agent_api as api;

use super::accumulator::{StreamAccumulator, ToolCallAccumulator};
use super::config::{StreamEmissionConfig, StreamLimitConfig};
use super::error::LocalStreamError;
use super::protocol::{CanonicalStreamEvent, ProviderTerminalReason};
use super::warp_events;
use crate::ai::agent::conversation::AIConversationId;
use crate::ai::local_brain::tools;
use crate::ai::observability::usage::entry::CanonicalTokenUsage;
use crate::ai::response_experience::durability::EventDurability;
use crate::ai::response_experience::event::{
    CanonicalToolCallSummary, ContentEvent, ContentKind, RichAgentEvent, RichEventPayload,
    RichTerminalOutcome, RunLifecycleEvent, RunLifecycleState, TerminalEvent, ToolLifecycleEvent,
    UsageEvent,
};
use crate::ai::response_experience::source::RichEventSource;
use crate::ai::response_experience::visibility::EventVisibility;

/// How one model turn's tool calls split between the harness and the client.
#[derive(Debug, Default)]
pub struct TurnToolCalls {
    /// Calls the harness executes in-process, in call order.
    pub in_process: Vec<ToolCallAccumulator>,
    /// Calls left in the accumulator for the client action path.
    pub client_count: u32,
    /// Calls beyond the per-turn cap, which are never executed.
    pub dropped: u32,
}

pub struct WarpStreamEmitter {
    config: StreamEmissionConfig,
    accumulator: StreamAccumulator,
    task_id: String,
    request_id: String,
    conversation_id: AIConversationId,
    model_config_key: String,
    max_tool_calls_per_turn: usize,
    sequence: u64,

    unemitted_reasoning_chars: usize,
    unemitted_output_chars: usize,
    last_flush_time: Option<Instant>,

    // Track message IDs created
    reasoning_msg_id: Option<String>,
    output_msg_id: Option<String>,
    reasoning_finalized: bool,
    reasoning_started_at: Option<Instant>,
}

impl WarpStreamEmitter {
    pub fn new(
        config: StreamEmissionConfig,
        limits: StreamLimitConfig,
        task_id: String,
        request_id: String,
        conversation_id: AIConversationId,
        model_config_key: String,
        max_tool_calls_per_turn: usize,
        starting_sequence: u64,
    ) -> Self {
        Self {
            config,
            accumulator: StreamAccumulator::new(limits),
            task_id,
            request_id,
            conversation_id,
            model_config_key,
            max_tool_calls_per_turn,
            sequence: starting_sequence,
            unemitted_reasoning_chars: 0,
            unemitted_output_chars: 0,
            last_flush_time: None,
            reasoning_msg_id: None,
            output_msg_id: None,
            reasoning_finalized: false,
            reasoning_started_at: None,
        }
    }

    pub fn has_tool_calls(&self) -> bool {
        !self.accumulator.tool_calls.is_empty()
    }

    /// Highest rich-event sequence number assigned so far. Later attempts or
    /// turns of the same request resume numbering after this value.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn has_started_emitting(&self) -> bool {
        self.output_msg_id.is_some() || self.reasoning_msg_id.is_some()
    }

    pub fn has_output_text(&self) -> bool {
        self.output_msg_id.is_some()
    }

    /// Number of tool calls that `finalize` will actually emit, after the
    /// per-turn cap. This is what the harness records against the run budget.
    pub fn tool_call_count(&self) -> u32 {
        self.accumulator
            .tool_calls
            .values()
            .filter(|call| !call.name.trim().is_empty())
            .take(self.max_tool_calls_per_turn)
            .count() as u32
    }

    /// Drop provider-requested tool calls without emitting them, so a final
    /// text-only turn cannot keep the client loop going.
    pub fn suppress_tool_calls(&mut self) {
        self.accumulator.tool_calls.clear();
    }

    /// The assistant text produced by this turn, if any.
    pub fn output_text(&self) -> Option<String> {
        if self.accumulator.output_buffer.is_empty() {
            None
        } else {
            Some(self.accumulator.output_buffer.clone())
        }
    }

    /// Remove this turn's harness-owned calls from the accumulator and assign
    /// any missing call IDs. The per-turn cap is applied across the whole turn,
    /// so owned and client calls together never exceed it; calls beyond the cap
    /// are reported as `dropped`. Whatever stays in the accumulator is what
    /// `finalize` hands to the client.
    pub fn take_in_process_tool_calls(
        &mut self,
        mut is_in_process: impl FnMut(&str) -> bool,
    ) -> TurnToolCalls {
        let cap = self.max_tool_calls_per_turn;
        let mut in_process = Vec::new();
        let mut remaining = std::collections::BTreeMap::new();
        let mut client_count = 0u32;
        let mut dropped = 0u32;
        let mut named_seen = 0usize;

        for (index, mut call) in std::mem::take(&mut self.accumulator.tool_calls) {
            if call.name.trim().is_empty() {
                continue;
            }
            named_seen += 1;
            if named_seen > cap {
                dropped += 1;
                continue;
            }
            if is_in_process(&call.name) {
                if call.id.is_none() {
                    call.id = Some(warp_events::new_uuid());
                }
                in_process.push(call);
            } else {
                remaining.insert(index, call);
                client_count += 1;
            }
        }

        self.accumulator.tool_calls = remaining;
        TurnToolCalls {
            in_process,
            client_count,
            dropped,
        }
    }

    pub fn apply_event(
        &mut self,
        event: CanonicalStreamEvent,
    ) -> Result<(Vec<api::ResponseEvent>, Vec<RichAgentEvent>), LocalStreamError> {
        self.accumulator.apply(&event)?;

        let mut out_events = Vec::new();
        let mut rich_events = Vec::new();
        let now = Instant::now();

        let mut force_flush = false;

        match event {
            CanonicalStreamEvent::StreamOpened => {
                rich_events.push(self.rich_event(
                    "run.lifecycle",
                    RichEventPayload::Run(RunLifecycleEvent {
                        state: RunLifecycleState::Generating,
                    }),
                    EventVisibility::UserVisibleWhenDetailed,
                ));
                force_flush = true;
            }
            CanonicalStreamEvent::ReasoningSummaryDelta(delta) => {
                if self.reasoning_msg_id.is_none() {
                    let id = warp_events::new_uuid();
                    self.reasoning_msg_id = Some(id.clone());
                    self.reasoning_started_at = Some(now);
                    out_events.push(warp_events::add_messages_event(
                        self.task_id.clone(),
                        vec![warp_events::agent_reasoning_msg(
                            &self.task_id,
                            &self.request_id,
                            &id,
                            "",
                            None,
                        )],
                    ));
                    force_flush = true;
                }
                self.unemitted_reasoning_chars += delta.chars().count();
            }
            CanonicalStreamEvent::OutputTextDelta(delta) => {
                if !self.reasoning_finalized && self.reasoning_msg_id.is_some() {
                    self.reasoning_finalized = true;
                    let finished_duration = self.reasoning_started_at.map(|t| t.elapsed());
                    out_events.push(warp_events::update_message_event(
                        self.task_id.clone(),
                        warp_events::agent_reasoning_msg(
                            &self.task_id,
                            &self.request_id,
                            self.reasoning_msg_id.as_ref().unwrap(),
                            "",
                            finished_duration,
                        ),
                        "agent_reasoning.finished_duration",
                    ));
                    force_flush = true;
                }

                if self.output_msg_id.is_none() {
                    let id = warp_events::new_uuid();
                    self.output_msg_id = Some(id.clone());
                    out_events.push(warp_events::add_messages_event(
                        self.task_id.clone(),
                        vec![warp_events::agent_output_msg(
                            &self.task_id,
                            &self.request_id,
                            &id,
                            "",
                        )],
                    ));
                    rich_events.push(self.rich_event(
                        "content.started",
                        RichEventPayload::Content(ContentEvent::Started {
                            content_id: id,
                            kind: ContentKind::Text,
                        }),
                        EventVisibility::UserVisible,
                    ));
                    force_flush = true;
                }
                self.unemitted_output_chars += delta.chars().count();
            }
            CanonicalStreamEvent::ToolCallStarted { .. }
            | CanonicalStreamEvent::ToolArgumentsDelta { .. } => {
                // We don't stream tool calls incrementally to the UI.
            }
            CanonicalStreamEvent::Usage(u) => {
                rich_events.push(self.rich_event(
                    "usage.observed",
                    RichEventPayload::Usage(UsageEvent {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                    }),
                    EventVisibility::DeveloperOnly,
                ));
            }
            CanonicalStreamEvent::Terminal(_) | CanonicalStreamEvent::DoneMarker => {
                force_flush = true;
            }
        }

        let time_since_flush = self
            .last_flush_time
            .map(|t| now.duration_since(t))
            .unwrap_or(Duration::MAX);

        let exceeds_buffer = self.unemitted_reasoning_chars >= self.config.max_buffer_chars
            || self.unemitted_output_chars >= self.config.max_buffer_chars;

        let exceeds_time = time_since_flush >= self.config.flush_interval;

        if force_flush || exceeds_buffer || exceeds_time {
            let (flush_events, flush_rich) = self.flush()?;
            out_events.extend(flush_events);
            rich_events.extend(flush_rich);
            self.last_flush_time = Some(now);
        }

        Ok((out_events, rich_events))
    }

    fn flush(
        &mut self,
    ) -> Result<(Vec<api::ResponseEvent>, Vec<RichAgentEvent>), LocalStreamError> {
        let mut out_events = Vec::new();
        let mut rich_events = Vec::new();

        if self.unemitted_reasoning_chars > 0 {
            if let Some(id) = &self.reasoning_msg_id {
                let tail = self
                    .accumulator
                    .reasoning_buffer
                    .chars()
                    .rev()
                    .take(self.unemitted_reasoning_chars)
                    .collect::<String>();
                let delta: String = tail.chars().rev().collect();

                out_events.push(warp_events::append_content_event(
                    self.task_id.clone(),
                    warp_events::agent_reasoning_msg(
                        &self.task_id,
                        &self.request_id,
                        id,
                        &delta,
                        None,
                    ),
                    "agent_reasoning.reasoning",
                ));
            }
            self.unemitted_reasoning_chars = 0;
        }

        if self.unemitted_output_chars > 0 {
            if let Some(id) = &self.output_msg_id {
                let tail = self
                    .accumulator
                    .output_buffer
                    .chars()
                    .rev()
                    .take(self.unemitted_output_chars)
                    .collect::<String>();
                let delta: String = tail.chars().rev().collect();

                out_events.push(warp_events::append_content_event(
                    self.task_id.clone(),
                    warp_events::agent_output_msg(&self.task_id, &self.request_id, id, &delta),
                    "agent_output.text",
                ));
                rich_events.push(self.rich_event(
                    "content.delta",
                    RichEventPayload::Content(ContentEvent::Delta {
                        content_id: id.clone(),
                        text: delta,
                    }),
                    EventVisibility::UserVisible,
                ));
            }
            self.unemitted_output_chars = 0;
        }

        Ok((out_events, rich_events))
    }

    fn rich_event(
        &mut self,
        event_type: impl Into<String>,
        payload: RichEventPayload,
        visibility: EventVisibility,
    ) -> RichAgentEvent {
        self.sequence += 1;
        RichAgentEvent {
            schema_version: 1,
            event_id: warp_events::new_uuid(),
            event_type: event_type.into(),
            stream_id: self.request_id.clone(),
            sequence: self.sequence,
            entity_revision: self.sequence,
            run_id: self.request_id.clone(),
            turn_id: self.request_id.clone(),
            conversation_id: self.conversation_id,
            task_id: Some(self.task_id.clone()),
            exchange_id: Some(self.request_id.clone()),
            parent_event_id: None,
            correlation_id: Some(self.model_config_key.clone()),
            occurred_at: SystemTime::now(),
            source: RichEventSource::LocalRuntime,
            visibility,
            durability: EventDurability::Durable,
            payload,
        }
    }

    pub fn finalize(
        &mut self,
    ) -> Result<(Vec<api::ResponseEvent>, Vec<RichAgentEvent>), LocalStreamError> {
        self.finalize_turn(true)
    }

    /// Close out one assistant turn. `terminal` also emits the client tool
    /// calls and the terminal/success events; a non-terminal turn only flushes
    /// text, because the request keeps running with another provider turn.
    pub fn finalize_turn(
        &mut self,
        terminal: bool,
    ) -> Result<(Vec<api::ResponseEvent>, Vec<RichAgentEvent>), LocalStreamError> {
        let (mut out_events, mut rich_events) = self.flush()?;

        if !self.reasoning_finalized && self.reasoning_msg_id.is_some() {
            self.reasoning_finalized = true;
            let finished_duration = self.reasoning_started_at.map(|t| t.elapsed());
            out_events.push(warp_events::update_message_event(
                self.task_id.clone(),
                warp_events::agent_reasoning_msg(
                    &self.task_id,
                    &self.request_id,
                    self.reasoning_msg_id.as_ref().unwrap(),
                    "",
                    finished_duration,
                ),
                "agent_reasoning.finished_duration",
            ));
        }

        if !terminal {
            if let Some(content_id) = self.output_msg_id.clone() {
                rich_events.push(self.rich_event(
                    "content.completed",
                    RichEventPayload::Content(ContentEvent::Completed { content_id }),
                    EventVisibility::UserVisible,
                ));
            }
            return Ok((out_events, rich_events));
        }

        // Handle tool calls
        if !self.accumulator.tool_calls.is_empty() {
            let mut api_messages = Vec::new();
            let mut errors = Vec::new();
            let dropped_tool_calls = self
                .accumulator
                .tool_calls
                .len()
                .saturating_sub(self.max_tool_calls_per_turn);

            let tool_calls = self
                .accumulator
                .tool_calls
                .values()
                .take(self.max_tool_calls_per_turn)
                .cloned()
                .collect::<Vec<_>>();

            for call in tool_calls {
                if call.name.trim().is_empty() {
                    continue;
                }
                let args_val: serde_json::Value = match serde_json::from_str(&call.arguments) {
                    Ok(val) => val,
                    Err(e) => {
                        log::warn!(
                            "LocalBrain: tool '{}' had invalid json args: {}, falling back to empty object",
                            call.name,
                            e
                        );
                        serde_json::json!({})
                    }
                };

                if call.name == tools::todos::UPDATE_TODOS_NAME {
                    match tools::todos::decode_update(args_val) {
                        Ok(update) => {
                            api_messages.push(warp_events::update_todos_message(
                                &self.task_id,
                                &self.request_id,
                                update,
                            ));
                        }
                        Err(error) => errors.push(format!("- {}: {}", call.name, error)),
                    }
                    continue;
                }

                match tools::openai_to_api_tool(&call.name, args_val) {
                    Ok(tool) => {
                        let call_id = call.id.clone().unwrap_or_else(warp_events::new_uuid);
                        rich_events.push(self.rich_event(
                            "tool.proposed",
                            RichEventPayload::Tool(ToolLifecycleEvent::Proposed {
                                tool_call: CanonicalToolCallSummary {},
                            }),
                            EventVisibility::UserVisibleWhenDetailed,
                        ));
                        api_messages.push(warp_events::tool_call_message(
                            &self.task_id,
                            &self.request_id,
                            call_id,
                            tool,
                        ));
                    }
                    Err(error) => {
                        errors.push(format!("- {}: {}", call.name, error));
                    }
                }
            }

            let emitted_api_messages = !api_messages.is_empty();
            if emitted_api_messages {
                out_events.push(warp_events::add_messages_event(
                    self.task_id.clone(),
                    api_messages,
                ));
            }

            if dropped_tool_calls > 0 {
                out_events.push(warp_events::add_messages_event(
                    self.task_id.clone(),
                    vec![warp_events::output_message(
                        &self.task_id,
                        &self.request_id,
                        &format!("Local brain dropped {dropped_tool_calls} extra tool call(s)."),
                    )],
                ));
            }

            if !errors.is_empty() && self.output_msg_id.is_none() {
                let err_msg = format!(
                    "I attempted to use tools I can't run here:\n{}",
                    errors.join("\n")
                );
                out_events.push(warp_events::add_messages_event(
                    self.task_id.clone(),
                    vec![warp_events::output_message(
                        &self.task_id,
                        &self.request_id,
                        &err_msg,
                    )],
                ));
            }
        }

        let terminal = self.accumulator.terminal_reason.as_ref();

        let (pt, ct, tt) = if let Some(usage) = &self.accumulator.usage {
            (
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens,
            )
        } else {
            (0, 0, 0)
        };

        match terminal {
            Some(ProviderTerminalReason::Stop) | Some(ProviderTerminalReason::ToolCalls) => {
                out_events.push(warp_events::finished_event(
                    self.model_config_key.clone(),
                    pt,
                    ct,
                    tt,
                ));
                if let Some(content_id) = self.output_msg_id.clone() {
                    rich_events.push(self.rich_event(
                        "content.completed",
                        RichEventPayload::Content(ContentEvent::Completed { content_id }),
                        EventVisibility::UserVisible,
                    ));
                }
                rich_events.push(self.rich_event(
                    "terminal.success",
                    RichEventPayload::Terminal(TerminalEvent {
                        outcome: RichTerminalOutcome::Success,
                    }),
                    EventVisibility::UserVisible,
                ));
                rich_events.push(self.rich_event(
                    "run.lifecycle",
                    RichEventPayload::Run(RunLifecycleEvent {
                        state: RunLifecycleState::Completed,
                    }),
                    EventVisibility::UserVisibleWhenDetailed,
                ));
            }
            Some(ProviderTerminalReason::LengthLimit) => {
                // Just stop naturally (as incomplete), no finished event?
                // Wait, design says:
                // `length`: preserve visible partial output -> IncompleteLengthLimit -> no automatic transport retry.
                // Does it emit StreamFinished(Done) for length limit?
                // The acceptance criteria: "Cancellation or failure emits no normal StreamFinished."
                // "length is not classified as a transport error."
                // Wait, if it is not a transport error, and we don't emit StreamFinished, the stream will naturally close without it, creating UnexpectedEof if not handled.
                // Actually, the requirements: "Successful normal completion emits exactly one StreamFinished."
                if let Some(content_id) = self.output_msg_id.clone() {
                    rich_events.push(self.rich_event(
                        "content.completed",
                        RichEventPayload::Content(ContentEvent::Completed { content_id }),
                        EventVisibility::UserVisible,
                    ));
                }
                rich_events.push(self.rich_event(
                    "terminal.partial",
                    RichEventPayload::Terminal(TerminalEvent {
                        outcome: RichTerminalOutcome::Partial {
                            completed: vec![],
                            incomplete: vec![],
                        },
                    }),
                    EventVisibility::UserVisible,
                ));
                rich_events.push(self.rich_event(
                    "run.lifecycle",
                    RichEventPayload::Run(RunLifecycleEvent {
                        state: RunLifecycleState::Partial,
                    }),
                    EventVisibility::UserVisibleWhenDetailed,
                ));
            }
            Some(ProviderTerminalReason::ContentFilter)
            | Some(ProviderTerminalReason::Refusal)
            | Some(ProviderTerminalReason::Other(_))
            | None => {
                rich_events.push(self.rich_event(
                    "run.lifecycle",
                    RichEventPayload::Run(RunLifecycleEvent {
                        state: RunLifecycleState::Failed,
                    }),
                    EventVisibility::UserVisibleWhenDetailed,
                ));
            }
        }

        Ok((out_events, rich_events))
    }

    pub fn completion_outcome(&self) -> Result<(), LocalStreamError> {
        if let Some(ProviderTerminalReason::LengthLimit) = self.accumulator.terminal_reason.as_ref()
        {
            // Is length limit a failure?
            // "length is not classified as a transport error."
            // The design doc says:
            // pub enum StreamCompletion { Complete, ToolCalls, IncompleteLengthLimit, Filtered, Refusal, Empty }
            // So it's successful completion but with a different status. Warp `api::ResponseEvent` `StreamFinished` only has `Reason::Done`.
            // Actually, let's look at `StreamCompletion` in the design doc. I need to define it or map it.
        }
        if !self.accumulator.saw_done_marker && self.accumulator.terminal_reason.is_none() {
            return Err(LocalStreamError::UnexpectedEof);
        }
        Ok(())
    }

    pub fn canonical_usage(&self) -> Option<CanonicalTokenUsage> {
        self.accumulator
            .usage
            .as_ref()
            .map(|usage| CanonicalTokenUsage {
                input_tokens: Some(u64::from(usage.prompt_tokens)),
                output_tokens: Some(u64::from(usage.completion_tokens)),
                total_tokens: Some(u64::from(usage.total_tokens)),
                ..CanonicalTokenUsage::default()
            })
    }
}
