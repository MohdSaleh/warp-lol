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

pub const READ_DOCUMENTS: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::ReadDocuments,
    provider_name: "read_documents",
    execution_policy: ToolExecutionPolicy::SerialReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: read_documents_available,
    schema: read_documents_schema,
    decode: decode_read_documents,
    encode_warp_tool: encode_read_documents,
    encode_history: history_read_documents,
    render_result: render_read_documents_result,
};

pub const CREATE_DOCUMENTS: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::CreateDocuments,
    provider_name: "create_documents",
    execution_policy: ToolExecutionPolicy::SerialMutating,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: create_documents_available,
    schema: create_documents_schema,
    decode: decode_create_documents,
    encode_warp_tool: encode_create_documents,
    encode_history: history_create_documents,
    render_result: render_create_documents_result,
};

pub const EDIT_DOCUMENTS: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::EditDocuments,
    provider_name: "edit_documents",
    execution_policy: ToolExecutionPolicy::SerialMutating,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: edit_documents_available,
    schema: edit_documents_schema,
    decode: decode_edit_documents,
    encode_warp_tool: encode_edit_documents,
    encode_history: history_edit_documents,
    render_result: render_edit_documents_result,
};

pub const UPLOAD_FILE_ARTIFACT: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::UploadFileArtifact,
    provider_name: "upload_file_artifact",
    execution_policy: ToolExecutionPolicy::SerialMutating,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: upload_file_artifact_available,
    schema: upload_file_artifact_schema,
    decode: decode_upload_file_artifact,
    encode_warp_tool: encode_upload_file_artifact,
    encode_history: history_upload_file_artifact,
    render_result: render_upload_file_artifact_result,
};

fn read_documents_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Unavailable {
        reason: super::availability::ToolUnavailableReason::FeatureDisabled,
    }
}
fn read_documents_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "read_documents",
            "description": "Read documents by ID.",
            "parameters": {
                "type": "object",
                "properties": {
                    "documents": { "type": "array", "items": { "type": "object", "properties": { "document_id": {"type": "string"} } } }
                }
            }
        }
    })
}
fn decode_read_documents(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_read_documents(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    use tc::read_documents::Document;
    let documents = args
        .get("documents")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|v| Document {
                    document_id: v
                        .get("document_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    line_ranges: vec![],
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(tc::Tool::ReadDocuments(tc::ReadDocuments { documents }))
}
fn history_read_documents(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::ReadDocuments(c) = tool {
        Some((
            "read_documents".into(),
            json!({"documents": c.documents.len()}).to_string(),
        ))
    } else {
        None
    }
}
fn render_read_documents_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::read_documents_result::Result as Sr;
    match &tcr.result {
        Some(R::ReadDocuments(r)) => match &r.result {
            Some(Sr::Success(s)) => {
                let mut docs = vec![];
                for d in &s.documents {
                    let line_range = d
                        .line_range
                        .as_ref()
                        .map(|r| json!({"start_line": r.start, "end_line": r.end}));
                    docs.push(json!({ "document_id": d.document_id, "content": d.content, "line_range": line_range }));
                }
                ToolResultEnvelope::success(
                    LocalToolId::ReadDocuments,
                    json!({ "documents": docs }),
                )
            }
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadDocuments,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadDocuments,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::ReadDocuments,
            "Unexpected result type".into(),
        )),
    }
}

fn create_documents_available(context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if context.is_local_profile {
        // create_documents syncs through Warp Drive; forbidden in local-only mode.
        return ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::LocalOnlyViolation,
        };
    }
    ToolAvailability::Unavailable {
        reason: super::availability::ToolUnavailableReason::FeatureDisabled,
    }
}
fn create_documents_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "create_documents",
            "description": "Create new documents.",
            "parameters": {
                "type": "object",
                "properties": {
                    "new_documents": { "type": "array", "items": { "type": "object", "properties": { "title": {"type": "string"}, "content": {"type": "string"} } } }
                }
            }
        }
    })
}
fn decode_create_documents(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_create_documents(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    use tc::create_documents::NewDocument;
    let new_documents = args
        .get("new_documents")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|v| NewDocument {
                    title: v
                        .get("title")
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
    Ok(tc::Tool::CreateDocuments(tc::CreateDocuments {
        new_documents,
    }))
}
fn history_create_documents(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::CreateDocuments(c) = tool {
        Some((
            "create_documents".into(),
            json!({"new_documents": c.new_documents.len()}).to_string(),
        ))
    } else {
        None
    }
}
fn render_create_documents_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::create_documents_result::Result as Sr;
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    match &tcr.result {
        Some(R::CreateDocuments(r)) => match &r.result {
            Some(Sr::Success(s)) => {
                let mut docs = vec![];
                for d in &s.created_documents {
                    let line_range = d
                        .line_range
                        .as_ref()
                        .map(|r| json!({"start_line": r.start, "end_line": r.end}));
                    docs.push(json!({ "document_id": d.document_id, "content": d.content, "line_range": line_range }));
                }
                ToolResultEnvelope::success(
                    LocalToolId::CreateDocuments,
                    json!({ "documents": docs }),
                )
            }
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::CreateDocuments,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::CreateDocuments,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::CreateDocuments,
            "Unexpected result type".into(),
        )),
    }
}

