use serde::{Deserialize, Serialize};

use super::context::AgentObservationContext;
use super::identifiers::AgentErrorId;
use crate::ai::response_experience::source::{ProviderId, ProviderModelRef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentFailurePhase {
    Grounding,
    ContextPlanning,
    CredentialAcquisition,
    ProxyDispatch,
    ProviderRequest,
    ProviderStream,
    StreamDecoding,
    ToolValidation,
    ToolPermission,
    ToolExecution,
    ToolResultProjection,
    Persistence,
    ResponseProjection,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizedErrorType {
    Authentication,
    PermissionDenied,
    InvalidRequest,
    ContextLimit,
    RateLimited,
    ProviderOverloaded,
    Transport,
    Timeout,
    StreamProtocol,
    ToolFailed,
    ToolCancelled,
    UserCancelled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestReplaySafety {
    Safe,
    Unsafe,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    RetryImmediately,
    WaitForNetwork,
    ResumePartial,
    OauthRefresh,
    ProviderFallback,
    ModelFallback,
    ToolRetry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RichTerminalOutcomeClass {
    Success,
    PartialSuccess,
    Cancelled,
    Failed,
}

pub type ErrorMessageCode = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentErrorRecord {
    pub error_id: AgentErrorId,
    pub context: AgentObservationContext,
    pub phase: AgentFailurePhase,
    pub class: NormalizedErrorType,
    pub provider_code: Option<String>,
    pub http_status: Option<u16>,
    pub provider_request_id: Option<String>,
    pub retryable: bool,
    pub replay_safety: Option<RequestReplaySafety>,
    pub user_visible: bool,
    pub recovered: bool,
    pub recovery_action: Option<RecoveryAction>,
    pub terminal_outcome: Option<RichTerminalOutcomeClass>,
    pub safe_message_code: ErrorMessageCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserVisibleFailureStage {
    Initialization,
    Generation,
    ToolExecution,
    Finalization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserVisibleErrorCategory {
    AuthenticationRequired,
    RateLimited,
    ProviderUnavailable,
    ToolPermissionDenied,
    CommandFailed,
    ContextLimitReached,
    ConnectionInterrupted,
    RequestCancelled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRecoveryAction {
    Login,
    RetryLater,
    ChangeModel,
    GrantPermission,
}

pub type SafeDiagnosticReference = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorStateProjection {
    pub stage: UserVisibleFailureStage,
    pub category: UserVisibleErrorCategory,
    pub recovered: bool,
    pub retry_count: u32,
    pub safe_summary: String,
    pub recommended_action: Option<UserRecoveryAction>,
    pub provider: Option<ProviderId>,
    pub model: Option<ProviderModelRef>,
    pub diagnostic_reference: Option<SafeDiagnosticReference>,
}
