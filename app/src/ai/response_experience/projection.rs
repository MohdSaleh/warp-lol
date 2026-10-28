use std::collections::HashMap;

use indexmap::IndexMap;

use super::event::{
    ArtifactId, CitationId, ContentEntityId, GroundingSourceId, PlanId, RetrievalId, RichEventId,
    RichEventStreamId, RichSuggestion, RichTerminalOutcome, RunId,
};
use super::source::{AgentId, LocalToolCallId};

#[derive(Debug, Clone, Default)]
pub struct RunProjection {
    pub state: Option<super::event::RunLifecycleState>,
}
#[derive(Debug, Clone, Default)]
pub struct ContentProjection {
    pub content_id: ContentEntityId,
    pub kind: Option<super::event::ContentKind>,
    pub text: String,
    pub completed: bool,
}
#[derive(Debug, Clone, Default)]
pub struct ToolProjection {
    pub tool_call_id: LocalToolCallId,
    pub tool_name: String,
    pub status: String,
}
#[derive(Debug, Clone, Default)]
pub struct PlanProjection {
    pub plan_id: PlanId,
    pub items: Vec<super::event::PlanItem>,
    pub completed: bool,
}
#[derive(Debug, Clone, Default)]
pub struct RetrievalProjection {
    pub retrieval_id: RetrievalId,
    pub status: String,
    pub query: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct SourceProjection {
    pub source_id: GroundingSourceId,
    pub name: String,
    pub path: String,
}
#[derive(Debug, Clone, Default)]
pub struct CitationProjection {
    pub citation_id: CitationId,
    pub document_type: i32,
    pub document_id: String,
}
#[derive(Debug, Clone, Default)]
pub struct ArtifactProjection {
    pub artifact_id: ArtifactId,
    pub name: String,
    pub path: Option<String>,
    pub url: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct ChildAgentProjection {
    pub agent_id: AgentId,
    pub name: String,
    pub status: String,
}
#[derive(Debug, Clone, Default)]
pub struct RoutingProjection {
    pub provider: String,
    pub model: String,
}
#[derive(Debug, Clone, Default)]
pub struct ContextProjection {
    pub utilization: f32,
}
#[derive(Debug, Clone, Default)]
pub struct UsageProjection {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
#[derive(Debug, Clone, Default)]
pub struct DiffProjection {
    pub path: String,
    pub diff: String,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct RichResponseProjection {
    pub run: Option<RunProjection>,
    pub content: IndexMap<ContentEntityId, ContentProjection>,
    pub tools: IndexMap<LocalToolCallId, ToolProjection>,
    pub plans: IndexMap<PlanId, PlanProjection>,
    pub retrievals: IndexMap<RetrievalId, RetrievalProjection>,
    pub sources: IndexMap<GroundingSourceId, SourceProjection>,
    pub citations: IndexMap<CitationId, CitationProjection>,
    pub artifacts: IndexMap<ArtifactId, ArtifactProjection>,
    pub child_agents: IndexMap<AgentId, ChildAgentProjection>,
    pub diffs: IndexMap<String, DiffProjection>,

    pub routing: Option<RoutingProjection>,
    pub context: Option<ContextProjection>,
    pub usage: Option<UsageProjection>,
    pub suggestions: Vec<RichSuggestion>,

    pub terminal: Option<RichTerminalOutcome>,
}
