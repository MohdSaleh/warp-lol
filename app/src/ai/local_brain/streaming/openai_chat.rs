use serde::Deserialize;

use super::error::LocalStreamError;
use super::protocol::{
    CanonicalStreamEvent, ProviderTerminalReason, RawSseEvent, StreamProtocolDecoder, StreamUsage,
};
use super::think_tag_parser::ThinkTagParser;

#[derive(Debug, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<StreamUsageData>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: StreamDelta,
    #[serde(default)]
    finish_reason: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Default)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    think: Option<String>,
    #[serde(default)]
    thinking: Option<String>,
    #[serde(default)]
    thought: Option<String>,
    #[serde(default)]
    reasoning_text: Option<String>,
    #[serde(default)]
    thinking_process: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<StreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct StreamToolCall {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<StreamFunction>,
}

#[derive(Debug, Deserialize, Default)]
struct StreamFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamUsageData {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

#[derive(Default)]
pub struct OpenAiChatCompletionsDecoder {
    think_parser: ThinkTagParser,
}

impl StreamProtocolDecoder for OpenAiChatCompletionsDecoder {
    fn decode(
        &mut self,
        event: RawSseEvent,
    ) -> Result<Vec<CanonicalStreamEvent>, LocalStreamError> {
        match event {
            RawSseEvent::Open => Ok(vec![CanonicalStreamEvent::StreamOpened]),
            RawSseEvent::Message(data) => {
                let data = data.trim();
                if data == "[DONE]" {
                    let mut events = self.think_parser.flush();
                    events.push(CanonicalStreamEvent::DoneMarker);
                    return Ok(events);
                }

                let chunk: StreamChunk = serde_json::from_str(data)
                    .map_err(|e| LocalStreamError::MalformedFrame(e.to_string()))?;

                let mut events = Vec::new();

                for choice in chunk.choices {
                    let delta = choice.delta;

                    let reasoning = delta
                        .reasoning
                        .filter(|r| !r.is_empty())
                        .or_else(|| delta.reasoning_content.filter(|r| !r.is_empty()))
                        .or_else(|| delta.think.filter(|r| !r.is_empty()))
                        .or_else(|| delta.thinking.filter(|r| !r.is_empty()))
                        .or_else(|| delta.thought.filter(|r| !r.is_empty()))
                        .or_else(|| delta.reasoning_text.filter(|r| !r.is_empty()))
                        .or_else(|| delta.thinking_process.filter(|r| !r.is_empty()));

                    if let Some(reasoning) = reasoning {
                        events.push(CanonicalStreamEvent::ReasoningSummaryDelta(reasoning));
                    }

                    if let Some(content) = delta.content.filter(|c| !c.is_empty()) {
                        events.extend(self.think_parser.process(&content));
                    }

                    if let Some(tool_calls) = delta.tool_calls {
                        for call in tool_calls {
                            if call.id.is_some()
                                || call.function.as_ref().is_some_and(|f| f.name.is_some())
                            {
                                events.push(CanonicalStreamEvent::ToolCallStarted {
                                    index: call.index,
                                    id: call.id,
                                    name_fragment: call
                                        .function
                                        .as_ref()
                                        .and_then(|f| f.name.clone()),
                                });
                            }

                            if let Some(args) = call.function.and_then(|f| f.arguments) {
                                if !args.is_empty() {
                                    events.push(CanonicalStreamEvent::ToolArgumentsDelta {
                                        index: call.index,
                                        fragment: args,
                                    });
                                }
                            }
                        }
                    }

                    if let Some(finish_reason_val) = choice.finish_reason {
                        let reason_str = match &finish_reason_val {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            _ => String::new(),
                        };
                        if !reason_str.is_empty() {
                            let reason = match reason_str.as_str() {
                                "stop" => ProviderTerminalReason::Stop,
                                "tool_calls" => ProviderTerminalReason::ToolCalls,
                                "length" => ProviderTerminalReason::LengthLimit,
                                "content_filter" => ProviderTerminalReason::ContentFilter,
                                "refusal" => ProviderTerminalReason::Refusal,
                                other => ProviderTerminalReason::Other(other.to_string()),
                            };
                            events.push(CanonicalStreamEvent::Terminal(reason));
                        }
                    }
                }

                if let Some(usage) = chunk.usage {
                    events.push(CanonicalStreamEvent::Usage(StreamUsage {
                        prompt_tokens: usage.prompt_tokens,
                        completion_tokens: usage.completion_tokens,
                        total_tokens: usage.total_tokens,
                    }));
                }

                Ok(events)
            }
        }
    }
}
