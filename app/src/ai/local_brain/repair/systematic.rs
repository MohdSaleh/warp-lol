//! Systematic failure classification & auto-recall coordination.
//!
//! Systematic errors arise from infrastructure, network transports, provider outages,
//! or HTTP-level issues (e.g. 429 rate limits, 5xx server errors, timeouts, connection drops)
//! rather than agentic tool reasoning or parameter errors.
//!
//! Per system policy, systematic errors are automatically recalled up to 3 times with progressive
//! backoff before surfacing to the caller or ending the stream.

use std::time::Duration;

use crate::ai::local_brain::streaming::error::LocalStreamError;
use crate::server::server_api::AIApiError;

/// Maximum number of automatic recall attempts for systematic model failures.
pub const MAX_SYSTEMATIC_AUTO_RECALLS: usize = 3;

/// Maximum number of automatic recall attempts for rate limit failures.
pub const MAX_RATE_LIMIT_AUTO_RECALLS: usize = 5;

/// Maximum number of automatic context repairs when the provider reports context length exceeded.
pub const MAX_CONTEXT_REPAIR_AUTO_RECALLS: usize = 3;

/// Classifies whether a [`LocalStreamError`] is a systematic model/transport error
/// that qualifies for an automatic recall.
pub fn is_systematic_stream_error(err: &LocalStreamError) -> bool {
    match err {
        LocalStreamError::ConnectionFailed
        | LocalStreamError::FirstEventTimeout
        | LocalStreamError::IdleTimeout
        | LocalStreamError::TotalTimeout
        | LocalStreamError::UnexpectedEof
        | LocalStreamError::MalformedFrame(_)
        | LocalStreamError::InvalidContentType
        | LocalStreamError::RateLimitExceeded { .. } => true,
        LocalStreamError::InvalidStatusCode(code) => is_systematic_status_code(*code),
        LocalStreamError::ContextLimitExceeded(_) => false,
        // Agentic workflow errors (schema, validation, limits) are handled by the agentic repair loop
        LocalStreamError::InvalidToolArguments(_)
        | LocalStreamError::ConflictingToolCallId
        | LocalStreamError::ToolCallLimitExceeded
        | LocalStreamError::ToolArgumentsTooLarge
        | LocalStreamError::OutputLimitExceeded
        | LocalStreamError::ReasoningLimitExceeded => false,
    }
}

/// Checks whether an HTTP status code represents a transient / systematic failure.
pub fn is_systematic_status_code(status_code: u16) -> bool {
    status_code == 429
        || status_code == 408
        || status_code == 500
        || status_code == 502
        || status_code == 503
        || status_code == 504
}

/// Computes backoff duration for a given recall attempt (1-indexed).
pub fn systematic_backoff_duration(attempt: usize) -> Duration {
    match attempt {
        0 | 1 => Duration::from_millis(500),
        2 => Duration::from_millis(1200),
        _ => Duration::from_millis(2500),
    }
}

/// Computes backoff duration for rate-limit retries (1-indexed).
pub fn rate_limit_backoff_duration(attempt: usize) -> Duration {
    match attempt {
        0 | 1 => Duration::from_millis(1000),
        2 => Duration::from_millis(2000),
        3 => Duration::from_millis(4000),
        4 => Duration::from_millis(8000),
        _ => Duration::from_millis(12000),
    }
}

/// Checks whether a high-level [`AIApiError`] represents a systematic failure.
pub fn is_systematic_api_error(err: &AIApiError) -> bool {
    match err {
        AIApiError::Transport(_)
        | AIApiError::UnexpectedEof
        | AIApiError::ServerOverloaded => true,
        AIApiError::ErrorStatus(status, _) => is_systematic_status_code(status.as_u16()),
        AIApiError::Other(e) => {
            let msg = e.to_string().to_lowercase();
            msg.contains("timeout")
                || msg.contains("connection refused")
                || msg.contains("connection reset")
                || msg.contains("rate limit")
                || msg.contains("429")
                || msg.contains("500")
                || msg.contains("502")
                || msg.contains("503")
                || msg.contains("504")
                || msg.contains("stream ended")
                || msg.contains("unexpected eof")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systematic_error_classification() {
        assert!(is_systematic_stream_error(&LocalStreamError::ConnectionFailed));
        assert!(is_systematic_stream_error(&LocalStreamError::FirstEventTimeout));
        assert!(is_systematic_stream_error(&LocalStreamError::IdleTimeout));
        assert!(is_systematic_stream_error(&LocalStreamError::UnexpectedEof));
        assert!(is_systematic_stream_error(&LocalStreamError::InvalidStatusCode(429)));
        assert!(is_systematic_stream_error(&LocalStreamError::InvalidStatusCode(503)));
        assert!(is_systematic_stream_error(&LocalStreamError::RateLimitExceeded {
            status: 429,
            retry_after_secs: Some(5),
        }));

        // Context limit errors are handled by the context repair loop, not the systematic stream loop
        assert!(!is_systematic_stream_error(&LocalStreamError::ContextLimitExceeded(
            "context_length_exceeded".into()
        )));

        // Agentic errors must NOT be classified as systematic
        assert!(!is_systematic_stream_error(&LocalStreamError::InvalidToolArguments("bad json".into())));
        assert!(!is_systematic_stream_error(&LocalStreamError::ToolCallLimitExceeded));
        assert!(!is_systematic_stream_error(&LocalStreamError::InvalidStatusCode(400)));
        assert!(!is_systematic_stream_error(&LocalStreamError::InvalidStatusCode(404)));
    }

    #[test]
    fn test_systematic_backoff_progression() {
        assert_eq!(systematic_backoff_duration(1), Duration::from_millis(500));
        assert_eq!(systematic_backoff_duration(2), Duration::from_millis(1200));
        assert_eq!(systematic_backoff_duration(3), Duration::from_millis(2500));
    }

    #[test]
    fn test_rate_limit_backoff_progression() {
        assert_eq!(rate_limit_backoff_duration(1), Duration::from_millis(1000));
        assert_eq!(rate_limit_backoff_duration(2), Duration::from_millis(2000));
        assert_eq!(rate_limit_backoff_duration(3), Duration::from_millis(4000));
        assert_eq!(rate_limit_backoff_duration(4), Duration::from_millis(8000));
        assert_eq!(rate_limit_backoff_duration(5), Duration::from_millis(12000));
    }
}
