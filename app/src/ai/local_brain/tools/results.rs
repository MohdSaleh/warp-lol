use serde::Serialize;
use serde_json::Value;

use super::errors::ToolResultRenderError;
use super::registry::LocalToolId;

pub type ToolResultRenderResult = Result<ToolResultEnvelope, ToolResultRenderError>;

#[derive(Debug, Clone, Serialize)]
pub struct ToolResultEnvelope {
    pub schema_version: &'static str,
    pub tool: LocalToolId,
    pub status: ToolResultStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ToolResultError>,

    pub metadata: ToolResultMetadata,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultStatus {
    Success,
    Partial,
    Running,
    WaitingForInput,
    Denied,
    Cancelled,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolResultError {
    pub kind: ToolErrorKind,
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl From<&str> for ToolResultError {
    fn from(message: &str) -> Self {
        Self {
            kind: ToolErrorKind::ExecutionFailure,
            code: "EXECUTION_FAILURE",
            message: message.to_string(),
            retryable: false,
            field: None,
        }
    }
}

impl From<String> for ToolResultError {
    fn from(message: String) -> Self {
        Self {
            kind: ToolErrorKind::ExecutionFailure,
            code: "EXECUTION_FAILURE",
            message,
            retryable: false,
            field: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolErrorKind {
    InvalidArguments,
    Unavailable,
    PermissionDenied,
    ConfirmationRequired,
    NotFound,
    Conflict,
    Timeout,
    Cancelled,
    TransientFailure,
    ExecutionFailure,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ToolResultMetadata {
    pub truncated: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<ToolContinuation>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolContinuation {
    pub cursor: String,
    pub has_more: bool,
}

impl ToolResultEnvelope {
    pub fn success<T>(tool: LocalToolId, data: T) -> Result<Self, ToolResultRenderError>
    where
        T: Serialize,
    {
        Ok(Self {
            schema_version: "1.0",
            tool,
            status: ToolResultStatus::Success,
            data: Some(serde_json::to_value(data)?),
            error: None,
            metadata: ToolResultMetadata::default(),
        })
    }

    pub fn error(tool: LocalToolId, error: ToolResultError) -> Self {
        Self {
            schema_version: "1.0",
            tool,
            status: ToolResultStatus::Error,
            data: None,
            error: Some(error),
            metadata: ToolResultMetadata::default(),
        }
    }

    pub fn cancelled(tool: LocalToolId) -> Self {
        Self {
            schema_version: "1.0",
            tool,
            status: ToolResultStatus::Cancelled,
            data: None,
            error: Some(ToolResultError {
                kind: ToolErrorKind::Cancelled,
                code: "TOOL_CANCELLED",
                message: "The tool execution was cancelled.".to_string(),
                retryable: false,
                field: None,
            }),
            metadata: ToolResultMetadata::default(),
        }
    }
}
