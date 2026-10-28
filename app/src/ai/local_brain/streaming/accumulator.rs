use std::collections::BTreeMap;

use super::config::StreamLimitConfig;
use super::error::LocalStreamError;
use super::protocol::{CanonicalStreamEvent, ProviderTerminalReason, StreamUsage};

#[derive(Debug, Clone)]
pub struct ToolCallAccumulator {
    pub id: Option<String>,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct StreamAccumulator {
    pub reasoning_message_id: Option<String>,
    pub output_message_id: Option<String>,

    pub reasoning_buffer: String,
    pub output_buffer: String,

    pub reasoning_started: bool,
    pub reasoning_finalized: bool,
    pub output_started: bool,

    pub tool_calls: BTreeMap<usize, ToolCallAccumulator>,

    pub usage: Option<StreamUsage>,
    pub terminal_reason: Option<ProviderTerminalReason>,
    pub saw_done_marker: bool,

    limits: StreamLimitConfig,
    total_streamed_bytes: usize,
}

impl StreamAccumulator {
    pub fn new(limits: StreamLimitConfig) -> Self {
        Self {
            reasoning_message_id: None,
            output_message_id: None,
            reasoning_buffer: String::new(),
            output_buffer: String::new(),
            reasoning_started: false,
            reasoning_finalized: false,
            output_started: false,
            tool_calls: BTreeMap::new(),
            usage: None,
            terminal_reason: None,
            saw_done_marker: false,
            limits,
            total_streamed_bytes: 0,
        }
    }

    pub fn apply(&mut self, event: &CanonicalStreamEvent) -> Result<(), LocalStreamError> {
        match event {
            CanonicalStreamEvent::StreamOpened => {}
            CanonicalStreamEvent::ReasoningSummaryDelta(delta) => {
                self.reasoning_started = true;
                if self.reasoning_buffer.len() + delta.len() > self.limits.max_reasoning_bytes {
                    return Err(LocalStreamError::ReasoningLimitExceeded);
                }
                self.total_streamed_bytes += delta.len();
                if self.total_streamed_bytes > self.limits.max_total_streamed_bytes {
                    return Err(LocalStreamError::OutputLimitExceeded);
                }
                self.reasoning_buffer.push_str(delta);
            }
            CanonicalStreamEvent::OutputTextDelta(delta) => {
                if self.reasoning_started {
                    self.reasoning_finalized = true;
                }
                self.output_started = true;
                if self.output_buffer.len() + delta.len() > self.limits.max_output_bytes {
                    return Err(LocalStreamError::OutputLimitExceeded);
                }
                self.total_streamed_bytes += delta.len();
                if self.total_streamed_bytes > self.limits.max_total_streamed_bytes {
                    return Err(LocalStreamError::OutputLimitExceeded);
                }
                self.output_buffer.push_str(delta);
            }
            CanonicalStreamEvent::ToolCallStarted {
                index,
                id,
                name_fragment,
            } => {
                if !self.tool_calls.contains_key(index) {
                    if self.tool_calls.len() >= self.limits.max_tool_calls {
                        return Err(LocalStreamError::ToolCallLimitExceeded);
                    }
                    self.tool_calls.insert(
                        *index,
                        ToolCallAccumulator {
                            id: None,
                            name: String::new(),
                            arguments: String::new(),
                        },
                    );
                }
                let entry = self.tool_calls.get_mut(index).unwrap();

                if let Some(new_id) = id {
                    if let Some(existing_id) = &entry.id {
                        if existing_id != new_id {
                            return Err(LocalStreamError::ConflictingToolCallId);
                        }
                    } else {
                        entry.id = Some(new_id.clone());
                    }
                }

                if let Some(name) = name_fragment {
                    entry.name.push_str(name);
                }
            }
            CanonicalStreamEvent::ToolArgumentsDelta { index, fragment } => {
                if let Some(entry) = self.tool_calls.get_mut(index) {
                    if entry.arguments.len() + fragment.len() > self.limits.max_tool_argument_bytes
                    {
                        return Err(LocalStreamError::ToolArgumentsTooLarge);
                    }
                    entry.arguments.push_str(fragment);
                }
            }
            CanonicalStreamEvent::Usage(usage) => {
                self.usage = Some(usage.clone());
            }
            CanonicalStreamEvent::Terminal(reason) => {
                self.terminal_reason = Some(reason.clone());
            }
            CanonicalStreamEvent::DoneMarker => {
                self.saw_done_marker = true;
            }
        }
        Ok(())
    }
}