fn edit_documents_available(context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if context.is_local_profile {
        // edit_documents writes through Warp Drive; forbidden in local-only mode.
        return ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::LocalOnlyViolation,
        };
    }
    ToolAvailability::Unavailable {
        reason: super::availability::ToolUnavailableReason::FeatureDisabled,
    }
}
fn edit_documents_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "edit_documents",
            "description": "Edit existing documents.",
            "parameters": {
                "type": "object",
                "properties": {
                    "diffs": { "type": "array", "items": { "type": "object", "properties": { "document_id": {"type": "string"}, "search": {"type": "string"}, "replace": {"type": "string"} } } }
                }
            }
        }
    })
}
fn decode_edit_documents(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_edit_documents(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    use tc::edit_documents::DocumentDiff;
    let diffs = args
        .get("diffs")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|v| DocumentDiff {
                    document_id: v
                        .get("document_id")
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
    Ok(tc::Tool::EditDocuments(tc::EditDocuments { diffs }))
}
fn history_edit_documents(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::EditDocuments(c) = tool {
        Some((
            "edit_documents".into(),
            json!({"diffs": c.diffs.len()}).to_string(),
        ))
    } else {
        None
    }
}
fn render_edit_documents_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::edit_documents_result::Result as Sr;
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    match &tcr.result {
        Some(R::EditDocuments(r)) => match &r.result {
            Some(Sr::Success(s)) => {
                let mut docs = vec![];
                for d in &s.updated_documents {
                    docs.push(json!({ "document_id": d.document_id }));
                }
                ToolResultEnvelope::success(
                    LocalToolId::EditDocuments,
                    json!({ "updated_documents": docs }),
                )
            }
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::EditDocuments,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::EditDocuments,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::EditDocuments,
            "Unexpected result type".into(),
        )),
    }
}

fn upload_file_artifact_available(context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    if context.is_local_profile {
        // upload_file_artifact uploads to Warp's backend; forbidden in local-only mode.
        return ToolAvailability::Unavailable {
            reason: super::availability::ToolUnavailableReason::LocalOnlyViolation,
        };
    }
    ToolAvailability::Unavailable {
        reason: super::availability::ToolUnavailableReason::FeatureDisabled,
    }
}
fn upload_file_artifact_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "upload_file_artifact",
            "description": "Upload a file artifact.",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string" },
                    "description": { "type": "string" }
                },
                "required": ["file_path"]
            }
        }
    })
}
fn decode_upload_file_artifact(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_upload_file_artifact(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::UploadFileArtifact(
        warp_multi_agent_api::UploadFileArtifact {
            file: Some(warp_multi_agent_api::FilePathReference {
                file_path: args
                    .get("file_path")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
            }),
            description: args
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
        },
    ))
}
fn history_upload_file_artifact(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::UploadFileArtifact(c) = tool {
        Some((
            "upload_file_artifact".into(),
            json!({"file_path": c.file.as_ref().map(|f| &f.file_path)}).to_string(),
        ))
    } else {
        None
    }
}
fn render_upload_file_artifact_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::upload_file_artifact_result::Result as Sr;
    match &tcr.result {
        Some(R::UploadFileArtifact(r)) => match &r.result {
            Some(Sr::Success(s)) => ToolResultEnvelope::success(
                LocalToolId::UploadFileArtifact,
                json!({ "artifact_id": s.artifact_uid }),
            ),
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::UploadFileArtifact,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::UploadFileArtifact,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::UploadFileArtifact,
            "Unexpected result type".into(),
        )),
    }
}
