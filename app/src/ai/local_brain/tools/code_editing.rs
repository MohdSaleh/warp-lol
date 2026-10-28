use serde_json::{json, Value};
use warp_multi_agent_api::message::tool_call as tc;

use super::availability::ToolAvailability;
use super::context::ToolRuntimeContext;
use super::errors::{ToolArgumentError, ToolEncodingError};
use super::registry::{
    ContextRetentionPolicy, FingerprintSpec, LocalToolDefinition, LocalToolId,
    ProviderNeutralToolCall, ToolExecutionPolicy, ToolLoopSemantics,
};
use super::results::{ToolResultEnvelope, ToolResultRenderResult};

pub const SEARCH_CODEBASE: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::SearchCodebase,
    provider_name: "search_codebase",
    execution_policy: ToolExecutionPolicy::SerialReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: search_codebase_available,
    schema: search_codebase_schema,
    decode: decode_search_codebase,
    encode_warp_tool: encode_search_codebase,
    encode_history: history_search_codebase,
    render_result: render_search_codebase_result,
};

pub const APPLY_FILE_DIFFS: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::ApplyFileDiffs,
    provider_name: "apply_file_diffs",
    execution_policy: ToolExecutionPolicy::SerialMutating,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: apply_file_diffs_available,
    schema: apply_file_diffs_schema,
    decode: decode_apply_file_diffs,
    encode_warp_tool: encode_apply_file_diffs,
    encode_history: history_apply_file_diffs,
    render_result: render_apply_file_diffs_result,
};

pub const INIT_PROJECT: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::InitProject,
    provider_name: "init_project",
    execution_policy: ToolExecutionPolicy::SerialMutating,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: init_project_available,
    schema: init_project_schema,
    decode: decode_init_project,
    encode_warp_tool: encode_init_project,
    encode_history: history_init_project,
    render_result: render_init_project_result,
};

pub const OPEN_CODE_REVIEW: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::OpenCodeReview,
    provider_name: "open_code_review",
    execution_policy: ToolExecutionPolicy::Interactive,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: open_code_review_available,
    schema: open_code_review_schema,
    decode: decode_open_code_review,
    encode_warp_tool: encode_open_code_review,
    encode_history: history_open_code_review,
    render_result: render_open_code_review_result,
};

fn search_codebase_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if _context.working_directory.is_some() {
        ToolAvailability::Available
    } else {
        ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::CodebaseNotIndexed,
        }
    }
}
fn search_codebase_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "search_codebase",
            "description": "Lexical (keyword) search across the entire codebase. Returns a list of filenames containing the exact query, ranked by match count. This is NOT a semantic search.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "path_filters": { "type": "array", "items": { "type": "string" } },
                    "codebase_path": { "type": "string" }
                },
                "required": ["query"]
            }
        }
    })
}
fn decode_search_codebase(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_search_codebase(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::SearchCodebase(tc::SearchCodebase {
        query: args
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        path_filters: args
            .get("path_filters")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        codebase_path: args
            .get("codebase_path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    }))
}
fn history_search_codebase(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::SearchCodebase(c) = tool {
        Some((
            "search_codebase".into(),
            json!({"query": c.query, "path_filters": c.path_filters}).to_string(),
        ))
    } else {
        None
    }
}
fn render_search_codebase_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::search_codebase_result::Result as Sr;
    match &tcr.result {
        Some(R::SearchCodebase(r)) => match &r.result {
            Some(Sr::Success(s)) => {
                let mut files = vec![];
                for f in &s.files {
                    files.push(json!({"file_path": f.file_path, "content": f.content}));
                }
                ToolResultEnvelope::success(LocalToolId::SearchCodebase, json!({ "files": files }))
            }
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::SearchCodebase,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::SearchCodebase,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::SearchCodebase,
            "Unexpected result type".into(),
        )),
    }
}

fn apply_file_diffs_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn apply_file_diffs_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "apply_file_diffs",
            "description": "Apply diffs to files.",
            "parameters": {
                "type": "object",
                "properties": {
                    "summary": { "type": "string" },
                    "diffs": { "type": "array", "items": { "type": "object", "properties": { "file_path": {"type": "string"}, "search": {"type": "string"}, "replace": {"type": "string"} } } },
                    "new_files": { "type": "array", "items": { "type": "object", "properties": { "file_path": {"type": "string"}, "content": {"type": "string"} } } },
                    "deleted_files": { "type": "array", "items": { "type": "object", "properties": { "file_path": {"type": "string"} } } }
                }
            }
        }
    })
}
fn decode_apply_file_diffs(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_apply_file_diffs(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    use tc::apply_file_diffs::{DeleteFile, FileDiff, NewFile};
    let diffs = args
        .get("diffs")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|v| FileDiff {
                    file_path: v
                        .get("file_path")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    search: v
                        .get("search")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    replace: v
                        .get("replace")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                })
                .collect()
        })
        .unwrap_or_default();
    let new_files = args
        .get("new_files")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|v| NewFile {
                    file_path: v
                        .get("file_path")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    content: v
                        .get("content")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                })
                .collect()
        })
        .unwrap_or_default();
    let deleted_files = args
        .get("deleted_files")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|v| DeleteFile {
                    file_path: v
                        .get("file_path")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(tc::Tool::ApplyFileDiffs(tc::ApplyFileDiffs {
        summary: args
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        diffs,
        new_files,
        deleted_files,
        v4a_updates: vec![],
    }))
}
fn history_apply_file_diffs(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::ApplyFileDiffs(c) = tool {
        Some((
            "apply_file_diffs".into(),
            json!({"summary": c.summary}).to_string(),
        ))
    } else {
        None
    }
}
fn render_apply_file_diffs_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::apply_file_diffs_result::Result as Sr;
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    match &tcr.result {
        Some(R::ApplyFileDiffs(r)) => match &r.result {
            Some(Sr::Success(s)) => {
                let mut updated = vec![];
                for f in &s.updated_files_v2 {
                    if let Some(fc) = &f.file {
                        updated.push(json!({"file_path": fc.file_path, "content": fc.content, "was_edited_by_user": f.was_edited_by_user}));
                    }
                }
                ToolResultEnvelope::success(
                    LocalToolId::ApplyFileDiffs,
                    json!({ "updated_files": updated }),
                )
            }
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::ApplyFileDiffs,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::ApplyFileDiffs,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::ApplyFileDiffs,
            "Unexpected result type".into(),
        )),
    }
}

