pub mod agentic;
pub mod systematic;

pub use agentic::{
    diagnose_agentic_error, generate_self_reflection_prompt, is_repairing_agentic_error,
    AgenticErrorDiagnosis, AgenticFailureKind,
};
pub use systematic::{
    is_systematic_api_error, is_systematic_status_code, is_systematic_stream_error,
    rate_limit_backoff_duration, systematic_backoff_duration, MAX_CONTEXT_REPAIR_AUTO_RECALLS,
    MAX_RATE_LIMIT_AUTO_RECALLS, MAX_SYSTEMATIC_AUTO_RECALLS,
};
