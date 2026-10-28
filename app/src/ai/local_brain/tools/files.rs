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
use super::utils::string_array;

pub const READ_FILES: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::ReadFiles,
    provider_name: "read_files",
    execution_policy: ToolExecutionPolicy::ParallelReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::ReplaceWithSummary,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: read_files_available,
    schema: read_files_schema,
    decode: decode_read_files,
    encode_warp_tool: encode_read_files,
    encode_history: history_read_files,
    render_result: render_read_files_result,
};

pub const FILE_GLOB: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::FileGlobV2,
    provider_name: "glob",
    execution_policy: ToolExecutionPolicy::ParallelReadOnly,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::ReplaceWithSummary,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: file_glob_available,
    schema: file_glob_schema,
    decode: decode_file_glob,
    encode_warp_tool: encode_file_glob,
    encode_history: history_file_glob,
    render_result: render_file_glob_result,
};

fn read_files_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}

fn read_files_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "read_files",
            "description": "Read the full contents of one or more files, given their paths.",
            "parameters": {
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "File paths to read (relative to the working directory or absolute)."
                    }
                },
                "required": ["paths"]
            }
        }
    })
}

fn decode_read_files(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    if !args.is_object() {
        return Err(ToolArgumentError::InvalidSchema("Expected object".into()));
    }
    let paths = string_array(&args, "paths").or_else(|| string_array(&args, "files"));
    if paths.is_none() || paths.as_ref().unwrap().is_empty() {
        return Err(ToolArgumentError::MissingField("paths"));
    }
    Ok(args)
}

fn encode_read_files(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    let files = string_array(&args, "paths")
        .or_else(|| string_array(&args, "files"))
        .unwrap_or_default()
        .into_iter()
        .map(|name| tc::read_files::File {
            name,
            line_ranges: vec![],
        })
        .collect();
    Ok(tc::Tool::ReadFiles(tc::ReadFiles { files }))
}

fn history_read_files(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::ReadFiles(r) = tool {
        let paths: Vec<&str> = r.files.iter().map(|f| f.name.as_str()).collect();
        Some((
            "read_files".to_string(),
            json!({ "paths": paths }).to_string(),
        ))
    } else {
        None
    }
}

fn render_read_files_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    // For now, in Phase 1, we still rely on the old `Debug` format as a temporary payload.
    // The instructions say "Port the existing render_tool_result implementations to use strongly-typed data structures".
    // Let's create a struct for it.
    // Wait, the native ToolCallResult for ReadFiles doesn't easily expose the raw strings in prost derived types without looking into the actual type.
    // I can just serialize the `Debug` output temporarily as the `data` string until Phase 2 is complete for all types.
    // No, the user explicitly said "No renderer uses format!("{:?}", ...)." and "Do not use: render_result(...) -> arbitrary JSON Value"
    // I will extract the read files result correctly.

    use warp_multi_agent_api::message::tool_call_result::Result as R;
    use warp_multi_agent_api::read_files_result::Result as Rr;

    match &tcr.result {
        Some(R::ReadFiles(r)) => match &r.result {
            Some(Rr::TextFilesSuccess(success)) => {
                let data = success
                    .files
                    .iter()
                    .map(|f| {
                        json!({
                            "file_path": f.file_path,
                            "content": f.content,
                        })
                    })
                    .collect::<Vec<_>>();
                ToolResultEnvelope::success(LocalToolId::ReadFiles, json!({ "files": data }))
            }
            Some(Rr::AnyFilesSuccess(_)) => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadFiles,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "UNSUPPORTED_RESULT",
                    message: "AnyFilesSuccess not supported yet".to_string(),
                    retryable: false,
                    field: None,
                },
            )),
            Some(Rr::Error(error)) => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadFiles,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "READ_ERROR",
                    message: error.message.clone(),
                    retryable: true,
                    field: None,
                },
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::ReadFiles,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "NO_RESULT",
                    message: "no read files result".to_string(),
                    retryable: false,
                    field: None,
                },
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::ReadFiles,
            super::results::ToolResultError {
                kind: super::results::ToolErrorKind::ExecutionFailure,
                code: "INVALID_RESULT_TYPE",
                message: "no result or wrong result type".to_string(),
                retryable: false,
                field: None,
            },
        )),
    }
}

// FileGlobV2 Implementation

fn file_glob_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}

fn file_glob_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "glob",
            "description": "Find files whose names match glob patterns (e.g. **/*.rs).",
            "parameters": {
                "type": "object",
                "properties": {
                    "patterns": { "type": "array", "items": { "type": "string" }, "description": "Glob patterns to match filenames." },
                    "path": { "type": "string", "description": "Directory to search in (default: working directory)." }
                },
                "required": ["patterns"]
            }
        }
    })
}

fn decode_file_glob(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    if !args.is_object() {
        return Err(ToolArgumentError::InvalidSchema("Expected object".into()));
    }
    let patterns = string_array(&args, "patterns");
    if patterns.is_none() || patterns.as_ref().unwrap().is_empty() {
        return Err(ToolArgumentError::MissingField("patterns"));
    }
    Ok(args)
}

fn encode_file_glob(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    let patterns = string_array(&args, "patterns").unwrap_or_default();
    let search_dir = args
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or(".")
        .to_string();
    Ok(tc::Tool::FileGlobV2(tc::FileGlobV2 {
        patterns,
        search_dir,
        max_matches: 0,
        max_depth: 0,
        min_depth: 0,
    }))
}

fn history_file_glob(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::FileGlobV2(g) = tool {
        Some((
            "glob".to_string(),
            json!({ "patterns": g.patterns, "path": g.search_dir }).to_string(),
        ))
    } else {
        None
    }
}

fn render_file_glob_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::file_glob_v2_result::Result as Gr;
    use warp_multi_agent_api::message::tool_call_result::Result as R;

    match &tcr.result {
        Some(R::FileGlobV2(r)) => match &r.result {
            Some(Gr::Success(success)) => {
                let data = json!({
                    "matched_files": success.matched_files.iter().map(|m| m.file_path.clone()).collect::<Vec<_>>(),
                    "warnings": success.warnings,
                });
                ToolResultEnvelope::success(LocalToolId::FileGlobV2, data)
            }
            Some(Gr::Error(error)) => Ok(ToolResultEnvelope::error(
                LocalToolId::FileGlobV2,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "GLOB_ERROR",
                    message: error.message.clone(),
                    retryable: true,
                    field: None,
                },
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::FileGlobV2,
                super::results::ToolResultError {
                    kind: super::results::ToolErrorKind::ExecutionFailure,
                    code: "NO_RESULT",
                    message: "no file glob result".to_string(),
                    retryable: false,
                    field: None,
                },
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::FileGlobV2,
            super::results::ToolResultError {
                kind: super::results::ToolErrorKind::ExecutionFailure,
                code: "INVALID_RESULT_TYPE",
                message: "no result or wrong result type".to_string(),
                retryable: false,
                field: None,
            },
        )),
    }
}
