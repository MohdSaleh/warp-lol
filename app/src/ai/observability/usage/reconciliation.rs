use serde::{Deserialize, Serialize};

use crate::ai::observability::identifiers::UsageEntryId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageReconciliation {
    pub provisional_entry_id: UsageEntryId,
    pub authoritative_entry_id: UsageEntryId,

    pub input_delta: Option<i64>,
    pub output_delta: Option<i64>,
    pub total_delta: Option<i64>,

    pub relative_error: Option<f64>,
}
