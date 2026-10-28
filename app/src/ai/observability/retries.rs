use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::errors::RecoveryAction;
use super::identifiers::{LogicalRequestId, ProviderAttemptId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizedRetryReason {
    NetworkError,
    RateLimit,
    ServerBusy,
    Timeout,
    StreamInterrupted,
    InvalidResponse,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryObservation {
    pub logical_request_id: LogicalRequestId,
    pub failed_attempt_id: ProviderAttemptId,

    pub retry_index: u32,
    pub reason: NormalizedRetryReason,

    pub scheduled_delay: Duration,
    pub actual_delay: Duration,

    pub recovery_action: RecoveryAction,
    pub result_attempt_id: Option<ProviderAttemptId>,
}
