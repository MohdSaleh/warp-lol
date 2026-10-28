use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use super::context::AgentObservationContext;
use super::identifiers::ObservationEventId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentObservationEvent {
    RunStarted,
    RunCompleted,

    TurnStarted,
    TurnCompleted,

    LogicalRequestStarted,
    AttemptStarted,
    AttemptCompleted,

    RetryScheduled,
    RetryStarted,
    RetrySucceeded,

    FallbackStarted,
    FallbackCompleted,

    ToolProposed,
    ToolStarted,
    ToolProgressed,
    ToolCompleted,

    ContextCompactionStarted,
    ContextCompactionCompleted,

    UsageReceived,
    UsageReconciled,
    CostCalculated,

    ErrorObserved,
    ErrorRecovered,
    ErrorTerminal,

    CancellationRequested,
    CancellationCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationEventEnvelope {
    pub event_id: ObservationEventId,
    pub context: AgentObservationContext,
    pub occurred_at: SystemTime,
    pub monotonic_offset: Duration,
    pub payload: AgentObservationEvent,
}
