use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinPolicy {
    All,
    AnySuccess,
    AnyTerminal,
    Quorum(usize),
    Explicit,
    FireAndForget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildFailurePolicy {
    FailParent,
    Continue,
    RetryChild,
    EscalateToParent,
    RequestUserInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParentClosePolicy {
    RequestCancellation,
    TerminateChildren,
    AbandonChildren,
    RejectParentCompletion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildRunEdge {
    pub parent_run_id: crate::ai::orchestration_router::ids::RunId,
    pub child_run_id: crate::ai::orchestration_router::ids::RunId,
    pub join_policy: JoinPolicy,
    pub failure_policy: ChildFailurePolicy,
    pub parent_close_policy: ParentClosePolicy,
}
