pub type ProviderId = String;
pub type ProviderModelRef = String;
pub type LocalToolCallId = String;
pub type AgentId = String;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RichEventSource {
    LocalRuntime,
    CloudAgent,
    Provider {
        provider: ProviderId,
        model: ProviderModelRef,
    },
    Tool {
        tool_call_id: LocalToolCallId,
    },
    Grounding,
    ChildAgent {
        agent_id: AgentId,
    },
    SystemRecovery,
}
