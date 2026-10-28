#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodebaseCapabilityState {
    Unavailable,
    Ready,
    Failed,
}