fn init_project_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn init_project_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({ "type": "function", "function": { "name": "init_project", "description": "Initialize a new project", "parameters": { "type": "object", "properties": {} } } })
}
fn decode_init_project(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_init_project(_args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::InitProject(tc::InitProject {}))
}
fn history_init_project(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    match tool {
        tc::Tool::InitProject(_) => Some(("init_project".into(), "{}".into())),
        _ => None,
    }
}
fn render_init_project_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    match &tcr.result {
        Some(R::InitProject(_)) => ToolResultEnvelope::success(LocalToolId::InitProject, json!({})),
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::InitProject,
            "Unexpected result type".into(),
        )),
    }
}

fn open_code_review_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn open_code_review_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({ "type": "function", "function": { "name": "open_code_review", "description": "Open code review", "parameters": { "type": "object", "properties": {} } } })
}
fn decode_open_code_review(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_open_code_review(_args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::OpenCodeReview(tc::OpenCodeReview {}))
}
fn history_open_code_review(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    match tool {
        tc::Tool::OpenCodeReview(_) => Some(("open_code_review".into(), "{}".into())),
        _ => None,
    }
}
fn render_open_code_review_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    match &tcr.result {
        Some(R::OpenCodeReview(_)) => {
            ToolResultEnvelope::success(LocalToolId::OpenCodeReview, json!({}))
        }
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::OpenCodeReview,
            "Unexpected result type".into(),
        )),
    }
}

pub const INSERT_REVIEW_COMMENTS: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::InsertReviewComments,
    provider_name: "insert_review_comments",
    execution_policy: ToolExecutionPolicy::SerialMutating,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: insert_review_comments_available,
    schema: insert_review_comments_schema,
    decode: decode_insert_review_comments,
    encode_warp_tool: encode_insert_review_comments,
    encode_history: history_insert_review_comments,
    render_result: render_insert_review_comments_result,
};

fn insert_review_comments_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn insert_review_comments_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "insert_review_comments",
            "description": "Insert review comments into a file.",
            "parameters": {
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string" },
                    "base_branch": { "type": "string" },
                    "comments": { "type": "array", "items": { "type": "object", "properties": { "comment_id": {"type": "string"}, "author": {"type": "string"}, "last_modified_timestamp": {"type": "string"}, "comment_body": {"type": "string"}, "parent_comment_id": {"type": "string"} } } }
                }
            }
        }
    })
}
fn decode_insert_review_comments(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_insert_review_comments(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    use tc::insert_review_comments::Comment;
    let comments = args
        .get("comments")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|v| Comment {
                    comment_id: v
                        .get("comment_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    author: v
                        .get("author")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    last_modified_timestamp: v
                        .get("last_modified_timestamp")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    comment_body: v
                        .get("comment_body")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    parent_comment_id: v
                        .get("parent_comment_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    location: None,
                    html_url: "".into(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(tc::Tool::InsertReviewComments(tc::InsertReviewComments {
        repo_path: args
            .get("repo_path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        base_branch: args
            .get("base_branch")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        comments,
    }))
}
fn history_insert_review_comments(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::InsertReviewComments(c) = tool {
        Some((
            "insert_review_comments".into(),
            json!({"repo_path": c.repo_path}).to_string(),
        ))
    } else {
        None
    }
}
fn render_insert_review_comments_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    match &tcr.result {
        Some(R::InsertReviewComments(_)) => {
            ToolResultEnvelope::success(LocalToolId::InsertReviewComments, json!({}))
        }
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::InsertReviewComments,
            "Unexpected result type".into(),
        )),
    }
}
