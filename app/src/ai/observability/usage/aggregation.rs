use serde::{Deserialize, Serialize};

use super::entry::CanonicalTokenUsage;
use crate::ai::observability::cost::aggregation::CostRollup;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageRollupScope {
    Attempt,
    LogicalRequest,
    Turn,
    Run,
    ChildAgent,
    PrimaryAgent,
    Conversation,
    Provider,
    Model,
    RouteType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRollup {
    pub scope: UsageRollupScope,
    pub token_usage: CanonicalTokenUsage,
    pub cost: CostRollup,

    pub attempts: u32,
    pub retries: u32,
    pub fallbacks: u32,

    pub authoritative_entries: u32,
    pub estimated_entries: u32,
}
