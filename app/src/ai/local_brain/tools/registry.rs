use serde_json::Value;

use super::availability::ToolAvailability;
use super::context::ToolRuntimeContext;
use super::errors::{ToolArgumentError, ToolEncodingError};
use super::results::ToolResultRenderResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalToolId {
    RunShellCommand,
    ReadFiles,
    Grep,
    FileGlobV2,
    CallMcpTool,
    SearchCodebase,
    ApplyFileDiffs,
    InitProject,
    OpenCodeReview,
    InsertReviewComments,
    ReadShellCommandOutput,
    WriteToLongRunningShellCommand,
    ReadMcpResource,
    ReadSkill,
    FetchConversation,
    ReadDocuments,
    CreateDocuments,
    EditDocuments,
    UploadFileArtifact,
    AskUserQuestion,
    SuggestPrompt,
    SuggestNewConversation,
    RequestComputerUse,
    UseComputer,
    RunAgents,
    SendMessageToAgent,
    WaitForEvents,
    Subagent,
    YieldState,
    WebSearch,
    WebFetch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolExecutionPolicy {
    ParallelReadOnly,
    SerialReadOnly,
    SerialMutating,
    Interactive,
    LongRunning,
    ComputerUse,
    Orchestration,
}

pub type ProviderNeutralToolCall = (String, String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLoopSemantics {
    Deterministic,
    StateDependent,
    Mutating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextRetentionPolicy {
    HighValue,
    ReplaceWithSummary,
    DropOnCompaction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FingerprintSpec {
    ExcludeAllEphemeral,
    IncludeSpecificFields(&'static [&'static str]),
    ExactMatch,
}

pub struct LocalToolDefinition {
    pub id: LocalToolId,
    pub provider_name: &'static str,
    pub execution_policy: ToolExecutionPolicy,
    pub loop_semantics: ToolLoopSemantics,
    pub context_retention: ContextRetentionPolicy,
    pub fingerprint_spec: FingerprintSpec,

    pub availability: fn(&ToolRuntimeContext<'_>) -> ToolAvailability,
    pub schema: fn(&ToolRuntimeContext<'_>) -> Value,
    pub decode: fn(&str, Value) -> Result<Value, ToolArgumentError>,
    pub encode_warp_tool:
        fn(Value) -> Result<warp_multi_agent_api::message::tool_call::Tool, ToolEncodingError>,
    pub encode_history:
        fn(&warp_multi_agent_api::message::tool_call::Tool) -> Option<ProviderNeutralToolCall>,
    pub render_result: fn(&warp_multi_agent_api::message::ToolCallResult) -> ToolResultRenderResult,
}

pub static BUILTIN_TOOL_REGISTRY: &[LocalToolDefinition] = &[
    super::shell::RUN_SHELL_COMMAND,
    super::files::READ_FILES,
    super::files::FILE_GLOB,
    super::search::GREP,
    super::mcp::CALL_MCP_TOOL_ADAPTER,
    super::code_editing::SEARCH_CODEBASE,
    super::code_editing::APPLY_FILE_DIFFS,
    super::code_editing::INIT_PROJECT,
    super::code_editing::OPEN_CODE_REVIEW,
    super::code_editing::INSERT_REVIEW_COMMENTS,
    super::terminal::READ_SHELL_COMMAND_OUTPUT,
    super::terminal::WRITE_TO_LONG_RUNNING_SHELL_COMMAND,
    super::workspace::READ_MCP_RESOURCE,
    super::workspace::READ_SKILL,
    super::workspace::FETCH_CONVERSATION,
    super::documents::READ_DOCUMENTS,
    super::documents::CREATE_DOCUMENTS,
    super::documents::EDIT_DOCUMENTS,
    super::documents::UPLOAD_FILE_ARTIFACT,
    super::interaction::ASK_USER_QUESTION,
    super::interaction::SUGGEST_PROMPT,
    super::interaction::SUGGEST_NEW_CONVERSATION,
    super::computer::REQUEST_COMPUTER_USE,
    super::computer::USE_COMPUTER,
    super::orchestration::RUN_AGENTS,
    super::orchestration::SEND_MESSAGE_TO_AGENT,
    super::orchestration::WAIT_FOR_EVENTS,
    super::orchestration::SUBAGENT,
    super::orchestration::YIELD_STATE,
    super::web_search::WEB_SEARCH,
    super::web_fetch::WEB_FETCH,
];

pub struct LocalToolRegistry;

impl LocalToolRegistry {
    pub fn find_by_provider_name(name: &str) -> Option<&'static LocalToolDefinition> {
        if name.starts_with("mcp__") {
            return BUILTIN_TOOL_REGISTRY
                .iter()
                .find(|def| def.id == LocalToolId::CallMcpTool);
        }
        BUILTIN_TOOL_REGISTRY
            .iter()
            .find(|def| def.provider_name == name)
    }

    pub fn find_by_id(id: LocalToolId) -> Option<&'static LocalToolDefinition> {
        BUILTIN_TOOL_REGISTRY.iter().find(|def| def.id == id)
    }
}
