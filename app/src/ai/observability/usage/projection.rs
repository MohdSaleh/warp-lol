use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ai::observability::cost::aggregation::CostRollup;
use crate::ai::response_experience::source::{ProviderId, ProviderModelRef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageCompleteness {
    FullyProviderReported,
    MixedReportedAndEstimated,
    EstimatedOnly,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailedModelUsage {
    pub provider: ProviderId,
    pub model: ProviderModelRef,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailedTimingRollup {
    pub time_to_first_feedback: Option<Duration>,
    pub time_to_first_text: Option<Duration>,
    pub active_agent_time: Duration,
    pub wall_clock_time: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolUsageMetadata {
    pub call_count: u32,
    pub failure_count: u32,
    pub total_execution_time: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextUsageRollup {
    pub hard_limit_tokens: u64,
    pub visible_input_tokens: u64,
    pub opaque_provider_tokens: u64,
    pub utilization: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetailedConversationUsage {
    pub models: Vec<DetailedModelUsage>,

    pub retries: u32,
    pub fallbacks: u32,

    pub provider_attempts: u32,
    pub failed_attempts: u32,

    pub usage_completeness: UsageCompleteness,

    pub timing: DetailedTimingRollup,
    pub tool_usage: ToolUsageMetadata,
    pub context: ContextUsageRollup,

    pub cost: CostRollup,
}
