use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentRunId(pub Uuid);

impl AgentRunId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for AgentRunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentTurnId(pub Uuid);

impl AgentTurnId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for AgentTurnId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LogicalRequestId(pub Uuid);

impl LogicalRequestId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for LogicalRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProviderAttemptId(pub Uuid);

impl ProviderAttemptId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for ProviderAttemptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(pub String);

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObservationEventId(pub Uuid);

impl ObservationEventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for ObservationEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UsageEntryId(pub Uuid);

impl UsageEntryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for UsageEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CostEntryId(pub Uuid);

impl CostEntryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for CostEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentErrorId(pub Uuid);

impl AgentErrorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for AgentErrorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToolProgressId(pub Uuid);

impl ToolProgressId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for ToolProgressId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
