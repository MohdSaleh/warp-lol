use std::collections::BTreeMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::observability::identifiers::{
    AgentRunId, AgentTurnId, LogicalRequestId, ProviderAttemptId, UsageEntryId,
};
use crate::ai::response_experience::source::{AgentId, ProviderId, ProviderModelRef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageDataSource {
    ProviderReported,
    ProviderCountEndpoint,
    ExactLocalTokenizer,
    CalibratedEstimate,
    ConservativeEstimate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageEntryStatus {
    Provisional,
    Authoritative,
    Reconciled,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteClassification {
    Primary,
    Fallback,
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalTokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub audio_input_tokens: Option<u64>,
    pub audio_output_tokens: Option<u64>,
    pub image_input_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderUsageBreakdown {
    pub categories: BTreeMap<String, u64>,
    pub server_tool_usage: BTreeMap<String, u64>,
    pub raw_schema_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageCorrelationContext {
    pub conversation_id: AIConversationId,
    pub run_id: AgentRunId,
    pub turn_id: AgentTurnId,
    pub logical_request_id: LogicalRequestId,
    pub attempt_id: ProviderAttemptId,
    pub child_agent_id: Option<AgentId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEntry {
    pub schema_version: u32,
    pub usage_entry_id: UsageEntryId,

    pub context: UsageCorrelationContext,

    pub provider: ProviderId,
    pub model: ProviderModelRef,
    pub route: RouteClassification,

    pub source: UsageDataSource,
    pub status: UsageEntryStatus,

    pub tokens: CanonicalTokenUsage,
    pub provider_breakdown: ProviderUsageBreakdown,

    pub observed_at: SystemTime,
    pub provider_request_id: Option<String>,

    pub supersedes: Option<UsageEntryId>,
}
