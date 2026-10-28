#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalStreamError {
    ConnectionFailed,
    InvalidStatusCode(u16),
    InvalidContentType,
    FirstEventTimeout,
    IdleTimeout,
    TotalTimeout,
    MalformedFrame(String),
    InvalidToolArguments(String),
    ConflictingToolCallId,
    ToolCallLimitExceeded,
    ToolArgumentsTooLarge,
    OutputLimitExceeded,
    ReasoningLimitExceeded,
    RateLimitExceeded {
        status: u16,
        retry_after_secs: Option<u64>,
    },
    ContextLimitExceeded(String),
    UnexpectedEof,
}

impl std::fmt::Display for LocalStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed => write!(f, "connection failed"),
            Self::InvalidStatusCode(code) => write!(f, "invalid status code: {code}"),
            Self::RateLimitExceeded { status, retry_after_secs } => {
                write!(f, "rate limit exceeded (status {status}, retry-after: {retry_after_secs:?}s)")
            }
            Self::ContextLimitExceeded(reason) => {
                write!(f, "context limit exceeded: {reason}")
            }
            Self::InvalidContentType => write!(f, "invalid content type"),
            Self::FirstEventTimeout => write!(f, "first event timeout"),
            Self::IdleTimeout => write!(f, "idle timeout"),
            Self::TotalTimeout => write!(f, "total timeout"),
            Self::MalformedFrame(msg) => write!(f, "malformed frame: {msg}"),
            Self::InvalidToolArguments(msg) => write!(f, "invalid tool arguments: {msg}"),
            Self::ConflictingToolCallId => write!(f, "conflicting tool call id"),
            Self::ToolCallLimitExceeded => write!(f, "tool call limit exceeded"),
            Self::ToolArgumentsTooLarge => write!(f, "tool arguments too large"),
            Self::OutputLimitExceeded => write!(f, "output limit exceeded"),
            Self::ReasoningLimitExceeded => write!(f, "reasoning limit exceeded"),
            Self::UnexpectedEof => write!(f, "unexpected eof"),
        }
    }
}

impl std::error::Error for LocalStreamError {}
