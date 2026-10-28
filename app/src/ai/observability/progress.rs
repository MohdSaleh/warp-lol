use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::identifiers::ToolProgressId;
use crate::ai::response_experience::source::LocalToolCallId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizedProgressStage {
    Initializing,
    Downloading,
    Processing,
    Validating,
    Finalizing,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolProgressObservation {
    pub progress_id: ToolProgressId,
    pub tool_call_id: LocalToolCallId,

    pub completed: Option<f64>,
    pub total: Option<f64>,

    pub stage: Option<NormalizedProgressStage>,

    pub first_progress_at: Option<SystemTime>,
    pub latest_progress_at: Option<SystemTime>,
    pub update_count: u32,

    pub monotonic: bool,
}
