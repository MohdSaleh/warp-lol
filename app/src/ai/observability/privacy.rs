use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportDecision {
    Allowed,
    DeniedByUser,
    DeniedByPolicy,
    DeniedByDefault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentifierPolicy {
    Keep,
    Redact,
    Hash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTelemetryPrivacyPolicy {
    pub export_enabled: bool,
    pub content_export_enabled: bool,

    pub metadata_export: ExportDecision,
    pub crash_diagnostics_export: ExportDecision,
    pub ai_content_export: ExportDecision,

    pub allow_prompt_content: bool,
    pub allow_response_content: bool,
    pub allow_tool_arguments: bool,
    pub allow_tool_results: bool,
    pub allow_source_excerpts: bool,

    pub identifier_policy: IdentifierPolicy,

    pub enterprise_zero_retention: bool,
    pub secret_redaction_required: bool,

    pub policy_generation: u64,
}
