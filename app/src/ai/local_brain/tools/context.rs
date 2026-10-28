use std::collections::HashSet;
use std::path::Path;

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::MCPContext;
use crate::terminal::model::session::SessionType;

/// Derived contextual state passed to tools for availability checks and schema generation.
/// This prevents every tool definition from being coupled directly to the sprawling `RequestParams`.
pub struct ToolRuntimeContext<'a> {
    pub session_type: Option<SessionType>,
    pub working_directory: Option<&'a Path>,
    pub is_authenticated: bool,
    pub has_server_conversation: bool,
    pub active_command_ids: &'a HashSet<String>,
    pub available_document_ids: &'a HashSet<String>,
    pub available_skill_ids: &'a HashSet<String>,
    pub mcp_context: Option<&'a MCPContext>,
    pub computer_use_enabled: bool,
    pub orchestration_enabled: bool,
    /// True when the local-brain (BYO endpoint) mode is active. Tools that require a Warp backend
    /// or Warp Drive connection must gate on this field to preserve the zero-egress privacy rule.
    pub is_local_profile: bool,
    // Add other fields as necessary based on what tools actually need during migration
}

impl<'a> ToolRuntimeContext<'a> {
    pub fn new(
        params: &'a RequestParams,
        active_command_ids: &'a HashSet<String>,
        available_document_ids: &'a HashSet<String>,
        available_skill_ids: &'a HashSet<String>,
    ) -> Self {
        Self {
            session_type: params.session_context.session_type().clone(),
            working_directory: params
                .session_context
                .current_working_directory()
                .as_deref()
                .map(Path::new),
            is_authenticated: false, // TODO: Extract from auth context if needed
            has_server_conversation: params.conversation_token.is_some(),
            active_command_ids,
            available_document_ids,
            available_skill_ids,
            mcp_context: params.mcp_context.as_ref(),
            computer_use_enabled: params.computer_use_enabled,
            orchestration_enabled: params.orchestration_enabled,
            is_local_profile: params.use_local_byok_agent,
        }
    }
}
