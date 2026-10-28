use super::error::LocalStreamError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalStreamEvent {
    StreamOpened,
    ReasoningSummaryDelta(String),
    OutputTextDelta(String),
    ToolCallStarted {
        index: usize,
        id: Option<String>,
        name_fragment: Option<String>,
    },
    ToolArgumentsDelta {
        index: usize,
        fragment: String,
    },
    Usage(StreamUsage),
    Terminal(ProviderTerminalReason),
    DoneMarker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderTerminalReason {
    Stop,
    ToolCalls,
    LengthLimit,
    ContentFilter,
    Refusal,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub enum RawSseEvent {
    Open,
    Message(String),
}

pub trait StreamProtocolDecoder {
    fn decode(&mut self, event: RawSseEvent)
        -> Result<Vec<CanonicalStreamEvent>, LocalStreamError>;
}
