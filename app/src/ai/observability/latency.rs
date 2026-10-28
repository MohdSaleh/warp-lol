use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentLatencyBreakdown {
    pub queue_wait: Option<Duration>,
    pub grounding: Option<Duration>,
    pub context_planning: Option<Duration>,
    pub token_counting: Option<Duration>,

    pub credential_wait: Option<Duration>,
    pub retry_backoff: Duration,
    pub proxy_queue: Option<Duration>,

    pub provider_connect: Option<Duration>,
    pub provider_headers: Option<Duration>,
    pub time_to_first_chunk: Option<Duration>,
    pub time_to_first_text: Option<Duration>,
    pub provider_generation: Option<Duration>,

    pub stream_decode: Option<Duration>,
    pub response_projection: Option<Duration>,

    pub approval_wait: Duration,
    pub tool_execution: Duration,
    pub child_agent_wait: Duration,
    pub context_compaction: Duration,

    pub active_agent_time: Duration,
    pub wall_clock_time: Duration,
}
