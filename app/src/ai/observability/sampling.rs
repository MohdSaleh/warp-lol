use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentTraceSamplingPolicy {
    pub normal_success_rate: f64,
    pub recovered_error_rate: f64,
    pub cancellation_rate: f64,

    pub always_sample_terminal_failures: bool,
    pub slow_trace_threshold: Duration,
}
