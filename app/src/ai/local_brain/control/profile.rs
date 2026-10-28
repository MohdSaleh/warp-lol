use std::time::Duration;

pub const MAX_MODEL_TURNS_PER_ROOT_REQUEST: u32 = 6;
pub const MAX_TOTAL_TOOL_CALLS_PER_ROOT_REQUEST: u32 = 12;
pub const MAX_TOOL_CALLS_PER_TURN: usize = 4;
pub const MAX_EXACT_DUPLICATE_TOOL_CALLS: u32 = 3;

#[derive(Debug, Clone)]
pub struct LoopControlProfile {
    pub hard_model_turn_limit: u32,
    pub hard_tool_call_limit: u32,
    pub hard_tool_batch_limit: u32,
    pub max_tool_calls_per_turn: usize,
    pub max_exact_duplicate_calls: u32,
    pub max_deterministic_failure_streak: u32,
    pub hard_no_progress_batch_limit: u32,
    pub max_provider_continuations: u32,
    pub max_completion_repair_turns: u32,
    pub max_compactions: u32,
    /// Text-only turns granted after the working budget is exhausted so a run
    /// ends with a summary instead of an abrupt stop. These do not count
    /// against the model-turn limit.
    pub wrap_up_model_turns: u32,
    pub max_active_duration: Duration,
    pub max_cumulative_input_tokens: Option<u64>,
    pub max_cumulative_output_tokens: Option<u64>,
    pub unbounded_agentic_repair: bool,
}

impl LoopControlProfile {
    pub fn contract() -> Self {
        Self {
            hard_model_turn_limit: MAX_MODEL_TURNS_PER_ROOT_REQUEST,
            hard_tool_call_limit: MAX_TOTAL_TOOL_CALLS_PER_ROOT_REQUEST,
            hard_tool_batch_limit: MAX_TOTAL_TOOL_CALLS_PER_ROOT_REQUEST,
            max_tool_calls_per_turn: MAX_TOOL_CALLS_PER_TURN,
            max_exact_duplicate_calls: MAX_EXACT_DUPLICATE_TOOL_CALLS,
            max_deterministic_failure_streak: MAX_EXACT_DUPLICATE_TOOL_CALLS,
            hard_no_progress_batch_limit: 6,
            max_provider_continuations: 5,
            max_completion_repair_turns: 1,
            max_compactions: 5,
            wrap_up_model_turns: 1,
            max_active_duration: Duration::from_secs(60 * 10),
            max_cumulative_input_tokens: None,
            max_cumulative_output_tokens: None,
            unbounded_agentic_repair: false,
        }
    }

    pub fn contract_unbounded() -> Self {
        Self::contract().with_unbounded_repair(true)
    }

    pub fn simple() -> Self {
        Self::contract()
    }

    pub fn with_unbounded_repair(mut self, enabled: bool) -> Self {
        self.unbounded_agentic_repair = enabled;
        self
    }
}
