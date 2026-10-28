use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryPipelineHealth {
    pub queue_depth: usize,
    pub queue_capacity: usize,

    pub dropped_transient_events: u64,
    pub dropped_spans: u64,

    pub export_failures: u64,
    pub last_successful_export: Option<SystemTime>,
}
