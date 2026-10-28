#[cfg(test)]
mod tests {
    use crate::ai::orchestration_router::command::{OrchestrationCommand, ChildTaskSpec, RuntimeKind};
    use crate::ai::orchestration_router::event::{OrchestrationEvent, RuntimeEventEnvelope};
    use crate::ai::orchestration_router::ids::{RunId, EventId, CorrelationId};
    use crate::ai::orchestration_router::model::OrchestrationRouterModel;
    use crate::ai::orchestration_router::policies::{JoinPolicy, ChildFailurePolicy, ParentClosePolicy};
    use crate::AIConversationId;
    use warpui::{ModelContext, AppContext};
    // Since we don't have an easy way to construct ModelContext in a pure unit test without the App,
    // we might test the model logic by directly calling a trait or by extracting the pure command validation logic.
    // For now, let's just write a test for the reducer and event shape.
}
